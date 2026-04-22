use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::context::RequiredAction;
use crate::org_graph::ClusterFactor;

// ─────────────────────────────────────────────────────────────────────────────
// Decision
// ─────────────────────────────────────────────────────────────────────────────

/// The engine's verdict for this request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    /// Proceed. No friction added.
    Allow,

    /// Require the caller to complete an additional verification step before
    /// the original action is allowed. `required_action` is always set.
    Challenge,

    /// The action is blocked and queued for asynchronous human review.
    /// The caller receives a 202-equivalent with a hold reference ID.
    Hold,

    /// Hard block. No retry path. Account may be locked.
    Deny,
}

impl Decision {
    /// Returns true for any outcome that does not permit the action to proceed immediately.
    pub fn is_blocking(&self) -> bool {
        matches!(self, Decision::Challenge | Decision::Hold | Decision::Deny)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ScoreBreakdown — full diagnostic of how the score was computed
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    /// Device trust component (capped 0–50).
    pub d: u8,
    /// Session anomaly component (capped 0–50).
    pub s: u8,
    /// Network risk component (capped 0–50).
    pub n: u8,
    /// Behavioural velocity component (capped 0–50).
    pub b: u8,
    /// Pattern correlation bonus (capped 0–100).
    pub c: u8,
    /// Sum of D+S+N+B+C before multipliers.
    pub base: u8,
    /// Action sensitivity multiplier applied.
    pub m_action: f32,
    /// Org risk-level multiplier applied.
    pub m_org: f32,
    /// Absolute adder applied after multiplication.
    pub a: u8,
    /// Final clamped score (0–100).
    pub final_score: u8,
}

impl ScoreBreakdown {
    pub fn zeroed_max() -> Self {
        ScoreBreakdown {
            d: 50,
            s: 50,
            n: 50,
            b: 50,
            c: 100,
            base: 255, // Saturating u8 max
            m_action: 3.0,
            m_org: 2.0,
            a: 50,
            final_score: 100,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Factor — a single scored signal contributing to the result
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScoreComponent {
    Device,
    Session,
    Network,
    Behavioral,
    Correlation,
    Absolute,
    Gate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Factor {
    pub name: String,
    pub component: ScoreComponent,
    /// Raw points this factor added to its component bucket.
    pub contribution: u8,
    pub description: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// HardGate — named hard-block triggers
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HardGate {
    RevokedCredential,
    /// Credential explicitly marked as lost by its owner. Distinct from
    /// RevokedCredential so audit/alerting can differentiate the two causes.
    LostCredential,
    AccountLocked,
    NonceReplay,
    ImpossibleTravelViaTor,
    TorOnSensitiveAction,
    RecoveryCompletedWithNoPending,
    OauthNonceIpMismatchOnTor,
    /// OAuth authorization-code completion where the IP that initiated the
    /// `/authorize` request differs from the IP redeeming the code. Fires
    /// regardless of ASN — a stolen code redeemed from any different network
    /// is treated as a single-use credential compromise.
    OauthIpMismatch,
    SignCountRollback,
    /// Request originates from a sanctioned country (OFAC, EU sanctions).
    /// Immediate deny — legal compliance requirement, no override.
    SanctionedCountry,
    /// Credential found in a known breach database AND the action is sensitive.
    /// Immediate deny — the credential must be rotated before proceeding.
    BreachedCredentialOnSensitive,
    /// VPN detected on a critical privileged action (recovery approve/complete,
    /// add_admin, rotate_api_key). These actions require attribution.
    VpnOnCriticalAction,
    /// Tor detected on a recovery action. Zero-tolerance policy.
    TorOnRecovery,
    /// Browser automation detected during registration.
    AutomationOnRegister,
    /// JWT fingerprint mismatch matched with a sensitive action. Structural failure.
    SensitiveActionOnMismatch,
    /// Privileged/Management action attempted on a very fresh (< 5m) credential.
    PrivilegedActionOnFreshCredential,
    /// JWT explicitly expired (jwt_expires_at < evaluated_at) combined with a
    /// sensitive action. Defense-in-depth — the adapter is expected to reject
    /// expired tokens at the boundary; this gate catches cases where that
    /// validation is missing or buggy.
    ExpiredJwtOnSensitive,
}

// ─────────────────────────────────────────────────────────────────────────────
// RiskDecision — the full output contract
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskDecision {
    // ── Primary verdict ───────────────────────────────────────────────────────
    pub decision: Decision,
    pub score: u8,

    /// Set when `decision == Challenge`. Always populated for CHALLENGE; None otherwise.
    pub required_action: Option<RequiredAction>,

    // ── Diagnostics ───────────────────────────────────────────────────────────
    pub score_breakdown: ScoreBreakdown,

    /// Hard gates that fired and caused immediate DENY (if any).
    pub triggered_gates: Vec<HardGate>,

    /// Human-readable names of scoring rules that contributed.
    pub triggered_rules: Vec<String>,

    /// Per-factor contributions, ordered by `contribution` descending.
    pub contributing_factors: Vec<Factor>,

    // ── Side-effect directives ────────────────────────────────────────────────
    /// Engine instructs the adapter to set `login:locked:{uid}` in Redis.
    pub lock_account: bool,

    /// Engine instructs the adapter to send an async user notification.
    pub notify_user: bool,

    /// Engine instructs the adapter to alert the security team / admin channel.
    pub notify_admin: bool,

    /// Set when `decision == Hold`. Human-readable reason for the hold queue.
    pub hold_reason: Option<String>,

    /// Whether signal collection was degraded (some signals missing).
    /// The adapter should log this separately for observability.
    pub signals_degraded: bool,

    // ── Escalation metadata ───────────────────────────────────────────────────
    /// True if the decision was promoted by escalation memory
    /// (e.g., Challenge→Hold because of repeated recent CHALLENGEs).
    pub escalated: bool,

    // ── Org-graph intelligence output ─────────────────────────────────────────
    //
    // All fields below are additive and backward-compatible.
    // Consumers that do not understand these fields can safely ignore them.
    // `#[serde(default)]` ensures old decoders do not fail on missing fields.

    /// The raw base score before org/cluster bias and threshold shift.
    /// Same as `score` in pre-org-graph versions of the engine.
    /// Preserved for backward compatibility — `score` is never changed.
    #[serde(default)]
    pub base_score: u8,

    /// The effective score after applying org bias, cluster bias, and threshold
    /// shift. This is the score that drove the `decision` field.
    /// Equal to `score` when no org data is available (fail-open).
    #[serde(default)]
    pub adjusted_score: u8,

    /// Org-level risk score at time of evaluation (0–100).
    /// `None` if org graph is not deployed or Redis timed out.
    #[serde(default)]
    pub org_risk_score: Option<u8>,

    /// Per-cluster contributions that influenced the adjusted score.
    /// Empty if the user is not in any cluster or org data is unavailable.
    #[serde(default)]
    pub cluster_factors: Vec<ClusterFactor>,

    /// The threshold shift applied to produce `adjusted_score`.
    /// Positive = more strict (org under attack), negative = more lenient.
    #[serde(default)]
    pub applied_threshold_shift: i8,

    // ── Correlation ───────────────────────────────────────────────────────────
    pub request_id: Uuid,
    pub evaluated_at: DateTime<Utc>,
    pub engine_version: &'static str,

    /// Decision schema version (see [`SCHEMA_VERSION`]). Incremented on any
    /// breaking change to this struct's JSON shape so host deployments can
    /// reject stale consumers. Absent in pre-1.0 payloads → `0` after
    /// deserialisation via `#[serde(default)]`.
    #[serde(default)]
    pub schema_version: u32,
}

pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Wire-level schema version for [`RiskDecision`] (and, by extension, the
/// engine's public JSON surface). Bump whenever a field is renamed, removed,
/// or changes type. Additive changes (new optional field, new enum variant
/// on an `#[serde(other)]`-augmented enum) do NOT require a bump.
///
/// Versioning rules:
/// - `0` — unversioned (pre-F4) payloads; treated as v1-compatible.
/// - `1` — initial declared version. Covers everything through engine
///   `0.1.0`.
pub const SCHEMA_VERSION: u32 = 1;
