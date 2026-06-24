use std::net::IpAddr;
use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use chrono::Utc;
use uuid::Uuid;

use risk_engine::{
    evaluate,
    store::{SignalStore, VelocityCounters},
    CredentialStatus, Decision, OrgRiskLevel, PolicyConfig, RiskAction, RiskContext,
};

use crate::middleware::risk_store::GatewaySignalStore;
use crate::state::{AppState, Metrics};

// Injected into request extensions for downstream handlers and observability.
// risk_score is not consumed by the proxy itself but is available to any
// handler added in the future.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct PolicyContext {
    pub risk_score: u8,
}

/// Device context injected by [`super::device_context::inject_device_context`].
/// Populated from Postgres (status) and Redis (recently_added flag).
/// NOT derived from client-supplied headers.
#[derive(Clone, Debug)]
pub struct DeviceContext {
    pub device_id: String,
    pub status: String,
    // recently_added is consumed by the risk engine via CredentialStatus mapping;
    // kept for future enrichment (e.g. device_trust_level signal).
    #[allow(dead_code)]
    pub recently_added: bool,
}

/// Step-up verification result injected by [`super::step_up_context::inject_step_up_context`].
/// Populated by checking `stepup:completed:{session}` in Redis.
/// NOT derived from client-supplied headers.
#[derive(Clone, Debug)]
pub struct StepUpContext {
    // verified is available for future risk-engine enrichment (StepUpContext signal).
    #[allow(dead_code)]
    pub verified: bool,
}

/// Returns true when `segment` appears as a complete slash-delimited segment
/// in `path`.  Prevents `contains("/devices")` from matching `/devices-export`.
fn has_segment(path: &str, segment: &str) -> bool {
    path.split('/').any(|s| s == segment)
}

pub(crate) fn path_to_action(path: &str) -> RiskAction {
    if path.starts_with("/auth/login") {
        return RiskAction::Login;
    }
    if path.starts_with("/auth/register") {
        return RiskAction::Register;
    }
    if path.starts_with("/auth/oauth") {
        return RiskAction::OauthComplete;
    }
    // Specific device sub-actions checked before the general /devices segment.
    if path.contains("/devices/revoke") {
        return RiskAction::DeviceRevoke;
    }
    if path.contains("/devices/mark-lost") {
        return RiskAction::DeviceMarkLost;
    }
    if has_segment(path, "devices") {
        return RiskAction::DeviceList;
    }
    if path.contains("/recovery/start") {
        return RiskAction::RecoveryStart;
    }
    if path.contains("/recovery/approve") {
        return RiskAction::RecoveryApprove;
    }
    if path.contains("/recovery/complete") {
        return RiskAction::RecoveryComplete;
    }
    // /actions/challenge checked before general /actions segment.
    if path.contains("/actions/challenge") {
        return RiskAction::ActionChallenge;
    }
    if has_segment(path, "actions") {
        // Pass the action *name* (the segment after `actions/`), NOT the whole
        // path. The engine matches exact names ("add_admin", "rotate_api_key",
        // "delete_resource") in `RiskAction::is_critical_privileged`; a full path
        // never matches, so critical-action detection, the add_admin adder, and
        // the VPN-on-critical gate would all be silently dead.
        let action_name = path
            .split('/')
            .skip_while(|s| *s != "actions")
            .nth(1)
            .filter(|s| !s.is_empty())
            .unwrap_or(path)
            .to_string();
        return RiskAction::ActionExecute { action_name };
    }
    if has_segment(path, "audit") {
        return RiskAction::AuditLogExport;
    }
    RiskAction::Unknown {
        name: path.to_string(),
    }
}

/// Inputs needed to assemble the gateway's [`RiskContext`]. Grouped into a
/// struct (rather than threaded through `enforce_policy` inline) so the context
/// assembly is a pure function — unit-testable without a live Redis/HTTP stack.
struct RiskContextInputs {
    user_id: Uuid,
    username: String,
    credential_id: Option<Uuid>,
    credential_status: CredentialStatus,
    action: RiskAction,
    request_ip: Option<IpAddr>,
    request_ua: Option<String>,
    accept_language: Option<String>,
    nonce_present: bool,
    counters: VelocityCounters,
    /// True when the velocity-counter fetch from Redis failed. Propagated to the
    /// engine as `redis_signals_degraded` so a Redis outage force-challenges
    /// sensitive actions instead of being treated as a clean signal set.
    redis_degraded: bool,
}

