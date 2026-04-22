use crate::context::{AsnType, RiskContext, RiskAction};
use crate::decision::{Factor, ScoreComponent};

use crate::scoring::factor;

/// Evaluate pattern-based correlation bonuses (v3.0 Refinement).
/// 
/// These rules look for "Attacker Intent" by connecting weak signals into 
/// high-confidence patterns.
pub fn score_correlation(ctx: &RiskContext, factors: &mut Vec<Factor>) -> u8 {
    let mut c: u8 = 0;

    // ── Pattern 1: The ATO Cluster (+55) ─────────────────────────────────────
    // Intent: Account takeover attempt via recovery path on a new device.
    let is_anonymized = matches!(&ctx.ip_asn_type, Some(AsnType::Vpn) | Some(AsnType::Tor) | Some(AsnType::Proxy))
        || ctx.ip_is_vpn == Some(true) || ctx.ip_is_proxy == Some(true);
    let is_new_device = ctx.device_trust_level.as_ref().map_or(true, |t| t.is_untrusted());

    if ctx.action.is_recovery() && is_new_device && is_anonymized {
        c = c.saturating_add(ctx.policy.c_ato_cluster);
        factors.push(factor(
            "ato_cluster_pattern",
            ScoreComponent::Correlation,
            ctx.policy.c_ato_cluster,
            "High-risk recovery attempt: New device + Anonymizing network detected",
        ));
    }

    // ── Pattern 2: The Sybil Spree (+45) ─────────────────────────────────────
    // Intent: Automated mass account creation.
    let is_rapid_reg = ctx.registrations_from_ip_10m.map_or(false, |r| r >= 2);
    let is_shady_net = matches!(&ctx.ip_asn_type, Some(AsnType::Hosting) | Some(AsnType::Datacenter)) || ctx.ip_is_proxy == Some(true);
    let burner_email = ctx.email_domain_disposable == Some(true) || ctx.email_verified == Some(false);

    if matches!(&ctx.action, RiskAction::Register) && is_rapid_reg && (is_shady_net || burner_email) {
        c = c.saturating_add(ctx.policy.c_sybil_spree);
        factors.push(factor(
            "sybil_spree_pattern",
            ScoreComponent::Correlation,
            ctx.policy.c_sybil_spree,
            "Mass registration pattern: Multiple accounts from hosting/proxy infrastructure",
        ));
    }

    // ── Pattern 3: Automated Scraper (+30) ───────────────────────────────────
    // Intent: Low-sophistication crawler/bot using headless browser on cloud IP.
    let is_bot = ctx.webdriver_detected == Some(true)
        || ctx
            .request_ua
            .as_ref()
            .map_or(false, |ua| crate::scoring::device::is_headless_ua(ua));
    let is_cloud_ip = matches!(&ctx.ip_asn_type, Some(AsnType::Hosting) | Some(AsnType::Datacenter));

    if is_bot && is_cloud_ip {
        c = c.saturating_add(ctx.policy.c_automated_scraper);
        factors.push(factor(
            "automated_scraper_pattern",
            ScoreComponent::Correlation,
            ctx.policy.c_automated_scraper,
            "Bot activity detected from cloud/hosting infrastructure",
        ));
    }

    // ── Pattern 4: Shadow Session (+40) ──────────────────────────────────────
    // Intent: Potential session sharing or account takeover with concurrent usage.
    let high_concurrency = ctx.active_session_count.map_or(false, |s| s >= 3);
    let ua_change = ctx.credential_registered_ua.is_some() && ctx.request_ua != ctx.credential_registered_ua;
    let new_ip = ctx.known_ip_for_user == Some(false);

    if high_concurrency && ua_change && new_ip {
        c = c.saturating_add(ctx.policy.c_shadow_session);
        factors.push(factor(
            "shadow_session_pattern",
            ScoreComponent::Correlation,
            ctx.policy.c_shadow_session,
            "Concurrent session anomaly: Multiple IPs and browser mismatch",
        ));
    }

    // ── Pattern 5: Travel Anomaly (+15) ──────────────────────────────────────
    // Intent: Anonymized geographic shift (Challenge range).
    // Corrected v2 behavior: Keep this low so it stays in Challenge unless other flags exist.
    let is_geo_jump = geo_jump_detected(ctx);
    let is_vpn = ctx.ip_is_vpn == Some(true) || matches!(&ctx.ip_asn_type, Some(AsnType::Vpn));

    if is_geo_jump && is_vpn {
        c = c.saturating_add(ctx.policy.c_travel_anomaly);
        factors.push(factor(
            "travel_anomaly_pattern",
            ScoreComponent::Correlation,
            ctx.policy.c_travel_anomaly,
            "Anonymized location shift: VPN detected during geographic jump",
        ));
    }

    // Include the original Cloner pattern as well
    let sign_count_jump = ctx.credential_sign_count_prev > 0 
        && ctx.credential_sign_count_new > ctx.credential_sign_count_prev + 1;
    let ua_mismatch = ctx.credential_registered_ua.is_some() 
        && ctx.request_ua != ctx.credential_registered_ua;

    if sign_count_jump && ua_mismatch {
        c = c.saturating_add(ctx.policy.c_cloner_pattern);
        factors.push(factor(
            "credential_cloner_pattern",
            ScoreComponent::Correlation,
            ctx.policy.c_cloner_pattern,
            "Unexpected WebAuthn sign-count jump with browser fingerprint change",
        ));
    }

    c.min(ctx.policy.cap_correlation)
}

fn geo_jump_detected(ctx: &RiskContext) -> bool {
    if let (Some(last), Some(cur)) = (&ctx.last_login_geo, &ctx.request_geo) {
        last.distance_km(cur) > 500.0
    } else {
        false
    }
}
