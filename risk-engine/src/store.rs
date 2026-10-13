use async_trait::async_trait;
use std::net::IpAddr;
use uuid::Uuid;

use crate::decision::Decision;
use crate::error::RiskEngineError;
use crate::org_graph::{
    ClusterMembership, OrgGraphUpdate, OrgRiskSnapshot, TenantRiskProfile,
};

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Recommended nonce TTL in seconds. Implementations should use this or the
/// JWT lifetime (whichever is longer) to prevent replay after nonce expiry.
pub const NONCE_TTL_SECONDS: u64 = 300;

// ─────────────────────────────────────────────────────────────────────────────
// EscalationEntry — stored in risk:recent:{uid}
// ─────────────────────────────────────────────────────────────────────────────

/// A compact record of a past engine decision, serialised to JSON and stored
/// in a Redis list at key `risk:recent:{user_id}`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EscalationEntry {
    pub score: u8,
    pub decision: Decision,
    pub ts_unix: i64,
}

// ─────────────────────────────────────────────────────────────────────────────
// VelocityCounters — all Redis counter signals in one batch fetch
// ─────────────────────────────────────────────────────────────────────────────

/// All velocity signals fetched from Redis in a single pipeline call.
/// Each field is `Option` — `None` means the key was absent or the fetch timed out.
#[derive(Debug, Clone, Default)]
pub struct VelocityCounters {
    pub login_attempts_5m: Option<u32>,
    pub failed_login_attempts_1h: Option<u32>,
    pub actions_executed_5m: Option<u32>,
    pub recovery_requests_24h: Option<u32>,
    pub device_revocations_1h: Option<u32>,
    pub registrations_from_ip_10m: Option<u32>,
    pub active_session_count: Option<u32>,
    pub account_locked: bool,
    pub recovery_pending: bool,
    // ── Overlapping velocity windows for multi-window scoring ────────────────
    /// key: velocity:login:{uid}:1m
    pub login_attempts_1m: Option<u32>,
    /// key: velocity:login:{uid}:1h
    pub login_attempts_1h: Option<u32>,
    /// key: velocity:login:{uid}:24h
    pub login_attempts_24h: Option<u32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// SignalStore trait
// ─────────────────────────────────────────────────────────────────────────────

/// Abstraction over the Redis (and lightweight DB) calls that the engine
/// requires. The concrete implementation lives in the adapter layer and
/// depends on the actual Redis client in use by the host service.
///
/// All methods must complete within the time budget specified by the caller
/// (adapter sets a tokio::time::timeout around each call).
#[async_trait]
pub trait SignalStore: Send + Sync {
    // ── Velocity counter reads ────────────────────────────────────────────────

    /// Fetch all velocity counters for `user_id` and `request_ip` in a single
    /// pipelined call. If the pipeline partially fails, return what is available
    /// and leave the rest as `None`.
    ///
    /// **Ordering note**: to close the concurrent-burst race, adapters should
    /// pre-increment the relevant counters via [`IncrementStore`] *before*
    /// calling `evaluate()`. See the `IncrementStore` docs for the rationale
    /// and the set of counters that benefit.
    async fn get_velocity_counters(
        &self,
        user_id: Uuid,
        request_ip: Option<IpAddr>,
    ) -> Result<VelocityCounters, RiskEngineError>;

    // ── Escalation memory ─────────────────────────────────────────────────────

