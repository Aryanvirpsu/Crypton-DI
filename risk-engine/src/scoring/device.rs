use crate::context::{CredentialStatus, DeviceTrustLevel, RiskContext};
use crate::decision::{Factor, ScoreComponent};
use crate::scoring::{capped_add, factor};

/// D component — device trust score.
/// Cap: 25. Lower base score = more trusted device.
pub fn score_device(ctx: &RiskContext, factors: &mut Vec<Factor>) -> u8 {
    let mut d: u8 = 0;
    let cap = ctx.policy.cap_device;

    // ── Status check ──────────────────────────────────────────────────────────
    // Revoked is handled as a hard gate before scoring, but Lost still reaches
    // the scorer and carries maximum device distrust.
    if ctx.credential_status == CredentialStatus::Lost {
        capped_add(&mut d, cap, ctx.policy.d_credential_lost);
        factors.push(factor(
            "credential_lost",
            ScoreComponent::Device,
            d,
            "Device is explicitly marked as lost",
        ));
        return d;
    }

    // ── Credential age ────────────────────────────────────────────────────────
    let age_hours = ctx
        .evaluated_at
        .signed_duration_since(ctx.credential_created_at)
        .num_minutes() as f64
        / 60.0;

    let (age_score, age_desc) = if age_hours < 1.0 {
        (ctx.policy.d_credential_age_new_h, "Credential registered less than 1 hour ago")
    } else if age_hours < 24.0 {
        (ctx.policy.d_credential_age_recent_h, "Credential registered less than 24 hours ago")
    } else if age_hours < 72.0 {
        (ctx.policy.d_credential_age_stale_h, "Credential registered less than 72 hours ago")
    } else {
        (0u8, "")
    };

    if age_score > 0 {
        let eff = capped_add(&mut d, cap, age_score);
        if eff > 0 {
            factors.push(factor("credential_age", ScoreComponent::Device, eff, age_desc));
        }
    }

    // ── Dormant device ────────────────────────────────────────────────────────
    if let Some(last_used) = ctx.credential_last_used_at {
        let days_dormant = ctx
            .evaluated_at
            .signed_duration_since(last_used)
            .num_days();
        if days_dormant > 30 {
            let eff = capped_add(&mut d, cap, ctx.policy.d_dormant_device);
            if eff > 0 {
                factors.push(factor(
                    "dormant_device",
                    ScoreComponent::Device,
                    eff,
                    format!("Device unused for {} days", days_dormant),
                ));
            }
        }
    }

    // ── Sign-count anomaly (graduated thresholds) ─────────────────────────────
    // WebAuthn sign count should increment by exactly 1 per ceremony.
    // Jumps > 1 are unusual, > 10 is suspicious, > 100 is almost certainly
    // cloning or counter manipulation.
    // NOTE: delta == 0 or negative is handled as a hard gate (SignCountRollback).
    let count_delta = ctx
        .credential_sign_count_new
        .saturating_sub(ctx.credential_sign_count_prev);
    if count_delta > 100 {
        let eff = capped_add(&mut d, cap, ctx.policy.d_sign_count_jump_high);
        if eff > 0 {
            factors.push(factor(
                "sign_count_jump_critical",
                ScoreComponent::Device,
                eff,
                format!("Sign count jumped by {} (expected 1)", count_delta),
            ));
        }
    } else if count_delta > 10 {
        let eff = capped_add(&mut d, cap, ctx.policy.d_sign_count_jump_med);
        if eff > 0 {
            factors.push(factor(
                "sign_count_jump_high",
                ScoreComponent::Device,
                eff,
                format!("Sign count jumped by {} (expected 1)", count_delta),
            ));
        }
    } else if count_delta > 1 {
        let eff = capped_add(&mut d, cap, ctx.policy.d_sign_count_jump_low);
        if eff > 0 {
            factors.push(factor(
                "sign_count_jump",
                ScoreComponent::Device,
                eff,
                format!("Sign count jumped by {} (expected 1)", count_delta),
            ));
        }
    }

    // ── User-agent mismatch ───────────────────────────────────────────────────
    if let (Some(reg_ua), Some(req_ua)) =
        (&ctx.credential_registered_ua, &ctx.request_ua)
    {
        if ua_family_differs(reg_ua, req_ua) {
            let eff = capped_add(&mut d, cap, ctx.policy.d_ua_family_mismatch);
            if eff > 0 {
                factors.push(factor(
                    "ua_family_mismatch",
                    ScoreComponent::Device,
                    eff,
                    "Browser/OS family differs from registration user-agent",
                ));
            }
        }

        if is_headless_ua(req_ua) {
            let eff = capped_add(&mut d, cap, ctx.policy.d_headless_ua);
            if eff > 0 {
                factors.push(factor(
                    "headless_ua",
                    ScoreComponent::Device,
                    eff,
                    "Request user-agent matches a headless/automation client",
                ));
            }
        }
    }

    // ── WebDriver detection ──────────────────────────────────────────────────
    // navigator.webdriver = true is the strongest client-side bot signal.
    // Real browsers only set this under automation (Selenium, Puppeteer, etc.).
    // This fires independently of UA detection because some bots spoof the UA
    // but forget to mask the webdriver property.
    if ctx.webdriver_detected == Some(true) {
        let eff = capped_add(&mut d, cap, ctx.policy.d_webdriver_detected);
        if eff > 0 {
            factors.push(factor(
                "webdriver_detected",
                ScoreComponent::Device,
                eff,
                "Client reports navigator.webdriver=true (browser automation detected)",
            ));
        }
    }

    // ── CAPTCHA score ────────────────────────────────────────────────────────
    // Low CAPTCHA score = likely bot. Scale: 0.0 (bot) to 1.0 (human).
    // Only penalize scores below 0.5 — scores above 0.5 are considered human.
    if let Some(captcha) = ctx.captcha_score {
        if captcha < 0.3 {
            let eff = capped_add(&mut d, cap, ctx.policy.d_captcha_fail_critical);
            if eff > 0 {
                factors.push(factor(
                    "captcha_score_very_low",
                    ScoreComponent::Device,
                    eff,
                    format!("CAPTCHA score {:.2} indicates likely bot (< 0.3 threshold)", captcha),
                ));
            }
        } else if captcha < 0.5 {
            let eff = capped_add(&mut d, cap, ctx.policy.d_captcha_fail_suspicious);
            if eff > 0 {
                factors.push(factor(
                    "captcha_score_low",
                    ScoreComponent::Device,
                    eff,
                    format!("CAPTCHA score {:.2} is suspicious (< 0.5 threshold)", captcha),
                ));
            }
        }
    }

    // ── Touch capability mismatch ────────────────────────────────────────────
    // Mobile UA without touch capability is a strong bot signal (headless
    // mobile emulation). Desktop UA with touch is less suspicious (touch laptops).
    if let (Some(ua), Some(touch)) = (&ctx.request_ua, ctx.touch_capable) {
        let ua_lower = ua.to_ascii_lowercase();
        let is_mobile_ua = ua_lower.contains("mobile")
            || ua_lower.contains("android")
            || ua_lower.contains("iphone")
            || ua_lower.contains("ipad");

        if is_mobile_ua && !touch {
            let eff = capped_add(&mut d, cap, ctx.policy.d_touch_mismatch);
            if eff > 0 {
                factors.push(factor(
                    "mobile_ua_no_touch",
                    ScoreComponent::Device,
                    eff,
                    "Mobile user-agent but device reports no touch capability (emulation suspected)",
                ));
            }
        }
    }

    // ── Headless screen resolution ───────────────────────────────────────────
    // Common headless/default resolutions that rarely appear on real devices.
    if let Some(res) = &ctx.screen_resolution {
        if is_suspicious_resolution(res) {
            let eff = capped_add(&mut d, cap, ctx.policy.d_screen_res_suspicious);
            if eff > 0 {
                factors.push(factor(
                    "suspicious_resolution",
                    ScoreComponent::Device,
                    eff,
                    format!("Screen resolution {} is common in headless/automated environments", res),
                ));
            }
        }
    }

    // ── Device fingerprint absence on sensitive action ─────────────────────────
    // If the adapter provides a client-side device fingerprint field and it is
    // absent on a sensitive action, this is a mild signal (client SDK may not
    // yet be deployed everywhere).
    if ctx.device_fingerprint_hash.is_none() && ctx.action.is_sensitive() {
        let eff = capped_add(&mut d, cap, ctx.policy.d_no_device_fingerprint);
        if eff > 0 {
            factors.push(factor(
                "no_device_fingerprint",
                ScoreComponent::Device,
                eff,
                "No client-side device fingerprint provided on sensitive action",
            ));
        }
    }

    // ── JA3/JA4 TLS fingerprint anomaly ───────────────────────────────────────
    if let Some(ja3) = &ctx.ja3_fingerprint {
        if is_known_bot_ja3(ja3) {
            let eff = capped_add(&mut d, cap, ctx.policy.d_bot_tls_fingerprint);
            if eff > 0 {
                factors.push(factor(
                    "bot_tls_fingerprint",
                    ScoreComponent::Device,
                    eff,
                    "TLS fingerprint matches known automation tool",
                ));
            }
        }
    }

    // ── Device trust level ────────────────────────────────────────────────────
    // Penalize unknown/new devices, especially on sensitive actions.
    match &ctx.device_trust_level {
        Some(DeviceTrustLevel::New) if ctx.action.is_sensitive() => {
            let eff = capped_add(&mut d, cap, ctx.policy.d_trust_new_device_sensitive);
            if eff > 0 {
                factors.push(factor(
                    "new_device_sensitive_action",
                    ScoreComponent::Device,
                    eff,
                    "First-time device attempting a sensitive action",
                ));
            }
        }
        Some(DeviceTrustLevel::New) => {
            let eff = capped_add(&mut d, cap, ctx.policy.d_trust_new_device);
            if eff > 0 {
                factors.push(factor(
                    "new_device",
                    ScoreComponent::Device,
                    eff,
                    "Device has not been seen before for this user",
                ));
            }
        }
        Some(DeviceTrustLevel::Recognized) if ctx.action.is_sensitive() => {
            let eff = capped_add(&mut d, cap, ctx.policy.d_trust_recognized_sensitive);
            if eff > 0 {
                factors.push(factor(
                    "recognized_device_sensitive",
                    ScoreComponent::Device,
                    eff,
                    "Partially-trusted device on sensitive action (seen <7 days or <5 sessions)",
                ));
            }
        }
        _ => {
            // Trusted device or unknown trust level — no penalty.
        }
    }

    // ── Sole credential ───────────────────────────────────────────────────────
    if ctx.credential_count_for_user == 1 {
        let eff = capped_add(&mut d, cap, ctx.policy.d_sole_credential);
        if eff > 0 {
            factors.push(factor(
                "sole_credential",
                ScoreComponent::Device,
                eff,
                "This is the only active credential for the user (no fallback)",
            ));
        }
    }

    d
}

