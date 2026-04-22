//! Property tests (Phase 7) — invariants that must hold across the whole
//! input space, not just the golden scenarios.
//!
//! Covers:
//!   - `score_to_decision` monotonicity: score_a ≤ score_b ⇒ decision not stricter
//!   - `compute_damped_base` monotonicity + below-threshold identity + rate bound
//!   - `capped_add` never lets `current` exceed `cap`

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;

use chrono::{Duration, Utc};
use proptest::prelude::*;
use uuid::Uuid;

use risk_engine::{
    context::{CredentialStatus, OrgRiskLevel, RiskAction, RiskContext},
    decision::Decision,
    engine::{compute_damped_base, score_to_decision},
    policy_config::PolicyConfig,
    scoring::capped_add,
};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn decision_rank(d: &Decision) -> u8 {
    match d {
        Decision::Allow => 0,
        Decision::Challenge => 1,
        Decision::Hold => 2,
        Decision::Deny => 3,
    }
}

/// Minimal context with matching JWT fingerprints so the fingerprint-mismatch
/// Allow→Challenge upgrade path in `score_to_decision` never fires. Under
/// that condition the function is purely piecewise-monotonic in `score`.
fn ctx_no_fp_mismatch(policy: Arc<PolicyConfig>) -> RiskContext {
    RiskContext {
        request_id: Uuid::new_v4(),
        evaluated_at: Utc::now(),
        user_id: Uuid::new_v4(),
        username: "p".into(),
        credential_id: Some(Uuid::new_v4()),
        action: RiskAction::Login,
        resource_id: None,
        credential_status: CredentialStatus::Active,
        credential_created_at: Utc::now() - Duration::days(30),
        credential_last_used_at: Some(Utc::now() - Duration::hours(12)),
        credential_sign_count_prev: 100,
        credential_sign_count_new: 101,
        credential_registered_ua: None,
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
        request_ua: None,
        request_geo: None,
        ip_asn_type: None,
        login_attempts_5m: Some(0),
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
        tenant_id: "t".into(),
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

// ─────────────────────────────────────────────────────────────────────────────
// Properties
// ─────────────────────────────────────────────────────────────────────────────

proptest! {
    /// Increasing the score never relaxes the decision under a fixed policy.
    #[test]
    fn score_to_decision_is_monotonic(
        score_a in 0u8..=100,
        bump in 0u8..=100,
    ) {
        let policy = Arc::new(PolicyConfig::default());
        let ctx = ctx_no_fp_mismatch(policy);

        let score_b = score_a.saturating_add(bump);
        let (da, _) = score_to_decision(score_a, &ctx);
        let (db, _) = score_to_decision(score_b, &ctx);
        prop_assert!(
            decision_rank(&da) <= decision_rank(&db),
            "monotonicity broken: score {} → {:?}, score {} → {:?}",
            score_a, da, score_b, db
        );
    }

    /// Below the damping threshold the function is the identity.
    #[test]
    fn damping_is_identity_below_threshold(
        base in 0.0f32..100.0,
        rate in 0.0f32..=1.0,
    ) {
        let threshold = base + 1.0; // ensure base < threshold
        let out = compute_damped_base(base, threshold, rate);
        prop_assert!((out - base).abs() < 1e-5);
    }

    /// With rate ≤ 1.0 the damped value never exceeds the raw base.
    #[test]
    fn damping_cannot_inflate(
        base in 0.0f32..1000.0,
        threshold in 0.0f32..100.0,
        rate in 0.0f32..=1.0,
    ) {
        let out = compute_damped_base(base, threshold, rate);
        prop_assert!(out <= base + 1e-4, "damping inflated: {} -> {}", base, out);
    }

    /// Damping is monotone non-decreasing in the raw base (a larger input
    /// cannot yield a smaller damped output for a fixed threshold/rate).
    #[test]
    fn damping_is_monotone_in_base(
        base_a in 0.0f32..500.0,
        delta in 0.0f32..500.0,
        threshold in 0.0f32..100.0,
        rate in 0.0f32..=1.0,
    ) {
        let base_b = base_a + delta;
        let a = compute_damped_base(base_a, threshold, rate);
        let b = compute_damped_base(base_b, threshold, rate);
        prop_assert!(a <= b + 1e-4);
    }

    /// A single `capped_add` never pushes `current` above `cap`.
    #[test]
    fn capped_add_never_exceeds_cap(
        start in 0u8..=100,
        cap in 0u8..=100,
        raw in 0u8..=100,
    ) {
        let mut current = start.min(cap); // precondition: current ≤ cap
        let effective = capped_add(&mut current, cap, raw);
        prop_assert!(current <= cap);
        prop_assert!(effective <= raw);
        prop_assert!(effective <= cap);
    }

    /// Repeated applications of `capped_add` stay bounded by `cap` and the
    /// sum of effectives matches `current`.
    #[test]
    fn capped_add_is_stable_under_repetition(
        cap in 1u8..=100,
        raws in proptest::collection::vec(0u8..=50, 0..20),
    ) {
        let mut current: u8 = 0;
        let mut sum_effective: u32 = 0;
        for raw in &raws {
            let eff = capped_add(&mut current, cap, *raw);
            sum_effective += eff as u32;
            prop_assert!(current <= cap);
        }
        prop_assert_eq!(current as u32, sum_effective);
    }
}
