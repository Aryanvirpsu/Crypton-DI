//! Per-tenant policy overrides (Phase 6).
//!
//! A [`PolicyOverrides`] is a sparse patch over [`PolicyConfig`]: any field
//! mentioned in the overrides replaces the base value, everything else falls
//! through. Storage is an opaque JSON object of `{field_name: value}` pairs so
//! new tuning knobs added to `PolicyConfig` work with existing tenant records
//! without a migration.
//!
//! ```text
//! base (PolicyConfig::default)
//!    ↓ merge
//! tenant_overrides (from Redis)
//!    ↓ =
//! effective_policy  →  Arc<PolicyConfig>  →  RiskContext.policy
//! ```
//!
//! ## Storage shape
//!
//! The concrete store is pluggable via [`PolicyStore`]. The provided Fred
//! adapter ([`crate::adapters::fred_store::FredPolicyStore`]) reads a single
//! Redis hash field at `policy:{tenant_id}` with key `config`, holding the
//! JSON blob. Keeping the payload in one field avoids coordinating 100
//! separate HSET operations and still lets the caller swap in a per-field
//! scheme later without changing [`PolicyStore`].
//!
//! ## Caching
//!
//! [`CachedPolicyStore`] wraps any [`PolicyStore`] with a per-tenant TTL
//! cache. On miss or expiry it hits the inner store; hits serve from memory.
//! The cache is cleared via [`CachedPolicyStore::invalidate`] — call it when
//! your control plane publishes a tenant-update event so the next request
//! refetches instead of waiting for TTL.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::RiskEngineError;
use crate::policy_config::PolicyConfig;

// ─────────────────────────────────────────────────────────────────────────────
// PolicyOverrides
// ─────────────────────────────────────────────────────────────────────────────

/// Sparse map of `PolicyConfig` field name → override value.
///
/// Unknown keys error when applied (via the final `serde_json::from_value`
/// round-trip), which surfaces typos at load time rather than silently
/// ignoring them.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PolicyOverrides {
    pub fields: HashMap<String, Value>,
}

impl PolicyOverrides {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Convenience constructor for tests and programmatic overrides.
    pub fn insert<V: Into<Value>>(mut self, key: &str, value: V) -> Self {
        self.fields.insert(key.to_string(), value.into());
        self
    }

    /// Parse from a JSON string (the shape stored in Redis).
    pub fn from_json(s: &str) -> Result<Self, RiskEngineError> {
        serde_json::from_str(s)
            .map_err(|e| RiskEngineError::ConfigError(format!("parse overrides JSON: {e}")))
    }

