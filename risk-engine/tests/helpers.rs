/// Test utilities and helpers for risk-engine scenarios
/// 
/// This module provides builders, fixtures, and assertion helpers for testing
/// the risk engine with various scenarios.

use std::net::{IpAddr, Ipv4Addr};
use chrono::{Duration, Utc};
use uuid::Uuid;

use std::sync::Arc;

use risk_engine::{
    context::{
        AsnType, CredentialStatus, GeoPoint, OrgRiskLevel, RiskAction, RiskContext,
    },
    decision::Decision,
    policy_config::PolicyConfig,
};

// ─────────────────────────────────────────────────────────────────────────────
// Test Fixtures
// ─────────────────────────────────────────────────────────────────────────────

/// Standard test user ID for reproducible tests
pub const TEST_USER_ID: &str = "550e8400-e29b-41d4-a716-446655440000";

/// Standard test IP addresses
pub struct TestIps;

impl TestIps {
    pub fn home_us() -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))
    }
    
    pub fn home_au() -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1))
    }
    
    pub fn datacenter() -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(54, 239, 28, 30))
    }
    
    pub fn tor_exit() -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(5, 135, 176, 65))
    }
}

/// Standard geographic locations
pub struct TestGeos;

impl TestGeos {
    pub fn new_york() -> GeoPoint {
        GeoPoint {
            lat: 40.7128,
            lon: -74.0060,
            country_code: "US".into(),
            city: Some("New York".into()),
        }
    }
    
    pub fn sydney() -> GeoPoint {
        GeoPoint {
            lat: -33.8688,
            lon: 151.2093,
            country_code: "AU".into(),
            city: Some("Sydney".into()),
        }
    }
    
    pub fn london() -> GeoPoint {
        GeoPoint {
            lat: 51.5074,
            lon: -0.1278,
            country_code: "GB".into(),
            city: Some("London".into()),
        }
    }
    