/// Assemble the `RiskContext` for a proxied request.
///
/// The gateway is a **coarse pre-filter**, not the authoritative risk evaluator:
/// it does not enrich (no GeoIP/breach/ASN lookups), so it only feeds the engine
/// the signals it can directly observe (account lock, velocity/escalation, nonce,
/// device status). Enrichment-dependent signals are explicitly marked
/// "not evaluated here" (`Some(false)`) so the engine's fail-closed hard gates
/// (sanctions, breach) do not deny all traffic. The identity service, which runs
/// the engine *after* enrichment, remains authoritative for those.
fn build_risk_context(input: RiskContextInputs) -> RiskContext {
    let RiskContextInputs {
        user_id,
        username,
        credential_id,
        credential_status,
        action,
        request_ip,
        request_ua,
        accept_language,
        nonce_present,
        counters,
        redis_degraded,
    } = input;

    RiskContext {
        request_id: Uuid::new_v4(),
        evaluated_at: Utc::now(),
        user_id,
        username,
        credential_id,
        action,
        resource_id: None,
        credential_status,
        // The gateway doesn't fetch the full credential row — identity owns that
        // data. Credential age is unknown here, so use a far-past sentinel: a
        // fake `now` would make GATE 15 (fresh-credential, identity-enforced)
        // fire on every privileged action.
        credential_created_at: Utc::now() - chrono::Duration::days(3650),
        credential_last_used_at: None,
        credential_sign_count_prev: 0,
        credential_sign_count_new: 0,
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
        request_ip,
        request_ua,
        request_geo: None,
        ip_asn_type: None,
        login_attempts_5m: counters.login_attempts_5m,
        failed_login_attempts_1h: counters.failed_login_attempts_1h,
        actions_executed_5m: counters.actions_executed_5m,
        recovery_requests_24h: counters.recovery_requests_24h,
        device_revocations_1h: counters.device_revocations_1h,
        registrations_from_ip_10m: counters.registrations_from_ip_10m,
        active_session_count: counters.active_session_count,
        account_locked: counters.account_locked,
        recovery_pending: counters.recovery_pending,
        oauth_authorize_ip: None,
        nonce_present,
        nonce_already_used: false,
        org_risk_level: OrgRiskLevel::Normal,
        // The gateway is a no-DB adapter *by design*, not a degraded one — so
        // db_signals_degraded is false. Marking it true would force-challenge
        // every sensitive action regardless of risk (over-block); the coarse
        // pre-filter should allow unless a concrete observable signal fires.
        // redis_signals_degraded reflects the actual counter fetch.
        redis_signals_degraded: redis_degraded,
        db_signals_degraded: false,
        tenant_id: "default".to_string(),
        org_risk_score: None,
        cluster_membership: None,
        threshold_shift: None,
        org_active_cluster_count: 0,
        login_attempts_1m: counters.login_attempts_1m,
        login_attempts_1h: counters.login_attempts_1h,
        login_attempts_24h: counters.login_attempts_24h,
        client_timestamp: None,
        device_fingerprint_hash: None,
        ja3_fingerprint: None,
        known_ip_for_user: None,
        ip_is_vpn: None,
        ip_is_proxy: None,
        ip_is_relay: None,
        ip_abuse_confidence: None,
        geo_allowed_countries: None,
        // Sanctions and breach are enrichment-dependent and the gateway does not
        // enrich. `Some(false)` = "not evaluated at this tier" so the fail-closed
        // gates (GATE 9 sanctioned, GATE 10 breached) don't deny all traffic;
        // the identity service enforces these authoritatively downstream.
        is_sanctioned_country: Some(false),
        webdriver_detected: None,
        captcha_score: None,
        screen_resolution: None,
        touch_capable: None,
        account_age_days: None,
        email_verified: None,
        email_domain_disposable: None,
        breached_credential: Some(false),
        user_typical_hours: None,
        accept_language,
        previous_accept_language: None,
        device_trust_level: None,
        policy: Arc::new(PolicyConfig::default()),
    }
}