    /// Return recent engine decisions for this user (key: `risk:recent:{uid}`).
    /// Returns at most 10 entries, newest first.
    async fn get_recent_decisions(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<EscalationEntry>, RiskEngineError>;

    /// Append a new decision entry and trim the list to 10 entries.
    /// Key TTL: 300 seconds (5 minutes).
    async fn record_decision(
        &self,
        user_id: Uuid,
        entry: EscalationEntry,
    ) -> Result<(), RiskEngineError>;

    /// Atomically check escalation history and record the new decision.
    ///
    /// Prevents the TOCTOU race where concurrent requests read the same
    /// escalation history and fail to escalate. Implementations MUST use
    /// Redis Lua scripts or MULTI/EXEC for true atomicity — a naive
    /// get → check → record sequence is a security bug, not an
    /// acceptable fallback.
    ///
    /// Returns `(final_decision, was_escalated)`. The `final_decision` is
    /// either the `current_decision` (if no escalation) or the promoted
    /// decision (Hold/Deny).
    ///
    /// No default implementation is provided — the previous non-atomic
    /// default was racy under concurrency and has been removed so the
    /// trait cannot be implemented without explicitly addressing atomicity.
    /// Helpers `escalation::check_escalation` and `EscalationEntry` are
    /// available for use inside a Lua script or transaction.
    async fn check_and_record_escalation(
        &self,
        user_id: Uuid,
        current_decision: &Decision,
        score: u8,
        ts_unix: i64,
    ) -> Result<(Decision, bool), RiskEngineError>;

    // ── Account lock ──────────────────────────────────────────────────────────

    /// Set `login:locked:{uid}` with the given TTL in seconds.
    async fn lock_account(
        &self,
        user_id: Uuid,
        ttl_seconds: u64,
    ) -> Result<(), RiskEngineError>;

    /// Block an IP address for the specified TTL. Called on registration DENY
    /// to prevent repeated sybil registration from the same IP.
    /// Key: `ip:blocked:{ip}` with the given TTL.
    ///
    /// No default implementation is provided — the previous default was a
    /// silent no-op, which made sybil defence appear active while actually
    /// doing nothing. Adapters MUST implement this (a Redis `SET ip:blocked:{ip} 1 EX {ttl}`
    /// is typical); return `Ok(())` only if intentionally disabling IP blocking
    /// for this deployment.
    async fn block_ip(
        &self,
        ip: IpAddr,
        ttl_seconds: u64,
    ) -> Result<(), RiskEngineError>;

    // ── JWT fingerprint ───────────────────────────────────────────────────────

    /// Look up a stored fingerprint by its key `jwt:fingerprint:{fingerprint_key}`.
    /// Returns `None` if the key does not exist (token pre-dates fingerprinting).
    async fn get_jwt_fingerprint(
        &self,
        fingerprint_key: &str,
    ) -> Result<Option<String>, RiskEngineError>;

    /// Store a fingerprint at `jwt:fingerprint:{fingerprint_key}` with the
    /// remaining JWT TTL so the key auto-expires when the token expires.
    async fn set_jwt_fingerprint(
        &self,
        fingerprint_key: &str,
        fingerprint_value: &str,
        ttl_seconds: u64,
    ) -> Result<(), RiskEngineError>;

    // ── Nonce ─────────────────────────────────────────────────────────────────

    /// Check whether `nonce:{nonce}` exists (already used).
    async fn is_nonce_used(&self, nonce: &str) -> Result<bool, RiskEngineError>;

    /// Mark the nonce as used. TTL should be at least `NONCE_TTL_SECONDS`
    /// or the JWT lifetime, whichever is longer.
    async fn consume_nonce(&self, nonce: &str) -> Result<(), RiskEngineError>;
}

// ─────────────────────────────────────────────────────────────────────────────
// IncrementStore trait — write-side counter increments
// ─────────────────────────────────────────────────────────────────────────────

/// Separate write-side trait so the engine core only needs read access.
///
/// # Ordering: pre-increment vs post-increment
///
/// These methods are safe to call at any time, but **ordering relative to
/// [`SignalStore::get_velocity_counters`] matters for correctness under
/// concurrency**:
///
/// ```text
///   ┌────────────────────────── Fire-and-forget (post-increment) ───────────┐
///   │  engine::evaluate(ctx, &store)   ← reads counters                     │
///   │  store.increment_login_attempts(user_id).await?  ← after the decision │
///   └──────────────────────────────────────────────────────────────────────┘
///
///   ┌────────────────────────── Pre-increment (recommended) ────────────────┐
///   │  store.increment_login_attempts(user_id).await?  ← before evaluate    │
///   │  engine::evaluate(ctx, &store)                                        │
///   └──────────────────────────────────────────────────────────────────────┘
/// ```
///
/// **Post-increment has a TOCTOU race.** Under a burst of N concurrent
/// requests, each one reads the same pre-burst counter value, decides
/// independently, and only *then* bumps the counter — the engine never sees
/// the actual velocity until the burst is over. Attackers who pipeline
/// requests can step right through per-window limits.
///
/// **Pre-increment closes the race.** Each request atomically bumps the
/// counter before `evaluate()`, so the Nth concurrent request observes its
/// own position in the burst and can be scored as a velocity violation.
///
/// For counters that drive scoring decisions (login attempts, failed
/// attempts, action velocity, revocation spree, IP registrations), host
/// adapters SHOULD pre-increment. For counters that only feed *future*
/// decisions (active session count), post-increment is fine.
#[async_trait]
pub trait IncrementStore: Send + Sync {
    async fn increment_login_attempts(&self, user_id: Uuid) -> Result<(), RiskEngineError>;
    async fn increment_failed_attempts(&self, user_id: Uuid) -> Result<(), RiskEngineError>;
    async fn increment_actions_executed(&self, user_id: Uuid) -> Result<(), RiskEngineError>;
    async fn increment_recovery_requests(&self, user_id: Uuid) -> Result<(), RiskEngineError>;
    async fn increment_device_revocations(&self, user_id: Uuid) -> Result<(), RiskEngineError>;
    async fn increment_registrations_from_ip(
        &self,
        ip: IpAddr,
    ) -> Result<(), RiskEngineError>;
    async fn increment_active_sessions(&self, user_id: Uuid) -> Result<(), RiskEngineError>;
    async fn decrement_active_sessions(&self, user_id: Uuid) -> Result<(), RiskEngineError>;
}

// ─────────────────────────────────────────────────────────────────────────────
// OrgGraphStore trait — adapter-side org graph I/O
// ─────────────────────────────────────────────────────────────────────────────

/// Redis I/O contract for the org-level attack graph.
///
/// This trait is implemented by the adapter, NOT the engine core. The engine
/// reads org data from `RiskContext` fields (pre-populated by the adapter).
/// The adapter calls these methods to:
///   1. Read pre-computed org signals before calling `evaluate()`.
///   2. Push graph updates after `evaluate()` returns (fire-and-forget).
///   3. Trigger background recomputation when staleness thresholds are crossed.
///
/// All read methods must complete within 10ms. Write methods are best-effort.
#[async_trait]
pub trait OrgGraphStore: Send + Sync {
    // ── Per-request reads (hot path) ─────────────────────────────────────────

    async fn get_org_risk_snapshot(
        &self,
        tenant_id: &str,
    ) -> Result<Option<OrgRiskSnapshot>, RiskEngineError>;

    async fn get_cluster_membership(
        &self,
        tenant_id: &str,
        node_key: &str,
    ) -> Result<Option<ClusterMembership>, RiskEngineError>;

    async fn get_tenant_profile(
        &self,
        tenant_id: &str,
    ) -> Result<Option<TenantRiskProfile>, RiskEngineError>;

    // ── Post-request writes (fire-and-forget, off hot path) ──────────────────

    async fn push_graph_update(
        &self,
        tenant_id: &str,
        update: OrgGraphUpdate,
    ) -> Result<(), RiskEngineError>;

    async fn record_decision_stats(
        &self,
        tenant_id: &str,
        was_blocked: bool,
    ) -> Result<(), RiskEngineError>;

    // ── Background-only operations (NOT called on hot path) ──────────────────

    async fn recompute_org_graph(
        &self,
        tenant_id: &str,
        total_users: u32,
    ) -> Result<(), RiskEngineError>;

    async fn prune_graph(
        &self,
        tenant_id: &str,
    ) -> Result<(), RiskEngineError>;
}