// ─────────────────────────────────────────────────────────────────────────────
// UA helpers (pure functions, no I/O)
// ─────────────────────────────────────────────────────────────────────────────

/// Extracts a coarse "family" from a user-agent string for comparison.
/// Deliberately avoids full UA parsing to stay deterministic and dependency-free.
fn ua_family(ua: &str) -> &'static str {
    let lower = ua.to_ascii_lowercase();
    // Order matters — more specific strings first.
    if lower.contains("edg/") || lower.contains("edge/")      { return "edge"; }
    if lower.contains("vivaldi/")                              { return "vivaldi"; }
    if lower.contains("brave/")                                { return "brave"; }
    if lower.contains("firefox/")                              { return "firefox"; }
    if lower.contains("opr/") || lower.contains("opera")      { return "opera"; }
    if lower.contains("safari/") && !lower.contains("chrome")  { return "safari"; }
    if lower.contains("chrome/")                               { return "chrome"; }
    "other"
}

pub fn ua_family_differs(registered: &str, current: &str) -> bool {
    ua_family(registered) != ua_family(current)
}

pub fn is_headless_ua(ua: &str) -> bool {
    let lower = ua.to_ascii_lowercase();
    lower.contains("headlesschrome")
        || lower.contains("phantomjs")
        || lower.contains("selenium")
        || lower.contains("puppeteer")
        || lower.contains("playwright")
        || lower.contains("python-requests")
        || lower.contains("curl/")
        || lower.contains("go-http-client")
        || lower.contains("node-fetch")
        || lower.contains("axios/")
        || lower.contains("scrapy")
        || lower.contains("httpclient")
        || lower.contains("java/")
        || lower.contains("okhttp")
        || lower.contains("libwww-perl")
        || lower.contains("wget/")
        || lower.contains("python-urllib")
        || lower.contains("httpx/")
        || lower.contains("aiohttp/")
        || lower.contains("undici/")
        || lower.contains("got/")
        || (lower.contains("mozilla/") && !lower.contains("applewebkit") && !lower.contains("gecko"))
}

