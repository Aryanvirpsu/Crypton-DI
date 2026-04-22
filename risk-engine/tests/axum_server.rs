//! Integration test for the Axum HTTP wrapper.
//!
//! Runs only with `--features axum-server`. Uses an in-process MockStore
//! (no network) so this test can run in CI without Redis or Postgres.
#![cfg(feature = "axum-server")]

use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use chrono::{Duration, Utc};
use tower::ServiceExt;
use uuid::Uuid;

use risk_engine::{
    adapters::axum_server::{router, AppState, ReadinessProbe},
    context::{CredentialStatus, OrgRiskLevel, RiskAction, RiskContext},
    decision::Decision,
    store::{EscalationEntry, SignalStore, VelocityCounters},
    PolicyConfig, RiskEngineError,
};

#[derive(Default)]
struct MockStore {
    recent: Mutex<Vec<EscalationEntry>>,
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
    async fn get_jwt_fingerprint(
        &self,
        _k: &str,
    ) -> Result<Option<String>, RiskEngineError> { Ok(None) }
    async fn set_jwt_fingerprint(
        &self,
        _k: &str,
        _v: &str,
        _t: u64,
    ) -> Result<(), RiskEngineError> { Ok(()) }
    async fn is_nonce_used(&self, _n: &str) -> Result<bool, RiskEngineError> { Ok(false) }
    async fn consume_nonce(&self, _n: &str) -> Result<(), RiskEngineError> { Ok(()) }
}

fn minimal_ctx() -> RiskContext {
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
async fn health_endpoint_returns_ok() {
    let state = AppState::new(Arc::new(MockStore::default()));
    let app = router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), 1024).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn evaluate_endpoint_returns_decision() {
    let state = AppState::new(Arc::new(MockStore::default()));
    let app = router(state);

    let ctx = minimal_ctx();
    let body = serde_json::to_vec(&ctx).unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/evaluate")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
    // RiskDecision holds `engine_version: &'static str`, which forces its
    // Deserialize bound to `'static`. For transport-level assertions we just
    // read the JSON as a loose Value and check the shaped fields.
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["decision"], serde_json::json!(Decision::Allow));
    assert!(body["score"].as_u64().is_some());
    assert!(body["engine_version"].as_str().is_some());
}

#[tokio::test]
async fn evaluate_endpoint_rejects_bad_json() {
    let state = AppState::new(Arc::new(MockStore::default()));
    let app = router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/evaluate")
                .header("content-type", "application/json")
                .body(Body::from("{not json"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(resp.status().is_client_error());
}

// ─────────────────────────────────────────────────────────────────────────────
// Readiness probe tests
// ─────────────────────────────────────────────────────────────────────────────

struct AlwaysReady;
#[async_trait]
impl ReadinessProbe for AlwaysReady {
    async fn check(&self) -> Result<(), String> {
        Ok(())
    }
}

struct AlwaysBroken;
#[async_trait]
impl ReadinessProbe for AlwaysBroken {
    async fn check(&self) -> Result<(), String> {
        Err("redis unreachable".into())
    }
}

struct NeverReturns;
#[async_trait]
impl ReadinessProbe for NeverReturns {
    async fn check(&self) -> Result<(), String> {
        // Longer than the handler's 1s budget — forces the timeout path.
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        Ok(())
    }
}

#[tokio::test]
async fn ready_endpoint_200_when_no_probe_configured() {
    let state = AppState::new(Arc::new(MockStore::default()));
    let app = router(state);
    let resp = app
        .oneshot(Request::builder().uri("/ready").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn ready_endpoint_200_when_probe_ok() {
    let state =
        AppState::new(Arc::new(MockStore::default())).with_readiness(Arc::new(AlwaysReady));
    let app = router(state);
    let resp = app
        .oneshot(Request::builder().uri("/ready").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn ready_endpoint_503_when_probe_fails() {
    let state =
        AppState::new(Arc::new(MockStore::default())).with_readiness(Arc::new(AlwaysBroken));
    let app = router(state);
    let resp = app
        .oneshot(Request::builder().uri("/ready").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    let bytes = to_bytes(resp.into_body(), 1024).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["status"], "unavailable");
    assert_eq!(body["error"], "redis unreachable");
}

#[tokio::test]
async fn ready_endpoint_503_when_probe_stalls() {
    let state =
        AppState::new(Arc::new(MockStore::default())).with_readiness(Arc::new(NeverReturns));
    let app = router(state);
    let resp = app
        .oneshot(Request::builder().uri("/ready").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    let bytes = to_bytes(resp.into_body(), 1024).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["status"], "unavailable");
    assert!(body["error"].as_str().unwrap().contains("exceeded"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Body-size limit
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn evaluate_rejects_body_over_limit() {
    let state = AppState::new(Arc::new(MockStore::default()));
    let app = router(state);

    // 1 MiB of `a` — well past the 64 KiB default cap.
    let oversized = vec![b'a'; 1024 * 1024];

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/evaluate")
                .header("content-type", "application/json")
                .body(Body::from(oversized))
                .unwrap(),
        )
        .await
        .unwrap();

    // Axum/tower returns 413 (Payload Too Large) from the limit layer.
    assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
}
