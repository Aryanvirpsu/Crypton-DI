/// Logged test suite — same scenarios as active_tests but prints full JSON
/// decision output for each case. Run with:
///   cargo test --test logged_tests -- --nocapture

#[path = "helpers.rs"]
mod helpers;

use std::sync::Mutex;
use async_trait::async_trait;
use std::net::IpAddr;
use uuid::Uuid;

use risk_engine::{
    evaluate, decision::Decision, store::{EscalationEntry, SignalStore, VelocityCounters},
    RiskEngineError,
};

use helpers::{scenarios::*, RiskContextBuilder, TestGeos, TestIps};

// ─────────────────────────────────────────────────────────────────────────────
// Mock store (identical to active_tests)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct TestStore {
    recent_decisions: Mutex<Vec<EscalationEntry>>,
}

#[async_trait]
impl SignalStore for TestStore {
    async fn get_velocity_counters(&self, _user_id: Uuid, _ip: Option<IpAddr>) -> Result<VelocityCounters, RiskEngineError> {
        Ok(VelocityCounters::default())
    }
    async fn get_recent_decisions(&self, _user_id: Uuid) -> Result<Vec<EscalationEntry>, RiskEngineError> {
        Ok(self.recent_decisions.lock().unwrap().clone())
    }
    async fn record_decision(&self, _user_id: Uuid, entry: EscalationEntry) -> Result<(), RiskEngineError> {
        let mut v = self.recent_decisions.lock().unwrap();
        v.insert(0, entry);
        v.truncate(10);
        Ok(())
    }
    async fn lock_account(&self, _user_id: Uuid, _ttl: u64) -> Result<(), RiskEngineError> { Ok(()) }
    async fn block_ip(&self, _ip: IpAddr, _ttl: u64) -> Result<(), RiskEngineError> { Ok(()) }
    async fn check_and_record_escalation(&self, _user_id: Uuid, current_decision: &Decision, score: u8, ts_unix: i64) -> Result<(Decision, bool), RiskEngineError> {
        let mut v = self.recent_decisions.lock().unwrap();
        let outcome = risk_engine::engine::escalation::check_escalation(current_decision, &v);
        let (final_decision, escalated) = match outcome {
            risk_engine::engine::escalation::EscalationOutcome::EscalateTo(d) => (d, true),
            risk_engine::engine::escalation::EscalationOutcome::NoChange => (current_decision.clone(), false),
        };
        v.insert(0, EscalationEntry { score, decision: final_decision.clone(), ts_unix });
        v.truncate(10);
        Ok((final_decision, escalated))
    }
    async fn get_jwt_fingerprint(&self, _key: &str) -> Result<Option<String>, RiskEngineError> { Ok(None) }
    async fn set_jwt_fingerprint(&self, _key: &str, _value: &str, _ttl: u64) -> Result<(), RiskEngineError> { Ok(()) }
    async fn is_nonce_used(&self, _nonce: &str) -> Result<bool, RiskEngineError> { Ok(false) }
    async fn consume_nonce(&self, _nonce: &str) -> Result<(), RiskEngineError> { Ok(()) }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper: pretty-print a decision
// ─────────────────────────────────────────────────────────────────────────────

fn log_decision(label: &str, d: &risk_engine::decision::RiskDecision) {
    println!(
        "\n[{}]\n  decision={:?}  score={}  escalated={}\n  gates={:?}\n  factors={:?}\n  breakdown={{ d={} s={} n={} b={} c={} base={} m_action={} m_org={} a={} final={} }}\n  rules={:?}",
        label,
        d.decision, d.score, d.escalated,
        d.triggered_gates,
        d.contributing_factors.iter().map(|f| format!("{}(+{})", f.name, f.contribution)).collect::<Vec<_>>(),
        d.score_breakdown.d, d.score_breakdown.s, d.score_breakdown.n, d.score_breakdown.b, d.score_breakdown.c,
        d.score_breakdown.base, d.score_breakdown.m_action, d.score_breakdown.m_org,
        d.score_breakdown.a, d.score_breakdown.final_score,
        d.triggered_rules,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Category 1 – Basic Login
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn log_clean_login_allows() {
    let d = evaluate(clean_login(), &TestStore::default()).await;
    log_decision("clean_login → ALLOW", &d);
    assert_eq!(d.decision, Decision::Allow);
}

#[tokio::test]
async fn log_login_low_velocity() {
    let ctx = RiskContextBuilder::new().login_attempts_5m(1).failed_login_attempts_1h(0).build();
    let d = evaluate(ctx, &TestStore::default()).await;
    log_decision("low_velocity → ALLOW", &d);
    assert_eq!(d.decision, Decision::Allow);
    assert!(d.score < 30);
}

#[tokio::test]
async fn log_login_moderate_velocity() {
    let ctx = RiskContextBuilder::new().login_attempts_5m(7).failed_login_attempts_1h(4).build();
    let d = evaluate(ctx, &TestStore::default()).await;
    log_decision("moderate_velocity → challenge or score≥30", &d);
    assert!(d.decision.is_blocking() || d.score >= 30);
}

// ─────────────────────────────────────────────────────────────────────────────
// Category 2 – Brute Force & Velocity
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn log_brute_force_high_velocity() {
    let d = evaluate(brute_force_attack(), &TestStore::default()).await;
    log_decision("brute_force_attack → BLOCKING", &d);
    assert!(d.decision.is_blocking());
    assert!(d.score >= 30);
}

#[tokio::test]
async fn log_brute_force_datacenter() {
    let ctx = RiskContextBuilder::new()
        .login_attempts_5m(12).failed_login_attempts_1h(8)
        .asn_type(risk_engine::context::AsnType::Datacenter).build();
    let d = evaluate(ctx, &TestStore::default()).await;
    log_decision("brute_force_datacenter → BLOCKING score>50", &d);
    assert!(d.decision.is_blocking());
    assert!(d.score > 50);
}

#[tokio::test]
async fn log_excessive_failed_attempts() {
    let ctx = RiskContextBuilder::new().login_attempts_5m(12).failed_login_attempts_1h(20).build();
    let d = evaluate(ctx, &TestStore::default()).await;
    log_decision("excessive_failed_attempts → BLOCKING", &d);
    assert!(d.decision.is_blocking());
}

// ─────────────────────────────────────────────────────────────────────────────
// Category 3 – Credentials & Policy Gates
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn log_revoked_credential_denies() {
    let d = evaluate(revoked_credential(), &TestStore::default()).await;
    log_decision("revoked_credential → DENY", &d);
    assert_eq!(d.decision, Decision::Deny);
}

#[tokio::test]
async fn log_lost_credential_denies() {
    let ctx = RiskContextBuilder::new().credential_status(risk_engine::context::CredentialStatus::Lost).build();
    let d = evaluate(ctx, &TestStore::default()).await;
    log_decision("lost_credential → DENY", &d);
    assert_eq!(d.decision, Decision::Deny);
}

#[tokio::test]
async fn log_new_credential_privileged_action() {
    let d = evaluate(new_cred_privileged_action(), &TestStore::default()).await;
    log_decision("new_cred_privileged → HOLD", &d);
    assert_eq!(d.decision, Decision::Hold);
}

// ─────────────────────────────────────────────────────────────────────────────
// Category 4 – Network & Geo
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn log_tor_sensitive_denies() {
    let d = evaluate(tor_sensitive_action(), &TestStore::default()).await;
    log_decision("tor_sensitive → DENY", &d);
    assert_eq!(d.decision, Decision::Deny);
}

#[tokio::test]
async fn log_vpn_geo_anomaly() {
    let d = evaluate(vpn_geo_anomaly(), &TestStore::default()).await;
    log_decision("vpn_geo_anomaly → score≥30", &d);
    assert!(d.score >= 30);
}

#[tokio::test]
async fn log_same_ip_same_geo_low_risk() {
    let ctx = RiskContextBuilder::new()
        .request_ip(TestIps::home_us()).last_login_ip(TestIps::home_us())
        .request_geo(TestGeos::new_york()).last_login_geo(TestGeos::new_york()).build();
    let d = evaluate(ctx, &TestStore::default()).await;
    log_decision("same_ip_same_geo → low risk", &d);
    assert!(d.score < 30);
}

#[tokio::test]
async fn log_impossible_travel() {
    let ctx = RiskContextBuilder::new()
        .request_ip(TestIps::home_au()).last_login_ip(TestIps::home_us())
        .request_geo(TestGeos::sydney()).last_login_geo(TestGeos::london()).build();
    let d = evaluate(ctx, &TestStore::default()).await;
    log_decision("impossible_travel (Sydney←London 1h) → score>20", &d);
    assert!(d.score > 20);
}

// ─────────────────────────────────────────────────────────────────────────────
// Category 5 – Session & JWT
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn log_jwt_fingerprint_mismatch() {
    let d = evaluate(jwt_mismatch(), &TestStore::default()).await;
    log_decision("jwt_fingerprint_mismatch → BLOCKING", &d);
    assert!(d.decision.is_blocking());
}

#[tokio::test]
async fn log_nonce_replay_denies() {
    let d = evaluate(nonce_replay(), &TestStore::default()).await;
    log_decision("nonce_replay → DENY", &d);
    assert_eq!(d.decision, Decision::Deny);
}

// ─────────────────────────────────────────────────────────────────────────────
// Category 6 – Recovery & Account Management
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn log_recovery_without_pending() {
    let d = evaluate(recovery_not_pending(), &TestStore::default()).await;
    log_decision("recovery_not_pending → DENY", &d);
    assert_eq!(d.decision, Decision::Deny);
}

#[tokio::test]
async fn log_device_revoke_low_risk() {
    let ctx = RiskContextBuilder::new()
        .action(risk_engine::context::RiskAction::DeviceRevoke)
        .device_revocations_1h(0).build();
    let d = evaluate(ctx, &TestStore::default()).await;
    log_decision("device_revoke_low_risk → score≤50", &d);
    assert!(d.score <= 50);
}

// ─────────────────────────────────────────────────────────────────────────────
// Category 7 – Service Degradation
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn log_redis_degraded_sensitive() {
    let d = evaluate(redis_degraded_sensitive(), &TestStore::default()).await;
    log_decision("redis_degraded_sensitive → CHALLENGE", &d);
    assert_eq!(d.decision, Decision::Challenge);
    assert!(d.signals_degraded);
}

#[tokio::test]
async fn log_redis_degraded_non_sensitive() {
    let ctx = RiskContextBuilder::new()
        .action(risk_engine::context::RiskAction::DeviceList)
        .redis_degraded().build();
    let d = evaluate(ctx, &TestStore::default()).await;
    log_decision("redis_degraded_non_sensitive → ALLOW", &d);
    assert_eq!(d.decision, Decision::Allow);
    assert!(d.signals_degraded);
}

// ─────────────────────────────────────────────────────────────────────────────
// Category 8 – Org Risk
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn log_org_under_attack_multiplies_score() {
    let store = TestStore::default();
    let d_normal = evaluate(RiskContextBuilder::new().login_attempts_5m(5).build(), &store).await;
    let d_attack = evaluate(
        RiskContextBuilder::new().org_risk_level(risk_engine::context::OrgRiskLevel::UnderAttack).login_attempts_5m(5).build(),
        &store,
    ).await;
    log_decision("org_normal (5 attempts)", &d_normal);
    log_decision("org_under_attack (5 attempts)", &d_attack);
    assert!(d_attack.score > d_normal.score);
}

// ─────────────────────────────────────────────────────────────────────────────
// Category 9 – Score Breakdown & Factors
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn log_score_breakdown_structure() {
    let d = evaluate(clean_login(), &TestStore::default()).await;
    log_decision("score_breakdown (clean_login)", &d);
    let b = &d.score_breakdown;
    assert!(b.m_action >= 1.0);
    assert!(b.m_org >= 1.0);
}

#[tokio::test]
async fn log_decision_has_factors() {
    let d = evaluate(brute_force_attack(), &TestStore::default()).await;
    log_decision("decision_factors (brute_force)", &d);
    assert!(!d.contributing_factors.is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// Integration – Multi-factor flows
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn log_integration_normal_user_flow() {
    let store = TestStore::default();
    let d1 = evaluate(RiskContextBuilder::new().action(risk_engine::context::RiskAction::Login).login_attempts_5m(1).build(), &store).await;
    let d2 = evaluate(RiskContextBuilder::new().action(risk_engine::context::RiskAction::DeviceList).login_attempts_5m(1).build(), &store).await;
    log_decision("integration/normal – step1 Login", &d1);
    log_decision("integration/normal – step2 DeviceList", &d2);
    assert_eq!(d1.decision, Decision::Allow);
    assert_eq!(d2.decision, Decision::Allow);
}

#[tokio::test]
async fn log_integration_suspicious_to_attack() {
    let store = TestStore::default();
    let d1 = evaluate(
        RiskContextBuilder::new().login_attempts_5m(6).asn_type(risk_engine::context::AsnType::Vpn)
            .request_geo(TestGeos::sydney()).last_login_geo(TestGeos::new_york()).build(),
        &store,
    ).await;
    let d2 = evaluate(
        RiskContextBuilder::new().login_attempts_5m(20).failed_login_attempts_1h(15)
            .asn_type(risk_engine::context::AsnType::Datacenter)
            .request_geo(TestGeos::sydney()).last_login_geo(TestGeos::new_york()).build(),
        &store,
    ).await;
    log_decision("integration/escalation – step1 suspicious", &d1);
    log_decision("integration/escalation – step2 attack", &d2);
    assert!(d2.score > d1.score);
    assert!(d2.decision.is_blocking());
}
