/// Comprehensive test suite for risk-engine
/// 
/// This module provides organized, easy-to-read tests organized by scenario category.
/// Use these as templates for your own test cases.

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

use helpers::{
    scenarios::*, RiskContextBuilder, TestGeos, TestIps, DecisionAssertions,
};

// ─────────────────────────────────────────────────────────────────────────────
// Mock store implementation
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct TestStore {
    recent_decisions: Mutex<Vec<EscalationEntry>>,
}

#[async_trait]
impl SignalStore for TestStore {
    async fn get_velocity_counters(
        &self,
        _user_id: Uuid,
        _ip: Option<IpAddr>,
    ) -> Result<VelocityCounters, RiskEngineError> {
        Ok(VelocityCounters::default())
    }

    async fn get_recent_decisions(
        &self,
        _user_id: Uuid,
    ) -> Result<Vec<EscalationEntry>, RiskEngineError> {
        Ok(self.recent_decisions.lock().unwrap().clone())
    }

    async fn record_decision(
        &self,
        _user_id: Uuid,
        entry: EscalationEntry,
    ) -> Result<(), RiskEngineError> {
        let mut v = self.recent_decisions.lock().unwrap();
        v.insert(0, entry);
        v.truncate(10);
        Ok(())
    }

    async fn lock_account(&self, _user_id: Uuid, _ttl: u64) -> Result<(), RiskEngineError> {
        Ok(())
    }

    async fn block_ip(&self, _ip: IpAddr, _ttl: u64) -> Result<(), RiskEngineError> {
        Ok(())
    }

    async fn check_and_record_escalation(
        &self,
        _user_id: Uuid,
        current_decision: &Decision,
        score: u8,
        ts_unix: i64,
    ) -> Result<(Decision, bool), RiskEngineError> {
        let mut v = self.recent_decisions.lock().unwrap();
        let outcome = risk_engine::engine::escalation::check_escalation(current_decision, &v);
        let (final_decision, escalated) = match outcome {
            risk_engine::engine::escalation::EscalationOutcome::EscalateTo(d) => (d, true),
            risk_engine::engine::escalation::EscalationOutcome::NoChange => {
                (current_decision.clone(), false)
            }
        };
        v.insert(0, EscalationEntry { score, decision: final_decision.clone(), ts_unix });
        v.truncate(10);
        Ok((final_decision, escalated))
    }

    async fn get_jwt_fingerprint(&self, _key: &str) -> Result<Option<String>, RiskEngineError> {
        Ok(None)
    }

    async fn set_jwt_fingerprint(
        &self,
        _key: &str,
        _value: &str,
        _ttl: u64,
    ) -> Result<(), RiskEngineError> {
        Ok(())
    }

    async fn is_nonce_used(&self, _nonce: &str) -> Result<bool, RiskEngineError> {
        Ok(false)
    }

    async fn consume_nonce(&self, _nonce: &str) -> Result<(), RiskEngineError> {
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CATEGORY 1: Basic Login Scenarios
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_clean_login_allows() {
    let ctx = clean_login();
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;
    
    DecisionAssertions::assert_allow(decision.score, decision.decision.clone());
    assert_eq!(decision.decision, Decision::Allow);
}

#[tokio::test]
async fn test_login_with_low_velocity() {
    let ctx = RiskContextBuilder::new()
        .login_attempts_5m(1)
        .failed_login_attempts_1h(0)
        .build();
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;
    
    assert_eq!(decision.decision, Decision::Allow);
    assert!(decision.score < 30);
}

#[tokio::test]
async fn test_login_with_moderate_velocity() {
    let ctx = RiskContextBuilder::new()
        .login_attempts_5m(7)
        .failed_login_attempts_1h(4)
        .build();
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;

    // Moderate velocity should trigger challenge
    assert!(decision.decision.is_blocking() || decision.score >= 30);
}

// ─────────────────────────────────────────────────────────────────────────────
// CATEGORY 2: Brute Force & Velocity Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_brute_force_high_velocity() {
    let ctx = brute_force_attack();
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;
    
    DecisionAssertions::assert_blocking(decision.score, decision.decision.clone());
    assert!(decision.score >= 30);
}

#[tokio::test]
async fn test_brute_force_from_datacenter() {
    let ctx = RiskContextBuilder::new()
        .login_attempts_5m(12)
        .failed_login_attempts_1h(8)
        .asn_type(risk_engine::context::AsnType::Datacenter)
        .build();
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;
    
    assert!(decision.decision.is_blocking());
    // Datacenter IP + high velocity = elevated risk
    assert!(decision.score > 50);
}

#[tokio::test]
async fn test_excessive_failed_attempts() {
    // Under v3.1, failed-only velocity is intentionally damped; realistic
    // brute-force also has elevated login attempts. Compound signal here.
    let ctx = RiskContextBuilder::new()
        .login_attempts_5m(12)
        .failed_login_attempts_1h(20)
        .build();
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;

    DecisionAssertions::assert_blocking(decision.score, decision.decision.clone());
}

// ─────────────────────────────────────────────────────────────────────────────
// CATEGORY 3: Credential & Policy Gate Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_revoked_credential_denies() {
    let ctx = revoked_credential();
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;
    
