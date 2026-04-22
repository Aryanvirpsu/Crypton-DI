use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{Duration, Utc};
use uuid::Uuid;

use risk_engine::{
    context::{
        AsnType, CredentialStatus, GeoPoint, OrgRiskLevel, RequiredAction, RiskAction, RiskContext,
    },
    decision::Decision,
    evaluate,
    org_graph::{AttackType, ClusterMembership},
    policy_config::PolicyConfig,
    store::{EscalationEntry, SignalStore, VelocityCounters},
    RiskEngineError,
};

// ─────────────────────────────────────────────────────────────────────────────
// In-memory mock store for tests
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct MockStore {
    recent_decisions: Mutex<Vec<EscalationEntry>>,
    account_locked: Mutex<bool>,
}

#[async_trait]
impl SignalStore for MockStore {
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
        *self.account_locked.lock().unwrap() = true;
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
// Test context builder
// ─────────────────────────────────────────────────────────────────────────────

fn clean_login_ctx() -> RiskContext {
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
        last_login_geo: Some(GeoPoint {
            lat: 40.7128,
            lon: -74.0060,
            country_code: "US".into(),
            city: Some("New York".into()),
        }),
        jwt_issued_at: Some(Utc::now() - Duration::minutes(5)),
        jwt_expires_at: Some(Utc::now() + Duration::minutes(55)),
        jwt_fingerprint_stored: Some("test_fingerprint".into()),
        jwt_fingerprint_current: Some("test_fingerprint".into()),
        request_ip: Some(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))),
        request_ua: Some("Mozilla/5.0 Chrome/120".into()),
        request_geo: Some(GeoPoint {
            lat: 40.7128,
            lon: -74.0060,
            country_code: "US".into(),
            city: Some("New York".into()),
        }),
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
        // Org-graph fields — neutral defaults for baseline tests
        tenant_id: "test-tenant".into(),
        org_risk_score: None,
        cluster_membership: None,
        threshold_shift: None,
        org_active_cluster_count: 0,
        // Extended signals
        login_attempts_1m: None,
        login_attempts_1h: None,
        login_attempts_24h: None,
        client_timestamp: None,
        device_fingerprint_hash: None,
        ja3_fingerprint: None,
        known_ip_for_user: None,
        // IP intelligence
        ip_is_vpn: None,
        ip_is_proxy: None,
        ip_is_relay: None,
        ip_abuse_confidence: None,
        // Geo intelligence
        geo_allowed_countries: None,
        is_sanctioned_country: None,
        // Bot / browser integrity
        webdriver_detected: None,
        captcha_score: None,
        screen_resolution: None,
        touch_capable: None,
        // Account intelligence
        account_age_days: None,
        email_verified: None,
        email_domain_disposable: None,
        breached_credential: None,
        // Time / behavioral
        user_typical_hours: None,
        accept_language: None,
        previous_accept_language: None,
        // Device trust
        device_trust_level: None,
        policy: Arc::new(PolicyConfig::default()),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn clean_login_allows() {
    let ctx = clean_login_ctx();
    let store = MockStore::default();
    let decision = evaluate(ctx, &store).await;
    assert_eq!(decision.decision, Decision::Allow);
    assert!(decision.score < 30);
    assert!(decision.required_action.is_none());
}

#[tokio::test]
async fn revoked_credential_hard_denies() {
    let mut ctx = clean_login_ctx();
    ctx.credential_status = CredentialStatus::Revoked;
    let store = MockStore::default();
    let decision = evaluate(ctx, &store).await;
    assert_eq!(decision.decision, Decision::Deny);
    assert_eq!(decision.score, 100);
    assert!(!decision.triggered_gates.is_empty());
}

#[tokio::test]
async fn nonce_replay_hard_denies() {
    let mut ctx = clean_login_ctx();
    ctx.nonce_already_used = true;
    let store = MockStore::default();
    let decision = evaluate(ctx, &store).await;
    assert_eq!(decision.decision, Decision::Deny);
}

#[tokio::test]
async fn tor_on_sensitive_action_hard_denies() {
    let mut ctx = clean_login_ctx();
    ctx.ip_asn_type = Some(AsnType::Tor);
    ctx.action = RiskAction::DeviceRevoke;
    let store = MockStore::default();
    let decision = evaluate(ctx, &store).await;
    assert_eq!(decision.decision, Decision::Deny);
}

