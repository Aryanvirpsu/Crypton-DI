use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::sync::Arc;
use uuid::Uuid;

use crate::org_graph::ClusterMembership;
use crate::policy_config::PolicyConfig;

fn default_policy_arc() -> Arc<PolicyConfig> {
    Arc::new(PolicyConfig::default())
}

// ─────────────────────────────────────────────────────────────────────────────
// Enums
// ─────────────────────────────────────────────────────────────────────────────

/// The action the caller is attempting. Drives M_action multiplier selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RiskAction {
    Register,
    Login,
    OauthComplete,
    DeviceList,
    DeviceRevoke,
    DeviceMarkLost,
    RecoveryStart,
    RecoveryApprove,
    RecoveryComplete,
    ActionChallenge,
    ActionExecute { action_name: String },
    AuditLogExport,
    /// Catch-all for unknown future actions; treated as non-sensitive.
    Unknown { name: String },
}

impl RiskAction {
    /// Whether this action is considered sensitive enough that partial signal
    /// collection forces a CHALLENGE rather than an allow.
    pub fn is_sensitive(&self) -> bool {
        matches!(
            self,
            RiskAction::Login
                | RiskAction::OauthComplete
                | RiskAction::DeviceRevoke
                | RiskAction::DeviceMarkLost
                | RiskAction::RecoveryStart
                | RiskAction::RecoveryApprove
                | RiskAction::RecoveryComplete
                | RiskAction::ActionExecute { .. }
                | RiskAction::AuditLogExport
        )
    }

    pub fn is_recovery(&self) -> bool {
        matches!(
            self,
            RiskAction::RecoveryStart
                | RiskAction::RecoveryApprove
                | RiskAction::RecoveryComplete
        )
    }

    pub fn is_privileged(&self) -> bool {
        match self {
            RiskAction::RecoveryApprove | RiskAction::RecoveryComplete => true,
            RiskAction::ActionExecute { .. } => true,
            RiskAction::DeviceRevoke | RiskAction::DeviceMarkLost => true,
            RiskAction::AuditLogExport => true,
            _ => false,
        }
    }

    /// Human-readable label used in audit entries and factor descriptions.
    pub fn label(&self) -> &str {
        match self {
            RiskAction::Register => "register",
            RiskAction::Login => "login",
            RiskAction::OauthComplete => "oauth_complete",
            RiskAction::DeviceList => "device_list",
            RiskAction::DeviceRevoke => "device_revoke",
            RiskAction::DeviceMarkLost => "device_mark_lost",
            RiskAction::RecoveryStart => "recovery_start",
            RiskAction::RecoveryApprove => "recovery_approve",
            RiskAction::RecoveryComplete => "recovery_complete",
            RiskAction::ActionChallenge => "action_challenge",
            RiskAction::ActionExecute { action_name } => action_name.as_str(),
            RiskAction::AuditLogExport => "audit_log_export",
            RiskAction::Unknown { name } => name.as_str(),
        }
    }
}

/// Current status of the authenticating credential, as read from PostgreSQL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialStatus {
    Active,
    Lost,
    Revoked,
}

/// Network ASN classification derived from a local GeoIP database (MaxMind)
/// or an IP intelligence provider (IPinfo, MaxMind GeoIP2, ip2location, etc.).
/// Never inferred from request headers — always from IP lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AsnType {
    /// End-user home or mobile ISP.
    Residential,
    /// Enterprise or corporate ASN.
    Corporate,
    /// Cloud providers (AWS, Azure, GCP), VPS, or hosting provider.
    Datacenter,
    /// Known Tor exit node (from public exit list).
    Tor,
    /// Commercial or consumer VPN provider (NordVPN, ExpressVPN, Mullvad, etc.).
    /// Detected via IP intelligence feed — NOT from user-agent or self-report.
    Vpn,
    /// Open or anonymous proxy server (SOCKS, HTTP CONNECT, etc.).
    Proxy,
    /// Web hosting / shared hosting provider (distinct from major cloud).
    Hosting,
    /// Privacy relay service (Apple Private Relay, Cloudflare WARP, iCloud+).
    /// Lower risk than VPN/Proxy — used for legitimate privacy, not evasion.
    Relay,
    /// Could not classify the ASN.
    Unknown,
}

