use std::net::{IpAddr, Ipv4Addr};
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use uuid::Uuid;

use std::sync::Arc;

use risk_engine::{
    context::{
        AsnType, CredentialStatus, GeoPoint, OrgRiskLevel, RiskAction, RiskContext,
    },
    decision::Decision,
    evaluate,
    policy_config::PolicyConfig,
    store::{EscalationEntry, SignalStore, VelocityCounters},
    RiskEngineError,
};

// ─────────────────────────────────────────────────────────────────────────────
// Mock Store
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
// Default context builder
// ─────────────────────────────────────────────────────────────────────────────

fn default_context() -> RiskContext {
    RiskContext {
        request_id: Uuid::new_v4(),
        evaluated_at: Utc::now(),
        user_id: Uuid::new_v4(),
        username: "test_user".into(),
        credential_id: Some(Uuid::new_v4()),
        action: RiskAction::Login,
        resource_id: None,
        credential_status: CredentialStatus::Active,
        credential_created_at: Utc::now() - Duration::days(2),
        credential_last_used_at: Some(Utc::now() - Duration::hours(12)),
        credential_sign_count_prev: 100,
        credential_sign_count_new: 101,
        credential_registered_ua: Some("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".into()),
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
        request_ua: Some("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".into()),
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
        tenant_id: "test-tenant".into(),
        org_risk_score: None,
        cluster_membership: None,
        threshold_shift: None,
        org_active_cluster_count: 0,
        login_attempts_1m: None,
        login_attempts_1h: None,
        login_attempts_24h: None,
        client_timestamp: None,
        device_fingerprint_hash: Some("test_device_fingerprint".into()),
        ja3_fingerprint: None,
        known_ip_for_user: None,
        ip_is_vpn: None,
        ip_is_proxy: None,
        ip_is_relay: None,
        ip_abuse_confidence: None,
        geo_allowed_countries: None,
        is_sanctioned_country: None,
        webdriver_detected: Some(false),
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

// ─────────────────────────────────────────────────────────────────────────────
// Test Scenarios
// ─────────────────────────────────────────────────────────────────────────────

async fn run_scenario(name: &str, ctx: RiskContext) -> (String, RiskContext, String) {
    let store = MockStore::default();
    let decision = evaluate(ctx.clone(), &store).await;

    let result = format!(
        "│ {} │ Score: {} │ Decision: {:?} │ Action: {:?} │\n    ├─ Base Score: {}\n    ├─ Adjusted Score: {}\n    └─ Triggered Gates: {}",
        name,
        decision.score,
        decision.decision,
        decision.required_action,
        decision.base_score,
        decision.adjusted_score,
        if decision.triggered_gates.is_empty() {
            "None".to_string()
        } else {
            decision
                .triggered_gates
                .iter()
                .map(|g| format!("{:?}", g))
                .collect::<Vec<_>>()
                .join(", ")
        }
    );

    (name.to_string(), ctx, result)
}

// ─────────────────────────────────────────────────────────────────────────────
// Main simulation tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn sim_scenario_1_clean_login() {
    println!("\n┌────────────────────────────────────────────────────────────┐");
    println!("│  SCENARIO 1: CLEAN LOGIN                                  │");
    println!("└────────────────────────────────────────────────────────────┘");

    let ctx = default_context();
    let (_, _, result) = run_scenario("Clean login from known device", ctx).await;
    println!("{}", result);
    println!("✓ This should ALLOW with score < 30");
}

#[tokio::test]
async fn sim_scenario_2_brute_force() {
    println!("\n┌────────────────────────────────────────────────────────────┐");
    println!("│  SCENARIO 2: BRUTE FORCE ATTACK                           │");
    println!("└────────────────────────────────────────────────────────────┘");

    let mut ctx = default_context();
    ctx.login_attempts_5m = Some(15);
    ctx.failed_login_attempts_1h = Some(10);
    ctx.username = "targeted_user".into();

    let (_, _, result) = run_scenario("High-velocity login attempts", ctx).await;
    println!("{}", result);
    println!("✓ High behavioral velocity → CHALLENGE or DENY");
}

#[tokio::test]
async fn sim_scenario_3_new_credential_sensitive_action() {
    println!("\n┌────────────────────────────────────────────────────────────┐");
    println!("│  SCENARIO 3: NEW CREDENTIAL + SENSITIVE ACTION            │");
    println!("└────────────────────────────────────────────────────────────┘");

    let mut ctx = default_context();
    ctx.credential_created_at = Utc::now() - Duration::minutes(2); // very new
    ctx.credential_count_for_user = 1;
    ctx.prior_audit_event_count = 0;
    ctx.action = RiskAction::ActionExecute {
        action_name: "add_admin".into(),
    };

    let (_, _, result) = run_scenario("Add admin with brand-new credential", ctx).await;
    println!("{}", result);
    println!("✓ New credential + sensitive action → DENY (max score)");
}

#[tokio::test]
async fn sim_scenario_4_tor_exit_node() {
    println!("\n┌────────────────────────────────────────────────────────────┐");
    println!("│  SCENARIO 4: TOR EXIT NODE ON SENSITIVE ACTION            │");
    println!("└────────────────────────────────────────────────────────────┘");

    let mut ctx = default_context();
    ctx.ip_asn_type = Some(AsnType::Tor);
    ctx.action = RiskAction::DeviceRevoke;
    ctx.request_ip = Some(IpAddr::V4(Ipv4Addr::new(10, 20, 30, 40))); // different IP

    let (_, _, result) = run_scenario("Tor exit node revoking device", ctx).await;
    println!("{}", result);
    println!("✓ Tor + sensitive action → HARD DENY (policy gate)");
}

#[tokio::test]
async fn sim_scenario_5_vpn_geographic_anomaly() {
    println!("\n┌────────────────────────────────────────────────────────────┐");
    println!("│  SCENARIO 5: VPN + GEOGRAPHIC ANOMALY                     │");
    println!("└────────────────────────────────────────────────────────────┘");

    let mut ctx = default_context();
    ctx.ip_asn_type = Some(AsnType::Vpn);
    ctx.request_geo = Some(GeoPoint {
        lat: -33.8688,
        lon: 151.2093,
        country_code: "AU".into(),
        city: Some("Sydney".into()),
    });
    ctx.last_login_geo = Some(GeoPoint {
        lat: 40.7128,
        lon: -74.0060,
        country_code: "US".into(),
        city: Some("New York".into()),
    });

    let (_, _, result) = run_scenario("VPN login from Sydney vs last login NYC", ctx).await;
    println!("{}", result);
    println!("✓ Geographic anomaly + VPN → elevated risk score");
}

#[tokio::test]
async fn sim_scenario_6_revoked_credential() {
    println!("\n┌────────────────────────────────────────────────────────────┐");
    println!("│  SCENARIO 6: REVOKED CREDENTIAL                           │");
    println!("└────────────────────────────────────────────────────────────┘");

    let mut ctx = default_context();
    ctx.credential_status = CredentialStatus::Revoked;

    let (_, _, result) = run_scenario("Login with revoked credential", ctx).await;
    println!("{}", result);
    println!("✓ Revoked credential → HARD DENY (policy gate)");
}

#[tokio::test]
async fn sim_scenario_7_redis_degraded_sensitive() {
    println!("\n┌────────────────────────────────────────────────────────────┐");
    println!("│  SCENARIO 7: REDIS DEGRADED + SENSITIVE ACTION            │");
    println!("└────────────────────────────────────────────────────────────┘");

    let mut ctx = default_context();
    ctx.redis_signals_degraded = true;
    ctx.action = RiskAction::DeviceRevoke;

    let (_, _, result) = run_scenario("Device revoke during Redis outage", ctx).await;
    println!("{}", result);
    println!("✓ Signals degraded + sensitive → force CHALLENGE");
}

#[tokio::test]
async fn sim_scenario_8_device_mark_lost() {
    println!("\n┌────────────────────────────────────────────────────────────┐");
    println!("│  SCENARIO 8: DEVICE MARK LOST (RECOVERY FLOW)             │");
    println!("└────────────────────────────────────────────────────────────┘");

    let mut ctx = default_context();
    ctx.action = RiskAction::DeviceMarkLost;
    ctx.device_revocations_1h = Some(1); // recently revoked another

    let (_, _, result) = run_scenario("Mark device as lost", ctx).await;
    println!("{}", result);
    println!("✓ Recovery action with context → may CHALLENGE or ALLOW");
}

#[tokio::test]
async fn sim_scenario_9_nonce_replay() {
    println!("\n┌────────────────────────────────────────────────────────────┐");
    println!("│  SCENARIO 9: NONCE REPLAY ATTACK                          │");
    println!("└────────────────────────────────────────────────────────────┘");

    let mut ctx = default_context();
    ctx.nonce_already_used = true;

    let (_, _, result) = run_scenario("Nonce already consumed", ctx).await;
    println!("{}", result);
    println!("✓ Replay attack detected → HARD DENY");
}

#[tokio::test]
async fn sim_scenario_10_recovery_flow() {
    println!("\n┌────────────────────────────────────────────────────────────┐");
    println!("│  SCENARIO 10: RECOVERY COMPLETE WITHOUT PENDING           │");
    println!("└────────────────────────────────────────────────────────────┘");

    let mut ctx = default_context();
    ctx.action = RiskAction::RecoveryComplete;
    ctx.recovery_pending = false;

    let (_, _, result) = run_scenario("Recovery complete without pending", ctx).await;
    println!("{}", result);
    println!("✓ Invalid recovery state → HARD DENY");
}

// ─────────────────────────────────────────────────────────────────────────────
// Summary test - runs all scenarios and prints results
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn full_simulation_summary() {
    println!("\n");
    println!("╔═══════════════════════════════════════════════════════════════════════════════╗");
    println!("║                CRYPTON-DI RISK ENGINE FULL SIMULATION                         ║");
    println!("║                     Testing Risk Scoring & Decision Logic                     ║");
    println!("╚═══════════════════════════════════════════════════════════════════════════════╝\n");

    // Scenario 1: Clean login
    print!("1. Clean Login ... ");
    let mut ctx = default_context();
    println!("stored: {:?}", ctx.jwt_fingerprint_stored);
    let store = MockStore::default();
    let d = evaluate(ctx.clone(), &store).await;
    println!(
        "✓ Score: {}, Decision: {:?}",
        d.score, d.decision
    );
    println!("   Breakdown: D={}, S={}, N={}, B={}, Base={}, M_action={:.1}, M_org={:.1}, A={}, Final={}",
        d.score_breakdown.d,
        d.score_breakdown.s,
        d.score_breakdown.n,
        d.score_breakdown.b,
        d.score_breakdown.base,
        d.score_breakdown.m_action,
        d.score_breakdown.m_org,
        d.score_breakdown.a,
        d.score_breakdown.final_score
    );
    assert_eq!(d.decision, Decision::Allow);

    // Scenario 2: Brute force
    print!("2. Brute Force (15 attempts in 5m) ... ");
    ctx.login_attempts_5m = Some(15);
    ctx.failed_login_attempts_1h = Some(10);
    let d = evaluate(ctx.clone(), &store).await;
    println!(
        "✓ Score: {}, Decision: {:?} (BLOCKING)",
        d.score, d.decision
    );
    assert!(d.decision.is_blocking());

    // Scenario 3: New credential + sensitive
    print!("3. New Credential + Add Admin ... ");
    ctx = default_context();
    ctx.credential_created_at = Utc::now() - Duration::minutes(2);
    ctx.credential_count_for_user = 1;
    ctx.prior_audit_event_count = 0;
    ctx.action = RiskAction::ActionExecute {
        action_name: "add_admin".into(),
    };
    let d = evaluate(ctx.clone(), &store).await;
    println!("✓ Score: {}, Decision: {:?}", d.score, d.decision);
    // PrivilegedActionOnFreshCredential routes to Hold (trust-spike protection).
    assert_eq!(d.decision, Decision::Hold);

    // Scenario 4: Tor on sensitive
    print!("4. Tor Exit Node + DeviceRevoke ... ");
    ctx = default_context();
    ctx.ip_asn_type = Some(AsnType::Tor);
    ctx.action = RiskAction::DeviceRevoke;
    let d = evaluate(ctx.clone(), &store).await;
    println!(
        "✓ Score: {}, Decision: {:?} (POLICY GATE)",
        d.score, d.decision
    );
    assert_eq!(d.decision, Decision::Deny);

    // Scenario 5: VPN + Geo anomaly
    print!("5. VPN + Geographic Anomaly ... ");
    ctx = default_context();
    ctx.ip_asn_type = Some(AsnType::Vpn);
    ctx.request_geo = Some(GeoPoint {
        lat: -33.8688,
        lon: 151.2093,
        country_code: "AU".into(),
        city: Some("Sydney".into()),
    });
    let d = evaluate(ctx.clone(), &store).await;
    println!("✓ Score: {}, Decision: {:?}", d.score, d.decision);
    assert!(d.score > 20); // should elevate risk

    // Scenario 6: Revoked credential
    print!("6. Revoked Credential ... ");
    ctx = default_context();
    ctx.credential_status = CredentialStatus::Revoked;
    let d = evaluate(ctx.clone(), &store).await;
    println!("✓ Score: {}, Decision: {:?}", d.score, d.decision);
    assert_eq!(d.decision, Decision::Deny);

    // Scenario 7: Redis degraded + sensitive
    print!("7. Redis Degraded + Sensitive Action ... ");
    ctx = default_context();
    ctx.redis_signals_degraded = true;
    ctx.action = RiskAction::DeviceRevoke;
    let d = evaluate(ctx.clone(), &store).await;
    println!(
        "✓ Score: {}, Decision: {:?} (CHALLENGE)",
        d.score, d.decision
    );
    assert_eq!(d.decision, Decision::Challenge);

    // Scenario 8: Nonce replay
    print!("8. Nonce Replay ... ");
    ctx = default_context();
    ctx.nonce_already_used = true;
    let d = evaluate(ctx.clone(), &store).await;
    println!("✓ Score: {}, Decision: {:?}", d.score, d.decision);
    assert_eq!(d.decision, Decision::Deny);

    // Scenario 9: Recovery without pending
    print!("9. Recovery Complete (No Pending) ... ");
    ctx = default_context();
    ctx.action = RiskAction::RecoveryComplete;
    ctx.recovery_pending = false;
    let d = evaluate(ctx.clone(), &store).await;
    println!("✓ Score: {}, Decision: {:?}", d.score, d.decision);
    assert_eq!(d.decision, Decision::Deny);

    // Scenario 10: JWT fingerprint mismatch
    print!("10. JWT Fingerprint Mismatch ... ");
    ctx = default_context();
    ctx.jwt_fingerprint_stored = Some("aabbcc".into());
    ctx.jwt_fingerprint_current = Some("ddeeff".into());
    ctx.action = RiskAction::DeviceRevoke;
    let d = evaluate(ctx.clone(), &store).await;
    println!(
        "✓ Score: {}, Decision: {:?} (BLOCKING)",
        d.score, d.decision
    );
    assert!(d.decision.is_blocking());

    println!("\n╔═══════════════════════════════════════════════════════════════════════════════╗");
    println!("║                         ✓ ALL SCENARIOS PASSED                               ║");
    println!("╚═══════════════════════════════════════════════════════════════════════════════╝\n");
}
