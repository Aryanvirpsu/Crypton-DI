//! End-to-end test for Phase 6 per-tenant policy overrides.
//!
//! Proves that a tenant-specific override loaded from an in-memory
//! [`PolicyStore`], cached via [`CachedPolicyStore`], and merged onto the
//! default [`PolicyConfig`] actually reaches [`evaluate`] and changes the
//! decision it produces for an otherwise-identical context.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use chrono::{Duration as ChronoDuration, Utc};
use uuid::Uuid;

use risk_engine::{
    context::{CredentialStatus, OrgRiskLevel, RiskAction, RiskContext},
    decision::Decision,
    evaluate,
    policy_config::PolicyConfig,
    policy_overrides::{CachedPolicyStore, PolicyOverrides, PolicyStore},
    store::{EscalationEntry, SignalStore, VelocityCounters},
    RiskEngineError,
};

// ─────────────────────────────────────────────────────────────────────────────
// In-memory PolicyStore for tests
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct InMemoryPolicyStore {
    rows: Mutex<std::collections::HashMap<String, PolicyOverrides>>,
}

impl InMemoryPolicyStore {
    fn set(&self, tenant_id: &str, overrides: PolicyOverrides) {
        self.rows.lock().unwrap().insert(tenant_id.into(), overrides);
    }
}

#[async_trait]
impl PolicyStore for InMemoryPolicyStore {
    async fn load_overrides(&self, tenant_id: &str) -> Result<PolicyOverrides, RiskEngineError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .get(tenant_id)
            .cloned()
            .unwrap_or_default())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SignalStore mock
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct MockStore;

#[async_trait]
impl SignalStore for MockStore {
    async fn get_velocity_counters(
        &self,
        _u: Uuid,
        _ip: Option<IpAddr>,
    ) -> Result<VelocityCounters, RiskEngineError> {
        Ok(VelocityCounters::default())
    }
    async fn get_recent_decisions(
        &self,
        _u: Uuid,
    ) -> Result<Vec<EscalationEntry>, RiskEngineError> {
        Ok(vec![])
    }
    async fn record_decision(
        &self,
        _u: Uuid,
        _e: EscalationEntry,
    ) -> Result<(), RiskEngineError> {
        Ok(())
    }
    async fn check_and_record_escalation(
        &self,
        _u: Uuid,
        d: &Decision,
        _s: u8,
        _t: i64,
    ) -> Result<(Decision, bool), RiskEngineError> {
        Ok((d.clone(), false))
    }
    async fn lock_account(&self, _u: Uuid, _t: u64) -> Result<(), RiskEngineError> { Ok(()) }
    async fn block_ip(&self, _i: IpAddr, _t: u64) -> Result<(), RiskEngineError> { Ok(()) }
    async fn get_jwt_fingerprint(&self, _k: &str) -> Result<Option<String>, RiskEngineError> { Ok(None) }
    async fn set_jwt_fingerprint(
        &self,
        _k: &str,
        _v: &str,
        _t: u64,
    ) -> Result<(), RiskEngineError> {
        Ok(())
    }
    async fn is_nonce_used(&self, _n: &str) -> Result<bool, RiskEngineError> { Ok(false) }
    async fn consume_nonce(&self, _n: &str) -> Result<(), RiskEngineError> { Ok(()) }
}

fn login_ctx(policy: Arc<PolicyConfig>) -> RiskContext {
    RiskContext {
        request_id: Uuid::new_v4(),
        evaluated_at: Utc::now(),
        user_id: Uuid::new_v4(),
        username: "alice".into(),
        credential_id: Some(Uuid::new_v4()),
        action: RiskAction::Login,
        resource_id: None,
        credential_status: CredentialStatus::Active,
        credential_created_at: Utc::now() - ChronoDuration::days(30),
        credential_last_used_at: Some(Utc::now() - ChronoDuration::hours(12)),
        credential_sign_count_prev: 100,
        credential_sign_count_new: 101,
        credential_registered_ua: Some("Mozilla/5.0 Chrome/120".into()),
        credential_count_for_user: 2,
        prior_audit_event_count: 100,
        last_login_ip: Some(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))),
        last_login_at: Some(Utc::now() - ChronoDuration::hours(24)),
        last_login_geo: None,
        jwt_issued_at: Some(Utc::now() - ChronoDuration::minutes(5)),
        jwt_expires_at: Some(Utc::now() + ChronoDuration::minutes(55)),
        jwt_fingerprint_stored: Some("fp".into()),
        jwt_fingerprint_current: Some("fp".into()),
        request_ip: Some(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))),
        request_ua: Some("Mozilla/5.0 Chrome/120".into()),
        request_geo: None,
        ip_asn_type: None,
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
        tenant_id: "acme".into(),
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
        policy,
    }
}

#[tokio::test]
async fn tenant_override_changes_live_decision() {
    // Stricter tenant: allow_max=0 forces everything to Challenge.
    let inner = Arc::new(InMemoryPolicyStore::default());
    inner.set(
        "acme",
        PolicyOverrides::new()
            .insert("threshold_allow_max", 0_u8)
            .insert("threshold_challenge_max", 100_u8)
            .insert("version", "acme-strict"),
    );
    let cache = CachedPolicyStore::new(inner, Duration::from_secs(60));

    let base = PolicyConfig::default();
    let effective = PolicyConfig::for_tenant(&base, &cache, "acme").await.unwrap();
    assert_eq!(effective.version, "acme-strict");

    let store = MockStore::default();
    let ctx = login_ctx(Arc::new(effective));

    let decision = evaluate(ctx, &store).await;
    assert_eq!(decision.decision, Decision::Challenge);
}

#[tokio::test]
async fn missing_tenant_falls_back_to_defaults() {
    let inner = Arc::new(InMemoryPolicyStore::default());
    let cache = CachedPolicyStore::new(inner, Duration::from_secs(60));

    let base = PolicyConfig::default();
    let effective = PolicyConfig::for_tenant(&base, &cache, "unknown-tenant")
        .await
        .unwrap();

    assert_eq!(effective, base);
}
