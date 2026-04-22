use crate::context::RiskContext;
use crate::decision::{Factor, ScoreComponent};
use crate::scoring::{capped_add, factor};

/// S component — session anomaly score.
/// Cap: 25.
pub fn score_session(ctx: &RiskContext, factors: &mut Vec<Factor>) -> u8 {
    let mut s: u8 = 0;
    let cap = ctx.policy.cap_session;

    // ── JWT fingerprint mismatch ──────────────────────────────────────────────
    // Primary anti-session-hijacking signal.
    // fingerprint = sha256(ip || ua), stored in Redis at JWT issue time.
    match (&ctx.jwt_fingerprint_stored, &ctx.jwt_fingerprint_current) {
        (Some(stored), Some(current)) if stored != current => {
            let eff = capped_add(&mut s, cap, ctx.policy.s_fingerprint_mismatch);
            if eff > 0 {
                factors.push(factor(
                    "jwt_fingerprint_mismatch",
                    ScoreComponent::Session,
                    eff,
                    "JWT fingerprint (IP+UA hash) does not match the stored value from issuance",
                ));
            }
        }
        (None, _) => {
            // Token pre-dates fingerprinting; handled in absolute adder, not here.
        }
        _ => {}
    }

    // ── JWT near expiry on sensitive action ───────────────────────────────────
    if let (Some(iat), Some(exp)) = (ctx.jwt_issued_at, ctx.jwt_expires_at) {
        let total_mins = (exp - iat).num_minutes().max(1);
        let used_mins = (ctx.evaluated_at - iat).num_minutes().max(0);
        let used_pct = (used_mins * 100) / total_mins;
        if used_pct >= 90 {
            let eff = capped_add(&mut s, cap, ctx.policy.s_near_expiry);
            if eff > 0 {
                factors.push(factor(
                    "jwt_near_expiry",
                    ScoreComponent::Session,
                    eff,
                    format!("JWT is {}% of its lifetime elapsed (>=90% threshold)", used_pct),
                ));
            }
        }
    }

    // ── Concurrent session count ──────────────────────────────────────────────
    if let Some(count) = ctx.active_session_count {
        if count > 5 {
            let eff = capped_add(&mut s, cap, ctx.policy.s_concurrent_high);
            if eff > 0 {
                factors.push(factor(
                    "concurrent_sessions_high",
                    ScoreComponent::Session,
                    eff,
                    format!("{} concurrent active sessions (>5 threshold)", count),
                ));
            }
        } else if count > 3 {
            let eff = capped_add(&mut s, cap, ctx.policy.s_concurrent_elevated);
            if eff > 0 {
                factors.push(factor(
                    "concurrent_sessions_elevated",
                    ScoreComponent::Session,
                    eff,
                    format!("{} concurrent active sessions (>3 threshold)", count),
                ));
            }
        }
    }

    // ── OAuth authorize-IP mismatch ───────────────────────────────────────────
    // If the IP that hit /authorize differs from /oauth/complete, the session
    // may have been transferred (potential session hijack).
    if let (Some(auth_ip), Some(req_ip)) = (ctx.oauth_authorize_ip, ctx.request_ip) {
        if auth_ip != req_ip {
            let eff = capped_add(&mut s, cap, ctx.policy.s_oauth_ip_mismatch);
            if eff > 0 {
                factors.push(factor(
                    "oauth_ip_mismatch",
                    ScoreComponent::Session,
                    eff,
                    format!(
                        "OAuth session started from {} but completing from {}",
                        auth_ip, req_ip
                    ),
                ));
            }
        }
    }

    // ── Client timestamp skew ─────────────────────────────────────────────────
    // Compares the client-supplied timestamp (from x-timestamp header) against
    // the server's evaluated_at. Detects clock manipulation or replayed requests.
    //
    // NOTE: This replaced a broken check that compared evaluated_at vs Utc::now()
    // which always yielded ~0 skew (both set within microseconds of each other).
    if let Some(client_ts) = ctx.client_timestamp {
        let skew_secs = (ctx.evaluated_at - client_ts).num_seconds().unsigned_abs();
        if skew_secs > 30 {
            let eff = capped_add(&mut s, cap, ctx.policy.s_timestamp_skew);
            if eff > 0 {
                factors.push(factor(
                    "timestamp_skew",
                    ScoreComponent::Session,
                    eff,
                    format!("Client timestamp skew of {}s exceeds 30s threshold", skew_secs),
                ));
            }
        }
    }

    // ── Nonce missing on sensitive path ───────────────────────────────────────
    if !ctx.nonce_present && ctx.action.is_sensitive() {
        let eff = capped_add(&mut s, cap, ctx.policy.s_nonce_absent);
        if eff > 0 {
            factors.push(factor(
                "nonce_absent",
                ScoreComponent::Session,
                eff,
                "Sensitive action submitted without x-nonce header",
            ));
        }
    }

    // ── Accept-Language change detection ──────────────────────────────────────
    // A sudden change in Accept-Language header can indicate account takeover
    // (attacker's browser has different language preferences).
    // We compare the primary language tag only (first component before comma).
    if let (Some(current_lang), Some(prev_lang)) =
        (&ctx.accept_language, &ctx.previous_accept_language)
    {
        let cur_primary = primary_language(current_lang);
        let prev_primary = primary_language(prev_lang);
        if !cur_primary.is_empty() && !prev_primary.is_empty() && cur_primary != prev_primary {
            let eff = capped_add(&mut s, cap, ctx.policy.s_language_change);
            if eff > 0 {
                factors.push(factor(
                    "language_change",
                    ScoreComponent::Session,
                    eff,
                    format!(
                        "Accept-Language changed from '{}' to '{}' since last login",
                        prev_primary, cur_primary
                    ),
                ));
            }
        }
    }

    // ── Out-of-hours activity detection ──────────────────────────────────────
    // If the user has established typical active hours, flag activity outside
    // that window. Useful for detecting compromised accounts being used in
    // different timezones.
    if let Some((start_hour, end_hour)) = ctx.user_typical_hours {
        let current_hour = ctx.evaluated_at.format("%H").to_string()
            .parse::<u8>()
            .unwrap_or(12);

        let is_outside = if start_hour <= end_hour {
            // Normal range: e.g., 8..22
            current_hour < start_hour || current_hour >= end_hour
        } else {
            // Wrapping range: e.g., 22..6 (night shift)
            current_hour < start_hour && current_hour >= end_hour
        };

        if is_outside {
            let eff = capped_add(&mut s, cap, ctx.policy.s_out_of_hours);
            if eff > 0 {
                factors.push(factor(
                    "out_of_hours",
                    ScoreComponent::Session,
                    eff,
                    format!(
                        "Activity at {}:00 UTC is outside typical hours ({:02}:00–{:02}:00 UTC)",
                        current_hour, start_hour, end_hour
                    ),
                ));
            }
        }
    }

    // ── Email verification status ────────────────────────────────────────────
    // Unverified email on a sensitive action is a mild signal. The user may
    // have skipped verification, or an attacker registered with a throwaway.
    if ctx.email_verified == Some(false) && ctx.action.is_sensitive() {
        let eff = capped_add(&mut s, cap, ctx.policy.s_email_not_verified);
        if eff > 0 {
            factors.push(factor(
                "email_not_verified",
                ScoreComponent::Session,
                eff,
                "Email address has not been verified and action is sensitive",
            ));
        }
    }

    s
}

// ───────────────────────────────────────���─────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Extract the primary language tag from an Accept-Language header.
/// "en-US,en;q=0.9,fr;q=0.8" → "en"
fn primary_language(accept_lang: &str) -> &str {
    let first = accept_lang.split(',').next().unwrap_or("");
    let tag = first.split(';').next().unwrap_or("");
    let primary = tag.split('-').next().unwrap_or("");
    primary.trim()
}
