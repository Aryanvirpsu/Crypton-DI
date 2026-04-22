//! Fred-backed implementation of [`SignalStore`].
//!
//! Built against the [`fred`] Redis/Valkey client. The adapter owns a
//! [`fred::clients::Client`] and drives every trait method through standard
//! Redis commands, with the one exception of
//! [`FredSignalStore::check_and_record_escalation`], which uses a Lua script
//! so the snapshot-read + push + trim runs as one atomic step.
//!
//! This module is gated behind `--features fred-store` and adds zero bytes to
//! the default build.

use std::collections::HashMap;
use std::future::Future;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use fred::clients::Client;
use fred::interfaces::{HashesInterface, KeysInterface, ListInterface, LuaInterface};
use fred::types::{Expiration, SetOptions};
use uuid::Uuid;

use crate::decision::Decision;
use crate::engine::escalation::{check_escalation, EscalationOutcome};
use crate::error::RiskEngineError;
use crate::org_graph::{
    recompute_tenant, ClusterMembership, GraphEdge, OrgGraphUpdate, OrgRiskSnapshot,
    RecomputeInputs, TenantRiskProfile,
};
use crate::policy_overrides::{PolicyOverrides, PolicyStore};
use crate::store::{
    EscalationEntry, IncrementStore, OrgGraphStore, SignalStore, VelocityCounters,
    NONCE_TTL_SECONDS,
};

// ─────────────────────────────────────────────────────────────────────────────
// Time budgets
//
// Every method wraps its Redis call in a `tokio::time::timeout`. These are the
// defaults from the Phase 3 plan: 20ms for simple reads and writes, 50ms for
// Lua scripts (which do more work server-side). On timeout we bubble a
// `RiskEngineError::Timeout`; the engine catches that and flips
// `redis_signals_degraded = true` so the degraded-signal policy kicks in.
// ─────────────────────────────────────────────────────────────────────────────
const READ_TIMEOUT_MS: u64 = 20;
const WRITE_TIMEOUT_MS: u64 = 20;
const LUA_TIMEOUT_MS: u64 = 50;

/// Background recompute is allowed more headroom because it reads the full
/// edge set, runs union-find + classification, and writes everything back.
/// A per-tenant cycle that exceeds this timeout means the tenant's graph has
/// outgrown the single-node worker design; shard tenants across workers.
const RECOMPUTE_TIMEOUT_MS: u64 = 10_000;

/// TTL for `org:risk:{tenant}` — the snapshot the engine reads. Set to 3×
/// the default worker recompute interval (300s) so a single missed cycle is
/// not immediately fatal to per-request bias.
const TTL_ORG_RISK: i64 = 900;

/// TTL for `org:tenant:{tenant}` — the TenantRiskProfile. Lives longer than
/// the snapshot so `prev_shift` inertia survives brief worker outages.
const TTL_ORG_TENANT: i64 = 7_200;

/// TTL for `org:graph:{tenant}:edges` — the persisted edge hash. Matches the
/// 24h time-decay window after which every edge is prunable anyway.
const TTL_ORG_EDGES: i64 = TTL_24H;

/// TTL for per-node membership keys. Short enough that a node dropping out
/// of a cluster stops contributing cluster_bias within one refresh window.
const TTL_ORG_MEMBERSHIP: i64 = 900;

