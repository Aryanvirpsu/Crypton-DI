use std::net::IpAddr;

use crate::context::{AsnType, CredentialStatus, RiskAction, RiskContext};
use crate::decision::HardGate;

/// Result of hard gate evaluation.
pub enum GateOutcome {
    /// All gates passed. Scoring may proceed.
    Passed,

    /// One or more gates fired a hard block.
    Blocked(Vec<HardGate>),
}

/// Evaluate all hard policy gates against the provided context.
///
/// Hard gates are binary: they either block immediately (returning `Blocked`)
/// or pass through to continuous scoring. No partial scores are assigned here.
///
/// All 16 gates are evaluated unconditionally so the returned `Blocked` vec
/// contains every gate that fired — this gives callers full diagnostic
/// visibility rather than stopping at the first match.
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
    let is_tor = matches!(&ctx.ip_asn_type, Some(AsnType::Tor));
    let impossible_travel_via_tor = is_impossible_travel(ctx) && is_tor;
    if impossible_travel_via_tor {
        blocking.push(HardGate::ImpossibleTravelViaTor);
    }

    // ── GATE 6: Tor on sensitive action ──────────────────────────────────────
    // Dedup via flag — if Gate 5 already fired, don't double-count Tor.
    // Per-tenant configurable via `gate_block_tor_on_sensitive`.
    if ctx.policy.gate_block_tor_on_sensitive
        && is_tor
        && ctx.action.is_sensitive()
        && !impossible_travel_via_tor
    {
        blocking.push(HardGate::TorOnSensitiveAction);
    }

    // ── GATE 7: Recovery complete with no active recovery ────────────────────
    if matches!(&ctx.action, RiskAction::RecoveryComplete) && !ctx.recovery_pending {
        blocking.push(HardGate::RecoveryCompletedWithNoPending);
    }

    // ── GATE 8: OAuth nonce IP mismatch ───────────────────────────────────────
    // The OAuth authorization code is a single-use credential; any change of
    // IP between /authorize and /token is treated as code theft.
    // Tor-specific variant is preserved for alerting differentiation.
    // IPs are canonicalized so ::ffff:1.2.3.4 matches 1.2.3.4.
    if let (Some(auth_ip), Some(req_ip)) = (ctx.oauth_authorize_ip, ctx.request_ip) {
        if canonicalize_ip(auth_ip) != canonicalize_ip(req_ip)
            && matches!(&ctx.action, RiskAction::OauthComplete)
        {
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
    // FAIL-CLOSED: if the adapter could not determine sanctions status (None),
    // the request is blocked. The adapter MUST resolve this field; a GeoIP
    // outage must not silently bypass sanctions enforcement.
    if ctx.is_sanctioned_country != Some(false) {
        blocking.push(HardGate::SanctionedCountry);
    }

    // ── GATE 10: Breached credential on sensitive action ─────────────────────
    // If the credential has been found in a known breach database, deny any
    // sensitive action. The user must rotate the credential first.
    // FAIL-CLOSED on sensitive actions: if the breach-check service is
    // unavailable (None), sensitive actions are blocked. A breach-database
    // outage must not allow compromised credentials through.
    if ctx.action.is_sensitive() && ctx.breached_credential != Some(false) {
        blocking.push(HardGate::BreachedCredentialOnSensitive);
    }

    // ── GATE 11: VPN on critical privileged action ───────────────────────────
    // Critical actions that require full attribution (recovery approve/complete,
    // add_admin, rotate_api_key, delete_resource) are blocked when a VPN is
    // detected. This forces the user to connect from an attributable network.
    // Per-tenant configurable via `gate_block_vpn_on_critical`.
    if ctx.policy.gate_block_vpn_on_critical && is_vpn_detected(ctx) && ctx.action.is_critical_privileged() {
        blocking.push(HardGate::VpnOnCriticalAction);
    }

    // ── GATE 12: Tor on recovery (zero tolerance) ────────────────────────────
    // Per-tenant configurable via `gate_block_tor_on_recovery`.
    if ctx.policy.gate_block_tor_on_recovery && is_tor && ctx.action.is_recovery() {
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
    // Negative age (credential_created_at in the future) is anomalous and
    // satisfies `< threshold`, blocking correctly without special handling.
    // Per-tenant configurable via `gate_fresh_credential_minutes`.
    let age_mins = (ctx.evaluated_at - ctx.credential_created_at).num_minutes();
    if age_mins < ctx.policy.gate_fresh_credential_minutes as i64 && ctx.action.is_privileged() {
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
        GateOutcome::Passed
    } else {
        GateOutcome::Blocked(blocking)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Canonicalize IPv4-mapped IPv6 addresses (::ffff:a.b.c.d) to their IPv4
/// equivalent so comparisons work regardless of which representation the
/// adapter or load balancer used.
fn canonicalize_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => {
            let segs = v6.segments();
            if segs[..5] == [0, 0, 0, 0, 0] && segs[5] == 0xffff {
                IpAddr::V4(std::net::Ipv4Addr::new(
                    (segs[6] >> 8) as u8,
                    segs[6] as u8,
                    (segs[7] >> 8) as u8,
                    segs[7] as u8,
                ))
            } else {
                ip
            }
        }
        _ => ip,
    }
}

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
            is_sanctioned_country: Some(false),
            // Bot / browser integrity
            webdriver_detected: None,
            captcha_score: None,
            screen_resolution: None,
            touch_capable: None,
            // Account intelligence
            account_age_days: None,
            email_verified: None,
            email_domain_disposable: None,
            breached_credential: Some(false),
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
        assert!(matches!(evaluate_hard_gates(&ctx), GateOutcome::Passed));
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
            GateOutcome::Passed => panic!("Lost credential must block"),
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
        assert!(matches!(outcome, GateOutcome::Passed));
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
            GateOutcome::Passed => {} // expected
        }
    }

    #[test]
    fn negative_sign_count_blocks() {
        let mut ctx = base_ctx();
        ctx.credential_sign_count_new = -1;
        let outcome = evaluate_hard_gates(&ctx);
        assert!(matches!(
            outcome,
            GateOutcome::Blocked(gates) if gates.contains(&HardGate::SignCountRollback)
        ));
    }

    #[test]
    fn sign_count_normal_increment_passes() {
        let mut ctx = base_ctx();
        ctx.credential_sign_count_prev = 10;
        ctx.credential_sign_count_new = 11;
        let outcome = evaluate_hard_gates(&ctx);
        assert!(matches!(outcome, GateOutcome::Passed));
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
        assert!(matches!(outcome, GateOutcome::Passed));
    }

    #[test]
    fn unknown_sanctions_status_blocks() {
        let mut ctx = base_ctx();
        ctx.is_sanctioned_country = None;
        let outcome = evaluate_hard_gates(&ctx);
        assert!(matches!(
            outcome,
            GateOutcome::Blocked(gates) if gates.contains(&HardGate::SanctionedCountry)
        ));
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
            GateOutcome::Passed => {} // expected
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
            GateOutcome::Passed => {}
        }
    }

    #[test]
    fn unknown_breach_status_on_sensitive_action_blocks() {
        let mut ctx = base_ctx();
        ctx.breached_credential = None;
        ctx.action = RiskAction::DeviceRevoke; // sensitive
        let outcome = evaluate_hard_gates(&ctx);
        assert!(matches!(
            outcome,
            GateOutcome::Blocked(gates) if gates.contains(&HardGate::BreachedCredentialOnSensitive)
        ));
    }

    #[test]
    fn unknown_breach_status_on_non_sensitive_passes() {
        let mut ctx = base_ctx();
        ctx.breached_credential = None;
        ctx.action = RiskAction::DeviceList; // non-sensitive
        let outcome = evaluate_hard_gates(&ctx);
        match outcome {
            GateOutcome::Blocked(gates) => {
                assert!(!gates.contains(&HardGate::BreachedCredentialOnSensitive));
            }
            GateOutcome::Passed => {}
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
            GateOutcome::Passed => {}
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
            GateOutcome::Passed => {}
        }
    }

    // ── IPv4-mapped IPv6 canonicalization ────────────────────────────────────

    #[test]
    fn canonicalize_ipv4_mapped_ipv6() {
        use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
        let v4 = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));
        let v6_mapped: IpAddr = IpAddr::V6("::ffff:1.2.3.4".parse::<Ipv6Addr>().unwrap());
        assert_eq!(super::canonicalize_ip(v4), super::canonicalize_ip(v6_mapped));
    }

    #[test]
    fn oauth_ip_mismatch_ignores_ipv4_v6_representation() {
        use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
        let mut ctx = base_ctx();
        ctx.action = RiskAction::OauthComplete;
        // Same IP, different representations — must NOT fire the gate
        ctx.oauth_authorize_ip = Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
        ctx.request_ip = Some(IpAddr::V6("::ffff:10.0.0.1".parse::<Ipv6Addr>().unwrap()));
        let outcome = evaluate_hard_gates(&ctx);
        match outcome {
            GateOutcome::Blocked(gates) => {
                assert!(!gates.contains(&HardGate::OauthIpMismatch),
                    "Same IP in v4 vs v6-mapped form must not trigger OauthIpMismatch");
            }
            GateOutcome::Passed => {}
        }
    }

    #[test]
    fn oauth_ip_mismatch_fires_on_truly_different_ips() {
        use std::net::{IpAddr, Ipv4Addr};
        let mut ctx = base_ctx();
        ctx.action = RiskAction::OauthComplete;
        ctx.oauth_authorize_ip = Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
        ctx.request_ip = Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)));
        let outcome = evaluate_hard_gates(&ctx);
        assert!(matches!(
            outcome,
            GateOutcome::Blocked(gates) if gates.contains(&HardGate::OauthIpMismatch)
        ));
    }

    // ── N1: Per-tenant gate configuration ───────────────────────────────────

    #[test]
    fn gate_tor_on_sensitive_disabled_by_policy() {
        let mut policy = PolicyConfig::default();
        policy.gate_block_tor_on_sensitive = false;
        let mut ctx = base_ctx();
        ctx.policy = Arc::new(policy);
        ctx.ip_asn_type = Some(AsnType::Tor);
        ctx.action = RiskAction::DeviceRevoke; // sensitive
        let outcome = evaluate_hard_gates(&ctx);
        match outcome {
            GateOutcome::Blocked(gates) => {
                assert!(
                    !gates.contains(&HardGate::TorOnSensitiveAction),
                    "Gate 6 should be disabled by policy"
                );
            }
            GateOutcome::Passed => {} // expected
        }
    }

    #[test]
    fn gate_vpn_on_critical_disabled_by_policy() {
        let mut policy = PolicyConfig::default();
        policy.gate_block_vpn_on_critical = false;
        let mut ctx = base_ctx();
        ctx.policy = Arc::new(policy);
        ctx.ip_asn_type = Some(AsnType::Vpn);
        ctx.action = RiskAction::RecoveryApprove;
        let outcome = evaluate_hard_gates(&ctx);
        match outcome {
            GateOutcome::Blocked(gates) => {
                assert!(
                    !gates.contains(&HardGate::VpnOnCriticalAction),
                    "Gate 11 should be disabled by policy"
                );
            }
            GateOutcome::Passed => {} // expected
        }
    }

    #[test]
    fn gate_tor_on_recovery_disabled_by_policy() {
        let mut policy = PolicyConfig::default();
        policy.gate_block_tor_on_recovery = false;
        let mut ctx = base_ctx();
        ctx.policy = Arc::new(policy);
        ctx.ip_asn_type = Some(AsnType::Tor);
        ctx.action = RiskAction::RecoveryStart;
        let outcome = evaluate_hard_gates(&ctx);
        match outcome {
            GateOutcome::Blocked(gates) => {
                assert!(
                    !gates.contains(&HardGate::TorOnRecovery),
                    "Gate 12 should be disabled by policy"
                );
            }
            GateOutcome::Passed => {} // expected
        }
    }

    #[test]
    fn gate_fresh_credential_uses_policy_threshold() {
        // Default is 5 minutes — a 2-minute-old credential should block.
        let mut ctx = base_ctx();
        ctx.credential_created_at = ctx.evaluated_at - chrono::Duration::minutes(2);
        ctx.action = RiskAction::DeviceRevoke; // privileged
        let outcome = evaluate_hard_gates(&ctx);
        assert!(matches!(
            outcome,
            GateOutcome::Blocked(gates) if gates.contains(&HardGate::PrivilegedActionOnFreshCredential)
        ));

        // Now set threshold to 1 minute — 2-minute-old credential passes.
        let mut policy = PolicyConfig::default();
        policy.gate_fresh_credential_minutes = 1;
        ctx.policy = Arc::new(policy);
        let outcome = evaluate_hard_gates(&ctx);
        match outcome {
            GateOutcome::Blocked(gates) => {
                assert!(
                    !gates.contains(&HardGate::PrivilegedActionOnFreshCredential),
                    "2-minute-old credential should pass with 1-minute threshold"
                );
            }
            GateOutcome::Passed => {} // expected
        }
    }

    #[test]
    fn gate_fresh_credential_zero_effectively_disables() {
        let mut policy = PolicyConfig::default();
        policy.gate_fresh_credential_minutes = 0;
        let mut ctx = base_ctx();
        ctx.policy = Arc::new(policy);
        ctx.credential_created_at = ctx.evaluated_at - chrono::Duration::seconds(30);
        ctx.action = RiskAction::DeviceRevoke; // privileged
        let outcome = evaluate_hard_gates(&ctx);
        match outcome {
            GateOutcome::Blocked(gates) => {
                assert!(
                    !gates.contains(&HardGate::PrivilegedActionOnFreshCredential),
                    "gate_fresh_credential_minutes=0 should effectively disable gate 15"
                );
            }
            GateOutcome::Passed => {} // expected
        }
    }
}