/// Known bot/automation TLS (JA3) fingerprint hashes.
/// Starter set — maintain and expand based on threat intelligence feeds.
fn is_known_bot_ja3(ja3: &str) -> bool {
    const KNOWN_BOT_JA3: &[&str] = &[
        "e7d705a3286e19ea42f587b344ee6865", // Python requests
        "b32309a26951912be7dba376398abc3b", // Go default
        "3b5074b1b5d032e5620f69f9f700ff0e", // curl
        "cd08e31494f9531f560d64c695473da9", // Java HttpClient
        "473cd7cb9faa642487833865d516e578", // Python urllib3
        "a0e9f5d64349fb13191bc781f81f42e1", // Node.js default
        "6734f37431670b3ab4292b8f60f29984", // Golang fasthttp
    ];
    KNOWN_BOT_JA3.contains(&ja3)
}

/// Screen resolutions commonly used by headless browsers / CI environments.
/// These are default viewport sizes that real users almost never have.
fn is_suspicious_resolution(res: &str) -> bool {
    const SUSPICIOUS: &[&str] = &[
        "800x600",    // Headless Chrome default
        "1024x768",   // Legacy CI / Selenium default
        "0x0",        // No display (headless)
        "1x1",        // Minimal headless
    ];
    SUSPICIOUS.contains(&res)
}