/// Device trust level, computed by the adapter from device history.
/// - `New`: never seen before (no prior successful auth from this device fingerprint).
/// - `Recognized`: seen before but not yet fully trusted (<7 days or <5 sessions).
/// - `Trusted`: established device with consistent usage history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceTrustLevel {
    New,
    Recognized,
    Trusted,
}

impl DeviceTrustLevel {
    pub fn is_untrusted(&self) -> bool {
        matches!(self, DeviceTrustLevel::New | DeviceTrustLevel::Recognized)
    }
}

/// Organisation-wide risk posture, set out-of-band by an operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrgRiskLevel {
    Normal,
    Elevated,
    UnderAttack,
}

impl OrgRiskLevel {
    pub fn multiplier(&self) -> f32 {
        match self {
            OrgRiskLevel::Normal => 1.0,
            OrgRiskLevel::Elevated => 1.5,
            OrgRiskLevel::UnderAttack => 2.0,
        }
    }
}

/// What the caller must do to satisfy a CHALLENGE decision.
/// Never left ambiguous — the adapter maps this to a concrete HTTP directive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequiredAction {
    /// Re-authenticate with a registered passkey (WebAuthn assertion).
    StepUpWebAuthn,
    /// Re-authenticate AND confirm device ownership (e.g., confirm nickname + WebAuthn).
    ReverifyDevice,
    /// Full re-login required — session is too stale or token too old.
    ReLogin,
    /// Async: action blocked until a human admin reviews and approves.
    AdminApproval,
    /// Complete a CAPTCHA challenge (reCAPTCHA, hCaptcha, Cloudflare Turnstile).
    /// Used as progressive friction for bot-suspected traffic before escalating
    /// to full WebAuthn step-up.
    CompleteCaptcha,
    /// Verify email ownership (click-through link or OTP to registered email).
    /// Used when email verification status is missing or stale.
    VerifyEmail,
}

// ─────────────────────────────────────────────────────────────────────────────
// GeoPoint
// ─────────────────────────────────────────────────────────────────────────────

/// Lat/lon pair with country and optional city, resolved via local GeoIP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoPoint {
    pub lat: f64,
    pub lon: f64,
    /// ISO 3166-1 alpha-2.
    pub country_code: String,
    pub city: Option<String>,
}

