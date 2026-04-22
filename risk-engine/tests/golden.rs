//! Golden snapshot harness.
//!
//! Each scenario evaluates a pinned `RiskContext` against the engine and
//! renders the result through [`render_decision`] into a deterministic
//! string that excludes timestamps, UUIDs, and build-version metadata.
//!
//! Run `cargo test --test golden` to evaluate; if the engine output changes
//! intentionally, run `cargo insta review` (or `cargo insta accept`) to
//! update the committed snapshots under `tests/snapshots/`.

#[path = "helpers.rs"]
mod helpers;

use std::fmt::Write as _;
use std::net::IpAddr;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{Duration, TimeZone, Utc};
use uuid::Uuid;

use risk_engine::{
    context::{
        AsnType, CredentialStatus, OrgRiskLevel, RiskAction, RiskContext,
    },
    decision::{Decision, Factor, RiskDecision},
    evaluate,
    store::{EscalationEntry, SignalStore, VelocityCounters},
    RiskEngineError,
};

use helpers::{RiskContextBuilder, TestGeos, TestIps};

// ─────────────────────────────────────────────────────────────────────────────
// Pinned clock — every scenario evaluates against the same `now` so that
// relative durations (credential age, jwt_expires_at, last_login_at) render
// identically across runs and machines.
// ─────────────────────────────────────────────────────────────────────────────

fn pinned_now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 4, 1, 12, 0, 0).unwrap()
}

/// Rebase all `DateTime<Utc>` fields on the context onto [`pinned_now`] while
/// preserving the *offset* from the builder's `Utc::now()`.
fn pin_context(mut ctx: RiskContext) -> RiskContext {
    let builder_now = ctx.evaluated_at;
    let delta = pinned_now() - builder_now;
    ctx.evaluated_at = pinned_now();
    ctx.credential_created_at = ctx.credential_created_at + delta;
    ctx.credential_last_used_at = ctx.credential_last_used_at.map(|t| t + delta);
    ctx.last_login_at = ctx.last_login_at.map(|t| t + delta);
    ctx.jwt_issued_at = ctx.jwt_issued_at.map(|t| t + delta);
    ctx.jwt_expires_at = ctx.jwt_expires_at.map(|t| t + delta);
    ctx.client_timestamp = ctx.client_timestamp.map(|t| t + delta);
    // Pin the request_id too — otherwise it leaks into some factor descriptions.
    ctx.request_id = Uuid::nil();
    ctx
}

// ─────────────────────────────────────────────────────────────────────────────
// Mock store
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct GoldenStore {
    recent_decisions: Mutex<Vec<EscalationEntry>>,
}

impl GoldenStore {
    fn with_recent(entries: Vec<EscalationEntry>) -> Self {
        Self {
            recent_decisions: Mutex::new(entries),
        }
    }
}

#[async_trait]
impl SignalStore for GoldenStore {
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
        v.insert(0, EscalationEntry {
            score,
            decision: final_decision.clone(),
            ts_unix,
        });
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
// Render — deterministic projection of a RiskDecision
// ─────────────────────────────────────────────────────────────────────────────

fn render_decision(name: &str, d: &RiskDecision) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "scenario: {name}");
    let _ = writeln!(s, "decision: {:?}", d.decision);
    let _ = writeln!(s, "score: {}", d.score);
    let _ = writeln!(s, "base_score: {}", d.base_score);
    let _ = writeln!(s, "adjusted_score: {}", d.adjusted_score);
    let _ = writeln!(
        s,
        "required_action: {}",
        match &d.required_action {
            Some(a) => format!("{a:?}"),
            None => "None".into(),
        }
    );
    let _ = writeln!(s, "hold_reason: {:?}", d.hold_reason);
    let _ = writeln!(
        s,
        "triggered_gates: {}",
        if d.triggered_gates.is_empty() {
            "[]".into()
        } else {
            let mut gs: Vec<String> = d.triggered_gates.iter().map(|g| format!("{g:?}")).collect();
            gs.sort();
            format!("[{}]", gs.join(", "))
        }
    );
    let _ = writeln!(s, "escalated: {}", d.escalated);
    let _ = writeln!(s, "signals_degraded: {}", d.signals_degraded);
    let _ = writeln!(s, "lock_account: {}", d.lock_account);
    let _ = writeln!(s, "notify_user: {}", d.notify_user);
    let _ = writeln!(s, "notify_admin: {}", d.notify_admin);

    // Score breakdown
    let b = &d.score_breakdown;
    let _ = writeln!(
        s,
        "breakdown: D={} S={} N={} B={} C={} base={} m_action={:.2} m_org={:.2} a={} final={}",
        b.d, b.s, b.n, b.b, b.c, b.base, b.m_action, b.m_org, b.a, b.final_score,
    );

    // Top-3 factors by contribution (ties broken by name for determinism)
    let mut factors: Vec<&Factor> = d.contributing_factors.iter().collect();
    factors.sort_by(|a, b| b.contribution.cmp(&a.contribution).then(a.name.cmp(&b.name)));
    let _ = writeln!(s, "top_factors:");
    for f in factors.iter().take(3) {
        let _ = writeln!(
            s,
            "  - name: {}  component: {:?}  contribution: {}",
            f.name, f.component, f.contribution
        );
    }

    // Triggered rules — sorted for determinism
    let mut rules = d.triggered_rules.clone();
    rules.sort();
    let _ = writeln!(s, "triggered_rules: {rules:?}");
    s
}

