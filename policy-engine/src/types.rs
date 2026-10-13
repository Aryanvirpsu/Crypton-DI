use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Everything a policy evaluator can inspect about the current login attempt.
/// Built once in `oauth_complete` and passed through all enabled evaluators.
#[derive(Debug, Clone)]
pub struct PolicyContext {
    pub user_id:                 Uuid,
    pub username:                String,
    pub ip:                      Option<String>,
    pub country_code:            Option<String>,        // current login: X-CF-IPCountry or GeoIP
    pub last_login_country:      Option<String>,        // from login_attempts — used by geo_velocity
    pub is_tor_exit:             bool,                  // Redis SET tor:exit_nodes lookup
    pub device_id:               Uuid,
    pub credential_status:       String,                // 'active' | 'pending_approval' | 'revoked' | 'lost'
    pub risk_score:              u32,                   // 0–100; 0 until risk engine is wired
    /// Most recent prior successful login (DateTime so evaluators share `login_at` as "now").
    pub last_login_at:           Option<DateTime<Utc>>,
    /// `credentials.last_used_at` for device_trust / geo_velocity.
    pub last_credential_used_at: Option<DateTime<Utc>>,
    pub login_hour:              u8,                    // 0–23 UTC (kept for legacy; prefer login_at)
    /// Full login timestamp — the authoritative "now" for all evaluators (P2-8).
    pub login_at:                DateTime<Utc>,
    /// Failure count pre-fetched using the live `window_minutes` from the policy config.
    pub failed_attempts_count:   u32,
    /// The window (minutes) that was used when fetching `failed_attempts_count`.
    /// Stored here so the evaluator can validate that the config matches the query.
    pub failed_window_minutes:   u32,
}

/// Decision returned by a policy evaluator.
#[derive(Debug, Clone, PartialEq)]
pub enum PolicyDecision {
    Allow,
    Deny(String),    // reason code, returned to client
    StepUp(String),  // reason code; frontend must trigger additional verification
}

/// A policy row as loaded from the `policies` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRecord {
    pub key:          String,
    pub display_name: String,
    pub description:  String,
    pub enabled:      bool,
    pub config:       serde_json::Value,
    /// DB-sourced timestamp used by `PolicyCache::refresh()` to resolve write races:
    /// a background refresh only overwrites an in-memory entry when the DB record
    /// is strictly newer than what is already cached.
    pub updated_at:   chrono::DateTime<chrono::Utc>,
}

/// Sync evaluator trait — no async I/O allowed.
/// All DB data must be pre-fetched into `PolicyContext` before evaluation.
pub trait PolicyEvaluator: Send + Sync {
    fn key(&self) -> &'static str;
    fn evaluate(&self, ctx: &PolicyContext, config: &serde_json::Value) -> PolicyDecision;
}