    DecisionAssertions::assert_deny(decision.score, decision.decision.clone());
    assert_eq!(decision.decision, Decision::Deny);
    assert!(!decision.triggered_gates.is_empty());
}

#[tokio::test]
async fn test_new_credential_privileged_action() {
    // Design: PrivilegedActionOnFreshCredential is a "trust-spike protection"
    // gate — routes to Hold + admin approval, NOT Deny.
    let ctx = new_cred_privileged_action();
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;

    assert_eq!(decision.decision, Decision::Hold);
    assert!(decision
        .triggered_gates
        .contains(&risk_engine::decision::HardGate::PrivilegedActionOnFreshCredential));
}

#[tokio::test]
async fn test_lost_credential_denies() {
    let ctx = RiskContextBuilder::new()
        .credential_status(risk_engine::context::CredentialStatus::Lost)
        .build();
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;
    
    assert_eq!(decision.decision, Decision::Deny);
}

// ─────────────────────────────────────────────────────────────────────────────
// CATEGORY 4: Network & Geographic Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_tor_exit_sensitive_action_denies() {
    let ctx = tor_sensitive_action();
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;
    
    DecisionAssertions::assert_deny(decision.score, decision.decision.clone());
    assert!(!decision.triggered_gates.is_empty());
}

#[tokio::test]
async fn test_vpn_with_geo_anomaly() {
    let ctx = vpn_geo_anomaly();
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;
    
    // Geographic anomaly + VPN should elevate score
    assert!(decision.score >= 30);
}

#[tokio::test]
async fn test_same_ip_same_geo_low_risk() {
    let ctx = RiskContextBuilder::new()
        .request_ip(TestIps::home_us())
        .last_login_ip(TestIps::home_us())
        .request_geo(TestGeos::new_york())
        .last_login_geo(TestGeos::new_york())
        .build();
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;
    
    assert!(decision.score < 30);
}

#[tokio::test]
async fn test_impossible_travel_scenario() {
    // Sydney to London in 1 hour = impossible
    let ctx = RiskContextBuilder::new()
        .request_ip(TestIps::home_au())
        .last_login_ip(TestIps::home_us())
        .request_geo(TestGeos::sydney())
        .last_login_geo(TestGeos::london())
        .build();
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;
    
    // Should flag geographic anomaly
    assert!(decision.score > 20);
}

// ─────────────────────────────────────────────────────────────────────────────
// CATEGORY 5: Session & JWT Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_jwt_fingerprint_mismatch_holds() {
    let ctx = jwt_mismatch();
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;
    
    // Fingerprint mismatch on sensitive action should block
    assert!(decision.decision.is_blocking());
}

#[tokio::test]
async fn test_nonce_replay_denies() {
    let ctx = nonce_replay();
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;
    
    DecisionAssertions::assert_deny(decision.score, decision.decision.clone());
    assert!(!decision.triggered_gates.is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// CATEGORY 6: Recovery & Account Management Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_recovery_without_pending_denies() {
    let ctx = recovery_not_pending();
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;
    
    DecisionAssertions::assert_deny(decision.score, decision.decision.clone());
}

#[tokio::test]
async fn test_device_revoke_low_risk() {
    let ctx = RiskContextBuilder::new()
        .action(risk_engine::context::RiskAction::DeviceRevoke)
        .device_revocations_1h(0)
        .build();
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;
    
    // Single revocation should not immediately deny
    assert!(decision.score <= 50);
}

// ─────────────────────────────────────────────────────────────────────────────
// CATEGORY 7: Service Degradation Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_redis_degraded_sensitive_action_challenges() {
    let ctx = redis_degraded_sensitive();
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;
    
    // Signals degraded on sensitive should challenge
    assert_eq!(decision.decision, risk_engine::decision::Decision::Challenge);
    assert!(decision.signals_degraded);
}

#[tokio::test]
async fn test_redis_degraded_non_sensitive_allows() {
    let ctx = RiskContextBuilder::new()
        .action(risk_engine::context::RiskAction::DeviceList)
        .redis_degraded()
        .build();
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;
    
    // Non-sensitive action should allow even if Redis down
    assert_eq!(decision.decision, Decision::Allow);
    assert!(decision.signals_degraded);
}

