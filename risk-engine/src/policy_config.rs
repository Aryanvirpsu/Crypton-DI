//! Runtime-tunable policy configuration.
//!
//! Every scoring weight, component cap, threshold, and multiplier the engine
//! uses lives on [`PolicyConfig`]. Scoring modules read values off
//! `ctx.policy`, which holds an `Arc<PolicyConfig>` — cheap to clone, and
//! set once per request by the adapter (or per-tenant in future phases).
//!
//! `PolicyConfig::default()` reproduces the hard-coded v3.1 values that used
//! to live as `pub const` in [`crate::config`]. Those constants are retained
//! as the source of truth for the defaults; this struct is a runtime mirror.

use serde::{Deserialize, Serialize};

use crate::config;
use crate::error::RiskEngineError;

fn default_version() -> String {
    "v3.1".to_string()
}

/// All tuning knobs for the risk engine. Construct via [`PolicyConfig::default`]
/// for the standard v3.1 weights, then mutate specific fields for tenant-level
/// overrides or A/B experiments.
///
/// `deny_unknown_fields` makes the deserializer reject any override that names
/// a non-existent knob. Tenant override merges (see [`crate::policy_overrides`])
/// go through a JSON round-trip; without this attribute a typo like
/// `theshold_allow_max` would silently no-op.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyConfig {
    /// Human-readable label identifying this policy (e.g. `"v3.1"`, `"v3.2-shadow"`).
    /// Used by shadow-mode divergence logs and per-tenant overrides so operators
    /// can tell at a glance which tuning produced a given decision.
    #[serde(default = "default_version")]
    pub version: String,

    // ── Component caps ──────────────────────────────────────────────────────
    pub cap_device: u8,
    pub cap_session: u8,
    pub cap_network: u8,
    pub cap_behavioral: u8,
    pub cap_correlation: u8,
    pub max_org_cluster_bias: u8,

    // ── Decision thresholds ─────────────────────────────────────────────────
    pub threshold_allow_max: u8,
    pub threshold_challenge_max: u8,
    pub threshold_hold_max: u8,

    // ── Scoring math ────────────────────────────────────────────────────────
    pub score_damping_threshold: u8,
    pub score_damping_rate: f32,

    // ── Action multipliers (M_action) ───────────────────────────────────────
    pub multiplier_login: f32,
    pub multiplier_register: f32,
    pub multiplier_oauth_complete: f32,
    pub multiplier_device_mark_lost: f32,
    pub multiplier_device_revoke: f32,
    pub multiplier_recovery_start: f32,
    pub multiplier_audit_log_export: f32,
    pub multiplier_recovery_approve: f32,
    pub multiplier_recovery_complete: f32,
    pub multiplier_action_challenge: f32,
    pub multiplier_execute_export_data: f32,
    pub multiplier_execute_rotate_api_key: f32,
    pub multiplier_execute_delete_resource: f32,
    pub multiplier_execute_add_admin: f32,
    pub multiplier_execute_default: f32,

    // ── Device scoring weights (D) ──────────────────────────────────────────
    pub d_credential_lost: u8,
    pub d_credential_age_new_h: u8,
    pub d_credential_age_recent_h: u8,
    pub d_credential_age_stale_h: u8,
    pub d_dormant_device: u8,
    pub d_sign_count_jump_low: u8,
    pub d_sign_count_jump_med: u8,
    pub d_sign_count_jump_high: u8,
    pub d_ua_family_mismatch: u8,
    pub d_webdriver_detected: u8,
    pub d_bot_tls_fingerprint: u8,
    pub d_headless_ua: u8,
    pub d_captcha_fail_critical: u8,
    pub d_captcha_fail_suspicious: u8,
    pub d_touch_mismatch: u8,
    pub d_screen_res_suspicious: u8,
    pub d_no_device_fingerprint: u8,
    pub d_trust_new_device_sensitive: u8,
    pub d_trust_new_device: u8,
    pub d_trust_recognized_sensitive: u8,
    pub d_sole_credential: u8,

    // ── Session scoring weights (S) ─────────────────────────────────────────
    pub s_fingerprint_mismatch: u8,
    pub s_near_expiry: u8,
    pub s_concurrent_high: u8,
    pub s_concurrent_elevated: u8,
    pub s_oauth_ip_mismatch: u8,
    pub s_timestamp_skew: u8,
    pub s_nonce_absent: u8,
    pub s_language_change: u8,
    pub s_out_of_hours: u8,
    pub s_email_not_verified: u8,

    // ── Network scoring weights (N) ─────────────────────────────────────────
    pub n_tor_exit: u8,
    pub n_vpn_ip: u8,
    pub n_proxy_ip: u8,
    pub n_hosting_ip: u8,
    pub n_datacenter_ip: u8,
    pub n_relay_ip: u8,
    pub n_flagged_vpn: u8,
    pub n_flagged_proxy: u8,
    pub n_flagged_relay: u8,
    pub n_abuse_critical: u8,
    pub n_abuse_moderate: u8,
    pub n_abuse_light: u8,
    pub n_rfc1918_bonus: u8,
    pub n_impossible_travel: u8,
    pub n_geo_jump_large: u8,
    pub n_geo_jump_moderate: u8,
    pub n_country_change: u8,
    pub n_geo_not_allowed: u8,
    pub n_unknown_ip_user: u8,
    pub n_new_ip_history: u8,
    pub n_no_ip_history: u8,

    // ── Behavioral velocity weights (B) ─────────────────────────────────────
    pub b_login_burst: u8,
    pub b_failed_high: u8,
    pub b_login_elevated: u8,
    pub b_login_moderate: u8,
    pub b_login_hourly: u8,
    pub b_login_daily: u8,
    pub b_failed_elevated: u8,
    pub b_recovery_velocity: u8,
    pub b_action_velocity: u8,
    pub b_revocation_spree: u8,
    pub b_revocation_elevated: u8,
    pub b_sybil_registration: u8,
    pub b_zero_audit_history: u8,
    pub b_disposable_email: u8,
    pub b_young_account_critical: u8,
    pub b_young_account_new: u8,

    // ── Correlation bonuses (C) ─────────────────────────────────────────────
    pub c_ato_cluster: u8,
    pub c_sybil_spree: u8,
    pub c_automated_scraper: u8,
    pub c_shadow_session: u8,
    pub c_travel_anomaly: u8,
    pub c_cloner_pattern: u8,

    // ── Gate configuration (per-tenant overridable) ────────────────────────
    /// Minutes after credential creation during which privileged actions are
    /// blocked by hard gate 15. Set to 0 to effectively disable.
    pub gate_fresh_credential_minutes: u8,
    /// Whether hard gate 11 blocks VPN/proxy/relay on critical privileged actions.
    pub gate_block_vpn_on_critical: bool,
    /// Whether hard gate 6 blocks Tor exit nodes on sensitive actions.
    pub gate_block_tor_on_sensitive: bool,
    /// Whether hard gate 12 blocks Tor exit nodes on recovery flows.
    pub gate_block_tor_on_recovery: bool,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            version: default_version(),
            cap_device: config::CAP_DEVICE,
            cap_session: config::CAP_SESSION,
            cap_network: config::CAP_NETWORK,
            cap_behavioral: config::CAP_BEHAVIORAL,
            cap_correlation: config::CAP_CORRELATION,
            max_org_cluster_bias: config::MAX_ORG_CLUSTER_BIAS,

            threshold_allow_max: config::THRESHOLD_ALLOW_MAX,
            threshold_challenge_max: config::THRESHOLD_CHALLENGE_MAX,
            threshold_hold_max: config::THRESHOLD_HOLD_MAX,

            score_damping_threshold: config::SCORE_DAMPING_THRESHOLD,
            score_damping_rate: config::SCORE_DAMPING_RATE,

            multiplier_login: config::MULTIPLIER_LOGIN,
            multiplier_register: config::MULTIPLIER_REGISTER,
            multiplier_oauth_complete: config::MULTIPLIER_OAUTH_COMPLETE,
            multiplier_device_mark_lost: config::MULTIPLIER_DEVICE_MARK_LOST,
            multiplier_device_revoke: config::MULTIPLIER_DEVICE_REVOKE,
            multiplier_recovery_start: config::MULTIPLIER_RECOVERY_START,
            multiplier_audit_log_export: config::MULTIPLIER_AUDIT_LOG_EXPORT,
            multiplier_recovery_approve: config::MULTIPLIER_RECOVERY_APPROVE,
            multiplier_recovery_complete: config::MULTIPLIER_RECOVERY_COMPLETE,
            multiplier_action_challenge: config::MULTIPLIER_ACTION_CHALLENGE,
            multiplier_execute_export_data: config::MULTIPLIER_EXECUTE_EXPORT_DATA,
            multiplier_execute_rotate_api_key: config::MULTIPLIER_EXECUTE_ROTATE_API_KEY,
            multiplier_execute_delete_resource: config::MULTIPLIER_EXECUTE_DELETE_RESOURCE,
            multiplier_execute_add_admin: config::MULTIPLIER_EXECUTE_ADD_ADMIN,
            multiplier_execute_default: config::MULTIPLIER_EXECUTE_DEFAULT,

            d_credential_lost: config::D_CREDENTIAL_LOST,
            d_credential_age_new_h: config::D_CREDENTIAL_AGE_NEW_H,
            d_credential_age_recent_h: config::D_CREDENTIAL_AGE_RECENT_H,
            d_credential_age_stale_h: config::D_CREDENTIAL_AGE_STALE_H,
            d_dormant_device: config::D_DORMANT_DEVICE,
            d_sign_count_jump_low: config::D_SIGN_COUNT_JUMP_LOW,
            d_sign_count_jump_med: config::D_SIGN_COUNT_JUMP_MED,
            d_sign_count_jump_high: config::D_SIGN_COUNT_JUMP_HIGH,
            d_ua_family_mismatch: config::D_UA_FAMILY_MISMATCH,
            d_webdriver_detected: config::D_WEBDRIVER_DETECTED,
            d_bot_tls_fingerprint: config::D_BOT_TLS_FINGERPRINT,
            d_headless_ua: config::D_HEADLESS_UA,
            d_captcha_fail_critical: config::D_CAPTCHA_FAIL_CRITICAL,
            d_captcha_fail_suspicious: config::D_CAPTCHA_FAIL_SUSPICIOUS,
            d_touch_mismatch: config::D_TOUCH_MISMATCH,
            d_screen_res_suspicious: config::D_SCREEN_RES_SUSPICIOUS,
            d_no_device_fingerprint: config::D_NO_DEVICE_FINGERPRINT,
            d_trust_new_device_sensitive: config::D_TRUST_NEW_DEVICE_SENSITIVE,
            d_trust_new_device: config::D_TRUST_NEW_DEVICE,
            d_trust_recognized_sensitive: config::D_TRUST_RECOGNIZED_SENSITIVE,
            d_sole_credential: config::D_SOLE_CREDENTIAL,

            s_fingerprint_mismatch: config::S_FINGERPRINT_MISMATCH,
            s_near_expiry: config::S_NEAR_EXPIRY,
            s_concurrent_high: config::S_CONCURRENT_HIGH,
            s_concurrent_elevated: config::S_CONCURRENT_ELEVATED,
            s_oauth_ip_mismatch: config::S_OAUTH_IP_MISMATCH,
            s_timestamp_skew: config::S_TIMESTAMP_SKEW,
            s_nonce_absent: config::S_NONCE_ABSENT,
            s_language_change: config::S_LANGUAGE_CHANGE,
            s_out_of_hours: config::S_OUT_OF_HOURS,
            s_email_not_verified: config::S_EMAIL_NOT_VERIFIED,

            n_tor_exit: config::N_TOR_EXIT,
            n_vpn_ip: config::N_VPN_IP,
            n_proxy_ip: config::N_PROXY_IP,
            n_hosting_ip: config::N_HOSTING_IP,
            n_datacenter_ip: config::N_DATACENTER_IP,
            n_relay_ip: config::N_RELAY_IP,
            n_flagged_vpn: config::N_FLAGGED_VPN,
            n_flagged_proxy: config::N_FLAGGED_PROXY,
            n_flagged_relay: config::N_FLAGGED_RELAY,
            n_abuse_critical: config::N_ABUSE_CRITICAL,
            n_abuse_moderate: config::N_ABUSE_MODERATE,
            n_abuse_light: config::N_ABUSE_LIGHT,
            n_rfc1918_bonus: config::N_RFC1918_BONUS,
            n_impossible_travel: config::N_IMPOSSIBLE_TRAVEL,
            n_geo_jump_large: config::N_GEO_JUMP_LARGE,
            n_geo_jump_moderate: config::N_GEO_JUMP_MODERATE,
            n_country_change: config::N_COUNTRY_CHANGE,
            n_geo_not_allowed: config::N_GEO_NOT_ALLOWED,
            n_unknown_ip_user: config::N_UNKNOWN_IP_USER,
            n_new_ip_history: config::N_NEW_IP_HISTORY,
            n_no_ip_history: config::N_NO_IP_HISTORY,

            b_login_burst: config::B_LOGIN_BURST,
            b_failed_high: config::B_FAILED_HIGH,
            b_login_elevated: config::B_LOGIN_ELEVATED,
            b_login_moderate: config::B_LOGIN_MODERATE,
            b_login_hourly: config::B_LOGIN_HOURLY,
            b_login_daily: config::B_LOGIN_DAILY,
            b_failed_elevated: config::B_FAILED_ELEVATED,
            b_recovery_velocity: config::B_RECOVERY_VELOCITY,
            b_action_velocity: config::B_ACTION_VELOCITY,
            b_revocation_spree: config::B_REVOCATION_SPREE,
            b_revocation_elevated: config::B_REVOCATION_ELEVATED,
            b_sybil_registration: config::B_SYBIL_REGISTRATION,
            b_zero_audit_history: config::B_ZERO_AUDIT_HISTORY,
            b_disposable_email: config::B_DISPOSABLE_EMAIL,
            b_young_account_critical: config::B_YOUNG_ACCOUNT_CRITICAL,
            b_young_account_new: config::B_YOUNG_ACCOUNT_NEW,

            c_ato_cluster: config::C_ATO_CLUSTER,
            c_sybil_spree: config::C_SYBIL_SPREE,
            c_automated_scraper: config::C_AUTOMATED_SCRAPER,
            c_shadow_session: config::C_SHADOW_SESSION,
            c_travel_anomaly: config::C_TRAVEL_ANOMALY,
            c_cloner_pattern: config::C_CLONER_PATTERN,

            gate_fresh_credential_minutes: config::GATE_FRESH_CREDENTIAL_MINUTES,
            gate_block_vpn_on_critical: config::GATE_BLOCK_VPN_ON_CRITICAL,
            gate_block_tor_on_sensitive: config::GATE_BLOCK_TOR_ON_SENSITIVE,
            gate_block_tor_on_recovery: config::GATE_BLOCK_TOR_ON_RECOVERY,
        }
    }
}

