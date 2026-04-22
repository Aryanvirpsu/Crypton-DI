//! Criterion bench for `engine::evaluate`.
//!
//! We measure the *engine* — not Redis, not Postgres — so the bench uses
//! an in-process `NoopStore` that returns empty state from every trait
//! method. The scoring logic itself is fully CPU-bound; the async wrapping
//! costs one Tokio hop per call.
//!
//! Run:
//! ```bash
//! cargo bench --bench engine_evaluate
//! ```
//!
//! The numbers you care about:
//!   - **allow_hot_path**: minimal context, no gates, no degraded signals.
//!     Represents the 99th-percentile common case. Should stay under a
//!     few microseconds.
//!   - **deny_hard_gate**: context that trips a hard gate (sanctioned
//!     country). Exercises the short-circuit path; should be faster
//!     than the full scoring path.
//!   - **challenge_degraded**: redis_signals_degraded on a sensitive
//!     action. Exercises the early-exit degraded branch.
//!
//! These are relative baselines — rerun after any scoring change to spot
//! regressions. Absolute targets belong in RUNBOOK.md.

use std::net::IpAddr;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tokio::runtime::Runtime;
use uuid::Uuid;

use risk_engine::{
    context::{CredentialStatus, OrgRiskLevel, RiskAction, RiskContext},
    decision::Decision,
    engine::evaluate,
    store::{EscalationEntry, SignalStore, VelocityCounters},
    PolicyConfig, RiskEngineError,
};

struct NoopStore;

#[async_trait]
impl SignalStore for NoopStore {
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
    async fn lock_account(&self, _u: Uuid, _ttl: u64) -> Result<(), RiskEngineError> {
        Ok(())
    }
    async fn block_ip(&self, _ip: IpAddr, _ttl: u64) -> Result<(), RiskEngineError> {
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
        Ok(())
    }
}

fn base_ctx() -> RiskContext {
    RiskContext {
        request_id: Uuid::new_v4(),
        evaluated_at: Utc::now(),
        user_id: Uuid::new_v4(),
        username: "bench-user".into(),
        credential_id: Some(Uuid::new_v4()),
        action: RiskAction::Login,
        resource_id: None,
        credential_status: CredentialStatus::Active,
        credential_created_at: Utc::now(),
        credential_last_used_at: None,
        credential_sign_count_prev: 0,
        credential_sign_count_new: 1,
        credential_registered_ua: None,
        credential_count_for_user: 1,
        prior_audit_event_count: 0,
        last_login_ip: None,
        last_login_at: None,
        last_login_geo: None,
        jwt_issued_at: None,
        jwt_expires_at: None,
        jwt_fingerprint_stored: None,
        jwt_fingerprint_current: None,
        request_ip: Some("203.0.113.1".parse().unwrap()),
        request_ua: Some("bench".into()),
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
        nonce_present: false,
        nonce_already_used: false,
        org_risk_level: OrgRiskLevel::Normal,
        redis_signals_degraded: false,
        db_signals_degraded: false,
        tenant_id: "bench".into(),
        org_risk_score: None,
        cluster_membership: None,
        threshold_shift: None,
        org_active_cluster_count: 0,
        login_attempts_1m: Some(0),
        login_attempts_1h: Some(1),
        login_attempts_24h: Some(1),
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
        account_age_days: Some(365),
        email_verified: Some(true),
        email_domain_disposable: Some(false),
        breached_credential: Some(false),
        user_typical_hours: None,
        accept_language: None,
        previous_accept_language: None,
        device_trust_level: None,
        policy: Arc::new(PolicyConfig::default()),
    }
}

fn bench_engine(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let store = NoopStore;

    c.bench_function("allow_hot_path", |b| {
        b.to_async(&rt).iter(|| async {
            let ctx = base_ctx();
            let d = evaluate(black_box(ctx), black_box(&store)).await;
            black_box(d);
        });
    });

    c.bench_function("deny_hard_gate", |b| {
        b.to_async(&rt).iter(|| async {
            let mut ctx = base_ctx();
            // Trip the "revoked credential" hard gate — fastest deny path.
            ctx.credential_status = CredentialStatus::Revoked;
            let d = evaluate(black_box(ctx), black_box(&store)).await;
            black_box(d);
        });
    });

    c.bench_function("challenge_degraded", |b| {
        b.to_async(&rt).iter(|| async {
            let mut ctx = base_ctx();
            // Sensitive action + degraded redis → early challenge exit.
            ctx.action = RiskAction::DeviceRevoke;
            ctx.redis_signals_degraded = true;
            let d = evaluate(black_box(ctx), black_box(&store)).await;
            black_box(d);
        });
    });
}

criterion_group!(benches, bench_engine);
criterion_main!(benches);