// ─────────────────────────────────────────────────────────────────────────────
// CATEGORY 8: Organization Risk Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_org_under_attack_multiplies_score() {
    let ctx_normal = RiskContextBuilder::new()
        .login_attempts_5m(5)
        .build();
    
    let ctx_attack = RiskContextBuilder::new()
        .org_risk_level(risk_engine::context::OrgRiskLevel::UnderAttack)
        .login_attempts_5m(5)
        .build();
    
    let store = TestStore::default();
    let d_normal = evaluate(ctx_normal, &store).await;
    let d_attack = evaluate(ctx_attack, &store).await;
    
    // Same velocity, but org under attack should score higher
    assert!(d_attack.score > d_normal.score);
}

// ─────────────────────────────────────────────────────────────────────────────
// CATEGORY 9: Custom Scenario Builder Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_builder_fluent_api() {
    let ctx = RiskContextBuilder::new()
        .action(risk_engine::context::RiskAction::Login)
        .login_attempts_5m(2)
        .failed_login_attempts_1h(0)
        .asn_type(risk_engine::context::AsnType::Residential)
        .request_geo(TestGeos::new_york())
        .credential_age(chrono::Duration::days(30))
        .build();
    
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;
    
    // Should be a relatively safe login
    assert!(decision.score < 40);
}

#[tokio::test]
async fn test_builder_high_risk_scenario() {
    let ctx = RiskContextBuilder::new()
        .action(risk_engine::context::RiskAction::ActionExecute { 
            action_name: "grant_access".into() 
        })
        .login_attempts_5m(10)
        .failed_login_attempts_1h(5)
        .asn_type(risk_engine::context::AsnType::Datacenter)
        .request_geo(TestGeos::moscow())
        .last_login_geo(TestGeos::new_york())
        .credential_age(chrono::Duration::minutes(10))
        .audit_event_count(0)
        .build();
    
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;
    
    // Should be blocked or held
    assert!(decision.decision.is_blocking() || decision.score >= 60);
}

// ─────────────────────────────────────────────────────────────────────────────
// CATEGORY 10: Score Breakdown & Analysis Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_score_breakdown_structure() {
    let ctx = clean_login();
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;
    
    // Verify score breakdown is populated
    let b = &decision.score_breakdown;
    assert!(b.d >= 0);
    assert!(b.s >= 0);
    assert!(b.n >= 0);
    assert!(b.b >= 0);
    assert!(b.m_action >= 1.0);
    assert!(b.m_org >= 1.0);
}

#[tokio::test]
async fn test_decision_has_factors() {
    let ctx = brute_force_attack();
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;
    
    // Blocking decision should have factors explaining the decision
    assert!(!decision.contributing_factors.is_empty(),
            "decision should include factors explaining the score");
}

// ─────────────────────────────────────────────────────────────────────────────
// INTEGRATION TESTS - Complex Multi-Factor Scenarios
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn integration_normal_user_flow() {
    let store = TestStore::default();
    
    // Scenario: Normal user progression
    let ctx1 = RiskContextBuilder::new()
        .action(risk_engine::context::RiskAction::Login)
        .login_attempts_5m(1)
        .build();
    let d1 = evaluate(ctx1, &store).await;
    assert_eq!(d1.decision, Decision::Allow);
    
    // Then accessing a device
    let ctx2 = RiskContextBuilder::new()
        .action(risk_engine::context::RiskAction::DeviceList)
        .login_attempts_5m(1)
        .build();
    let d2 = evaluate(ctx2, &store).await;
    assert_eq!(d2.decision, Decision::Allow);
}

#[tokio::test]
async fn integration_suspicious_to_attack() {
    let store = TestStore::default();
    
    // Start with suspicious (moderate risk)
    let ctx1 = RiskContextBuilder::new()
        .login_attempts_5m(6)
        .asn_type(risk_engine::context::AsnType::Vpn)
        .request_geo(TestGeos::sydney())
        .last_login_geo(TestGeos::new_york())
        .build();
    let d1 = evaluate(ctx1.clone(), &store).await;
    let initial_score = d1.score;
    
    // Then escalate to clear attack — compound velocity + network + geo
    let ctx2 = RiskContextBuilder::new()
        .login_attempts_5m(20)
        .failed_login_attempts_1h(15)
        .asn_type(risk_engine::context::AsnType::Datacenter)
        .request_geo(TestGeos::sydney())
        .last_login_geo(TestGeos::new_york())
        .build();
    let d2 = evaluate(ctx2, &store).await;
    assert!(
        d2.score > initial_score,
        "d2={} d1={}",
        d2.score,
        initial_score
    );
    assert!(d2.decision.is_blocking());
}
