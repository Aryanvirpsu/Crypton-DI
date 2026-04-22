//! Shadow-mode (Phase 5) integration tests.
//!
//! Verifies that [`evaluate_with_shadow`] (a) returns the active decision as
//! the authoritative verdict, (b) flags divergence when the shadow policy
//! differs, and (c) does not double-write escalation memory on the shadow
//! pass — the `ReadOnlyStore` wrapper must swallow writes.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{Duration, Utc};
use uuid::Uuid;

use risk_engine::{
    context::{CredentialStatus, OrgRiskLevel, RiskAction, RiskContext},
    decision::Decision,
    evaluate_with_shadow,
    policy_config::PolicyConfig,
    store::{EscalationEntry, SignalStore, VelocityCounters},
    RiskEngineError,
};

#[derive(Default)]
struct MockStore {
    recent: Mutex<Vec<EscalationEntry>>,
    records: Mutex<u32>,
    lock_calls: Mutex<u32>,
    block_calls: Mutex<u32>,
    consume_calls: Mutex<u32>,
}

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
        Ok(self.recent.lock().unwrap().clone())
    }
    async fn record_decision(
        &self,
        _u: Uuid,
        e: EscalationEntry,
    ) -> Result<(), RiskEngineError> {
        self.recent.lock().unwrap().insert(0, e);
        *self.records.lock().unwrap() += 1;
        Ok(())
    }
    async fn check_and_record_escalation(
        &self,
        _u: Uuid,
        d: &Decision,
        score: u8,
        ts_unix: i64,
    ) -> Result<(Decision, bool), RiskEngineError> {
        let mut v = self.recent.lock().unwrap();
        let outcome = risk_engine::engine::escalation::check_escalation(d, &v);
        let (final_d, escalated) = match outcome {
            risk_engine::engine::escalation::EscalationOutcome::EscalateTo(x) => (x, true),
            risk_engine::engine::escalation::EscalationOutcome::NoChange => (d.clone(), false),
        };
        v.insert(0, EscalationEntry { score, decision: final_d.clone(), ts_unix });
        *self.records.lock().unwrap() += 1;
        Ok((final_d, escalated))
    }
    async fn lock_account(&self, _u: Uuid, _t: u64) -> Result<(), RiskEngineError> {
        *self.lock_calls.lock().unwrap() += 1;
        Ok(())
    }
    async fn block_ip(&self, _i: IpAddr, _t: u64) -> Result<(), RiskEngineError> {
        *self.block_calls.lock().unwrap() += 1;
        Ok(())
    }
    async fn get_jwt_fingerprint(&self, _k: &str) -> Result<Option<String>, RiskEngineError> {
        Ok(None)
    }
    async fn set_jwt_fingerprint(
        &self,
        _k: &str,
        _v: &str,
        _t: u64,
    ) -> Result<(), RiskEngineError> {
        Ok(())
    }
    async fn is_nonce_used(&self, _n: &str) -> Result<bool, RiskEngineError> {
        Ok(false)
    }
    async fn consume_nonce(&self, _n: &str) -> Result<(), RiskEngineError> {
        *self.consume_calls.lock().unwrap() += 1;
        Ok(())
    }
}

fn minimal_login_ctx() -> RiskContext {
    RiskContext {
        request_id: Uuid::new_v4(),
        evaluated_at: Utc::now(),
        user_id: Uuid::new_v4(),
        username: "alice".into(),
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
        last_login_ip: Some(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))),
        last_login_at: Some(Utc::now() - Duration::hours(24)),
        last_login_geo: None,
        jwt_issued_at: Some(Utc::now() - Duration::minutes(5)),
        jwt_expires_at: Some(Utc::now() + Duration::minutes(55)),
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
        tenant_id: "test-tenant".into(),
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

#[tokio::test]
async fn identical_policies_do_not_diverge() {
    let ctx = minimal_login_ctx();
    let store = MockStore::default();
    let policy = Arc::new(PolicyConfig::default());

    let result = evaluate_with_shadow(ctx, &store, policy.clone(), policy).await;

    assert!(!result.diverged, "identical policies must not diverge");
    assert_eq!(result.active.decision, result.shadow.decision);
    assert_eq!(result.active.score, result.shadow.score);
}

#[tokio::test]
async fn divergent_threshold_flips_shadow_decision() {
    let ctx = minimal_login_ctx();
    let store = MockStore::default();

    let active = Arc::new(PolicyConfig::default()); // Allow for a clean login
    let mut strict = PolicyConfig::default();
    strict.version = "strict".into();
    strict.threshold_allow_max = 0; // Shadow refuses to allow anything
    strict.threshold_challenge_max = 100;
    let shadow = Arc::new(strict);

    let result = evaluate_with_shadow(ctx, &store, active, shadow).await;

    assert_eq!(result.active.decision, Decision::Allow);
    assert_eq!(result.shadow.decision, Decision::Challenge);
    assert!(result.diverged);
}

#[tokio::test]
async fn shadow_pass_does_not_double_write() {
    // Scoring path (not a hard gate) — active run records exactly once; shadow
    // path must be silent on `record_decision`.
    let ctx = minimal_login_ctx();
    let store = MockStore::default();
    let policy = Arc::new(PolicyConfig::default());

    let _ = evaluate_with_shadow(ctx, &store, policy.clone(), policy).await;

    let records = *store.records.lock().unwrap();
    assert_eq!(records, 1, "shadow pass must not call record_decision");
    assert_eq!(*store.lock_calls.lock().unwrap(), 0);
    assert_eq!(*store.block_calls.lock().unwrap(), 0);
}
