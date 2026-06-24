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

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock};
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

impl std::fmt::Display for PolicyOverrides {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.fields.is_empty() {
            return write!(f, "(no overrides)");
        }
        let mut keys: Vec<&str> = self.fields.keys().map(|k| k.as_str()).collect();
        keys.sort_unstable();
        write!(f, "{} override(s): {}", keys.len(), keys.join(", "))
    }
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

    /// Validated insert: checks the key name and value type against
    /// [`PolicyConfig`] by trial-merging a single-field override onto the
    /// default config. Returns `ConfigError` on unknown keys or type
    /// mismatches so callers catch mistakes at build time rather than at the
    /// final `with_overrides` merge.
    pub fn insert_validated<V: Into<Value>>(
        mut self,
        key: &str,
        value: V,
    ) -> Result<Self, RiskEngineError> {
        let val = value.into();
        // Trial merge — exercises deny_unknown_fields + type checking.
        let trial = PolicyOverrides {
            fields: [(key.to_string(), val.clone())].into_iter().collect(),
        };
        PolicyConfig::default().with_overrides(&trial)?;
        self.fields.insert(key.to_string(), val);
        Ok(self)
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
        let merged: Self = serde_json::from_value(base).map_err(|e| {
            RiskEngineError::ConfigError(format!("apply overrides (likely a typo or type mismatch): {e}"))
        })?;
        merged.validate()?;
        // Security bounds: overrides cannot weaken policy beyond the base.
        merged.validate_security_bounds(&self)?;
        Ok(merged)
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
        let overrides = store.load_overrides(tenant_id).await.map_err(|e| {
            RiskEngineError::ConfigError(format!("tenant {tenant_id}: load overrides: {e}"))
        })?;
        base.clone().with_overrides(&overrides).map_err(|e| {
            RiskEngineError::ConfigError(format!("tenant {tenant_id}: {e}"))
        })
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
///
/// ## Thundering-herd protection
///
/// When a cached entry expires, the first request to notice promotes itself to
/// "refresher" and fetches from the inner store. Concurrent requests that arrive
/// while the refresh is in-flight receive the stale (expired) entry rather than
/// stampeding the backing store. Cold misses (no stale entry) cannot be
/// deduplicated without async coordination, so those still fan out.
const DEFAULT_MAX_TENANTS: usize = 10_000;

pub struct CachedPolicyStore<S: PolicyStore + ?Sized> {
    inner: Arc<S>,
    ttl: Duration,
    max_entries: usize,
    entries: RwLock<HashMap<String, (PolicyOverrides, Instant)>>,
    /// Tenant IDs currently being refreshed by another task. Used for
    /// stale-while-revalidate: if a refresh is already in flight, concurrent
    /// readers serve the expired entry instead of stampeding the store.
    refreshing: Mutex<HashSet<String>>,
}

impl<S: PolicyStore + ?Sized> CachedPolicyStore<S> {
    pub fn new(inner: Arc<S>, ttl: Duration) -> Self {
        Self::with_max_entries(inner, ttl, DEFAULT_MAX_TENANTS)
    }

    pub fn with_max_entries(inner: Arc<S>, ttl: Duration, max_entries: usize) -> Self {
        Self {
            inner,
            ttl,
            max_entries,
            entries: RwLock::new(HashMap::new()),
            refreshing: Mutex::new(HashSet::new()),
        }
    }

    /// Drop the cached entry for `tenant_id`. Call this from whatever channel
    /// your control plane uses to announce tenant-policy updates.
    pub fn invalidate(&self, tenant_id: &str) {
        self.entries
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(tenant_id);
    }

    /// Drop every cached entry. Intended for operator tooling, not a hot path.
    pub fn invalidate_all(&self) {
        self.entries
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
    }
}

#[async_trait]
impl<S: PolicyStore + ?Sized> PolicyStore for CachedPolicyStore<S> {
    async fn load_overrides(
        &self,
        tenant_id: &str,
    ) -> Result<PolicyOverrides, RiskEngineError> {
        // Fast path: fresh cache hit.
        if let Some((cached, inserted_at)) = self
            .entries
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(tenant_id)
            .cloned()
        {
            if inserted_at.elapsed() < self.ttl {
                return Ok(cached);
            }
            // Stale entry — try to become the sole refresher.
            let already_refreshing = {
                let mut set = self.refreshing
                    .lock()
                    .unwrap_or_else(|p| p.into_inner());
                !set.insert(tenant_id.to_string()) // false if we just added it
            };
            if already_refreshing {
                // Another task is already refreshing — serve stale.
                return Ok(cached);
            }
            // We are the refresher — fall through to fetch.
        }

        // Cold miss or we are the designated refresher for a stale entry.
        let result = self.inner.load_overrides(tenant_id).await;

        // Always clear the refreshing flag so the next miss retries.
        self.refreshing
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(tenant_id);

        let fresh = result?;
        let now = Instant::now();
        let mut map = self.entries
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if map.len() >= self.max_entries && !map.contains_key(tenant_id) {
            let ttl = self.ttl;
            map.retain(|_, (_, inserted)| now.duration_since(*inserted) < ttl);
            // Hard bound: if TTL-based eviction didn't free space (all entries
            // are still fresh), evict the oldest entry to guarantee the cache
            // never exceeds max_entries. Without this, a burst of distinct
            // tenant IDs could grow the map unboundedly.
            if map.len() >= self.max_entries {
                if let Some(oldest_key) = map
                    .iter()
                    .min_by_key(|(_, (_, ts))| *ts)
                    .map(|(k, _)| k.clone())
                {
                    map.remove(&oldest_key);
                }
            }
        }
        map.insert(tenant_id.to_string(), (fresh.clone(), now));
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
        // Value must be <= base (39) to satisfy security bounds.
        let overrides = PolicyOverrides::new()
            .insert("threshold_allow_max", 20_u8)
            .insert("version", "tenant-acme");
        let merged = base.clone().with_overrides(&overrides).unwrap();
        assert_eq!(merged.threshold_allow_max, 20);
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

    // ── I1: validate() ──────────────────────────────────────────────────────

    #[test]
    fn validate_rejects_inverted_thresholds() {
        let base = PolicyConfig::default();
        let overrides = PolicyOverrides::new()
            .insert("threshold_allow_max", 70_u8)
            .insert("threshold_challenge_max", 50_u8);
        let err = base.with_overrides(&overrides).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("threshold_allow_max"), "{msg}");
    }

    #[test]
    fn validate_rejects_negative_multiplier() {
        let mut cfg = PolicyConfig::default();
        cfg.multiplier_login = -1.0;
        let err = cfg.validate().unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("multiplier_login"), "{msg}");
    }

    #[test]
    fn validate_rejects_zero_damping_rate() {
        let mut cfg = PolicyConfig::default();
        cfg.score_damping_rate = 0.0;
        let err = cfg.validate().unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("score_damping_rate"), "{msg}");
    }

    #[test]
    fn validate_accepts_default_config() {
        PolicyConfig::default().validate().unwrap();
    }

    // ── I8: Display ─────────────────────────────────────────────────────────

    #[test]
    fn display_empty_overrides() {
        let o = PolicyOverrides::default();
        assert_eq!(format!("{o}"), "(no overrides)");
    }

    #[test]
    fn display_shows_override_keys() {
        let o = PolicyOverrides::new()
            .insert("threshold_allow_max", 10_u8)
            .insert("cap_device", 40_u8);
        let s = format!("{o}");
        assert!(s.contains("2 override(s)"), "{s}");
        assert!(s.contains("cap_device"), "{s}");
        assert!(s.contains("threshold_allow_max"), "{s}");
    }

    // ── N3: Component cap validation ────────────────────────────────────────

    #[test]
    fn validate_rejects_cap_over_100() {
        let mut cfg = PolicyConfig::default();
        cfg.cap_device = 101;
        let err = cfg.validate().unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("cap_device"), "{msg}");
        assert!(msg.contains("101"), "{msg}");
    }

    #[test]
    fn validate_accepts_cap_at_100() {
        let mut cfg = PolicyConfig::default();
        cfg.cap_device = 100;
        cfg.validate().unwrap();
    }

    #[test]
    fn validate_accepts_cap_at_zero() {
        let mut cfg = PolicyConfig::default();
        cfg.cap_device = 0;
        cfg.validate().unwrap();
    }

    #[test]
    fn validate_rejects_org_cluster_bias_over_100() {
        let mut cfg = PolicyConfig::default();
        cfg.max_org_cluster_bias = 101;
        let err = cfg.validate().unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("max_org_cluster_bias"), "{msg}");
    }

    // ── N4: insert_validated ────────────────────────────────────────────────

    #[test]
    fn insert_validated_accepts_valid_field() {
        let o = PolicyOverrides::new()
            .insert_validated("threshold_allow_max", 20_u8)
            .unwrap();
        assert_eq!(o.fields["threshold_allow_max"], serde_json::json!(20));
    }

    #[test]
    fn insert_validated_rejects_unknown_field() {
        let err = PolicyOverrides::new()
            .insert_validated("not_a_real_field", 1_u8)
            .unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("not_a_real_field"), "{msg}");
    }

    #[test]
    fn insert_validated_rejects_type_mismatch() {
        let err = PolicyOverrides::new()
            .insert_validated("threshold_allow_max", "oops")
            .unwrap_err();
        assert!(matches!(err, RiskEngineError::ConfigError(_)), "{err:?}");
    }

    #[test]
    fn insert_validated_rejects_invalid_value() {
        // threshold_allow_max = 70 would violate threshold ordering (>= challenge_max=64)
        let err = PolicyOverrides::new()
            .insert_validated("threshold_allow_max", 70_u8)
            .unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("threshold_allow_max"), "{msg}");
    }

    #[test]
    fn insert_validated_accepts_gate_config() {
        let o = PolicyOverrides::new()
            .insert_validated("gate_fresh_credential_minutes", 30_u8)
            .unwrap()
            .insert_validated("gate_block_vpn_on_critical", false)
            .unwrap();
        assert_eq!(
            o.fields["gate_fresh_credential_minutes"],
            serde_json::json!(30)
        );
        assert_eq!(
            o.fields["gate_block_vpn_on_critical"],
            serde_json::json!(false)
        );
    }

    // ── V1: Security bounds (validate_security_bounds) ──────────────────────

    #[test]
    fn security_bounds_rejects_raised_allow_threshold() {
        let base = PolicyConfig::default();
        // Attempt to raise threshold_allow_max (make Allow easier = weaker security)
        let overrides = PolicyOverrides::new()
            .insert("threshold_allow_max", (base.threshold_allow_max + 1));
        let err = base.with_overrides(&overrides).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("threshold_allow_max"), "{msg}");
        assert!(msg.contains("cannot exceed base"), "{msg}");
    }

    #[test]
    fn security_bounds_rejects_raised_hold_threshold() {
        let base = PolicyConfig::default();
        let overrides = PolicyOverrides::new()
            .insert("threshold_hold_max", (base.threshold_hold_max + 1));
        let err = base.with_overrides(&overrides).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("threshold_hold_max"), "{msg}");
    }

    #[test]
    fn security_bounds_allows_lowered_thresholds() {
        let base = PolicyConfig::default();
        // Lowering thresholds = stricter policy, always allowed
        let overrides = PolicyOverrides::new()
            .insert("threshold_allow_max", 20_u8)
            .insert("threshold_challenge_max", 50_u8)
            .insert("threshold_hold_max", 80_u8);
        let merged = base.with_overrides(&overrides).unwrap();
        assert_eq!(merged.threshold_allow_max, 20);
        assert_eq!(merged.threshold_challenge_max, 50);
        assert_eq!(merged.threshold_hold_max, 80);
    }

    #[test]
    fn security_bounds_rejects_zeroed_cap() {
        let base = PolicyConfig::default();
        let overrides = PolicyOverrides::new().insert("cap_device", 0_u8);
        let err = base.with_overrides(&overrides).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("cap_device"), "{msg}");
        assert!(msg.contains("cannot be 0"), "{msg}");
    }

    #[test]
    fn security_bounds_allows_lowered_cap() {
        let base = PolicyConfig::default();
        let overrides = PolicyOverrides::new().insert("cap_device", 1_u8);
        let merged = base.with_overrides(&overrides).unwrap();
        assert_eq!(merged.cap_device, 1);
    }

    #[test]
    fn security_bounds_allows_same_thresholds() {
        let base = PolicyConfig::default();
        // Same values as base — should pass
        let overrides = PolicyOverrides::new()
            .insert("threshold_allow_max", base.threshold_allow_max);
        base.with_overrides(&overrides).unwrap();
    }

    // ── V5: NaN/Infinity validation ─────────────────────────────────────────

    #[test]
    fn validate_rejects_nan_damping_rate() {
        let mut cfg = PolicyConfig::default();
        cfg.score_damping_rate = f32::NAN;
        let err = cfg.validate().unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("score_damping_rate"), "{msg}");
    }

    #[test]
    fn validate_rejects_infinite_multiplier() {
        let mut cfg = PolicyConfig::default();
        cfg.multiplier_login = f32::INFINITY;
        let err = cfg.validate().unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("multiplier_login"), "{msg}");
    }

    #[test]
    fn validate_rejects_nan_multiplier() {
        let mut cfg = PolicyConfig::default();
        cfg.multiplier_login = f32::NAN;
        let err = cfg.validate().unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("multiplier_login"), "{msg}");
    }

    // ── I1: Threshold ceiling ───────────────────────────────────────────────

    #[test]
    fn validate_rejects_hold_max_over_100() {
        let mut cfg = PolicyConfig::default();
        cfg.threshold_hold_max = 101;
        let err = cfg.validate().unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("threshold_hold_max"), "{msg}");
        assert!(msg.contains("100"), "{msg}");
    }
}
