//! # config
//!
//! Centralized scoring constants, multipliers, and thresholds.
//! v3.1: Rebalanced signal weights + adjusted decision boundaries for reduced attacker-signal sensitivity.

// ─────────────────────────────────────────────────────────────────────────────
// COMPONENT CAPS
// ─────────────────────────────────────────────────────────────────────────────

/// Maximum points allowed for the Device (D) component.
pub const CAP_DEVICE: u8 = 50;

/// Maximum points allowed for the Session (S) component.
pub const CAP_SESSION: u8 = 50;

/// Maximum points allowed for the Network (N) component.
pub const CAP_NETWORK: u8 = 50;

/// Maximum points allowed for the Behavioral (B) component.
pub const CAP_BEHAVIORAL: u8 = 50;

/// Maximum additive bias from org/cluster signals.
pub const MAX_ORG_CLUSTER_BIAS: u8 = 25;

// ─────────────────────────────────────────────────────────────────────────────
// DECISION THRESHOLDS (v3.1)
// ─────────────────────────────────────────────────────────────────────────────

pub const THRESHOLD_ALLOW_MAX: u8 = 39;
pub const THRESHOLD_CHALLENGE_MAX: u8 = 64;
pub const THRESHOLD_HOLD_MAX: u8 = 89;
// Any score >= 85 is DENY.

// ─────────────────────────────────────────────────────────────────────────────
// SCORING MATH CONSTANTS (v3.1)
// ─────────────────────────────────────────────────────────────────────────────

/// Point at which scoring growth begins to damp (diminishing returns).
pub const SCORE_DAMPING_THRESHOLD: u8 = 60;

/// Rate at which excess points contribute to final score after threshold.
pub const SCORE_DAMPING_RATE: f32 = 0.45;

// ─────────────────────────────────────────────────────────────────────────────
// ACTION MULTIPLIERS (M_action)
// ─────────────────────────────────────────────────────────────────────────────

pub const MULTIPLIER_LOGIN: f32 = 1.2;
pub const MULTIPLIER_REGISTER: f32 = 1.3;
pub const MULTIPLIER_OAUTH_COMPLETE: f32 = 1.3;
pub const MULTIPLIER_DEVICE_MARK_LOST: f32 = 1.5;
pub const MULTIPLIER_DEVICE_REVOKE: f32 = 1.8;
pub const MULTIPLIER_RECOVERY_START: f32 = 2.0;
pub const MULTIPLIER_AUDIT_LOG_EXPORT: f32 = 2.0;
pub const MULTIPLIER_RECOVERY_APPROVE: f32 = 2.2;
pub const MULTIPLIER_RECOVERY_COMPLETE: f32 = 2.5;

// ActionExecute multipliers
pub const MULTIPLIER_EXECUTE_EXPORT_DATA: f32 = 2.0;
pub const MULTIPLIER_EXECUTE_ROTATE_API_KEY: f32 = 2.5;
pub const MULTIPLIER_EXECUTE_DELETE_RESOURCE: f32 = 2.5;
pub const MULTIPLIER_EXECUTE_ADD_ADMIN: f32 = 2.8;
pub const MULTIPLIER_EXECUTE_DEFAULT: f32 = 1.5;

// ─────────────────────────────────────────────────────────────────────────────
// DEVICE SCORING (D)
// ─────────────────────────────────────────────────────────────────────────────

pub const D_CREDENTIAL_LOST: u8 = 40;
pub const D_CREDENTIAL_AGE_NEW_H: u8 = 15;
pub const D_CREDENTIAL_AGE_RECENT_H: u8 = 10;
pub const D_CREDENTIAL_AGE_STALE_H: u8 = 5;
pub const D_DORMANT_DEVICE: u8 = 8;

pub const D_SIGN_COUNT_JUMP_LOW: u8 = 15;
pub const D_SIGN_COUNT_JUMP_MED: u8 = 30;
pub const D_SIGN_COUNT_JUMP_HIGH: u8 = 50;

pub const D_UA_FAMILY_MISMATCH: u8 = 15;

/// Reduced (previously 50)
pub const D_WEBDRIVER_DETECTED: u8 = 45;

/// Reduced (previously 40)
pub const D_BOT_TLS_FINGERPRINT: u8 = 35;

pub const D_HEADLESS_UA: u8 = 5;
pub const D_CAPTCHA_FAIL_CRITICAL: u8 = 20;
pub const D_CAPTCHA_FAIL_SUSPICIOUS: u8 = 10;
pub const D_TOUCH_MISMATCH: u8 = 12;
pub const D_SCREEN_RES_SUSPICIOUS: u8 = 8;
pub const D_NO_DEVICE_FINGERPRINT: u8 = 5;

