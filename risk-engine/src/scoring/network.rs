use std::net::IpAddr;

use crate::context::{AsnType, RiskContext};
use crate::decision::{Factor, ScoreComponent};
use crate::scoring::{capped_add, factor};

/// N component — network risk score.
/// Cap: 20.
pub fn score_network(ctx: &RiskContext, factors: &mut Vec<Factor>) -> u8 {
    let mut n: u8 = 0;
    let cap = ctx.policy.cap_network;
    let mut trusted_bonus: u8 = 0;

    // ── ASN classification ────────────────────────────────────────────────────
    match &ctx.ip_asn_type {
        Some(AsnType::Tor) => {
            let eff = capped_add(&mut n, cap, ctx.policy.n_tor_exit);
            if eff > 0 {
                factors.push(factor(
                    "tor_exit_node",
                    ScoreComponent::Network,
                    eff,
                    "Source IP is a known Tor exit node",
                ));
            }
        }
        Some(AsnType::Vpn) => {
            let eff = capped_add(&mut n, cap, ctx.policy.n_vpn_ip);
            if eff > 0 {
                factors.push(factor(
                    "vpn_ip",
                    ScoreComponent::Network,
                    eff,
                    "Source IP belongs to a known VPN provider",
                ));
            }
        }
        Some(AsnType::Proxy) => {
            let eff = capped_add(&mut n, cap, ctx.policy.n_proxy_ip);
            if eff > 0 {
                factors.push(factor(
                    "proxy_ip",
                    ScoreComponent::Network,
                    eff,
                    "Source IP is a known open/anonymous proxy",
                ));
            }
        }
        Some(AsnType::Hosting) => {
            let eff = capped_add(&mut n, cap, ctx.policy.n_hosting_ip);
            if eff > 0 {
                factors.push(factor(
                    "hosting_ip",
                    ScoreComponent::Network,
                    eff,
                    "Source IP belongs to a web hosting provider",
                ));
            }
        }
        Some(AsnType::Datacenter) => {
            let eff = capped_add(&mut n, cap, ctx.policy.n_datacenter_ip);
            if eff > 0 {
                factors.push(factor(
                    "datacenter_ip",
                    ScoreComponent::Network,
                    eff,
                    "Source IP belongs to a cloud/datacenter ASN (VPS/proxy indicator)",
                ));
            }
        }
        Some(AsnType::Relay) => {
            // Privacy relays (Apple Private Relay, Cloudflare WARP) are lower
            // risk — used for legitimate privacy, not evasion. Mild penalty.
            let eff = capped_add(&mut n, cap, ctx.policy.n_relay_ip);
            if eff > 0 {
                factors.push(factor(
                    "relay_ip",
                    ScoreComponent::Network,
                    eff,
                    "Source IP is a privacy relay (Apple Private Relay / Cloudflare WARP)",
                ));
            }
        }
        _ => {}
    }

    // ── IP intelligence flags (independent of ASN type) ──────────────────────
    // A residential ASN can still be a VPN exit node. These flags are from
    // IP intelligence feeds and are checked in addition to ASN type.
    if ctx.ip_is_vpn == Some(true) && !matches!(&ctx.ip_asn_type, Some(AsnType::Vpn)) {
        let eff = capped_add(&mut n, cap, ctx.policy.n_flagged_vpn);
        if eff > 0 {
            factors.push(factor(
                "ip_flagged_vpn",
                ScoreComponent::Network,
                eff,
                "IP intelligence feed flagged this IP as VPN (despite non-VPN ASN type)",
            ));
        }
    }

    if ctx.ip_is_proxy == Some(true) && !matches!(&ctx.ip_asn_type, Some(AsnType::Proxy)) {
        let eff = capped_add(&mut n, cap, ctx.policy.n_flagged_proxy);
        if eff > 0 {
            factors.push(factor(
                "ip_flagged_proxy",
                ScoreComponent::Network,
                eff,
                "IP intelligence feed flagged this IP as anonymous proxy",
            ));
        }
    }

    if ctx.ip_is_relay == Some(true) && !matches!(&ctx.ip_asn_type, Some(AsnType::Relay)) {
        let eff = capped_add(&mut n, cap, ctx.policy.n_flagged_relay);
        if eff > 0 {
            factors.push(factor(
                "ip_flagged_relay",
                ScoreComponent::Network,
                eff,
                "IP flagged as privacy relay service",
            ));
        }
    }

    // ── IP abuse confidence score ────────────────────────────────────────────
    // From AbuseIPDB or similar. Scale: 0 (clean) to 100 (heavily reported).
    if let Some(abuse) = ctx.ip_abuse_confidence {
        let (score, desc) = if abuse >= 80 {
            (ctx.policy.n_abuse_critical, format!("IP abuse confidence {}% — heavily reported", abuse))
        } else if abuse >= 50 {
            (ctx.policy.n_abuse_moderate, format!("IP abuse confidence {}% — moderately reported", abuse))
        } else if abuse >= 25 {
            (ctx.policy.n_abuse_light, format!("IP abuse confidence {}% — lightly reported", abuse))
        } else {
            (0u8, String::new())
        };
        if score > 0 {
            let eff = capped_add(&mut n, cap, score);
            if eff > 0 {
                factors.push(factor(
                    "ip_abuse_confidence",
                    ScoreComponent::Network,
                    eff,
                    desc,
                ));
            }
        }
    }

    // ── Trusted internal network (RFC-1918) ───────────────────────────────────
    if let Some(ip) = ctx.request_ip {
        if is_rfc1918(ip) {
            trusted_bonus = ctx.policy.n_rfc1918_bonus;
            factors.push(factor(
                "internal_network",
                ScoreComponent::Network,
                0, // negative contribution shown as 0; applied as post-cap subtraction
                "Source IP is within a private (RFC-1918) address range (-5 applied)",
            ));
        }
    }

    // ── Geographic distance from last login ───────────────────────────────────
    if let (Some(last_geo), Some(cur_geo), Some(last_at)) =
        (&ctx.last_login_geo, &ctx.request_geo, ctx.last_login_at)
    {
        let km = last_geo.distance_km(cur_geo);
        let elapsed_h = crate::signals::geo::elapsed_hours(last_at, ctx.evaluated_at);

        // Impossible travel: distance exceeds what a commercial flight can cover.
        // Hard gate handles Tor + impossible travel; here we score the non-Tor variant.
        let max_possible_km = elapsed_h * 900.0 + 500.0;

        if km > max_possible_km {
            let eff = capped_add(&mut n, cap, ctx.policy.n_impossible_travel);
            if eff > 0 {
                factors.push(factor(
                    "impossible_travel",
                    ScoreComponent::Network,
                    eff,
                    format!(
                        "{}km in {:.1}h exceeds max possible travel distance ({:.0}km)",
                        km as u32, elapsed_h, max_possible_km
                    ),
                ));
            }
        } else if km > 1000.0 {
            let eff = capped_add(&mut n, cap, ctx.policy.n_geo_jump_large);
            if eff > 0 {
                factors.push(factor(
                    "large_geo_jump",
                    ScoreComponent::Network,
                    eff,
                    format!("Login location changed by {}km since last session", km as u32),
                ));
            }
        } else if km > 500.0 {
            let eff = capped_add(&mut n, cap, ctx.policy.n_geo_jump_moderate);
            if eff > 0 {
                factors.push(factor(
                    "moderate_geo_jump",
                    ScoreComponent::Network,
                    eff,
                    format!("Login location changed by {}km since last session", km as u32),
                ));
            }
        }

        // ── Country change detection ─────────────────────────────────────────
        // Even if distance is moderate, a country change is noteworthy.
        if last_geo.country_code != cur_geo.country_code {
            let eff = capped_add(&mut n, cap, ctx.policy.n_country_change);
            if eff > 0 {
                factors.push(factor(
                    "country_change",
                    ScoreComponent::Network,
                    eff,
                    format!(
                        "Country changed from {} to {} since last login",
                        last_geo.country_code, cur_geo.country_code
                    ),
                ));
            }
        }
    }

    // ── Geo allow-list violation ──────────────────────────────────────────────
    // If the org/user has a country allow-list and the request country is NOT
    // in it, penalize. Sanctioned countries are handled as a hard gate — this
    // covers the "unexpected but not illegal" case.
    if let (Some(allowed), Some(req_geo)) = (&ctx.geo_allowed_countries, &ctx.request_geo) {
        if !allowed.is_empty() && !allowed.contains(&req_geo.country_code) {
            let eff = capped_add(&mut n, cap, ctx.policy.n_geo_not_allowed);
            if eff > 0 {
                factors.push(factor(
                    "geo_not_in_allowlist",
                    ScoreComponent::Network,
                    eff,
                    format!(
                        "Request from {} which is not in the allowed country list",
                        req_geo.country_code
                    ),
                ));
            }
        }
    }

    // ── IP history for this user ──────────────────────────────────────────────
    match ctx.known_ip_for_user {
        Some(false) => {
            // Adapter confirmed this IP has never been seen for this user.
            let eff = capped_add(&mut n, cap, ctx.policy.n_unknown_ip_user);
            if eff > 0 {
                factors.push(factor(
                    "unknown_ip_for_user",
                    ScoreComponent::Network,
                    eff,
                    "IP address has never been seen for this user",
                ));
            }
        }
        None => {
            // Fallback: compare against last login IP only (legacy adapters).
            if let (Some(req_ip), Some(last_ip)) = (ctx.request_ip, ctx.last_login_ip) {
                if req_ip != last_ip {
                    let eff = capped_add(&mut n, cap, ctx.policy.n_new_ip_history);
                    if eff > 0 {
                        factors.push(factor(
                            "new_ip_address",
                            ScoreComponent::Network,
                            eff,
                            format!("IP {} not seen in recent login history (last: {})", req_ip, last_ip),
                        ));
                    }
                }
            } else if ctx.request_ip.is_some() && ctx.last_login_ip.is_none() {
                // First login — no historical IP to compare.
                let eff = capped_add(&mut n, cap, ctx.policy.n_no_ip_history);
                if eff > 0 {
                    factors.push(factor(
                        "no_ip_history",
                        ScoreComponent::Network,
                        eff,
                        "No prior login IP on record for this user",
                    ));
                }
            }
        }
        Some(true) => {
            // Known IP — no penalty.
        }
    }

    // Apply trusted-network bonus (subtract from score, floor at 0).
    n.saturating_sub(trusted_bonus)
}

// ─────────────────────────────────────────────────────────────────────────────
// Network helpers (pure functions)
// ─────────────────────────────────────────────────────────────────────────────

pub fn is_rfc1918(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            // 10.0.0.0/8
            o[0] == 10
            // 172.16.0.0/12
            || (o[0] == 172 && (16..=31).contains(&o[1]))
            // 192.168.0.0/16
            || (o[0] == 192 && o[1] == 168)
            // 127.0.0.0/8 loopback
            || o[0] == 127
        }
        IpAddr::V6(v6) => {
            if v6.is_loopback() {
                return true;
            }
            let first = v6.segments()[0];
            // fc00::/7 Unique Local Addresses
            (first & 0xfe00) == 0xfc00
                // fe80::/10 link-local
                || (first & 0xffc0) == 0xfe80
        }
    }
}