async fn run(name: &str, ctx: RiskContext) -> String {
    let store = GoldenStore::default();
    let decision = evaluate(pin_context(ctx), &store).await;
    render_decision(name, &decision)
}

async fn run_with_recent(
    name: &str,
    ctx: RiskContext,
    recent: Vec<EscalationEntry>,
) -> String {
    let store = GoldenStore::with_recent(recent);
    let decision = evaluate(pin_context(ctx), &store).await;
    render_decision(name, &decision)
}

// ─────────────────────────────────────────────────────────────────────────────
// CATEGORY 1 — Baseline / happy path
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn g_clean_login() {
    let ctx = RiskContextBuilder::new().action(RiskAction::Login).build();
    insta::assert_snapshot!(run("clean_login", ctx).await);
}

#[tokio::test]
async fn g_clean_register() {
    let ctx = RiskContextBuilder::new().action(RiskAction::Register).build();
    insta::assert_snapshot!(run("clean_register", ctx).await);
}

#[tokio::test]
async fn g_clean_device_list() {
    let ctx = RiskContextBuilder::new().action(RiskAction::DeviceList).build();
    insta::assert_snapshot!(run("clean_device_list", ctx).await);
}

// ─────────────────────────────────────────────────────────────────────────────
// CATEGORY 2 — Velocity / brute force
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn g_moderate_velocity_login() {
    let ctx = RiskContextBuilder::new()
        .action(RiskAction::Login)
        .login_attempts_5m(5)
        .failed_login_attempts_1h(4)
        .build();
    insta::assert_snapshot!(run("moderate_velocity_login", ctx).await);
}

#[tokio::test]
async fn g_brute_force_login() {
    let ctx = RiskContextBuilder::new()
        .action(RiskAction::Login)
        .login_attempts_5m(15)
        .failed_login_attempts_1h(10)
        .asn_type(AsnType::Datacenter)
        .request_ip(TestIps::datacenter())
        .build();
    insta::assert_snapshot!(run("brute_force_login", ctx).await);
}

#[tokio::test]
async fn g_rapid_registration_from_hosting() {
    let mut ctx = RiskContextBuilder::new()
        .action(RiskAction::Register)
        .asn_type(AsnType::Hosting)
        .request_ip(TestIps::datacenter())
        .build();
    ctx.registrations_from_ip_10m = Some(5);
    ctx.email_domain_disposable = Some(true);
    insta::assert_snapshot!(run("rapid_registration_from_hosting", ctx).await);
}

// ─────────────────────────────────────────────────────────────────────────────
// CATEGORY 3 — Credential gates
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn g_revoked_credential() {
    let ctx = RiskContextBuilder::new()
        .action(RiskAction::Login)
        .credential_status(CredentialStatus::Revoked)
        .build();
    insta::assert_snapshot!(run("revoked_credential", ctx).await);
}