pub const D_TRUST_NEW_DEVICE_SENSITIVE: u8 = 10;
pub const D_TRUST_NEW_DEVICE: u8 = 5;
pub const D_TRUST_RECOGNIZED_SENSITIVE: u8 = 3;
pub const D_SOLE_CREDENTIAL: u8 = 5;

// ─────────────────────────────────────────────────────────────────────────────
// SESSION SCORING (S)
// ─────────────────────────────────────────────────────────────────────────────

pub const S_FINGERPRINT_MISMATCH: u8 = 45;
pub const S_NEAR_EXPIRY: u8 = 5;
pub const S_CONCURRENT_HIGH: u8 = 20;
pub const S_CONCURRENT_ELEVATED: u8 = 10;
pub const S_OAUTH_IP_MISMATCH: u8 = 15;
pub const S_TIMESTAMP_SKEW: u8 = 8;
pub const S_NONCE_ABSENT: u8 = 10;
pub const S_LANGUAGE_CHANGE: u8 = 5;
pub const S_OUT_OF_HOURS: u8 = 5;
pub const S_EMAIL_NOT_VERIFIED: u8 = 8;

// ─────────────────────────────────────────────────────────────────────────────
// NETWORK SCORING (N)
// ─────────────────────────────────────────────────────────────────────────────

pub const N_TOR_EXIT: u8 = 50;

/// Reduced (previously 4)
pub const N_VPN_IP: u8 = 4;

pub const N_PROXY_IP: u8 = 20;
pub const N_HOSTING_IP: u8 = 20;
pub const N_DATACENTER_IP: u8 = 15;
pub const N_RELAY_IP: u8 = 5;
pub const N_FLAGGED_VPN: u8 = 10;
pub const N_FLAGGED_PROXY: u8 = 15;
pub const N_FLAGGED_RELAY: u8 = 5;
pub const N_ABUSE_CRITICAL: u8 = 30;
pub const N_ABUSE_MODERATE: u8 = 15;
pub const N_ABUSE_LIGHT: u8 = 8;
pub const N_RFC1918_BONUS: u8 = 10;

/// Reduced (previously 40)
pub const N_IMPOSSIBLE_TRAVEL: u8 = 40;

/// Reduced (previously 35)
pub const N_GEO_JUMP_LARGE: u8 = 20;

pub const N_GEO_JUMP_MODERATE: u8 = 15;

/// Reduced (previously 15)
pub const N_COUNTRY_CHANGE: u8 = 10;

pub const N_GEO_NOT_ALLOWED: u8 = 25;
pub const N_UNKNOWN_IP_USER: u8 = 12;
pub const N_NEW_IP_HISTORY: u8 = 8;
pub const N_NO_IP_HISTORY: u8 = 5;

// ─────────────────────────────────────────────────────────────────────────────
// BEHAVIORAL VELOCITY SCORING (B)
// ─────────────────────────────────────────────────────────────────────────────

/// Reduced (previously 45)
pub const B_LOGIN_BURST: u8 = 35;

/// Reduced (previously 40)
pub const B_FAILED_HIGH: u8 = 28;

pub const B_LOGIN_ELEVATED: u8 = 20;
pub const B_LOGIN_MODERATE: u8 = 10;
pub const B_LOGIN_HOURLY: u8 = 15;
pub const B_LOGIN_DAILY: u8 = 10;

pub const B_FAILED_ELEVATED: u8 = 20;

pub const B_RECOVERY_VELOCITY: u8 = 25;

/// Increased slightly for balance
pub const B_ACTION_VELOCITY: u8 = 22;

/// Reduced (previously 40)
pub const B_REVOCATION_SPREE: u8 = 30;

pub const B_REVOCATION_ELEVATED: u8 = 15;
pub const B_SYBIL_REGISTRATION: u8 = 25;

/// Increased (was 5)
pub const B_ZERO_AUDIT_HISTORY: u8 = 8;

pub const B_DISPOSABLE_EMAIL: u8 = 20;

/// Increased (was 15)
pub const B_YOUNG_ACCOUNT_CRITICAL: u8 = 20;

pub const B_YOUNG_ACCOUNT_NEW: u8 = 8;

// ─────────────────────────────────────────────────────────────────────────────
// CORRELATION BONUSES (C)
// ─────────────────────────────────────────────────────────────────────────────

/// Reduced (previously 55)
pub const C_ATO_CLUSTER: u8 = 40;

/// Reduced (previously 45)
pub const C_SYBIL_SPREE: u8 = 35;

pub const C_AUTOMATED_SCRAPER: u8 = 30;
pub const C_SHADOW_SESSION: u8 = 40;
pub const C_TRAVEL_ANOMALY: u8 = 15;
pub const C_CLONER_PATTERN: u8 = 45;