/// A pure `Challenge` is satisfied once the user has completed step-up for the
/// current session; `Hold`/`Deny` (including escalated verdicts) are never
/// downgraded. Returns the decision the gateway should actually enforce.
fn apply_step_up(decision: Decision, step_up_verified: bool) -> Decision {
    if step_up_verified && matches!(decision, Decision::Challenge) {
        Decision::Allow
    } else {
        decision
    }
}

pub async fn enforce_policy(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Own the path string so we can mutably borrow req later (extensions_mut).
    let raw_path = req.uri().path().to_owned();
    let path: &str = raw_path.strip_prefix("/api").unwrap_or(&raw_path);

    let claims_opt = req
        .extensions()
        .get::<crate::middleware::auth::Claims>()
        .cloned();

    // Without an authenticated session, only allow public paths.
    let Some(claims) = claims_opt else {
        if path.starts_with("/private") {
            return Err(StatusCode::UNAUTHORIZED);
        }
        req.extensions_mut().insert(PolicyContext { risk_score: 0 });
        return Ok(next.run(req).await);
    };

    let device_ctx = req.extensions().get::<DeviceContext>().cloned();

    // The identity service mints `sub = user_id` (a UUID). A valid-signature
    // token whose `sub` is not a UUID is anomalous — reject it rather than
    // silently mapping it to the nil UUID, which would pool every such request's
    // velocity / escalation / lock state into one shared bucket.
    let Ok(user_id) = Uuid::parse_str(&claims.sub) else {
        tracing::warn!(sub = %claims.sub, "policy: non-UUID subject claim rejected");
        return Err(StatusCode::UNAUTHORIZED);
    };

    // Derive the client IP from the trusted-peer logic — NOT raw X-Forwarded-For.
    // request_ip feeds IP-based risk signals (RFC-1918 trust bonus, known-IP
    // history, geo). Trusting client-supplied XFF here would let an attacker
    // send `X-Forwarded-For: 10.0.0.1` to claim an internal address and lower
    // their own risk score.
    let request_ip: Option<IpAddr> = crate::middleware::client_ip(&req);

    let request_ua = req
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let accept_language = req
        .headers()
        .get("accept-language")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let (credential_id, credential_status) = match &device_ctx {
        Some(d) => (
            Uuid::parse_str(&d.device_id).ok(),
            match d.status.as_str() {
                "active" => CredentialStatus::Active,
                "lost" => CredentialStatus::Lost,
                "revoked" => CredentialStatus::Revoked,
                _ => CredentialStatus::Active,
            },
        ),
        None => (None, CredentialStatus::Active),
    };

    let action = path_to_action(path);
    let nonce_present = req.headers().contains_key("x-nonce");

    let store = GatewaySignalStore::new(state.redis.clone());

    // Fetch velocity counters before calling evaluate() so the engine sees the
    // current burst state for this user. Do NOT fail open on a Redis error: a
    // zeroed counter set is indistinguishable from a clean user and would read
    // `account_locked = false`. Flag the degradation instead so the engine
    // force-challenges sensitive actions rather than trusting empty signals.
    let (counters, redis_degraded) =
        match SignalStore::get_velocity_counters(&store, user_id, request_ip).await {
            Ok(c) => (c, false),
            Err(e) => {
                tracing::warn!(error = %e, "policy: velocity counter fetch failed; marking redis degraded");
                (VelocityCounters::default(), true)
            }
        };

    let ctx = build_risk_context(RiskContextInputs {
        user_id,
        username: claims.sub.clone(),
        credential_id,
        credential_status,
        action,
        request_ip,
        request_ua,
        accept_language,
        nonce_present,
        counters,
        redis_degraded,
    });

    let decision = evaluate(ctx, &store).await;
    // Use the score the verdict was actually made on (post org-bias), not the
    // raw pre-bias `score` field, for observability and the PolicyContext.
    let final_score = decision.adjusted_score;

    // Honour a completed step-up: a pure Challenge is satisfied if the user has
    // already verified for this session (StepUpContext, injected upstream from
    // `stepup:completed:{session}`). Otherwise the gateway re-challenges every
    // request and escalation eventually locks a legitimate user out.
    let step_up_verified = req
        .extensions()
        .get::<StepUpContext>()
        .map(|s| s.verified)
        .unwrap_or(false);
    let effective_decision = apply_step_up(decision.decision.clone(), step_up_verified);

    tracing::info!(
        %path,
        score = final_score,
        decision = ?decision.decision,
        effective = ?effective_decision,
        step_up_verified,
        enforce = state.policy_enforce,
        "policy: risk engine decision"
    );

    let blocking_status = match &effective_decision {
        Decision::Allow => None,
        // Challenge → 401 so the client knows to present a step-up prompt.
        Decision::Challenge => Some(StatusCode::UNAUTHORIZED),
        // Hold → 202 so the client knows the request is queued for review.
        Decision::Hold => Some(StatusCode::ACCEPTED),
        Decision::Deny => Some(StatusCode::FORBIDDEN),
    };

    if let Some(status) = blocking_status {
        Metrics::inc(&state.metrics.policy_blocked_locked);
        if state.policy_enforce {
            return Err(status);
        }
        Metrics::inc(&state.metrics.policy_would_block_dry_run);
        tracing::warn!(?status, score = final_score, "policy(dry-run): would block");
    }

    req.extensions_mut().insert(PolicyContext {
        risk_score: final_score,
    });
    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_paths() {
        assert!(matches!(path_to_action("/auth/login"), RiskAction::Login));
        assert!(matches!(path_to_action("/auth/login/passkey"), RiskAction::Login));
        assert!(matches!(path_to_action("/auth/register"), RiskAction::Register));
        assert!(matches!(path_to_action("/auth/oauth/callback"), RiskAction::OauthComplete));
    }

    #[test]
    fn device_paths_exact_segments() {
        assert!(matches!(path_to_action("/devices/revoke/abc"), RiskAction::DeviceRevoke));
        assert!(matches!(path_to_action("/devices/mark-lost/abc"), RiskAction::DeviceMarkLost));
        assert!(matches!(path_to_action("/devices"), RiskAction::DeviceList));
        assert!(matches!(path_to_action("/user/devices"), RiskAction::DeviceList));
        assert!(matches!(path_to_action("/user/devices/"), RiskAction::DeviceList));
    }

    #[test]
    fn device_substring_does_not_match() {
        // `/devices-export` is a different segment — must not be DeviceList.
        assert!(matches!(path_to_action("/vendor/devices-export"), RiskAction::Unknown { .. }));
        assert!(matches!(path_to_action("/my-devices-list"), RiskAction::Unknown { .. }));
    }

    #[test]
    fn recovery_paths() {
        assert!(matches!(path_to_action("/recovery/start"), RiskAction::RecoveryStart));
        assert!(matches!(path_to_action("/recovery/approve"), RiskAction::RecoveryApprove));
        assert!(matches!(path_to_action("/recovery/complete"), RiskAction::RecoveryComplete));
    }

    #[test]
    fn action_paths() {
        assert!(matches!(path_to_action("/actions/challenge"), RiskAction::ActionChallenge));
        // ActionExecute must carry the action *name* (segment after `actions/`),
        // not the full path — otherwise the engine's exact-name match for
        // critical privileged actions never fires.
        match path_to_action("/user/actions/do-this") {
            RiskAction::ActionExecute { action_name } => assert_eq!(action_name, "do-this"),
            other => panic!("expected ActionExecute, got {other:?}"),
        }
        match path_to_action("/actions/add_admin") {
            RiskAction::ActionExecute { action_name } => assert_eq!(action_name, "add_admin"),
            other => panic!("expected ActionExecute, got {other:?}"),
        }
        // `/actions-log` is a different segment — must not be ActionExecute.
        assert!(matches!(path_to_action("/user/actions-log"), RiskAction::Unknown { .. }));
    }

    #[test]
    fn audit_path() {
        assert!(matches!(path_to_action("/audit"), RiskAction::AuditLogExport));
        assert!(matches!(path_to_action("/admin/audit"), RiskAction::AuditLogExport));
        // `/pre-audit` is a different segment.
        assert!(matches!(path_to_action("/pre-audit"), RiskAction::Unknown { .. }));
    }

    #[test]
    fn unknown_path() {
        assert!(matches!(path_to_action("/profile"), RiskAction::Unknown { .. }));
        assert!(matches!(path_to_action("/"), RiskAction::Unknown { .. }));
    }

    // ── Regression: the gateway context must not trip the engine's fail-closed
    //    hard gates (the bug that denied all authenticated traffic). ──────────

    use async_trait::async_trait;
    use risk_engine::store::EscalationEntry;
    use risk_engine::RiskEngineError;

    /// Minimal store: every call succeeds with empty/neutral data, so the only
    /// thing under test is the context the gateway builds.
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
            current: &Decision,
            _score: u8,
            _ts: i64,
        ) -> Result<(Decision, bool), RiskEngineError> {
            Ok((current.clone(), false))
        }
        async fn lock_account(&self, _u: Uuid, _ttl: u64) -> Result<(), RiskEngineError> {
            Ok(())
        }
        async fn block_ip(&self, _ip: IpAddr, _ttl: u64) -> Result<(), RiskEngineError> {
            Ok(())
        }
        async fn get_jwt_fingerprint(
            &self,
            _k: &str,
        ) -> Result<Option<String>, RiskEngineError> {
            Ok(None)
        }
        async fn set_jwt_fingerprint(
            &self,
            _k: &str,
            _v: &str,
            _ttl: u64,
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

    fn benign_inputs(action: RiskAction) -> RiskContextInputs {
        RiskContextInputs {
            user_id: Uuid::new_v4(),
            username: "11111111-1111-1111-1111-111111111111".into(),
            credential_id: Some(Uuid::new_v4()),
            credential_status: CredentialStatus::Active,
            action,
            request_ip: None,
            request_ua: Some("Mozilla/5.0".into()),
            accept_language: Some("en-US".into()),
            nonce_present: true,
            counters: VelocityCounters::default(),
            redis_degraded: false,
        }
    }

    #[test]
    fn context_marks_enrichment_signals_not_evaluated() {
        let ctx = build_risk_context(benign_inputs(RiskAction::Login));
        // These are exactly the values that keep the fail-closed gates from
        // denying everything at the un-enriched gateway tier.
        assert_eq!(ctx.is_sanctioned_country, Some(false));
        assert_eq!(ctx.breached_credential, Some(false));
        assert!(!ctx.db_signals_degraded);
    }

    #[tokio::test]
    async fn benign_authenticated_request_is_allowed() {
        let ctx = build_risk_context(benign_inputs(RiskAction::Login));
        let decision = evaluate(ctx, &NoopStore).await;
        assert_eq!(
            decision.decision,
            Decision::Allow,
            "a benign authenticated request must not be blocked by the gateway pre-filter",
        );
    }

    #[tokio::test]
    async fn redis_degraded_forces_challenge_on_sensitive_action() {
        // When Redis is unreachable the gateway flags degradation; the engine
        // must then force a step-up on sensitive actions rather than allow.
        let mut inputs = benign_inputs(RiskAction::Login); // Login is sensitive
        inputs.redis_degraded = true;
        let ctx = build_risk_context(inputs);
        let decision = evaluate(ctx, &NoopStore).await;
        assert_eq!(decision.decision, Decision::Challenge);
    }

    #[test]
    fn step_up_clears_challenge_only() {
        // A completed step-up satisfies a pure Challenge…
        assert_eq!(apply_step_up(Decision::Challenge, true), Decision::Allow);
        // …does nothing when step-up was not completed…
        assert_eq!(apply_step_up(Decision::Challenge, false), Decision::Challenge);
        // …and never downgrades Hold/Deny (incl. escalated verdicts) or Allow.
        assert_eq!(apply_step_up(Decision::Hold, true), Decision::Hold);
        assert_eq!(apply_step_up(Decision::Deny, true), Decision::Deny);
        assert_eq!(apply_step_up(Decision::Allow, true), Decision::Allow);
    }
}