#[tokio::test]
async fn brute_force_login_scores_high() {
    let mut ctx = clean_login_ctx();
    ctx.login_attempts_5m = Some(15);
    ctx.failed_login_attempts_1h = Some(10);
    let store = MockStore::default();
    let decision = evaluate(ctx, &store).await;
    // High velocity should push score into CHALLENGE or higher
    assert!(decision.decision.is_blocking());
}

#[tokio::test]
async fn brand_new_credential_add_admin_holds_for_admin_approval() {
    // Design: fresh-credential-on-privileged is intentionally routed to Hold
    // (score 75) with required admin approval — this is trust-spike protection,
    // not a hard deny.
    let mut ctx = clean_login_ctx();
    ctx.credential_created_at = Utc::now() - Duration::minutes(2);
    ctx.credential_count_for_user = 1;
    ctx.prior_audit_event_count = 0;
    ctx.action = RiskAction::ActionExecute {
        action_name: "add_admin".into(),
    };
    let store = MockStore::default();
    let decision = evaluate(ctx, &store).await;
    assert_eq!(decision.decision, Decision::Hold);
    assert_eq!(
        decision.required_action,
        Some(RequiredAction::AdminApproval)
    );
}

#[tokio::test]
async fn jwt_fingerprint_mismatch_triggers_challenge() {
    let mut ctx = clean_login_ctx();
    ctx.jwt_fingerprint_stored = Some("aabbcc".into());
    ctx.jwt_fingerprint_current = Some("ddeeff".into()); // different
    ctx.action = RiskAction::DeviceRevoke;
    let store = MockStore::default();
    let decision = evaluate(ctx, &store).await;
    assert!(decision.decision.is_blocking());
    // Hard gate short-circuits scoring; confirm the structural-failure gate fired.
    // Gate name uses PascalCase via Debug — match by gate rather than factor name.
    assert!(decision
        .triggered_gates
        .contains(&risk_engine::decision::HardGate::SensitiveActionOnMismatch));
}

#[tokio::test]
async fn three_recent_challenges_escalate_to_hold() {
    let ctx = clean_login_ctx();
    let store = MockStore::default();

    // Pre-populate 3 recent CHALLENGE decisions
    {
        let mut v = store.recent_decisions.lock().unwrap();
        for i in 0u8..3 {
            v.push(EscalationEntry {
                score: 45,
                decision: Decision::Challenge,
                ts_unix: Utc::now().timestamp() - (i as i64 * 30),
            });
        }
    }

    // Tune signals so base decision lands in Challenge (40–64), not Hold —
    // otherwise escalation (Challenge→Hold) won't apply.
    let mut ctx = ctx;
    ctx.login_attempts_5m = Some(7);    // B_LOGIN_ELEVATED (20)
    ctx.failed_login_attempts_1h = Some(4); // B_FAILED_ELEVATED (20)
    ctx.action = RiskAction::Login;

    let decision = evaluate(ctx, &store).await;
    // Should be escalated to Hold
    assert!(
        matches!(decision.decision, Decision::Hold | Decision::Deny),
        "expected Hold or Deny due to escalation, got {:?}",
        decision.decision
    );
    // escalated flag must be set
    assert!(decision.escalated);
}

#[tokio::test]
async fn recovery_complete_with_no_pending_denies() {
    let mut ctx = clean_login_ctx();
    ctx.action = RiskAction::RecoveryComplete;
    ctx.recovery_pending = false;
    let store = MockStore::default();
    let decision = evaluate(ctx, &store).await;
    assert_eq!(decision.decision, Decision::Deny);
}

#[tokio::test]
async fn redis_degraded_on_sensitive_action_challenges() {
    let mut ctx = clean_login_ctx();
    ctx.redis_signals_degraded = true;
    ctx.action = RiskAction::DeviceRevoke; // sensitive
    let store = MockStore::default();
    let decision = evaluate(ctx, &store).await;
    assert_eq!(decision.decision, Decision::Challenge);
    assert!(decision.signals_degraded);
    assert_eq!(decision.required_action, Some(RequiredAction::StepUpWebAuthn));
}

