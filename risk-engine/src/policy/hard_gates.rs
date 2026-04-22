use crate::context::{AsnType, CredentialStatus, RiskAction, RiskContext};
use crate::decision::HardGate;

/// Result of hard gate evaluation.
pub enum GateOutcome {
    /// All gates passed. Scoring may proceed.
    Passed(Vec<HardGate>),

    /// One or more gates fired a hard block.
    Blocked(Vec<HardGate>),
}

/// Evaluate all hard policy gates against the provided context.
///
/// Hard gates are binary: they either block immediately (returning `Blocked`)
/// or pass through to continuous scoring. No partial scores are assigned here.
///
/// Evaluation order matters: earlier gates short-circuit later ones. The
/// ordering below prioritises the most severe conditions.
pub fn evaluate_hard_gates(ctx: &RiskContext) -> GateOutcome {
    let mut blocking: Vec<HardGate> = Vec::new();

    // ── GATE 1: Revoked credential ────────────────────────────────────────────
    // Defence-in-depth backstop — the identity service's AuthUser extractor
    // should have rejected this before reaching the risk engine.
    if ctx.credential_status == CredentialStatus::Revoked {
        blocking.push(HardGate::RevokedCredential);
    }

    // ── GATE 1.5: Lost credential ─────────────────────────────────────────────
    // Lost credentials are explicitly marked as compromised and should be denied.
    // Distinct gate variant from RevokedCredential so downstream audit and
    // alerting can tell the two causes apart.
    if ctx.credential_status == CredentialStatus::Lost {
        blocking.push(HardGate::LostCredential);
    }

    // ── GATE 2: Account locked ────────────────────────────────────────────────
    if ctx.account_locked {
        blocking.push(HardGate::AccountLocked);
    }

    // ── GATE 3: Nonce replay ──────────────────────────────────────────────────
    if ctx.nonce_already_used {
        blocking.push(HardGate::NonceReplay);
    }

    // ── GATE 4: Sign count rollback (credential cloning) ─────────────────────
    // WebAuthn sign count must always increase. A non-increasing counter when
    // the authenticator previously reported a non-zero count indicates the
    // credential has been cloned. Authenticators that don't support counters
    // always report 0 — those are not flagged.
    // WebAuthn counters are unsigned per spec; any negative value is either
    // an authenticator bug or tampering — block regardless of prev.
    if ctx.credential_sign_count_new < 0 {
        blocking.push(HardGate::SignCountRollback);
    } else if ctx.credential_sign_count_prev > 0
        && ctx.credential_sign_count_new <= ctx.credential_sign_count_prev
    {
        blocking.push(HardGate::SignCountRollback);
    }

    // ── GATE 5: Impossible travel + Tor ──────────────────────────────────────
    let impossible_travel = is_impossible_travel(ctx);
    if impossible_travel && matches!(&ctx.ip_asn_type, Some(AsnType::Tor)) {
        blocking.push(HardGate::ImpossibleTravelViaTor);
    }

    // ── GATE 6: Tor on sensitive action ──────────────────────────────────────
    if matches!(&ctx.ip_asn_type, Some(AsnType::Tor)) && ctx.action.is_sensitive() {
        if !blocking.contains(&HardGate::ImpossibleTravelViaTor) {
            blocking.push(HardGate::TorOnSensitiveAction);
        }
    }

    // ── GATE 7: Recovery complete with no active recovery ────────────────────
    if matches!(&ctx.action, RiskAction::RecoveryComplete) && !ctx.recovery_pending {
        blocking.push(HardGate::RecoveryCompletedWithNoPending);
    }

    // ── GATE 8: OAuth nonce IP mismatch ───────────────────────────────────────
    // The OAuth authorization code is a single-use credential; any change of
    // IP between /authorize and /token is treated as code theft.
    // Tor-specific variant is preserved for alerting differentiation.
    if let (Some(auth_ip), Some(req_ip)) = (ctx.oauth_authorize_ip, ctx.request_ip) {
        if auth_ip != req_ip && matches!(&ctx.action, RiskAction::OauthComplete) {
            if matches!(&ctx.ip_asn_type, Some(AsnType::Tor)) {
                blocking.push(HardGate::OauthNonceIpMismatchOnTor);
            } else {
                blocking.push(HardGate::OauthIpMismatch);
            }
        }
    }

    // ── GATE 9: Sanctioned country ───────────────────────────────────────────
    // Legal compliance requirement — requests from OFAC/EU sanctioned
    // jurisdictions must be denied regardless of score. No override path.
    if ctx.is_sanctioned_country == Some(true) {
        blocking.push(HardGate::SanctionedCountry);
    }

    // ── GATE 10: Breached credential on sensitive action ─────────────────────
    // If the credential has been found in a known breach database, deny any
    // sensitive action. The user must rotate the credential first.
    if ctx.breached_credential == Some(true) && ctx.action.is_sensitive() {
        blocking.push(HardGate::BreachedCredentialOnSensitive);
    }

    // ── GATE 11: VPN on critical privileged action ───────────────────────────
    // Critical actions that require full attribution (recovery approve/complete,
    // add_admin, rotate_api_key, delete_resource) are blocked when a VPN is
    // detected. This forces the user to connect from an attributable network.
    if is_vpn_detected(ctx) && is_critical_privileged_action(&ctx.action) {
        blocking.push(HardGate::VpnOnCriticalAction);
    }

    // ── GATE 12: Tor on recovery (zero tolerance) ────────────────────────────
    if matches!(&ctx.ip_asn_type, Some(AsnType::Tor)) && ctx.action.is_recovery() {
        blocking.push(HardGate::TorOnRecovery);
    }

    // ── GATE 13: Automation on registration ──────────────────────────────────
    if ctx.webdriver_detected == Some(true) && matches!(&ctx.action, RiskAction::Register) {
        blocking.push(HardGate::AutomationOnRegister);
    }

    // ── GATE 14: JWT Mismatch on Sensitive Action ─────────────────────────────
    let fingerprint_mismatch = ctx.jwt_fingerprint_stored.is_some() 
        && ctx.jwt_fingerprint_current != ctx.jwt_fingerprint_stored;
    
    if fingerprint_mismatch && ctx.action.is_sensitive() {
        blocking.push(HardGate::SensitiveActionOnMismatch);
    }

    // ── GATE 15: Fresh Credential + Privileged Action ────────────────────────
    let age_mins = (ctx.evaluated_at - ctx.credential_created_at).num_minutes();
    if age_mins < 5 && ctx.action.is_privileged() {
        blocking.push(HardGate::PrivilegedActionOnFreshCredential);
    }

    // ── GATE 16: Expired JWT + Sensitive Action ──────────────────────────────
    // Adapter should reject expired tokens at the boundary; this is a
    // defense-in-depth check for the sensitive-action path.
    if let Some(exp) = ctx.jwt_expires_at {
        if exp < ctx.evaluated_at && ctx.action.is_sensitive() {
            blocking.push(HardGate::ExpiredJwtOnSensitive);
        }
    }

    if blocking.is_empty() {
        GateOutcome::Passed(vec![])
    } else {
        GateOutcome::Blocked(blocking)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

fn is_impossible_travel(ctx: &RiskContext) -> bool {
    match (&ctx.last_login_geo, &ctx.request_geo, ctx.last_login_at) {
        (Some(last_geo), Some(cur_geo), Some(last_at)) => {
            // An inverted timestamp (last_at > evaluated_at) is itself anomalous
            // — usually clock drift or log injection. Treat as impossible rather
            // than silently granting the generous 500 km allowance.
            if last_at > ctx.evaluated_at {
                return true;
            }
            let km = last_geo.distance_km(cur_geo);
            let elapsed_h =
                crate::signals::geo::elapsed_hours(last_at, ctx.evaluated_at);
            let max_km = elapsed_h * 900.0 + 500.0;
            km > max_km
        }
        _ => false,
    }
}

/// Returns true if the request is coming through any VPN-like anonymization
/// service. Checks both the ASN classification AND the IP intelligence flags.
fn is_vpn_detected(ctx: &RiskContext) -> bool {
    matches!(&ctx.ip_asn_type, Some(AsnType::Vpn))
        || ctx.ip_is_vpn == Some(true)
        || ctx.ip_is_proxy == Some(true)
        || ctx.ip_is_relay == Some(true)
}

/// Critical privileged actions that require full network attribution.
/// These actions have irreversible security consequences if performed
/// by an attacker, so VPN/proxy anonymization is not tolerated.
fn is_critical_privileged_action(action: &RiskAction) -> bool {
    match action {
        RiskAction::RecoveryApprove | RiskAction::RecoveryComplete => true,
        RiskAction::ActionExecute { action_name } => matches!(
            action_name.as_str(),
            "add_admin" | "rotate_api_key" | "delete_resource"
        ),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{
        AsnType, CredentialStatus, OrgRiskLevel, RiskAction, RiskContext,
    };
    use crate::policy_config::PolicyConfig;
    use chrono::Utc;
    use std::sync::Arc;
    use uuid::Uuid;

    fn base_ctx() -> RiskContext {
        RiskContext {
            request_id: Uuid::new_v4(),
            evaluated_at: Utc::now(),
            user_id: Uuid::new_v4(),
            username: "alice".into(),
            credential_id: Some(Uuid::new_v4()),
            action: RiskAction::Login,
            resource_id: None,
            credential_status: CredentialStatus::Active,
            credential_created_at: Utc::now() - chrono::Duration::days(30),
            credential_last_used_at: Some(Utc::now() - chrono::Duration::days(1)),
            credential_sign_count_prev: 10,
            credential_sign_count_new: 11,
            credential_registered_ua: Some("Mozilla/5.0 Chrome/120".into()),
            credential_count_for_user: 2,
            prior_audit_event_count: 50,
            last_login_ip: None,
            last_login_at: None,
            last_login_geo: None,
            jwt_issued_at: None,
            jwt_expires_at: None,
            jwt_fingerprint_stored: None,
            jwt_fingerprint_current: None,
            request_ip: None,
            request_ua: Some("Mozilla/5.0 Chrome/120".into()),
            request_geo: None,
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
            // Org-graph fields
            tenant_id: String::new(),
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

    #[test]
    fn clean_context_passes_all_gates() {
        let ctx = base_ctx();
        assert!(matches!(evaluate_hard_gates(&ctx), GateOutcome::Passed(_)));
    }

    #[test]
    fn revoked_credential_blocks() {
        let mut ctx = base_ctx();
        ctx.credential_status = CredentialStatus::Revoked;
        let outcome = evaluate_hard_gates(&ctx);
        assert!(matches!(outcome, GateOutcome::Blocked(gates) if gates.contains(&HardGate::RevokedCredential)));
    }

    #[test]
    fn lost_credential_blocks_with_its_own_gate() {
        let mut ctx = base_ctx();
        ctx.credential_status = CredentialStatus::Lost;
        let outcome = evaluate_hard_gates(&ctx);
        match outcome {
            GateOutcome::Blocked(gates) => {
                assert!(gates.contains(&HardGate::LostCredential),
                    "Lost credentials must emit HardGate::LostCredential");
                assert!(!gates.contains(&HardGate::RevokedCredential),
                    "Lost credentials must NOT be misreported as RevokedCredential");
            }
            GateOutcome::Passed(_) => panic!("Lost credential must block"),
        }
    }

    #[test]
    fn nonce_replay_blocks() {
        let mut ctx = base_ctx();
        ctx.nonce_already_used = true;
        let outcome = evaluate_hard_gates(&ctx);
        assert!(matches!(outcome, GateOutcome::Blocked(gates) if gates.contains(&HardGate::NonceReplay)));
    }

    #[test]
    fn tor_on_sensitive_action_blocks() {
        let mut ctx = base_ctx();
        ctx.ip_asn_type = Some(AsnType::Tor);
        ctx.action = RiskAction::DeviceRevoke;
        let outcome = evaluate_hard_gates(&ctx);
        assert!(matches!(outcome, GateOutcome::Blocked(gates) if gates.contains(&HardGate::TorOnSensitiveAction)));
    }

    #[test]
    fn recovery_complete_with_no_pending_blocks() {
        let mut ctx = base_ctx();
        ctx.action = RiskAction::RecoveryComplete;
        ctx.recovery_pending = false;
        let outcome = evaluate_hard_gates(&ctx);
        assert!(matches!(
            outcome,
            GateOutcome::Blocked(gates) if gates.contains(&HardGate::RecoveryCompletedWithNoPending)
        ));
    }

    #[test]
    fn tor_on_non_sensitive_action_passes() {
        let mut ctx = base_ctx();
        ctx.ip_asn_type = Some(AsnType::Tor);
        ctx.action = RiskAction::DeviceList;
        let outcome = evaluate_hard_gates(&ctx);
        assert!(matches!(outcome, GateOutcome::Passed(_)));
    }

    #[test]
    fn sign_count_rollback_blocks() {
        let mut ctx = base_ctx();
        ctx.credential_sign_count_prev = 10;
        ctx.credential_sign_count_new = 10; // did not increment
        let outcome = evaluate_hard_gates(&ctx);
        assert!(matches!(
            outcome,
            GateOutcome::Blocked(gates) if gates.contains(&HardGate::SignCountRollback)
        ));
    }

    #[test]
    fn sign_count_backwards_blocks() {
        let mut ctx = base_ctx();
        ctx.credential_sign_count_prev = 10;
        ctx.credential_sign_count_new = 5; // went backwards
        let outcome = evaluate_hard_gates(&ctx);
        assert!(matches!(
            outcome,
            GateOutcome::Blocked(gates) if gates.contains(&HardGate::SignCountRollback)
        ));
    }

    #[test]
    fn sign_count_zero_does_not_block() {
        let mut ctx = base_ctx();
        // Authenticator doesn't support counters — always reports 0.
        ctx.credential_sign_count_prev = 0;
        ctx.credential_sign_count_new = 0;
        let outcome = evaluate_hard_gates(&ctx);
        match outcome {
            GateOutcome::Blocked(gates) => {
                assert!(!gates.contains(&HardGate::SignCountRollback));
            }
            GateOutcome::Passed(_) => {} // expected
        }
    }

    #[test]
    fn sign_count_normal_increment_passes() {
        let mut ctx = base_ctx();
        ctx.credential_sign_count_prev = 10;
        ctx.credential_sign_count_new = 11;
        let outcome = evaluate_hard_gates(&ctx);
        assert!(matches!(outcome, GateOutcome::Passed(_)));
    }

    // ── Gate 9: Sanctioned country ───────────────────────────────────────────

    #[test]
    fn sanctioned_country_blocks() {
        let mut ctx = base_ctx();
        ctx.is_sanctioned_country = Some(true);
        let outcome = evaluate_hard_gates(&ctx);
        assert!(matches!(
            outcome,
            GateOutcome::Blocked(gates) if gates.contains(&HardGate::SanctionedCountry)
        ));
    }

    #[test]
    fn non_sanctioned_country_passes() {
        let mut ctx = base_ctx();
        ctx.is_sanctioned_country = Some(false);
        let outcome = evaluate_hard_gates(&ctx);
        assert!(matches!(outcome, GateOutcome::Passed(_)));
    }

    #[test]
    fn unknown_sanctions_status_passes() {
        let mut ctx = base_ctx();
        ctx.is_sanctioned_country = None; // adapter didn't check
        let outcome = evaluate_hard_gates(&ctx);
        assert!(matches!(outcome, GateOutcome::Passed(_)));
    }

    // ── Gate 10: Breached credential on sensitive action ─────────────────────

    #[test]
    fn breached_credential_on_sensitive_action_blocks() {
        let mut ctx = base_ctx();
        ctx.breached_credential = Some(true);
        ctx.action = RiskAction::DeviceRevoke; // sensitive
        let outcome = evaluate_hard_gates(&ctx);
        assert!(matches!(
            outcome,
            GateOutcome::Blocked(gates) if gates.contains(&HardGate::BreachedCredentialOnSensitive)
        ));
    }

    #[test]
    fn breached_credential_on_non_sensitive_passes() {
        let mut ctx = base_ctx();
        ctx.breached_credential = Some(true);
        ctx.action = RiskAction::DeviceList; // non-sensitive
        let outcome = evaluate_hard_gates(&ctx);
        // Should not contain the breached gate (other gates may or may not fire)
        match outcome {
            GateOutcome::Blocked(gates) => {
                assert!(!gates.contains(&HardGate::BreachedCredentialOnSensitive));
            }
            GateOutcome::Passed(_) => {} // expected
        }
    }

    #[test]
    fn non_breached_credential_passes() {
        let mut ctx = base_ctx();
        ctx.breached_credential = Some(false);
        ctx.action = RiskAction::DeviceRevoke;
        let outcome = evaluate_hard_gates(&ctx);
        match outcome {
            GateOutcome::Blocked(gates) => {
                assert!(!gates.contains(&HardGate::BreachedCredentialOnSensitive));
            }
            GateOutcome::Passed(_) => {}
        }
    }

    // ── Gate 11: VPN on critical privileged action ───────────────────────────

    #[test]
    fn vpn_asn_on_recovery_approve_blocks() {
        let mut ctx = base_ctx();
        ctx.ip_asn_type = Some(AsnType::Vpn);
        ctx.action = RiskAction::RecoveryApprove;
        let outcome = evaluate_hard_gates(&ctx);
        assert!(matches!(
            outcome,
            GateOutcome::Blocked(gates) if gates.contains(&HardGate::VpnOnCriticalAction)
        ));
    }

    #[test]
    fn vpn_flag_on_add_admin_blocks() {
        let mut ctx = base_ctx();
        ctx.ip_is_vpn = Some(true);
        ctx.action = RiskAction::ActionExecute {
            action_name: "add_admin".into(),
        };
        let outcome = evaluate_hard_gates(&ctx);
        assert!(matches!(
            outcome,
            GateOutcome::Blocked(gates) if gates.contains(&HardGate::VpnOnCriticalAction)
        ));
    }

    #[test]
    fn proxy_flag_on_rotate_api_key_blocks() {
        let mut ctx = base_ctx();
        ctx.ip_is_proxy = Some(true);
        ctx.action = RiskAction::ActionExecute {
            action_name: "rotate_api_key".into(),
        };
        let outcome = evaluate_hard_gates(&ctx);
        assert!(matches!(
            outcome,
            GateOutcome::Blocked(gates) if gates.contains(&HardGate::VpnOnCriticalAction)
        ));
    }

    #[test]
    fn vpn_on_regular_login_passes() {
        let mut ctx = base_ctx();
        ctx.ip_asn_type = Some(AsnType::Vpn);
        ctx.action = RiskAction::Login; // not critical privileged
        let outcome = evaluate_hard_gates(&ctx);
        match outcome {
            GateOutcome::Blocked(gates) => {
                assert!(!gates.contains(&HardGate::VpnOnCriticalAction));
            }
            GateOutcome::Passed(_) => {}
        }
    }

    #[test]
    fn residential_ip_on_recovery_approve_passes() {
        let mut ctx = base_ctx();
        ctx.ip_asn_type = Some(AsnType::Residential);
        ctx.action = RiskAction::RecoveryApprove;
        let outcome = evaluate_hard_gates(&ctx);
        match outcome {
            GateOutcome::Blocked(gates) => {
                assert!(!gates.contains(&HardGate::VpnOnCriticalAction));
            }
            GateOutcome::Passed(_) => {}
        }
    }
}