    pub fn moscow() -> GeoPoint {
        GeoPoint {
            lat: 55.7558,
            lon: 37.6173,
            country_code: "RU".into(),
            city: Some("Moscow".into()),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// RiskContext Builder
// ─────────────────────────────────────────────────────────────────────────────

/// Fluent builder for constructing RiskContext test data
pub struct RiskContextBuilder {
    ctx: RiskContext,
}

impl RiskContextBuilder {
    /// Create a new builder with sensible defaults
    pub fn new() -> Self {
        Self {
            ctx: RiskContext {
                request_id: Uuid::new_v4(),
                evaluated_at: Utc::now(),
                user_id: Uuid::parse_str(TEST_USER_ID).unwrap(),
                username: "test_user".into(),
                credential_id: Some(Uuid::new_v4()),
                action: RiskAction::Login,
                resource_id: None,
                credential_status: CredentialStatus::Active,
                credential_created_at: Utc::now() - Duration::days(30),
                credential_last_used_at: Some(Utc::now() - Duration::hours(12)),
                credential_sign_count_prev: 100,
                credential_sign_count_new: 101,
                credential_registered_ua: Some("Mozilla/5.0 Chrome/120".into()),
                credential_count_for_user: 2,
                prior_audit_event_count: 100,
                last_login_ip: Some(TestIps::home_us()),
                last_login_at: Some(Utc::now() - Duration::hours(24)),
                last_login_geo: Some(TestGeos::new_york()),
                jwt_issued_at: Some(Utc::now() - Duration::minutes(5)),
                jwt_expires_at: Some(Utc::now() + Duration::minutes(55)),
                jwt_fingerprint_stored: Some("test_fingerprint".into()),
                jwt_fingerprint_current: Some("test_fingerprint".into()),
                webdriver_detected: Some(false),
                device_fingerprint_hash: Some("test_device_fingerprint".into()),
                request_ip: Some(TestIps::home_us()),
                request_ua: Some("Mozilla/5.0 Chrome/120".into()),
                request_geo: Some(TestGeos::new_york()),
                ip_asn_type: Some(AsnType::Residential),
                login_attempts_5m: Some(1),
                failed_login_attempts_1h: Some(0),
                actions_executed_5m: Some(0),
                recovery_requests_24h: Some(0),
                device_revocations_1h: Some(0),
                registrations_from_ip_10m: Some(0),
                active_session_count: Some(1),
                account_locked: false,
                recovery_pending: false,
                oauth_authorize_ip: None,
                nonce_present: true,
                nonce_already_used: false,
                org_risk_level: OrgRiskLevel::Normal,
                redis_signals_degraded: false,
                db_signals_degraded: false,
                tenant_id: "test-tenant".into(),
                org_risk_score: None,
                cluster_membership: None,
                threshold_shift: None,
                org_active_cluster_count: 0,
                login_attempts_1m: None,
                login_attempts_1h: None,
                login_attempts_24h: None,
                client_timestamp: None,
                ja3_fingerprint: None,
                known_ip_for_user: None,
                ip_is_vpn: None,
                ip_is_proxy: None,
                ip_is_relay: None,
                ip_abuse_confidence: None,
                geo_allowed_countries: None,
                is_sanctioned_country: None,
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
            },
        }
    }

    /// Override the policy config for this context (Phase 2: runtime tuning).
    pub fn policy(mut self, policy: PolicyConfig) -> Self {
        self.ctx.policy = Arc::new(policy);
        self
    }
    
    /// Set the action being performed
    pub fn action(mut self, action: RiskAction) -> Self {
        self.ctx.action = action;
        self
    }
    
    /// Set credential status
    pub fn credential_status(mut self, status: CredentialStatus) -> Self {
        self.ctx.credential_status = status;
        self
    }
    
    /// Set credential age
    pub fn credential_age(mut self, duration: Duration) -> Self {
        self.ctx.credential_created_at = Utc::now() - duration;
        self
    }
    
    /// Set login attempts in 5 minutes
    pub fn login_attempts_5m(mut self, count: u32) -> Self {
        self.ctx.login_attempts_5m = Some(count);
        self
    }
    
    /// Set failed login attempts in 1 hour
    pub fn failed_login_attempts_1h(mut self, count: u32) -> Self {
        self.ctx.failed_login_attempts_1h = Some(count);
        self
    }
    
    /// Set device revocations in 1 hour
    pub fn device_revocations_1h(mut self, count: u32) -> Self {
        self.ctx.device_revocations_1h = Some(count);
        self
    }
    
    /// Set ASN type (for network risk)
    pub fn asn_type(mut self, asn: AsnType) -> Self {
        self.ctx.ip_asn_type = Some(asn);
        self
    }
    
    /// Set request IP
    pub fn request_ip(mut self, ip: IpAddr) -> Self {
        self.ctx.request_ip = Some(ip);
        self
    }
    
    /// Set last login IP
    pub fn last_login_ip(mut self, ip: IpAddr) -> Self {
        self.ctx.last_login_ip = Some(ip);
        self
    }
    
    /// Set request geo location
    pub fn request_geo(mut self, geo: GeoPoint) -> Self {
        self.ctx.request_geo = Some(geo);
        self
    }
    
    /// Set last login geo location
    pub fn last_login_geo(mut self, geo: GeoPoint) -> Self {
        self.ctx.last_login_geo = Some(geo);
        self
    }
    
    /// Set JWT fingerprint (stored vs current)
    pub fn jwt_fingerprints(mut self, stored: &str, current: &str) -> Self {
        self.ctx.jwt_fingerprint_stored = Some(stored.into());
        self.ctx.jwt_fingerprint_current = Some(current.into());
        self
    }
    
    /// Mark nonce as already used (replay detection)
    pub fn nonce_replayed(mut self) -> Self {
        self.ctx.nonce_already_used = true;
        self
    }
    
    /// Set recovery pending state
    pub fn recovery_pending(mut self, pending: bool) -> Self {
        self.ctx.recovery_pending = pending;
        self
    }
    
    /// Set Redis signals as degraded
    pub fn redis_degraded(mut self) -> Self {
        self.ctx.redis_signals_degraded = true;
        self
    }
    
    /// Set organization risk level
    pub fn org_risk_level(mut self, level: OrgRiskLevel) -> Self {
        self.ctx.org_risk_level = level;
        self
    }
    
    /// Set credential count for user
    pub fn credential_count(mut self, count: u32) -> Self {
        self.ctx.credential_count_for_user = count;
        self
    }
    
    /// Set audit event count
    pub fn audit_event_count(mut self, count: u32) -> Self {
        self.ctx.prior_audit_event_count = count;
        self
    }
    
    /// Build and return the context
    pub fn build(self) -> RiskContext {
        self.ctx
    }
}

impl Default for RiskContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Assertion Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Assertion helpers for testing decisions
pub struct DecisionAssertions;

impl DecisionAssertions {
    /// Assert decision is Allow with score < 30
    pub fn assert_allow(score: u8, decision: Decision) {
        assert_eq!(decision, Decision::Allow, "expected Allow decision");
        assert!(score < 30, "Allow decision must have score < 30");
    }
    
    /// Assert decision is Challenge with score in 30-59 range
    pub fn assert_challenge(score: u8, decision: Decision) {
        assert!(decision.is_blocking() == false || decision == Decision::Challenge,
                "expected Challenge decision, got {:?}", decision);
        assert!((30..=59).contains(&score), 
                "Challenge decision should have score 30-59, got {}", score);
    }
    
    /// Assert decision is Hold with score 60-84
    pub fn assert_hold(score: u8, decision: Decision) {
        assert_eq!(decision, Decision::Hold, "expected Hold decision");
        assert!((60..=84).contains(&score),
                "Hold decision should have score 60-84, got {}", score);
    }
    
    /// Assert decision is Deny with score 85-100
    pub fn assert_deny(score: u8, decision: Decision) {
        assert_eq!(decision, Decision::Deny, "expected Deny decision");
        assert!((85..=100).contains(&score),
                "Deny decision should have score 85-100, got {}", score);
    }
    
    /// Assert decision is blocking (Challenge, Hold, or Deny)
    pub fn assert_blocking(_score: u8, decision: Decision) {
        assert!(decision.is_blocking(), 
                "expected blocking decision, got {:?}", decision);
    }
    
    /// Assert decision provides a required action
    pub fn assert_has_required_action(decision: &risk_engine::decision::RiskDecision) {
        assert!(decision.required_action.is_some(),
                "expected required_action to be set");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario Presets (Ready-to-use test contexts)
// ─────────────────────────────────────────────────────────────────────────────

pub mod scenarios {
    use super::*;
    
    /// Clean, low-risk login from familiar device
    pub fn clean_login() -> RiskContext {
        RiskContextBuilder::new()
            .action(RiskAction::Login)
            .login_attempts_5m(1)
            .failed_login_attempts_1h(0)
            .build()
    }
    
    /// Brute force attack: many login attempts
    pub fn brute_force_attack() -> RiskContext {
        RiskContextBuilder::new()
            .action(RiskAction::Login)
            .login_attempts_5m(15)
            .failed_login_attempts_1h(10)
            .asn_type(AsnType::Datacenter)
            .build()
    }
    
    /// New credential being used to execute privileged action
    pub fn new_cred_privileged_action() -> RiskContext {
        RiskContextBuilder::new()
            .action(RiskAction::ActionExecute { action_name: "add_admin".into() })
            .credential_age(Duration::minutes(2))
            .credential_count(1)
            .audit_event_count(0)
            .build()
    }
    
    /// Tor exit node attempting sensitive action
    pub fn tor_sensitive_action() -> RiskContext {
        RiskContextBuilder::new()
            .action(RiskAction::DeviceRevoke)
            .asn_type(AsnType::Tor)
            .request_ip(TestIps::tor_exit())
            .build()
    }
    
    /// VPN with large geographic jump
    pub fn vpn_geo_anomaly() -> RiskContext {
        RiskContextBuilder::new()
            .action(RiskAction::Login)
            .asn_type(AsnType::Vpn)
            .request_geo(TestGeos::sydney())
            .last_login_geo(TestGeos::new_york())
            .build()
    }
    
    /// Revoked credential
    pub fn revoked_credential() -> RiskContext {
        RiskContextBuilder::new()
            .action(RiskAction::Login)
            .credential_status(CredentialStatus::Revoked)
            .build()
    }
    
    /// Redis service degraded, sensitive action
    pub fn redis_degraded_sensitive() -> RiskContext {
        RiskContextBuilder::new()
            .action(RiskAction::DeviceRevoke)
            .redis_degraded()
            .build()
    }
    
    /// Nonce replay attack
    pub fn nonce_replay() -> RiskContext {
        RiskContextBuilder::new()
            .action(RiskAction::Login)
            .nonce_replayed()
            .build()
    }
    
    /// Recovery without pending request
    pub fn recovery_not_pending() -> RiskContext {
        RiskContextBuilder::new()
            .action(RiskAction::RecoveryComplete)
            .recovery_pending(false)
            .build()
    }
    
    /// JWT fingerprint mismatch (possible token hijacking)
    pub fn jwt_mismatch() -> RiskContext {
        RiskContextBuilder::new()
            .action(RiskAction::DeviceRevoke)
            .jwt_fingerprints("aabbcc1122334455", "ddeeff6677889900")
            .build()
    }
    
    /// Organization under attack
    pub fn org_under_attack() -> RiskContext {
        RiskContextBuilder::new()
            .action(RiskAction::Login)
            .org_risk_level(OrgRiskLevel::UnderAttack)
            .login_attempts_5m(6) // moderate but triggers multiplier
            .build()
    }
}