impl PolicyConfig {
    /// Verify that invariants hold after construction or override merge.
    /// Returns `ConfigError` on the first violation found.
    pub fn validate(&self) -> Result<(), RiskEngineError> {
        if self.threshold_allow_max >= self.threshold_challenge_max {
            return Err(RiskEngineError::ConfigError(format!(
                "threshold_allow_max ({}) must be < threshold_challenge_max ({})",
                self.threshold_allow_max, self.threshold_challenge_max
            )));
        }
        if self.threshold_challenge_max >= self.threshold_hold_max {
            return Err(RiskEngineError::ConfigError(format!(
                "threshold_challenge_max ({}) must be < threshold_hold_max ({})",
                self.threshold_challenge_max, self.threshold_hold_max
            )));
        }
        // Thresholds must not exceed the score range — a hold_max > 100 makes
        // Deny unreachable via scoring (only hard gates could trigger it).
        if self.threshold_hold_max > 100 {
            return Err(RiskEngineError::ConfigError(format!(
                "threshold_hold_max ({}) exceeds max score (100): Deny becomes unreachable via scoring",
                self.threshold_hold_max
            )));
        }

        // NaN comparisons always return false, so the negated form catches NaN.
        if !(self.score_damping_rate > 0.0 && self.score_damping_rate <= 1.0) {
            return Err(RiskEngineError::ConfigError(format!(
                "score_damping_rate ({}) must be in (0.0, 1.0]",
                self.score_damping_rate
            )));
        }
        if self.score_damping_threshold > 100 {
            return Err(RiskEngineError::ConfigError(format!(
                "score_damping_threshold ({}) exceeds max score (100)",
                self.score_damping_threshold
            )));
        }

        // ── Component caps must be in [0, 100] ──────────────────────────────
        let caps = [
            ("cap_device", self.cap_device),
            ("cap_session", self.cap_session),
            ("cap_network", self.cap_network),
            ("cap_behavioral", self.cap_behavioral),
            ("cap_correlation", self.cap_correlation),
            ("max_org_cluster_bias", self.max_org_cluster_bias),
        ];
        for (name, value) in caps {
            if value > 100 {
                return Err(RiskEngineError::ConfigError(format!(
                    "{name} ({value}) exceeds maximum of 100"
                )));
            }
        }

        let multipliers = [
            ("multiplier_login", self.multiplier_login),
            ("multiplier_register", self.multiplier_register),
            ("multiplier_oauth_complete", self.multiplier_oauth_complete),
            ("multiplier_device_mark_lost", self.multiplier_device_mark_lost),
            ("multiplier_device_revoke", self.multiplier_device_revoke),
            ("multiplier_recovery_start", self.multiplier_recovery_start),
            ("multiplier_audit_log_export", self.multiplier_audit_log_export),
            ("multiplier_recovery_approve", self.multiplier_recovery_approve),
            ("multiplier_recovery_complete", self.multiplier_recovery_complete),
            ("multiplier_action_challenge", self.multiplier_action_challenge),
            ("multiplier_execute_export_data", self.multiplier_execute_export_data),
            ("multiplier_execute_rotate_api_key", self.multiplier_execute_rotate_api_key),
            ("multiplier_execute_delete_resource", self.multiplier_execute_delete_resource),
            ("multiplier_execute_add_admin", self.multiplier_execute_add_admin),
            ("multiplier_execute_default", self.multiplier_execute_default),
        ];
        for (name, value) in multipliers {
            if !value.is_finite() || value <= 0.0 {
                return Err(RiskEngineError::ConfigError(format!(
                    "{name} ({value}) must be a finite positive number"
                )));
            }
        }

        Ok(())
    }