async fn with_timeout<T, F>(ms: u64, fut: F) -> Result<T, RiskEngineError>
where
    F: Future<Output = Result<T, RiskEngineError>>,
{
    match tokio::time::timeout(Duration::from_millis(ms), fut).await {
        Ok(r) => r,
        Err(_) => Err(RiskEngineError::Timeout { ms }),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Key helpers — kept private so key layout stays consistent with the Redis
// ops-level documentation in src/store.rs.
// ─────────────────────────────────────────────────────────────────────────────

fn k_velocity_login_1m(uid: Uuid) -> String { format!("velocity:login:{uid}:1m") }
fn k_velocity_login_5m(uid: Uuid) -> String { format!("velocity:login:{uid}:5m") }
fn k_velocity_login_1h(uid: Uuid) -> String { format!("velocity:login:{uid}:1h") }
fn k_velocity_login_24h(uid: Uuid) -> String { format!("velocity:login:{uid}:24h") }
fn k_velocity_failed(uid: Uuid) -> String { format!("velocity:failed:{uid}:1h") }
fn k_velocity_actions(uid: Uuid) -> String { format!("velocity:actions:{uid}:5m") }
fn k_velocity_recovery(uid: Uuid) -> String { format!("velocity:recovery:{uid}:24h") }
fn k_velocity_revocations(uid: Uuid) -> String { format!("velocity:revocations:{uid}:1h") }
fn k_velocity_registrations(ip: IpAddr) -> String { format!("velocity:registrations:{ip}:10m") }
fn k_active_sessions(uid: Uuid) -> String { format!("sessions:active:{uid}") }
fn k_locked(uid: Uuid) -> String { format!("login:locked:{uid}") }
fn k_recovery_pending(uid: Uuid) -> String { format!("recovery:pending:{uid}") }
fn k_ip_blocked(ip: IpAddr) -> String { format!("ip:blocked:{ip}") }
fn k_risk_recent(uid: Uuid) -> String { format!("risk:recent:{uid}") }
fn k_jwt_fingerprint(key: &str) -> String { format!("jwt:fingerprint:{key}") }
fn k_nonce(nonce: &str) -> String { format!("nonce:{nonce}") }

// ── Velocity-window TTLs (seconds) ──────────────────────────────────────────
// Match the key's window so the counter naturally resets after that window.
const TTL_1M: i64 = 60;
const TTL_5M: i64 = 300;
const TTL_10M: i64 = 600;
const TTL_1H: i64 = 3600;
const TTL_24H: i64 = 86_400;

// ── Org-graph keys ──────────────────────────────────────────────────────────
fn k_org_risk(tenant: &str) -> String { format!("org:risk:{tenant}") }
fn k_org_membership(tenant: &str, node_key: &str) -> String {
    format!("org:membership:{tenant}:{node_key}")
}
fn k_org_tenant(tenant: &str) -> String { format!("org:tenant:{tenant}") }
fn k_org_updates(tenant: &str) -> String { format!("org:updates:{tenant}") }
fn k_org_stats(tenant: &str) -> String { format!("org:meta:{tenant}:stats") }
fn k_org_edges(tenant: &str) -> String { format!("org:graph:{tenant}:edges") }

// ─────────────────────────────────────────────────────────────────────────────
// Lua script: atomic snapshot + push + trim.
//
// Contract:
//   KEYS[1] = risk:recent:{uid}
//   ARGV[1] = entry JSON
//   ARGV[2] = max-entries (e.g. "10")
//   ARGV[3] = TTL seconds (e.g. "300")
// Returns: array of pre-push entries (newest first, matching LRANGE 0 -1 order)
//
// By returning the pre-push snapshot, the caller can feed it into
// `check_escalation` without re-reading from Redis, so the decision and the
// record-write collapse into a single round-trip and the snapshot corresponds
// exactly to the state Redis observed just before this push.
// ─────────────────────────────────────────────────────────────────────────────
const ESCALATION_LUA: &str = r#"
local snapshot = redis.call('LRANGE', KEYS[1], 0, -1)
redis.call('LPUSH', KEYS[1], ARGV[1])
redis.call('LTRIM', KEYS[1], 0, tonumber(ARGV[2]) - 1)
redis.call('EXPIRE', KEYS[1], tonumber(ARGV[3]))
return snapshot
"#;

// ─────────────────────────────────────────────────────────────────────────────
// Lua script: atomic drain of the updates list.
//
//   KEYS[1] = org:updates:{tenant}
// Returns: every element that was in the list (newest first, LRANGE order)
//
// Atomic because Redis runs the whole script without interleaving other
// commands — no update can be pushed between LRANGE and DEL and subsequently
// lost.
// ─────────────────────────────────────────────────────────────────────────────
const UPDATES_DRAIN_LUA: &str = r#"
local items = redis.call('LRANGE', KEYS[1], 0, -1)
redis.call('DEL', KEYS[1])
return items
"#;

// ─────────────────────────────────────────────────────────────────────────────
// FredSignalStore
// ─────────────────────────────────────────────────────────────────────────────

/// A [`SignalStore`] backed by a connected Fred [`Client`].
///
/// The client is `Arc`-wrapped so one `FredSignalStore` can be cloned cheaply
/// across axum request handlers.
#[derive(Clone)]
pub struct FredSignalStore {
    client: Arc<Client>,
}

impl FredSignalStore {
    /// Wrap an already-initialised Fred client. The caller is responsible for
    /// calling [`Client::init`] before handing it over.
    pub fn new(client: Client) -> Self {
        Self { client: Arc::new(client) }
    }

    /// Access the underlying client — handy for pre-incrementing velocity
    /// counters via [`IncrementStore`]-shaped helpers in the calling service.
    pub fn client(&self) -> &Client {
        &self.client
    }
}

fn to_engine_err(e: fred::error::Error) -> RiskEngineError {
    // Source-preserving wrap: the Fred error is walkable from
    // `tracing::error!(error = ?e)` without us having to stringify it.
    RiskEngineError::store_with_source("redis operation failed", e)
}

#[async_trait]
impl SignalStore for FredSignalStore {
    async fn get_velocity_counters(
        &self,
        user_id: Uuid,
        request_ip: Option<IpAddr>,
    ) -> Result<VelocityCounters, RiskEngineError> {
        with_timeout(READ_TIMEOUT_MS, async move {
            let pipe = self.client.pipeline();

            let _: () = pipe.get(k_velocity_login_1m(user_id)).await.map_err(to_engine_err)?;
            let _: () = pipe.get(k_velocity_login_5m(user_id)).await.map_err(to_engine_err)?;
            let _: () = pipe.get(k_velocity_login_1h(user_id)).await.map_err(to_engine_err)?;
            let _: () = pipe.get(k_velocity_login_24h(user_id)).await.map_err(to_engine_err)?;
            let _: () = pipe.get(k_velocity_failed(user_id)).await.map_err(to_engine_err)?;
            let _: () = pipe.get(k_velocity_actions(user_id)).await.map_err(to_engine_err)?;
            let _: () = pipe.get(k_velocity_recovery(user_id)).await.map_err(to_engine_err)?;
            let _: () = pipe.get(k_velocity_revocations(user_id)).await.map_err(to_engine_err)?;
            let _: () = pipe.get(k_active_sessions(user_id)).await.map_err(to_engine_err)?;
            let _: () = pipe.exists(k_locked(user_id)).await.map_err(to_engine_err)?;
            let _: () = pipe.exists(k_recovery_pending(user_id)).await.map_err(to_engine_err)?;

            let results: Vec<Option<String>> = pipe.all().await.map_err(to_engine_err)?;

            let parse_u32 = |v: &Option<String>| v.as_ref().and_then(|s| s.parse::<u32>().ok());
            let mut c = VelocityCounters {
                login_attempts_1m: parse_u32(&results[0]),
                login_attempts_5m: parse_u32(&results[1]),
                login_attempts_1h: parse_u32(&results[2]),
                login_attempts_24h: parse_u32(&results[3]),
                failed_login_attempts_1h: parse_u32(&results[4]),
                actions_executed_5m: parse_u32(&results[5]),
                recovery_requests_24h: parse_u32(&results[6]),
                device_revocations_1h: parse_u32(&results[7]),
                active_session_count: parse_u32(&results[8]),
                account_locked: results.get(9).and_then(|v| v.as_ref()).is_some_and(|s| s != "0"),
                recovery_pending: results.get(10).and_then(|v| v.as_ref()).is_some_and(|s| s != "0"),
                registrations_from_ip_10m: None,
            };

            if let Some(ip) = request_ip {
                let v: Option<String> = self
                    .client
                    .get(k_velocity_registrations(ip))
                    .await
                    .map_err(to_engine_err)?;
                c.registrations_from_ip_10m = v.and_then(|s| s.parse::<u32>().ok());
            }

            Ok(c)
        })
        .await
    }

    async fn get_recent_decisions(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<EscalationEntry>, RiskEngineError> {
        with_timeout(READ_TIMEOUT_MS, async move {
            let raw: Vec<String> = self
                .client
                .lrange(k_risk_recent(user_id), 0, -1)
                .await
                .map_err(to_engine_err)?;
            Ok(decode_entries(&raw))
        })
        .await
    }

    async fn record_decision(
        &self,
        user_id: Uuid,
        entry: EscalationEntry,
    ) -> Result<(), RiskEngineError> {
        with_timeout(WRITE_TIMEOUT_MS, async move {
            let key = k_risk_recent(user_id);
            let payload = serde_json::to_string(&entry)
                .map_err(|e| RiskEngineError::store_with_source("serialize entry", e))?;
            let _: () = self
                .client
                .lpush(&key, payload)
                .await
                .map_err(to_engine_err)?;
            let _: () = self.client.ltrim(&key, 0, 9).await.map_err(to_engine_err)?;
            let _: () = self
                .client
                .expire(&key, NONCE_TTL_SECONDS as i64, None)
                .await
                .map_err(to_engine_err)?;
            Ok(())
        })
        .await
    }

    async fn check_and_record_escalation(
        &self,
        user_id: Uuid,
        current_decision: &Decision,
        score: u8,
        ts_unix: i64,
    ) -> Result<(Decision, bool), RiskEngineError> {
        with_timeout(LUA_TIMEOUT_MS, async move {
            let key = k_risk_recent(user_id);
            let entry = EscalationEntry {
                score,
                decision: current_decision.clone(),
                ts_unix,
            };
            let payload = serde_json::to_string(&entry)
                .map_err(|e| RiskEngineError::store_with_source("serialize entry", e))?;

            let raw: Vec<String> = self
                .client
                .eval(
                    ESCALATION_LUA,
                    vec![key],
                    vec![payload, "10".into(), NONCE_TTL_SECONDS.to_string()],
                )
                .await
                .map_err(to_engine_err)?;

            let snapshot = decode_entries(&raw);
            let outcome = check_escalation(current_decision, &snapshot);
            Ok(match outcome {
                EscalationOutcome::EscalateTo(d) => (d, true),
                EscalationOutcome::NoChange => (current_decision.clone(), false),
            })
        })
        .await
    }

    async fn lock_account(
        &self,
        user_id: Uuid,
        ttl_seconds: u64,
    ) -> Result<(), RiskEngineError> {
        with_timeout(WRITE_TIMEOUT_MS, async move {
            let _: () = self
                .client
                .set(
                    k_locked(user_id),
                    "1",
                    Some(Expiration::EX(ttl_seconds as i64)),
                    Some(SetOptions::NX),
                    false,
                )
                .await
                .map_err(to_engine_err)?;
            Ok(())
        })
        .await
    }

    async fn block_ip(
        &self,
        ip: IpAddr,
        ttl_seconds: u64,
    ) -> Result<(), RiskEngineError> {
        with_timeout(WRITE_TIMEOUT_MS, async move {
            let _: () = self
                .client
                .set(
                    k_ip_blocked(ip),
                    "1",
                    Some(Expiration::EX(ttl_seconds as i64)),
                    Some(SetOptions::NX),
                    false,
                )
                .await
                .map_err(to_engine_err)?;
            Ok(())
        })
        .await
    }

    async fn get_jwt_fingerprint(
        &self,
        fingerprint_key: &str,
    ) -> Result<Option<String>, RiskEngineError> {
        with_timeout(READ_TIMEOUT_MS, async move {
            let v: Option<String> = self
                .client
                .get(k_jwt_fingerprint(fingerprint_key))
                .await
                .map_err(to_engine_err)?;
            Ok(v)
        })
        .await
    }

    async fn set_jwt_fingerprint(
        &self,
        fingerprint_key: &str,
        fingerprint_value: &str,
        ttl_seconds: u64,
    ) -> Result<(), RiskEngineError> {
        with_timeout(WRITE_TIMEOUT_MS, async move {
            let _: () = self
                .client
                .set(
                    k_jwt_fingerprint(fingerprint_key),
                    fingerprint_value,
                    Some(Expiration::EX(ttl_seconds as i64)),
                    None,
                    false,
                )
                .await
                .map_err(to_engine_err)?;
            Ok(())
        })
        .await
    }

    async fn is_nonce_used(&self, nonce: &str) -> Result<bool, RiskEngineError> {
        with_timeout(READ_TIMEOUT_MS, async move {
            let exists: i64 = self
                .client
                .exists(k_nonce(nonce))
                .await
                .map_err(to_engine_err)?;
            Ok(exists > 0)
        })
        .await
    }

    async fn consume_nonce(&self, nonce: &str) -> Result<(), RiskEngineError> {
        with_timeout(WRITE_TIMEOUT_MS, async move {
            let _: () = self
                .client
                .set(
                    k_nonce(nonce),
                    "1",
                    Some(Expiration::EX(NONCE_TTL_SECONDS as i64)),
                    Some(SetOptions::NX),
                    false,
                )
                .await
                .map_err(to_engine_err)?;
            Ok(())
        })
        .await
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Inherent methods — extras beyond the SignalStore trait.
// ─────────────────────────────────────────────────────────────────────────────

impl FredSignalStore {
    /// Atomic "consume if unused" for nonces.
    ///
    /// Returns `true` when the nonce was freshly consumed, `false` when it was
    /// already used. Prefer this over `is_nonce_used()` + `consume_nonce()` —
    /// the two-call sequence is a TOCTOU race. Implemented via `SET … NX EX`,
    /// which is a single atomic Redis op.
    pub async fn try_consume_nonce_atomic(
        &self,
        nonce: &str,
    ) -> Result<bool, RiskEngineError> {
        with_timeout(WRITE_TIMEOUT_MS, async move {
            let prev: Option<String> = self
                .client
                .set(
                    k_nonce(nonce),
                    "1",
                    Some(Expiration::EX(NONCE_TTL_SECONDS as i64)),
                    Some(SetOptions::NX),
                    true, // GET — return the old value, which is None when NX succeeds
                )
                .await
                .map_err(to_engine_err)?;
            Ok(prev.is_none())
        })
        .await
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// IncrementStore — write-side counter increments called by the adapter
// *before* (pre-count) or *after* (fire-and-forget) the engine decision.
//
// Each INCR is paired with an EXPIRE so the key resets naturally at the window
// boundary. We use `set_options` with `NX` on the EXPIRE? — no, we just call
// EXPIRE unconditionally; Redis refreshes the TTL, which keeps the window
// rolling. For sharper semantics (true fixed windows) a Lua script can reset
// the counter on first increment; defer that to Phase 7 if hot-window accuracy
// matters.
// ─────────────────────────────────────────────────────────────────────────────

async fn incr_with_ttl(
    client: &Client,
    key: String,
    ttl: i64,
) -> Result<(), RiskEngineError> {
    with_timeout(WRITE_TIMEOUT_MS, async move {
        let _: i64 = client.incr(&key).await.map_err(to_engine_err)?;
        let _: () = client.expire(&key, ttl, None).await.map_err(to_engine_err)?;
        Ok(())
    })
    .await
}

#[async_trait]
impl IncrementStore for FredSignalStore {
    async fn increment_login_attempts(&self, user_id: Uuid) -> Result<(), RiskEngineError> {
        // Bump every overlapping window so all of them reflect this attempt.
        let client = self.client.clone();
        incr_with_ttl(&client, k_velocity_login_1m(user_id), TTL_1M).await?;
        incr_with_ttl(&client, k_velocity_login_5m(user_id), TTL_5M).await?;
        incr_with_ttl(&client, k_velocity_login_1h(user_id), TTL_1H).await?;
        incr_with_ttl(&client, k_velocity_login_24h(user_id), TTL_24H).await?;
        Ok(())
    }

    async fn increment_failed_attempts(&self, user_id: Uuid) -> Result<(), RiskEngineError> {
        incr_with_ttl(&self.client, k_velocity_failed(user_id), TTL_1H).await
    }

    async fn increment_actions_executed(&self, user_id: Uuid) -> Result<(), RiskEngineError> {
        incr_with_ttl(&self.client, k_velocity_actions(user_id), TTL_5M).await
    }

    async fn increment_recovery_requests(&self, user_id: Uuid) -> Result<(), RiskEngineError> {
        incr_with_ttl(&self.client, k_velocity_recovery(user_id), TTL_24H).await
    }

    async fn increment_device_revocations(&self, user_id: Uuid) -> Result<(), RiskEngineError> {
        incr_with_ttl(&self.client, k_velocity_revocations(user_id), TTL_1H).await
    }

    async fn increment_registrations_from_ip(
        &self,
        ip: IpAddr,
    ) -> Result<(), RiskEngineError> {
        incr_with_ttl(&self.client, k_velocity_registrations(ip), TTL_10M).await
    }

    async fn increment_active_sessions(&self, user_id: Uuid) -> Result<(), RiskEngineError> {
        // Active-session count is *not* windowed — sessions live until they're
        // decremented on logout/expiry. No EXPIRE set.
        with_timeout(WRITE_TIMEOUT_MS, async move {
            let _: i64 = self
                .client
                .incr(k_active_sessions(user_id))
                .await
                .map_err(to_engine_err)?;
            Ok(())
        })
        .await
    }

    async fn decrement_active_sessions(&self, user_id: Uuid) -> Result<(), RiskEngineError> {
        with_timeout(WRITE_TIMEOUT_MS, async move {
            let _: i64 = self
                .client
                .decr(k_active_sessions(user_id))
                .await
                .map_err(to_engine_err)?;
            Ok(())
        })
        .await
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// FredOrgGraphStore — reference implementation of OrgGraphStore.
//
// Shares the same Fred client, but lives in its own struct because the
// engine core only needs SignalStore on the hot path, and an adapter may
// want to inject a different (or no) OrgGraphStore.
//
// Key schema:
//   org:risk:{tenant}                         GET / SET                (JSON)
//   org:membership:{tenant}:{node_key}        GET / SET                (JSON)
//   org:tenant:{tenant}                       GET / SET                (JSON)
//   org:updates:{tenant}                      LPUSH + LTRIM            (JSON list)
//   org:meta:{tenant}:stats                   HINCRBY {blocked,total}  (hash)
//
// `recompute_org_graph` and `prune_graph` are deliberately no-ops with a
// tracing warn: they need the full graph/union-find logic from
// [`crate::org_graph`] to run end-to-end. A production deployment wires a
// scheduled job to rebuild `org:risk:{tenant}` on its own cadence; this
// reference impl does not carry that responsibility.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct FredOrgGraphStore {
    client: Arc<Client>,
}

impl FredOrgGraphStore {
    pub fn new(client: Client) -> Self {
        Self { client: Arc::new(client) }
    }

    /// Share the same underlying Arc<Client> as a SignalStore. Useful for
    /// building both stores from a single initialised client.
    pub fn from_signal_store(store: &FredSignalStore) -> Self {
        Self { client: store.client.clone() }
    }

    pub fn client(&self) -> &Client {
        &self.client
    }
}

#[async_trait]
impl OrgGraphStore for FredOrgGraphStore {
    async fn get_org_risk_snapshot(
        &self,
        tenant_id: &str,
    ) -> Result<Option<OrgRiskSnapshot>, RiskEngineError> {
        with_timeout(READ_TIMEOUT_MS, async move {
            let raw: Option<String> = self
                .client
                .get(k_org_risk(tenant_id))
                .await
                .map_err(to_engine_err)?;
            Ok(raw.and_then(|s| serde_json::from_str(&s).ok()))
        })
        .await
    }

    async fn get_cluster_membership(
        &self,
        tenant_id: &str,
        node_key: &str,
    ) -> Result<Option<ClusterMembership>, RiskEngineError> {
        with_timeout(READ_TIMEOUT_MS, async move {
            let raw: Option<String> = self
                .client
                .get(k_org_membership(tenant_id, node_key))
                .await
                .map_err(to_engine_err)?;
            Ok(raw.and_then(|s| serde_json::from_str(&s).ok()))
        })
        .await
    }

    async fn get_tenant_profile(
        &self,
        tenant_id: &str,
    ) -> Result<Option<TenantRiskProfile>, RiskEngineError> {
        with_timeout(READ_TIMEOUT_MS, async move {
            let raw: Option<String> = self
                .client
                .get(k_org_tenant(tenant_id))
                .await
                .map_err(to_engine_err)?;
            Ok(raw.and_then(|s| serde_json::from_str(&s).ok()))
        })
        .await
    }

    async fn push_graph_update(
        &self,
        tenant_id: &str,
        update: OrgGraphUpdate,
    ) -> Result<(), RiskEngineError> {
        with_timeout(WRITE_TIMEOUT_MS, async move {
            let key = k_org_updates(tenant_id);
            let payload = serde_json::to_string(&update).map_err(|e| {
                RiskEngineError::store_with_source("serialize graph update", e)
            })?;
            // Bounded buffer: keep the newest 500 updates so a crashed
            // background worker can't balloon memory.
            let _: () = self.client.lpush(&key, payload).await.map_err(to_engine_err)?;
            let _: () = self.client.ltrim(&key, 0, 499).await.map_err(to_engine_err)?;
            let _: () = self
                .client
                .expire(&key, TTL_24H, None)
                .await
                .map_err(to_engine_err)?;
            Ok(())
        })
        .await
    }

    async fn record_decision_stats(
        &self,
        tenant_id: &str,
        was_blocked: bool,
    ) -> Result<(), RiskEngineError> {
        with_timeout(WRITE_TIMEOUT_MS, async move {
            let key = k_org_stats(tenant_id);
            let _: i64 = self
                .client
                .hincrby(&key, "total", 1)
                .await
                .map_err(to_engine_err)?;
            if was_blocked {
                let _: i64 = self
                    .client
                    .hincrby(&key, "blocked", 1)
                    .await
                    .map_err(to_engine_err)?;
            }
            let _: () = self
                .client
                .expire(&key, TTL_1H, None)
                .await
                .map_err(to_engine_err)?;
            Ok(())
        })
        .await
    }

    async fn recompute_org_graph(
        &self,
        tenant_id: &str,
        total_users: u32,
    ) -> Result<(), RiskEngineError> {
        let tenant_owned = tenant_id.to_string();
        with_timeout(RECOMPUTE_TIMEOUT_MS, async move {
            self.recompute_inner(&tenant_owned, total_users).await
        })
        .await
    }

    async fn prune_graph(&self, tenant_id: &str) -> Result<(), RiskEngineError> {
        let tenant_owned = tenant_id.to_string();
        with_timeout(RECOMPUTE_TIMEOUT_MS, async move {
            self.prune_inner(&tenant_owned).await
        })
        .await
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// FredOrgGraphStore — background recompute / prune implementations.
//
// These are the write-heavy paths called by `org_graph::worker`. Broken out
// of the trait impl so they don't need the `#[async_trait]` boxed-future
// indirection internally and so the timeout wrapper sees a flat future.
// ─────────────────────────────────────────────────────────────────────────────

impl FredOrgGraphStore {
    async fn recompute_inner(
        &self,
        tenant_id: &str,
        total_users: u32,
    ) -> Result<(), RiskEngineError> {
        let now = chrono::Utc::now().timestamp();
        let edges_key = k_org_edges(tenant_id);

        // ── 1. Load previous edges ───────────────────────────────────────────
        let raw_edges: HashMap<String, String> = self
            .client
            .hgetall(&edges_key)
            .await
            .map_err(to_engine_err)?;
        let prev_edges: HashMap<String, GraphEdge> = raw_edges
            .into_iter()
            .filter_map(|(k, v)| {
                serde_json::from_str::<GraphEdge>(&v)
                    .ok()
                    .map(|edge| (k, edge))
            })
            .collect();

        // ── 2. Atomically drain the updates list ─────────────────────────────
        let raw_updates: Vec<String> = self
            .client
            .eval(
                UPDATES_DRAIN_LUA,
                vec![k_org_updates(tenant_id)],
                Vec::<String>::new(),
            )
            .await
            .map_err(to_engine_err)?;
        let updates: Vec<OrgGraphUpdate> = raw_updates
            .iter()
            .filter_map(|s| serde_json::from_str::<OrgGraphUpdate>(s).ok())
            .collect();

        // ── 3. Load prior shift from the previous profile ────────────────────
        let prev_shift: i8 = match self
            .client
            .get::<Option<String>, _>(k_org_tenant(tenant_id))
            .await
            .map_err(to_engine_err)?
        {
            Some(s) => serde_json::from_str::<TenantRiskProfile>(&s)
                .map(|p| p.threshold_shift)
                .unwrap_or(0),
            None => 0,
        };

        // ── 4. Load decision stats (blocked / total requests in last hour) ───
        let stats: HashMap<String, String> = self
            .client
            .hgetall(k_org_stats(tenant_id))
            .await
            .map_err(to_engine_err)?;
        let total_requests_1h = stats
            .get("total")
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        let blocked_requests_1h = stats
            .get("blocked")
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);

        // ── 5. Pure recompute ────────────────────────────────────────────────
        let outputs = recompute_tenant(RecomputeInputs {
            tenant_id: tenant_id.to_string(),
            prev_edges,
            updates,
            prev_shift,
            total_users,
            blocked_requests_1h,
            total_requests_1h,
            now,
            prune: true,
        });

        // ── 6. Persist outputs ───────────────────────────────────────────────
        self.write_outputs(tenant_id, &edges_key, &outputs).await?;

        tracing::info!(
            tenant_id,
            edges = outputs.edges.len(),
            clusters = outputs.clusters.len(),
            threshold_shift = outputs.profile.threshold_shift,
            org_risk_score = outputs.snapshot.org_risk_score,
            "org graph recomputed"
        );
        Ok(())
    }

    async fn prune_inner(&self, tenant_id: &str) -> Result<(), RiskEngineError> {
        let now = chrono::Utc::now().timestamp();
        let edges_key = k_org_edges(tenant_id);

        let raw_edges: HashMap<String, String> = self
            .client
            .hgetall(&edges_key)
            .await
            .map_err(to_engine_err)?;

        let mut dead_keys = Vec::new();
        for (k, v) in &raw_edges {
            if let Ok(edge) = serde_json::from_str::<GraphEdge>(v) {
                if !edge.is_alive(now) {
                    dead_keys.push(k.clone());
                }
            } else {
                // Corrupt value — better to drop than keep poisoning reads.
                dead_keys.push(k.clone());
            }
        }

        if !dead_keys.is_empty() {
            let count = dead_keys.len();
            let _: i64 = self
                .client
                .hdel(&edges_key, dead_keys)
                .await
                .map_err(to_engine_err)?;
            tracing::info!(tenant_id, pruned = count, "pruned decayed edges");
        }
        Ok(())
    }

    async fn write_outputs(
        &self,
        tenant_id: &str,
        edges_key: &str,
        outputs: &crate::org_graph::RecomputeOutputs,
    ) -> Result<(), RiskEngineError> {
        // Snapshot — what the engine reads on the hot path.
        let snap_json = serde_json::to_string(&outputs.snapshot).map_err(|e| {
            RiskEngineError::store_with_source("serialize org snapshot", e)
        })?;
        let _: () = self
            .client
            .set(
                k_org_risk(tenant_id),
                snap_json,
                Some(Expiration::EX(TTL_ORG_RISK)),
                None,
                false,
            )
            .await
            .map_err(to_engine_err)?;

        // Tenant profile — carries prev_shift for next cycle's inertia.
        let prof_json = serde_json::to_string(&outputs.profile).map_err(|e| {
            RiskEngineError::store_with_source("serialize tenant profile", e)
        })?;
        let _: () = self
            .client
            .set(
                k_org_tenant(tenant_id),
                prof_json,
                Some(Expiration::EX(TTL_ORG_TENANT)),
                None,
                false,
            )
            .await
            .map_err(to_engine_err)?;

        // Edges — HDEL the pruned ones, HSET the live set.
        if !outputs.pruned_keys.is_empty() {
            let _: i64 = self
                .client
                .hdel(edges_key, outputs.pruned_keys.clone())
                .await
                .map_err(to_engine_err)?;
        }
        for (field, edge) in &outputs.edges {
            let json = serde_json::to_string(edge).map_err(|e| {
                RiskEngineError::store_with_source("serialize edge", e)
            })?;
            let _: () = self
                .client
                .hset(edges_key, (field.clone(), json))
                .await
                .map_err(to_engine_err)?;
        }
        if !outputs.edges.is_empty() {
            let _: () = self
                .client
                .expire(edges_key, TTL_ORG_EDGES, None)
                .await
                .map_err(to_engine_err)?;
        }

        // Per-node memberships. Stale entries expire via TTL — no explicit
        // cleanup needed as long as the worker keeps running.
        for (node_key, membership) in &outputs.memberships {
            let m_json = serde_json::to_string(membership).map_err(|e| {
                RiskEngineError::store_with_source("serialize membership", e)
            })?;
            let _: () = self
                .client
                .set(
                    k_org_membership(tenant_id, node_key),
                    m_json,
                    Some(Expiration::EX(TTL_ORG_MEMBERSHIP)),
                    None,
                    false,
                )
                .await
                .map_err(to_engine_err)?;
        }

        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// FredPolicyStore — per-tenant policy overrides (Phase 6)
//
// Key schema:
//   policy:{tenant_id}    HGET field=config   →  JSON blob of PolicyOverrides
//
// Missing key or missing field = empty overrides (tenant uses defaults).
// ─────────────────────────────────────────────────────────────────────────────

fn k_policy(tenant_id: &str) -> String {
    format!("policy:{tenant_id}")
}

const POLICY_FIELD: &str = "config";

#[derive(Clone)]
pub struct FredPolicyStore {
    client: Arc<Client>,
}

impl FredPolicyStore {
    pub fn new(client: Client) -> Self {
        Self { client: Arc::new(client) }
    }

    pub fn from_signal_store(store: &FredSignalStore) -> Self {
        Self { client: store.client.clone() }
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Write-side helper for control-plane tooling: persist a tenant's full
    /// override set. Not part of the [`PolicyStore`] trait because the hot
    /// path only needs reads.
    pub async fn set_overrides(
        &self,
        tenant_id: &str,
        overrides: &PolicyOverrides,
    ) -> Result<(), RiskEngineError> {
        let json = overrides.to_json()?;
        with_timeout(WRITE_TIMEOUT_MS, async move {
            let _: () = self
                .client
                .hset(k_policy(tenant_id), (POLICY_FIELD, json))
                .await
                .map_err(to_engine_err)?;
            Ok(())
        })
        .await
    }
}

#[async_trait]
impl PolicyStore for FredPolicyStore {
    async fn load_overrides(
        &self,
        tenant_id: &str,
    ) -> Result<PolicyOverrides, RiskEngineError> {
        with_timeout(READ_TIMEOUT_MS, async move {
            let raw: Option<String> = self
                .client
                .hget(k_policy(tenant_id), POLICY_FIELD)
                .await
                .map_err(to_engine_err)?;
            match raw {
                Some(s) if !s.is_empty() => PolicyOverrides::from_json(&s),
                _ => Ok(PolicyOverrides::default()),
            }
        })
        .await
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn decode_entries(raw: &[String]) -> Vec<EscalationEntry> {
    raw.iter()
        .filter_map(|s| serde_json::from_str::<EscalationEntry>(s).ok())
        .collect()
}