#[tokio::test]
async fn g_lost_credential() {
    let ctx = RiskContextBuilder::new()
        .action(RiskAction::Login)
        .credential_status(CredentialStatus::Lost)
        .build();
    insta::assert_snapshot!(run("lost_credential", ctx).await);
}

#[tokio::test]
async fn g_account_locked() {
    let mut ctx = RiskContextBuilder::new().action(RiskAction::Login).build();
    ctx.account_locked = true;
    insta::assert_snapshot!(run("account_locked", ctx).await);
}

#[tokio::test]
async fn g_privileged_on_fresh_credential() {
    let ctx = RiskContextBuilder::new()
        .action(RiskAction::ActionExecute {
            action_name: "add_admin".into(),
        })
        .credential_age(Duration::minutes(2))
        .credential_count(1)
        .audit_event_count(0)
        .build();
    insta::assert_snapshot!(run("privileged_on_fresh_credential", ctx).await);
}

#[tokio::test]
async fn g_breached_credential_on_sensitive() {
    let mut ctx = RiskContextBuilder::new()
        .action(RiskAction::DeviceRevoke)
        .build();
    ctx.breached_credential = Some(true);
    insta::assert_snapshot!(run("breached_credential_on_sensitive", ctx).await);
}

#[tokio::test]
async fn g_sign_count_rollback() {
    let mut ctx = RiskContextBuilder::new().action(RiskAction::Login).build();
    ctx.credential_sign_count_prev = 100;
    ctx.credential_sign_count_new = -1;
    insta::assert_snapshot!(run("sign_count_rollback", ctx).await);
}

// ─────────────────────────────────────────────────────────────────────────────
// CATEGORY 4 — Session / structural
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn g_nonce_replay() {
    let ctx = RiskContextBuilder::new()
        .action(RiskAction::Login)
        .nonce_replayed()
        .build();
    insta::assert_snapshot!(run("nonce_replay", ctx).await);
}

#[tokio::test]
async fn g_jwt_mismatch_sensitive() {
    let ctx = RiskContextBuilder::new()
        .action(RiskAction::DeviceRevoke)
        .jwt_fingerprints("aabbcc1122334455", "ddeeff6677889900")
        .build();
    insta::assert_snapshot!(run("jwt_mismatch_sensitive", ctx).await);
}

#[tokio::test]
async fn g_jwt_mismatch_non_sensitive() {
    let ctx = RiskContextBuilder::new()
        .action(RiskAction::DeviceList)
        .jwt_fingerprints("aabbcc1122334455", "ddeeff6677889900")
        .build();
    insta::assert_snapshot!(run("jwt_mismatch_non_sensitive", ctx).await);
}

#[tokio::test]
async fn g_expired_jwt_on_sensitive() {
    let mut ctx = RiskContextBuilder::new().action(RiskAction::DeviceRevoke).build();
    ctx.jwt_expires_at = Some(ctx.evaluated_at - Duration::minutes(10));
    insta::assert_snapshot!(run("expired_jwt_on_sensitive", ctx).await);
}

#[tokio::test]
async fn g_recovery_complete_without_pending() {
    let ctx = RiskContextBuilder::new()
        .action(RiskAction::RecoveryComplete)
        .recovery_pending(false)
        .build();
    insta::assert_snapshot!(run("recovery_complete_without_pending", ctx).await);
}

#[tokio::test]
async fn g_oauth_ip_mismatch() {
    let mut ctx = RiskContextBuilder::new()
        .action(RiskAction::OauthComplete)
        .request_ip(TestIps::home_us())
        .build();
    ctx.oauth_authorize_ip = Some(TestIps::home_au());
    insta::assert_snapshot!(run("oauth_ip_mismatch", ctx).await);
}

// ─────────────────────────────────────────────────────────────────────────────
// CATEGORY 5 — Network gates / anomalies
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn g_tor_on_sensitive() {
    let ctx = RiskContextBuilder::new()
        .action(RiskAction::DeviceRevoke)
        .asn_type(AsnType::Tor)
        .request_ip(TestIps::tor_exit())
        .build();
    insta::assert_snapshot!(run("tor_on_sensitive", ctx).await);
}