    /// Validate that the merged config does not weaken security bounds
    /// relative to the base config. Called by [`crate::policy_overrides`]
    /// after structural validation passes.
    ///
    /// # Rules
    ///
    /// - **Thresholds can only be lowered** (stricter). Raising any threshold
    ///   widens the Allow/Challenge/Hold bands, which weakens security.
    /// - **Component caps cannot be zeroed.** A cap of 0 disables an entire
    ///   scoring dimension, silently removing signal coverage.
    ///
    /// These bounds ensure that tenant self-service overrides can make policy
    /// *stricter* but never *weaker* than the base config. Relaxations require
    /// a control-plane change to the base config itself.
    pub fn validate_security_bounds(&self, base: &PolicyConfig) -> Result<(), RiskEngineError> {
        // ── Thresholds: can only be lowered (stricter) ──────────────────────
        if self.threshold_allow_max > base.threshold_allow_max {
            return Err(RiskEngineError::ConfigError(format!(
                "override threshold_allow_max ({}) cannot exceed base ({}): \
                 tenants may lower thresholds (stricter) but not raise them",
                self.threshold_allow_max, base.threshold_allow_max
            )));
        }
        if self.threshold_challenge_max > base.threshold_challenge_max {
            return Err(RiskEngineError::ConfigError(format!(
                "override threshold_challenge_max ({}) cannot exceed base ({})",
                self.threshold_challenge_max, base.threshold_challenge_max
            )));
        }
        if self.threshold_hold_max > base.threshold_hold_max {
            return Err(RiskEngineError::ConfigError(format!(
                "override threshold_hold_max ({}) cannot exceed base ({})",
                self.threshold_hold_max, base.threshold_hold_max
            )));
        }

        // ── Component caps: cannot be zeroed ────────────────────────────────
        let caps = [
            ("cap_device", self.cap_device),
            ("cap_session", self.cap_session),
            ("cap_network", self.cap_network),
            ("cap_behavioral", self.cap_behavioral),
        ];
        for (name, value) in caps {
            if value == 0 {
                return Err(RiskEngineError::ConfigError(format!(
                    "override {name} cannot be 0: disabling a scoring dimension \
                     is not permitted via tenant overrides"
                )));
            }
        }

        Ok(())
    }
}