#[tokio::test]
async fn redis_degraded_on_non_sensitive_allows() {
    let mut ctx = clean_login_ctx();
    ctx.redis_signals_degraded = true;
    ctx.action = RiskAction::DeviceList; // non-sensitive
    let store = MockStore::default();
    let decision = evaluate(ctx, &store).await;
    assert_eq!(decision.decision, Decision::Allow);
    assert!(decision.signals_degraded);
}

#[tokio::test]
async fn decision_includes_score_breakdown() {
    let ctx = clean_login_ctx();
    let store = MockStore::default();
    let decision = evaluate(ctx, &store).await;
    // Breakdown fields should be internally consistent
    let b = &decision.score_breakdown;
    let expected_base = b.d + b.s + b.n + b.b;
    assert_eq!(b.base, expected_base);
    assert!(b.m_action >= 1.0);
    assert!(b.m_org >= 1.0);
}

#[tokio::test]
async fn org_under_attack_multiplies_score() {
    // Same context, different org risk level
    let mut ctx_normal = clean_login_ctx();
    ctx_normal.login_attempts_5m = Some(6); // moderate velocity

    let mut ctx_attack = ctx_normal.clone();
    ctx_attack.org_risk_level = OrgRiskLevel::UnderAttack;

    let store_a = MockStore::default();
    let store_b = MockStore::default();

    let d_normal = evaluate(ctx_normal, &store_a).await;
    let d_attack = evaluate(ctx_attack, &store_b).await;

    assert!(
        d_attack.score > d_normal.score,
        "UnderAttack org should score higher: {} vs {}",
        d_attack.score,
        d_normal.score
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Org-graph integration tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn fail_open_no_org_data_unchanged() {
    // When all org fields are None (the default), adjusted_score must equal
    // base_score and no cluster factors should be emitted.
    // This is the backward-compat / fail-open guarantee.
    let ctx = clean_login_ctx(); // org_risk_score=None, cluster_membership=None, threshold_shift=None
    let store = MockStore::default();
    let decision = evaluate(ctx, &store).await;

    assert_eq!(
        decision.adjusted_score, decision.base_score,
        "no org data: adjusted_score must equal base_score"
    );
    assert!(
        decision.cluster_factors.is_empty(),
        "no org data: cluster_factors must be empty"
    );
    assert_eq!(
        decision.applied_threshold_shift, 0,
        "no org data: applied_threshold_shift must be zero"
    );
}

#[tokio::test]
async fn org_bias_nudges_adjusted_score() {
    // org_risk_score=80 → compute_org_bias(80) = 5 + floor(20/7) = 7
    // adjusted_score must be base_score + 7, and an org factor must appear.
    let mut ctx = clean_login_ctx();
    ctx.org_risk_score = Some(80);
    ctx.org_active_cluster_count = 3;
    let store = MockStore::default();
    let decision = evaluate(ctx, &store).await;

    assert!(
        decision.adjusted_score > decision.base_score,
        "org_risk_score=80 should raise adjusted_score above base: adjusted={} base={}",
        decision.adjusted_score,
        decision.base_score
    );
    // The org-level factor record must be present
    assert!(
        !decision.cluster_factors.is_empty(),
        "cluster_factors must contain the org-level factor"
    );
    // Verify the exact bias: base_score=5, org_bias=7, threshold_shift=0
    // → adjusted_score = 5 + 7 = 12
    assert_eq!(
        decision.adjusted_score,
        decision.base_score + 7,
        "expected adjusted = base + 7 for org_risk_score=80"
    );
}

#[tokio::test]
async fn cluster_membership_adds_bias_to_adjusted_score() {
    // SessionHijackCluster severity=1.5, risk=90, confidence=0.85
    // cluster_bias = floor(90 * 0.85 * 1.5 / 10) = floor(11.475) = 11
    let mut ctx = clean_login_ctx();
    ctx.cluster_membership = Some(ClusterMembership {
        cluster_id: "test-tenant:cluster:ip:1.2.3.4".into(),
        attack_type: AttackType::SessionHijackCluster,
        cluster_risk_score: 90,
        member_count: 15,
        confidence: 0.85,
    });
    let store = MockStore::default();
    let decision = evaluate(ctx, &store).await;

    assert!(
        decision.adjusted_score > decision.base_score,
        "cluster membership should raise adjusted_score: adjusted={} base={}",
        decision.adjusted_score,
        decision.base_score
    );
    assert!(
        !decision.cluster_factors.is_empty(),
        "cluster_factors must be populated when cluster_membership is set"
    );
    // Exact: base=5, cluster_bias=11, threshold_shift=0 → adjusted=16
    assert_eq!(
        decision.adjusted_score,
        decision.base_score + 11,
        "expected adjusted = base + 11 for this cluster membership"
    );
}

#[tokio::test]
async fn cluster_factors_populated_with_meaningful_data() {
    let mut ctx = clean_login_ctx();
    ctx.cluster_membership = Some(ClusterMembership {
        cluster_id: "test-tenant:cluster:user:attacker".into(),
        attack_type: AttackType::CredentialStuffing,
        cluster_risk_score: 70,
        member_count: 20,
        confidence: 0.9,
    });
    let store = MockStore::default();
    let decision = evaluate(ctx, &store).await;

    assert!(!decision.cluster_factors.is_empty());
    let factor = &decision.cluster_factors[0];
    assert!(factor.contribution > 0, "cluster factor contribution must be nonzero");
    assert!(
        !factor.description.is_empty(),
        "cluster factor description must be non-empty"
    );
    assert!(
        factor.description.contains("credential_stuffing"),
        "description should name the attack type: {}",
        factor.description
    );
}

#[tokio::test]
async fn positive_threshold_shift_promotes_allow_to_challenge() {
    // Clean login base_score = 0 (Login m=1.0, D+S+N+B=0, A=0 fingerprinted JWT)
    // org_risk_score=80 → org_bias=7  →  after_bias = 0+7 = 7
    // threshold_shift=+23             →  effective_score = 7+23 = 30  → Challenge
    let mut ctx = clean_login_ctx();
    ctx.org_risk_score = Some(80);
    ctx.threshold_shift = Some(23);

    let store = MockStore::default();
    let decision = evaluate(ctx, &store).await;

    // Raw score must remain in Allow range (no change to the formula output)
    assert!(
        decision.base_score < 30,
        "base_score should still be below Allow threshold: {}",
        decision.base_score
    );
    // But the threshold-adjusted score must be in Challenge range
    assert!(
        decision.adjusted_score >= 30,
        "threshold_shift=+23 should push adjusted_score into Challenge: got {}",
        decision.adjusted_score
    );
    assert!(
        decision.decision.is_blocking(),
        "expected blocking decision after positive threshold shift, got {:?}",
        decision.decision
    );
    assert_eq!(decision.applied_threshold_shift, 23);
}

#[tokio::test]
async fn negative_threshold_shift_relaxes_borderline_score() {
    // login_attempts_5m=6 → B=10, base=floor(10*1.2)+5=17  (Challenge if ≥30 but 17<30 so Allow)
    // Set threshold_shift=-15 → effective_score = 17-15 = 2 → firmly in Allow
    // Also verify adjusted_score < base_score.
    let mut ctx = clean_login_ctx();
    ctx.login_attempts_5m = Some(6); // B=10 → final_score=17
    ctx.threshold_shift = Some(-15);

    let store = MockStore::default();
    let decision = evaluate(ctx, &store).await;

    assert!(
        decision.adjusted_score < decision.base_score,
        "negative shift should reduce effective score: adjusted={} base={}",
        decision.adjusted_score,
        decision.base_score
    );
    assert_eq!(decision.decision, Decision::Allow);
    assert_eq!(decision.applied_threshold_shift, -15);
}

#[tokio::test]
async fn combined_org_and_cluster_bias_capped_at_25() {
    // org_risk_score=100 → org_bias=10 (max)
    // RecoveryAbuse cluster: risk=100, confidence=1.0, severity=1.4
    // cluster_bias = floor(100*1.0*1.4/10) = 14 → capped at 15 → 14
    // total_bias = min(10+14, 25) = 24
    let mut ctx = clean_login_ctx();
    ctx.org_risk_score = Some(100);
    ctx.org_active_cluster_count = 10;
    ctx.cluster_membership = Some(ClusterMembership {
        cluster_id: "test-tenant:cluster:user:abuse_root".into(),
        attack_type: AttackType::RecoveryAbuse,
        cluster_risk_score: 100,
        member_count: 50,
        confidence: 1.0,
    });

    let store = MockStore::default();
    let decision = evaluate(ctx, &store).await;

    // adjusted_score = (base_score + total_bias).min(100)
    let expected_bias: u8 = 24; // org_bias=10 + cluster_bias=14
    assert_eq!(
        decision.adjusted_score,
        decision.base_score.saturating_add(expected_bias).min(100),
        "combined bias should be exactly {}: base={} adjusted={}",
        expected_bias,
        decision.base_score,
        decision.adjusted_score
    );
    // Both org and cluster factors must appear
    assert_eq!(
        decision.cluster_factors.len(),
        2,
        "expected 2 cluster_factors (org + cluster membership)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// PolicyConfig override verification
//
// Proves that tweaking policy values actually changes the engine output.
// This is the phase-2 acceptance test — without it, the whole PolicyConfig
// plumbing could silently be inert.
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn policy_weight_override_changes_score() {
    // Scenario: moderate login velocity. Under default weights this produces
    // a non-zero Behavioral score. Raising b_login_moderate must raise the
    // final score; zeroing it must drop it.
    let mk_ctx = || {
        let mut c = clean_login_ctx();
        c.login_attempts_5m = Some(4); // >3, <=5 → b_login_moderate path
        c
    };

    // Default
    let default_score = {
        let ctx = mk_ctx();
        let store = MockStore::default();
        evaluate(ctx, &store).await.score
    };

    // Amplified: triple the moderate velocity weight
    let amplified_score = {
        let mut policy = PolicyConfig::default();
        policy.b_login_moderate = policy.b_login_moderate.saturating_mul(3);
        let mut ctx = mk_ctx();
        ctx.policy = Arc::new(policy);
        let store = MockStore::default();
        evaluate(ctx, &store).await.score
    };

    // Suppressed: zero out the moderate velocity weight
    let suppressed_score = {
        let mut policy = PolicyConfig::default();
        policy.b_login_moderate = 0;
        let mut ctx = mk_ctx();
        ctx.policy = Arc::new(policy);
        let store = MockStore::default();
        evaluate(ctx, &store).await.score
    };

    assert!(
        amplified_score > default_score,
        "amplified b_login_moderate should raise score (default={}, amplified={})",
        default_score, amplified_score
    );
    assert!(
        suppressed_score < default_score,
        "zero b_login_moderate should lower score (default={}, suppressed={})",
        default_score, suppressed_score
    );
}

#[tokio::test]
async fn policy_threshold_override_flips_decision() {
    // Ratchet the Allow threshold down to 0 — a clean login that normally
    // lands in Allow must flip to Challenge.
    let ctx_default = clean_login_ctx();
    let store_default = MockStore::default();
    let default_decision = evaluate(ctx_default, &store_default).await;
    assert_eq!(default_decision.decision, Decision::Allow);

    let mut policy = PolicyConfig::default();
    policy.threshold_allow_max = 0;
    let mut ctx = clean_login_ctx();
    ctx.policy = Arc::new(policy);
    let store = MockStore::default();
    let tightened_decision = evaluate(ctx, &store).await;

    assert_ne!(
        tightened_decision.decision,
        Decision::Allow,
        "zero Allow threshold must flip a clean login off Allow (got {:?}, score={})",
        tightened_decision.decision, tightened_decision.score,
    );
}

#[tokio::test]
async fn policy_action_multiplier_override_changes_score() {
    // Higher login multiplier should raise the final score when there is any
    // non-zero base risk to multiply.
    let mk_ctx = || {
        let mut c = clean_login_ctx();
        c.login_attempts_5m = Some(4); // seeds a small non-zero behavioral score
        c
    };

    let default_score = {
        let ctx = mk_ctx();
        let store = MockStore::default();
        evaluate(ctx, &store).await.score
    };

    let mut policy = PolicyConfig::default();
    policy.multiplier_login = policy.multiplier_login + 2.0;
    let mut ctx = mk_ctx();
    ctx.policy = Arc::new(policy);
    let store = MockStore::default();
    let boosted_score = evaluate(ctx, &store).await.score;

    assert!(
        boosted_score > default_score,
        "raising multiplier_login should raise final score (default={}, boosted={})",
        default_score, boosted_score
    );
}