impl GeoPoint {
    /// Haversine distance in kilometres between two points.
    pub fn distance_km(&self, other: &GeoPoint) -> f64 {
        const R: f64 = 6371.0;
        let dlat = (other.lat - self.lat).to_radians();
        let dlon = (other.lon - self.lon).to_radians();
        let a = (dlat / 2.0).sin().powi(2)
            + self.lat.to_radians().cos()
                * other.lat.to_radians().cos()
                * (dlon / 2.0).sin().powi(2);
        R * 2.0 * a.sqrt().asin()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// RiskContext — the full input contract
// ─────────────────────────────────────────────────────────────────────────────

/// All signals available to the risk engine at evaluation time.
///
/// The adapter (or test harness) is responsible for populating this from
/// the identity service's data sources. Fields marked `Option` may be absent
/// when the signal is unavailable — the engine handles each absence explicitly.
///
/// Signals are split into two categories:
/// - **Redis signals**: velocity counters, locks, session counts — populated
///   via a pipelined multi-GET before engine evaluation.
/// - **DB signals**: credential metadata — populated from PostgreSQL, read once
///   per request and cached in the adapter layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskContext {
    // ── Request identity ─────────────────────────────────────────────────────
    pub request_id: Uuid,
    pub evaluated_at: DateTime<Utc>,

    // ── User / credential identity ───────────────────────────────────────────
    pub user_id: Uuid,
    pub username: String,
    /// UUID of the credential used in this request.
    /// None during registration (no credential yet).
    pub credential_id: Option<Uuid>,

    // ── Action ───────────────────────────────────────────────────────────────
    pub action: RiskAction,
    /// Target resource identifier if the action operates on a specific resource.
    pub resource_id: Option<String>,

    // ── DB signals: credential metadata ──────────────────────────────────────
    pub credential_status: CredentialStatus,
    pub credential_created_at: DateTime<Utc>,
    pub credential_last_used_at: Option<DateTime<Utc>>,
    /// sign_count as stored in DB *before* this authentication updated it.
    pub credential_sign_count_prev: i64,
    /// sign_count as reported by the WebAuthn assertion.
    pub credential_sign_count_new: i64,
    /// User-agent string stored at credential registration time.
    pub credential_registered_ua: Option<String>,
    /// Number of *active* credentials this user owns. Includes the current one.
    pub credential_count_for_user: u32,

    // ── DB signals: historical context ───────────────────────────────────────
    /// Total audit log entries for this user (approximated via COUNT query).
    pub prior_audit_event_count: u32,
    /// IP used at the most recent successful login (from audit_logs detail).
    pub last_login_ip: Option<IpAddr>,
    pub last_login_at: Option<DateTime<Utc>>,
    /// GeoIP-resolved location of `last_login_ip`.
    pub last_login_geo: Option<GeoPoint>,

    // ── Session / JWT signals ─────────────────────────────────────────────────
    pub jwt_issued_at: Option<DateTime<Utc>>,
    pub jwt_expires_at: Option<DateTime<Utc>>,
    /// sha256(request_ip || request_ua) stored in Redis at JWT issue time.
    /// None if the token was issued before fingerprinting was deployed.
    pub jwt_fingerprint_stored: Option<String>,
    /// sha256(request_ip || request_ua) computed from *this* request.
    pub jwt_fingerprint_current: Option<String>,

    // ── Network signals ───────────────────────────────────────────────────────
    pub request_ip: Option<IpAddr>,
    /// User-agent from the *current* request headers.
    pub request_ua: Option<String>,
    /// GeoIP-resolved location of `request_ip`.
    pub request_geo: Option<GeoPoint>,
    pub ip_asn_type: Option<AsnType>,

    // ── Redis signals: velocity counters ─────────────────────────────────────
    /// key: velocity:login:{uid}:5m
    pub login_attempts_5m: Option<u32>,
    /// key: velocity:failed:{uid}:1h
    pub failed_login_attempts_1h: Option<u32>,
    /// key: velocity:actions:{uid}:5m
    pub actions_executed_5m: Option<u32>,
    /// key: velocity:recovery:{uid}:24h
    pub recovery_requests_24h: Option<u32>,
    /// key: velocity:revoke:{uid}:1h
    pub device_revocations_1h: Option<u32>,
    /// key: velocity:reg:{ip}:10m
    pub registrations_from_ip_10m: Option<u32>,

    // ── Redis signals: session state ─────────────────────────────────────────
    /// key: sessions:active:{uid} — count of currently live sessions.
    pub active_session_count: Option<u32>,
    /// key: login:locked:{uid}
    pub account_locked: bool,
    /// True if a recovery_request row with status='pending' exists for this user.
    pub recovery_pending: bool,

    // ── OAuth-specific signals ────────────────────────────────────────────────
    /// The IP stored inside the oauth:session:{nonce} Redis value when
    /// /authorize was called. Compared against request_ip at /oauth/complete.
    pub oauth_authorize_ip: Option<IpAddr>,

    // ── Nonce / replay signals ────────────────────────────────────────────────
    pub nonce_present: bool,
    /// True if the nonce was already consumed (replay attempt).
    pub nonce_already_used: bool,

    // ── Org-level risk posture ────────────────────────────────────────────────
    pub org_risk_level: OrgRiskLevel,

    // ── Signal collection metadata ────────────────────────────────────────────
    /// True if any Redis signals were missing due to timeout or error.
    pub redis_signals_degraded: bool,
    /// True if any DB signals were missing due to timeout or error.
    pub db_signals_degraded: bool,

    // ── Org-graph intelligence (pre-fetched by adapter) ───────────────────────
    //
    // These fields are populated from `org:risk:{tenant_id}` and
    // `org:clusters:{tenant_id}` in Redis, BEFORE `evaluate()` is called.
    // All are `Option` — the engine fails open if org data is unavailable.

    /// Tenant identifier. Used to scope all org-level graph state.
    /// Use an empty string for single-tenant deployments.
    #[serde(default)]
    pub tenant_id: String,

    /// Pre-computed aggregate org risk score (0–100).
    /// Sourced from `OrgRiskSnapshot::org_risk_score`.
    /// `None` → org graph not deployed or Redis timeout; engine fails open.
    #[serde(default)]
    pub org_risk_score: Option<u8>,

    /// Cluster membership record if this user/IP belongs to a detected cluster.
    /// `None` → user is not in any known attack cluster.
    #[serde(default)]
    pub cluster_membership: Option<ClusterMembership>,

    /// Per-tenant threshold shift from `TenantRiskProfile`.
    /// Positive = more strict, negative = more lenient. Bounded [-15, +20].
    /// `None` → no tenant profile available; engine uses 0 shift.
    #[serde(default)]
    pub threshold_shift: Option<i8>,

    /// Number of active clusters in this tenant (from OrgRiskSnapshot).
    /// Used for factor description building. 0 if snapshot unavailable.
    #[serde(default)]
    pub org_active_cluster_count: u32,

    // ── Extended velocity counters (overlapping windows) ─────────────────────
    /// key: velocity:login:{uid}:1m
    #[serde(default)]
    pub login_attempts_1m: Option<u32>,
    /// key: velocity:login:{uid}:1h
    #[serde(default)]
    pub login_attempts_1h: Option<u32>,
    /// key: velocity:login:{uid}:24h
    #[serde(default)]
    pub login_attempts_24h: Option<u32>,

    // ── Client-side timestamp ────────────────────────────────────────────────
    /// Client-supplied timestamp from `x-timestamp` header, parsed by adapter.
    /// Used for clock skew detection against `evaluated_at`.
    #[serde(default)]
    pub client_timestamp: Option<DateTime<Utc>>,

    // ── Enhanced device signals ──────────────────────────────────────────────
    /// Client-side device fingerprint hash (canvas, WebGL, fonts, etc.).
    /// Collected by a client-side SDK, sent in a request header.
    #[serde(default)]
    pub device_fingerprint_hash: Option<String>,
    /// TLS JA3/JA4 fingerprint of the client connection.
    /// Extracted by the load balancer or gateway, injected as a header.
    #[serde(default)]
    pub ja3_fingerprint: Option<String>,

    // ── IP reputation ────────────────────────────────────────────────────────
    /// Whether this IP has been seen before for this user.
    /// `None` = adapter cannot determine (legacy), `Some(true)` = known,
    /// `Some(false)` = never seen for this user.
    #[serde(default)]
    pub known_ip_for_user: Option<bool>,

    // ── IP intelligence (from threat feed / IP reputation service) ────────
    //
    // These fields are populated by the adapter from an IP intelligence
    // provider (IPinfo, MaxMind, AbuseIPDB, ip2location, etc.).
    // Separate from `ip_asn_type` because a single IP can be both
    // "Residential" ASN and flagged as VPN (residential VPN exit nodes).

    /// True if the IP is associated with a known VPN provider.
    /// Detected via IP intelligence feed, NOT from user-agent.
    #[serde(default)]
    pub ip_is_vpn: Option<bool>,

    /// True if the IP is a known open/anonymous proxy.
    #[serde(default)]
    pub ip_is_proxy: Option<bool>,

    /// True if the IP is a privacy relay (Apple Private Relay, Cloudflare WARP).
    #[serde(default)]
    pub ip_is_relay: Option<bool>,

    /// Abuse confidence score from AbuseIPDB or similar (0–100).
    /// Higher = more reported abuse from this IP.
    #[serde(default)]
    pub ip_abuse_confidence: Option<u8>,

    // ── Geo intelligence ─────────────────────────────────────────────────────

    /// Per-user or per-org list of allowed country codes (ISO 3166-1 alpha-2).
    /// If set and the request country is NOT in the list, a penalty is applied.
    /// `None` = no geo restriction (all countries allowed).
    #[serde(default)]
    pub geo_allowed_countries: Option<Vec<String>>,

    /// True if the request originates from a sanctioned country
    /// (OFAC, EU sanctions list). Adapter resolves this from the GeoIP country.
    #[serde(default)]
    pub is_sanctioned_country: Option<bool>,

    // ── Bot / browser integrity signals ──────────────────────────────────────
    //
    // Collected by a client-side SDK (JavaScript fingerprinting library)
    // and forwarded as request headers or in the request body.

    /// True if `navigator.webdriver` was detected as `true` on the client.
    /// This is the strongest client-side bot signal — real browsers set this
    /// only when under automation control (Selenium, Puppeteer, Playwright).
    #[serde(default)]
    pub webdriver_detected: Option<bool>,

    /// CAPTCHA verification score (0.0 = bot, 1.0 = human).
    /// From reCAPTCHA v3, hCaptcha Enterprise, or Cloudflare Turnstile.
    /// `None` = CAPTCHA not deployed or not evaluated for this request.
    #[serde(default)]
    pub captcha_score: Option<f32>,

    /// Client screen resolution as "WIDTHxHEIGHT" (e.g., "1920x1080").
    /// Collected by client SDK. Used to detect headless browsers (common
    /// headless resolutions: 800x600, 1024x768 with no variation).
    #[serde(default)]
    pub screen_resolution: Option<String>,

    /// True if the client reports touch capability (`navigator.maxTouchPoints > 0`).
    /// Cross-referenced with user-agent: mobile UA without touch = suspicious.
    #[serde(default)]
    pub touch_capable: Option<bool>,

    // ── Account intelligence ─────────────────────────────────────────────────

    /// Age of the user account in days (from `users.created_at`).
    /// 0 = account created today. `None` = adapter cannot determine.
    #[serde(default)]
    pub account_age_days: Option<u32>,

    /// Whether the user's email address has been verified.
    /// `None` = adapter cannot determine or email verification not deployed.
    #[serde(default)]
    pub email_verified: Option<bool>,

    /// True if the user's email domain is on a known disposable/temporary
    /// email provider list (guerrillamail, tempmail, mailinator, etc.).
    #[serde(default)]
    pub email_domain_disposable: Option<bool>,

    /// True if the credential has been found in a known breach database
    /// (HaveIBeenPwned, internal breach monitoring, etc.).
    /// For WebAuthn credentials, this means the credential public key or
    /// its associated metadata appeared in a compromised dataset.
    #[serde(default)]
    pub breached_credential: Option<bool>,

    // ── Time / behavioral context ────────────────────────────────────────────

    /// The user's typical active hours in UTC, as (start_hour, end_hour).
    /// Computed by the adapter from historical login patterns.
    /// e.g., `Some((8, 22))` means the user typically authenticates between
    /// 08:00 and 22:00 UTC. Activity outside this window is penalized.
    /// `None` = insufficient history to determine (new user or feature not deployed).
    #[serde(default)]
    pub user_typical_hours: Option<(u8, u8)>,

    /// Accept-Language header from the current request.
    #[serde(default)]
    pub accept_language: Option<String>,

    /// Accept-Language header from the last successful login.
    /// Used to detect language switching (possible account takeover).
    #[serde(default)]
    pub previous_accept_language: Option<String>,

    // ── Device trust ─────────────────────────────────────────────────────────

    /// Trust level of the device making this request.
    /// Computed by the adapter from device fingerprint history.
    #[serde(default)]
    pub device_trust_level: Option<DeviceTrustLevel>,

    // ── Policy / tuning knobs ─────────────────────────────────────────────────
    /// Runtime-tunable scoring weights, thresholds, and multipliers.
    /// Populated by the adapter from tenant config; defaults reproduce the
    /// hard-coded v3.1 values.
    #[serde(skip, default = "default_policy_arc")]
    pub policy: Arc<PolicyConfig>,
}

// ─────────────────────────────────────────────────────────────────────────────
// RedactedContext — a log-safe Debug view of RiskContext (Phase F3)
// ─────────────────────────────────────────────────────────────────────────────
//
// The derived `Debug` on `RiskContext` prints every field, including PII
// (username, request_ip, user-agent, JWT fingerprint, accept-language). For
// most engine telemetry you want to see the *shape* of the request —
// identity IDs, action, decision-relevant flags — without the raw PII.
//
// Use this when you control the log site:
//
// ```ignore
// tracing::info!(?RedactedContext(&ctx), "evaluated");
// ```
//
// It elides every free-text / address field and keeps only opaque IDs, the
// action label, and the small set of Option<bool>/Option<u32> signals that
// inform why the engine reached its decision. The derived Debug is still
// available for tests and for ad-hoc `eprintln!("{:#?}", ctx)` in local
// debugging — this wrapper is purely opt-in.

/// Thin log-safe wrapper around `&RiskContext`. See [`crate::context`] module
/// docs for rationale. Implements `Debug`; `Display` is NOT provided (prod
/// log sites should use the `?` formatter to keep field names visible).
pub struct RedactedContext<'a>(pub &'a RiskContext);