#[tokio::test]
async fn g_tor_on_recovery() {
    let ctx = RiskContextBuilder::new()
        .action(RiskAction::RecoveryStart)
        .asn_type(AsnType::Tor)
        .request_ip(TestIps::tor_exit())
        .build();
    insta::assert_snapshot!(run("tor_on_recovery", ctx).await);
}

#[tokio::test]
async fn g_vpn_geo_jump_login() {
    let ctx = RiskContextBuilder::new()
        .action(RiskAction::Login)
        .asn_type(AsnType::Vpn)
        .request_geo(TestGeos::sydney())
        .last_login_geo(TestGeos::new_york())
        .build();
    insta::assert_snapshot!(run("vpn_geo_jump_login", ctx).await);
}

#[tokio::test]
async fn g_impossible_travel_no_tor() {
    let mut ctx = RiskContextBuilder::new()
        .action(RiskAction::Login)
        .request_geo(TestGeos::sydney())
        .last_login_geo(TestGeos::new_york())
        .build();
    // NY → Sydney is ~16000km; setting last login 30 minutes ago makes it impossible.
    ctx.last_login_at = Some(ctx.evaluated_at - Duration::minutes(30));
    insta::assert_snapshot!(run("impossible_travel_no_tor", ctx).await);
}

#[tokio::test]
async fn g_impossible_travel_via_tor() {
    let mut ctx = RiskContextBuilder::new()
        .action(RiskAction::Login)
        .asn_type(AsnType::Tor)
        .request_ip(TestIps::tor_exit())
        .request_geo(TestGeos::sydney())
        .last_login_geo(TestGeos::new_york())
        .build();
    ctx.last_login_at = Some(ctx.evaluated_at - Duration::minutes(30));
    insta::assert_snapshot!(run("impossible_travel_via_tor", ctx).await);
}

#[tokio::test]
async fn g_sanctioned_country() {
    let mut ctx = RiskContextBuilder::new().action(RiskAction::Login).build();
    ctx.is_sanctioned_country = Some(true);
    ctx.request_geo = Some(TestGeos::moscow());
    insta::assert_snapshot!(run("sanctioned_country", ctx).await);
}

#[tokio::test]
async fn g_vpn_on_critical_action() {
    let ctx = RiskContextBuilder::new()
        .action(RiskAction::RecoveryComplete)
        .asn_type(AsnType::Vpn)
        .recovery_pending(true)
        .build();
    insta::assert_snapshot!(run("vpn_on_critical_action", ctx).await);
}

#[tokio::test]
async fn g_rfc1918_trusted_login() {
    let mut ctx = RiskContextBuilder::new().action(RiskAction::Login).build();
    ctx.request_ip = Some("10.0.0.5".parse().unwrap());
    ctx.last_login_ip = Some("10.0.0.5".parse().unwrap());
    insta::assert_snapshot!(run("rfc1918_trusted_login", ctx).await);
}

// ─────────────────────────────────────────────────────────────────────────────
// CATEGORY 6 — Automation / breach / bots
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn g_automation_on_register() {
    let mut ctx = RiskContextBuilder::new().action(RiskAction::Register).build();
    ctx.webdriver_detected = Some(true);
    insta::assert_snapshot!(run("automation_on_register", ctx).await);
}

#[tokio::test]
async fn g_headless_ua_on_cloud() {
    let mut ctx = RiskContextBuilder::new()
        .action(RiskAction::Login)
        .asn_type(AsnType::Datacenter)
        .request_ip(TestIps::datacenter())
        .build();
    ctx.request_ua = Some("HeadlessChrome/120.0 (unknown)".into());
    ctx.webdriver_detected = Some(true);
    insta::assert_snapshot!(run("headless_ua_on_cloud", ctx).await);
}

// ─────────────────────────────────────────────────────────────────────────────
// CATEGORY 7 — Scoring-path correlation patterns
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn g_ato_cluster_pattern() {
    let mut ctx = RiskContextBuilder::new()
        .action(RiskAction::RecoveryStart)
        .asn_type(AsnType::Vpn)
        .build();
    ctx.ip_is_vpn = Some(true);
    ctx.known_ip_for_user = Some(false);
    insta::assert_snapshot!(run("ato_cluster_pattern", ctx).await);
}