    pub fn to_json(&self) -> Result<String, RiskEngineError> {
        serde_json::to_string(self)
            .map_err(|e| RiskEngineError::ConfigError(format!("serialize overrides JSON: {e}")))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PolicyConfig extensions
// ─────────────────────────────────────────────────────────────────────────────

impl PolicyConfig {
    /// Overlay `overrides` onto this config and return the merged result.
    ///
    /// Merge is performed at the JSON layer: the base is serialized, override
    /// keys replace matching base keys, and the result is deserialized back.
    /// This keeps the merge O(fields) without hand-maintaining a parallel
    /// Option-typed struct, and catches typos because the final deserialize
    /// will reject unknown values for strongly-typed fields.
    pub fn with_overrides(self, overrides: &PolicyOverrides) -> Result<Self, RiskEngineError> {
        if overrides.is_empty() {
            return Ok(self);
        }
        let mut base = serde_json::to_value(&self)
            .map_err(|e| RiskEngineError::ConfigError(format!("serialize base policy: {e}")))?;
        let obj = base
            .as_object_mut()
            .ok_or_else(|| RiskEngineError::ConfigError("policy did not serialize to object".into()))?;
        for (k, v) in &overrides.fields {
            obj.insert(k.clone(), v.clone());
        }
        serde_json::from_value(base).map_err(|e| {
            RiskEngineError::ConfigError(format!("apply overrides (likely a typo or type mismatch): {e}"))
        })
    }

    /// Fetch tenant overrides from `store`, merge them over `base`, and return
    /// the resulting policy ready to wrap in an `Arc`. On store failure or
    /// parse error the caller decides whether to fall back to `base` — this
    /// function propagates the error so policy-load failures are visible.
    pub async fn for_tenant<S: PolicyStore + ?Sized>(
        base: &PolicyConfig,
        store: &S,
        tenant_id: &str,
    ) -> Result<PolicyConfig, RiskEngineError> {
        let overrides = store.load_overrides(tenant_id).await?;
        base.clone().with_overrides(&overrides)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PolicyStore trait
// ─────────────────────────────────────────────────────────────────────────────

/// Source of per-tenant policy overrides. Impls typically hit Redis, Postgres,
/// or a config service.
#[async_trait]
pub trait PolicyStore: Send + Sync {
    /// Return the overrides for `tenant_id`. An empty [`PolicyOverrides`]
    /// means the tenant uses the defaults — this MUST NOT return an error in
    /// that common case.
    async fn load_overrides(
        &self,
        tenant_id: &str,
    ) -> Result<PolicyOverrides, RiskEngineError>;
}

// ─────────────────────────────────────────────────────────────────────────────
// CachedPolicyStore — TTL cache in front of any PolicyStore
// ─────────────────────────────────────────────────────────────────────────────

/// In-process TTL cache wrapper. Serves hits from memory and only calls the
/// inner store on miss or expiry.
///
/// The cache is an `RwLock<HashMap>` — fine for the expected cardinality (one
/// entry per active tenant). For very high tenant counts, swap to a bounded
/// LRU without changing the public API.
pub struct CachedPolicyStore<S: PolicyStore + ?Sized> {
    inner: Arc<S>,
    ttl: Duration,
    entries: RwLock<HashMap<String, (PolicyOverrides, Instant)>>,
}

impl<S: PolicyStore + ?Sized> CachedPolicyStore<S> {
    pub fn new(inner: Arc<S>, ttl: Duration) -> Self {
        Self {
            inner,
            ttl,
            entries: RwLock::new(HashMap::new()),
        }
    }

    /// Drop the cached entry for `tenant_id`. Call this from whatever channel
    /// your control plane uses to announce tenant-policy updates.
    pub fn invalidate(&self, tenant_id: &str) {
        self.entries.write().unwrap().remove(tenant_id);
    }

    /// Drop every cached entry. Intended for operator tooling, not a hot path.
    pub fn invalidate_all(&self) {
        self.entries.write().unwrap().clear();
    }
}

#[async_trait]
impl<S: PolicyStore + ?Sized> PolicyStore for CachedPolicyStore<S> {
    async fn load_overrides(
        &self,
        tenant_id: &str,
    ) -> Result<PolicyOverrides, RiskEngineError> {
        if let Some((cached, inserted_at)) = self.entries.read().unwrap().get(tenant_id).cloned() {
            if inserted_at.elapsed() < self.ttl {
                return Ok(cached);
            }
        }

        let fresh = self.inner.load_overrides(tenant_id).await?;
        self.entries
            .write()
            .unwrap()
            .insert(tenant_id.to_string(), (fresh.clone(), Instant::now()));
        Ok(fresh)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn empty_overrides_return_base_unchanged() {
        let base = PolicyConfig::default();
        let overrides = PolicyOverrides::default();
        let merged = base.clone().with_overrides(&overrides).unwrap();
        assert_eq!(base, merged);
    }

    #[test]
    fn overrides_replace_named_fields() {
        let base = PolicyConfig::default();
        let overrides = PolicyOverrides::new()
            .insert("threshold_allow_max", 42_u8)
            .insert("version", "tenant-acme");
        let merged = base.clone().with_overrides(&overrides).unwrap();
        assert_eq!(merged.threshold_allow_max, 42);
        assert_eq!(merged.version, "tenant-acme");
        // Untouched field still matches base.
        assert_eq!(merged.cap_device, base.cap_device);
    }

    #[test]
    fn unknown_key_surfaces_as_error() {
        let base = PolicyConfig::default();
        let overrides = PolicyOverrides::new().insert("not_a_real_field", 1_u8);
        // PolicyConfig carries #[serde(deny_unknown_fields)], so a typo in an
        // override key must surface as a ConfigError instead of silently dropping.
        let err = base.with_overrides(&overrides).unwrap_err();
        assert!(matches!(err, RiskEngineError::ConfigError(_)), "{err:?}");
        let msg = format!("{err}");
        assert!(
            msg.contains("not_a_real_field"),
            "error should name the offending field: {msg}"
        );
    }

    #[test]
    fn type_mismatch_surfaces_as_error() {
        let base = PolicyConfig::default();
        // threshold_allow_max is u8 — a string override must fail.
        let overrides = PolicyOverrides::new().insert("threshold_allow_max", "oops");
        let err = base.with_overrides(&overrides).unwrap_err();
        assert!(matches!(err, RiskEngineError::ConfigError(_)), "{err:?}");
    }

    // ── CachedPolicyStore ────────────────────────────────────────────────────

    struct CountingStore {
        hits: AtomicU32,
        payload: PolicyOverrides,
    }

    #[async_trait]
    impl PolicyStore for CountingStore {
        async fn load_overrides(
            &self,
            _tenant_id: &str,
        ) -> Result<PolicyOverrides, RiskEngineError> {
            self.hits.fetch_add(1, Ordering::SeqCst);
            Ok(self.payload.clone())
        }
    }

    #[tokio::test]
    async fn cache_serves_hits_from_memory() {
        let inner = Arc::new(CountingStore {
            hits: AtomicU32::new(0),
            payload: PolicyOverrides::new().insert("threshold_allow_max", 33_u8),
        });
        let cached = CachedPolicyStore::new(inner.clone(), Duration::from_secs(60));

        let _ = cached.load_overrides("t-1").await.unwrap();
        let _ = cached.load_overrides("t-1").await.unwrap();
        let _ = cached.load_overrides("t-1").await.unwrap();

        assert_eq!(inner.hits.load(Ordering::SeqCst), 1, "should only miss once");
    }

    #[tokio::test]
    async fn invalidate_forces_refetch() {
        let inner = Arc::new(CountingStore {
            hits: AtomicU32::new(0),
            payload: PolicyOverrides::default(),
        });
        let cached = CachedPolicyStore::new(inner.clone(), Duration::from_secs(60));

        let _ = cached.load_overrides("t-1").await.unwrap();
        cached.invalidate("t-1");
        let _ = cached.load_overrides("t-1").await.unwrap();

        assert_eq!(inner.hits.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn for_tenant_merges_and_caches() {
        let inner = Arc::new(CountingStore {
            hits: AtomicU32::new(0),
            payload: PolicyOverrides::new().insert("threshold_allow_max", 10_u8),
        });
        let cached = CachedPolicyStore::new(inner, Duration::from_secs(60));

        let base = PolicyConfig::default();
        let effective = PolicyConfig::for_tenant(&base, &cached, "acme").await.unwrap();
        assert_eq!(effective.threshold_allow_max, 10);
    }
}