impl<'a> std::fmt::Debug for RedactedContext<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c = self.0;
        f.debug_struct("RiskContext")
            // Identity — opaque IDs only. `username` is redacted.
            .field("request_id", &c.request_id)
            .field("user_id", &c.user_id)
            .field("tenant_id", &c.tenant_id)
            .field("credential_id", &c.credential_id)
            .field("action", &c.action.label())
            // Network — redact the IP itself; just record presence + anon class.
            .field("has_request_ip", &c.request_ip.is_some())
            .field("ip_asn_type", &c.ip_asn_type)
            // Credential posture (no IDs or UAs).
            .field("credential_status", &c.credential_status)
            .field("credential_count_for_user", &c.credential_count_for_user)
            // Redis-degradation signals (critical for root-cause).
            .field("redis_signals_degraded", &c.redis_signals_degraded)
            .field("db_signals_degraded", &c.db_signals_degraded)
            // Behavioural flags (small, already-categorical).
            .field("account_locked", &c.account_locked)
            .field("nonce_present", &c.nonce_present)
            .field("nonce_already_used", &c.nonce_already_used)
            .field("recovery_pending", &c.recovery_pending)
            // Org posture.
            .field("org_risk_level", &c.org_risk_level)
            .field("org_risk_score", &c.org_risk_score)
            // Signal presence (not values — redact the numbers too to keep
            // the log line short and not leak velocity fingerprints).
            .field("has_cluster_membership", &c.cluster_membership.is_some())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod redacted_tests {
    use super::*;
    use crate::context::RiskAction;

    fn minimal() -> RiskContext {
        RiskContext {
            request_id: Uuid::nil(),
            evaluated_at: Utc::now(),
            user_id: Uuid::nil(),
            username: "sensitive-user@example.com".into(),
            credential_id: None,
            action: RiskAction::Login,
            resource_id: None,
            credential_status: CredentialStatus::Active,
            credential_created_at: Utc::now(),
            credential_last_used_at: None,
            credential_sign_count_prev: 0,
            credential_sign_count_new: 0,
            credential_registered_ua: Some("Mozilla/5.0 (supersecret)".into()),
            credential_count_for_user: 1,
            prior_audit_event_count: 0,
            last_login_ip: Some("203.0.113.7".parse().unwrap()),
            last_login_at: None,
            last_login_geo: None,
            jwt_issued_at: None,
            jwt_expires_at: None,
            jwt_fingerprint_stored: Some("abcdef1234".into()),
            jwt_fingerprint_current: Some("abcdef1234".into()),
            request_ip: Some("203.0.113.42".parse().unwrap()),
            request_ua: Some("curl/8.0 PII".into()),
            request_geo: None,
            ip_asn_type: None,
            login_attempts_5m: Some(7),
            failed_login_attempts_1h: None,
            actions_executed_5m: None,
            recovery_requests_24h: None,
            device_revocations_1h: None,
            registrations_from_ip_10m: None,
            active_session_count: None,
            account_locked: false,
            recovery_pending: false,
            oauth_authorize_ip: None,
            nonce_present: true,
            nonce_already_used: false,
            org_risk_level: OrgRiskLevel::Normal,
            redis_signals_degraded: false,
            db_signals_degraded: false,
            tenant_id: "t1".into(),
            org_risk_score: None,
            cluster_membership: None,
            threshold_shift: None,
            org_active_cluster_count: 0,
            login_attempts_1m: None,
            login_attempts_1h: None,
            login_attempts_24h: None,
            client_timestamp: None,
            device_fingerprint_hash: None,
            ja3_fingerprint: None,
            known_ip_for_user: None,
            ip_is_vpn: None,
            ip_is_proxy: None,
            ip_is_relay: None,
            ip_abuse_confidence: None,
            geo_allowed_countries: None,
            is_sanctioned_country: None,
            webdriver_detected: None,
            captcha_score: None,
            screen_resolution: None,
            touch_capable: None,
            account_age_days: None,
            email_verified: None,
            email_domain_disposable: None,
            breached_credential: None,
            user_typical_hours: None,
            accept_language: None,
            previous_accept_language: None,
            device_trust_level: None,
            policy: Arc::new(PolicyConfig::default()),
        }
    }

    #[test]
    fn redacted_debug_elides_pii() {
        let ctx = minimal();
        let rendered = format!("{:?}", RedactedContext(&ctx));
        // Identity stays
        assert!(rendered.contains("user_id"));
        assert!(rendered.contains("request_id"));
        assert!(rendered.contains("login")); // action.label() is lowercase
        // PII should NOT appear
        assert!(!rendered.contains("sensitive-user@example.com"));
        assert!(!rendered.contains("203.0.113.42"));
        assert!(!rendered.contains("203.0.113.7"));
        assert!(!rendered.contains("curl/8.0 PII"));
        assert!(!rendered.contains("Mozilla"));
        assert!(!rendered.contains("abcdef1234"));
        // Presence booleans ARE present
        assert!(rendered.contains("has_request_ip"));
    }
}