#[tokio::test]
async fn g_shadow_session() {
    let mut ctx = RiskContextBuilder::new().action(RiskAction::Login).build();
    ctx.active_session_count = Some(4);
    ctx.request_ua = Some("Mozilla/5.0 Firefox/115".into());
    ctx.credential_registered_ua = Some("Mozilla/5.0 Chrome/120".into());
    ctx.known_ip_for_user = Some(false);
    insta::assert_snapshot!(run("shadow_session", ctx).await);
}

#[tokio::test]
async fn g_credential_cloner() {
    let mut ctx = RiskContextBuilder::new().action(RiskAction::Login).build();
    ctx.credential_sign_count_prev = 100;
    ctx.credential_sign_count_new = 500;
    ctx.credential_registered_ua = Some("Mozilla/5.0 Chrome/120".into());
    ctx.request_ua = Some("Mozilla/5.0 Safari/17".into());
    insta::assert_snapshot!(run("credential_cloner", ctx).await);
}

#[tokio::test]
async fn g_ip_abuse_heavily_reported() {
    let mut ctx = RiskContextBuilder::new().action(RiskAction::Login).build();
    ctx.ip_abuse_confidence = Some(92);
    insta::assert_snapshot!(run("ip_abuse_heavily_reported", ctx).await);
}

// ─────────────────────────────────────────────────────────────────────────────
// CATEGORY 8 — Escalation memory
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn g_repeated_challenges_escalate_to_hold() {
    let ctx = RiskContextBuilder::new()
        .action(RiskAction::Login)
        .login_attempts_5m(7)
        .failed_login_attempts_1h(4)
        .build();
    // Three prior Challenges within the last 5 minutes should promote to Hold.
    // The escalation module filters by wall-clock Utc::now(), so anchor
    // ts_unix to real time rather than the pinned snapshot clock.
    let ts = Utc::now().timestamp();
    let recent = vec![
        EscalationEntry { score: 55, decision: Decision::Challenge, ts_unix: ts - 30 },
        EscalationEntry { score: 52, decision: Decision::Challenge, ts_unix: ts - 60 },
        EscalationEntry { score: 58, decision: Decision::Challenge, ts_unix: ts - 90 },
    ];
    insta::assert_snapshot!(run_with_recent("repeated_challenges_escalate_to_hold", ctx, recent).await);
}

#[tokio::test]
async fn g_no_escalation_on_clean_history() {
    let ctx = RiskContextBuilder::new()
        .action(RiskAction::Login)
        .login_attempts_5m(7)
        .failed_login_attempts_1h(4)
        .build();
    insta::assert_snapshot!(run_with_recent("no_escalation_on_clean_history", ctx, vec![]).await);
}

// ─────────────────────────────────────────────────────────────────────────────
// CATEGORY 9 — Degraded signals
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn g_redis_degraded_sensitive_forces_challenge() {
    let ctx = RiskContextBuilder::new()
        .action(RiskAction::DeviceRevoke)
        .redis_degraded()
        .build();
    insta::assert_snapshot!(run("redis_degraded_sensitive_forces_challenge", ctx).await);
}

#[tokio::test]
async fn g_redis_degraded_non_sensitive_allows() {
    let ctx = RiskContextBuilder::new()
        .action(RiskAction::DeviceList)
        .redis_degraded()
        .build();
    insta::assert_snapshot!(run("redis_degraded_non_sensitive_allows", ctx).await);
}

// ─────────────────────────────────────────────────────────────────────────────
// CATEGORY 10 — Org multiplier / cluster shift
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn g_org_under_attack_amplifies_velocity() {
    let ctx = RiskContextBuilder::new()
        .action(RiskAction::Login)
        .org_risk_level(OrgRiskLevel::UnderAttack)
        .login_attempts_5m(5)
        .failed_login_attempts_1h(3)
        .build();
    insta::assert_snapshot!(run("org_under_attack_amplifies_velocity", ctx).await);
}

#[tokio::test]
async fn g_org_elevated_moderate_velocity() {
    let ctx = RiskContextBuilder::new()
        .action(RiskAction::Login)
        .org_risk_level(OrgRiskLevel::Elevated)
        .login_attempts_5m(4)
        .build();
    insta::assert_snapshot!(run("org_elevated_moderate_velocity", ctx).await);
}
