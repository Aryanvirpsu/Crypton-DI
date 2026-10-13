//! # client_sdk
//!
//! Header parsers for the client-side fingerprinting SDK.
//!
//! A FingerprintJS-style JavaScript SDK on the client runs browser integrity
//! checks and forwards the results to the host service as HTTP headers. This
//! module is the engine-side boundary: it parses those headers into the
//! typed fields on `RiskContext`.
//!
//! ## Header contract
//!
//! | Header                     | Populates                              | Example                      |
//! |----------------------------|----------------------------------------|------------------------------|
//! | `X-Client-Webdriver`       | `webdriver_detected`                   | `true` / `false`             |
//! | `X-Client-Captcha-Score`   | `captcha_score` (0.0–1.0)              | `0.9`                        |
//! | `X-Client-Screen`          | `screen_resolution` (`WxH`)            | `1920x1080`                  |
//! | `X-Client-Touch`           | `touch_capable`                        | `true` / `false`             |
//! | `X-Client-Fingerprint`     | `device_fingerprint_hash` (hex)        | `a1b2c3…` (≤128 hex chars)   |
//! | `X-TLS-JA3`                | `ja3_fingerprint` (md5 hex or raw JA3) | `e7d705a3286e19ea…`          |
//!
//! All parsers return `Option<T>` — an absent, empty, or malformed header
//! becomes `None` rather than an error. This matches the engine's fail-open
//! posture for enrichment signals.
//!
//! ## Trust model
//!
//! Client-supplied headers are **untrusted**. The gateway or load balancer
//! MUST strip any incoming copies and re-inject the SDK-signed versions. This
//! module does not verify signatures — that belongs in the adapter's ingress
//! path. Here we only enforce *shape* (e.g., screen resolution matches
//! `\d+x\d+`, fingerprint is hex within bounds) so a malformed client cannot
//! smuggle control characters or pathological strings into downstream logs.

/// Max length for a device fingerprint hash (hex). SHA-512 = 128 hex chars;
/// anything longer is rejected so a malicious client can't bloat request
/// context.
const MAX_FINGERPRINT_LEN: usize = 128;

/// Max length for a JA3/JA4 fingerprint. Raw JA3 strings are typically under
/// 300 chars; we allow up to 512 as a conservative bound.
const MAX_JA3_LEN: usize = 512;

/// Parse a boolean-valued header. Accepts `"true"`, `"1"`, `"yes"` as true
/// and `"false"`, `"0"`, `"no"` as false. Case-insensitive. Anything else
/// (including empty) returns `None`.
pub fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Some(true),
        "false" | "0" | "no" => Some(false),
        _ => None,
    }
}

/// Parse a CAPTCHA score — a float in `[0.0, 1.0]`. Clamps nothing: anything
/// outside the range is rejected so badly-calibrated callers surface as
/// `None` rather than a silently clamped value.
pub fn parse_captcha_score(value: &str) -> Option<f32> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parsed: f32 = trimmed.parse().ok()?;
    if !parsed.is_finite() || !(0.0..=1.0).contains(&parsed) {
        return None;
    }
    Some(parsed)
}

/// Parse a screen-resolution header in the form `WIDTHxHEIGHT`. Returns the
/// canonicalised string (`"1920x1080"`) on success. Values outside plausible
/// bounds (1..=16384 pixels per axis) are rejected.
pub fn parse_screen_resolution(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let (w, h) = trimmed.split_once('x').or_else(|| trimmed.split_once('X'))?;
    let w: u32 = w.trim().parse().ok()?;
    let h: u32 = h.trim().parse().ok()?;
    if !(1..=16_384).contains(&w) || !(1..=16_384).contains(&h) {
        return None;
    }
    Some(format!("{w}x{h}"))
}

/// Parse a device-fingerprint-hash header. Must be hex-encoded and within
/// [`MAX_FINGERPRINT_LEN`]. Returns the lowercase-canonicalised string.
pub fn parse_fingerprint_hash(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_FINGERPRINT_LEN {
        return None;
    }
    if !trimmed.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some(trimmed.to_ascii_lowercase())
}

/// Parse a JA3/JA4 fingerprint. Accepts either the raw 5-tuple form
/// (`"version,ciphers,extensions,curves,formats"` — digits, commas, hyphens)
/// or the MD5-hashed 32-char hex form. Rejects control characters and values
/// longer than [`MAX_JA3_LEN`].
pub fn parse_ja3(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_JA3_LEN {
        return None;
    }
    let ok = trimmed
        .bytes()
        .all(|b| b.is_ascii_hexdigit() || b == b',' || b == b'-');
    if !ok {
        return None;
    }
    Some(trimmed.to_ascii_lowercase())
}

/// Bundle of parsed client-SDK signals, ready to splat onto `RiskContext`.
/// Populated by the adapter from the incoming request headers.
#[derive(Debug, Clone, Default)]
pub struct ClientSdkSignals {
    pub webdriver_detected: Option<bool>,
    pub captcha_score: Option<f32>,
    pub screen_resolution: Option<String>,
    pub touch_capable: Option<bool>,
    pub device_fingerprint_hash: Option<String>,
    pub ja3_fingerprint: Option<String>,
}

impl ClientSdkSignals {
    /// Build from a header lookup closure. The closure returns `None` when
    /// the header is absent — this matches the shape of `http::HeaderMap::get`
    /// and keeps this module HTTP-library-agnostic.
    pub fn from_headers<'a, F>(mut get: F) -> Self
    where
        F: FnMut(&str) -> Option<&'a str>,
    {
        Self {
            webdriver_detected: get("x-client-webdriver").and_then(parse_bool),
            captcha_score: get("x-client-captcha-score").and_then(parse_captcha_score),
            screen_resolution: get("x-client-screen").and_then(parse_screen_resolution),
            touch_capable: get("x-client-touch").and_then(parse_bool),
            device_fingerprint_hash: get("x-client-fingerprint")
                .and_then(parse_fingerprint_hash),
            ja3_fingerprint: get("x-tls-ja3").and_then(parse_ja3),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bool_accepts_common_forms() {
        for t in ["true", "TRUE", "1", "yes", " Yes "] {
            assert_eq!(parse_bool(t), Some(true), "{t}");
        }
        for f in ["false", "FALSE", "0", "no"] {
            assert_eq!(parse_bool(f), Some(false), "{f}");
        }
        for bad in ["", "maybe", "2", "null"] {
            assert_eq!(parse_bool(bad), None, "{bad}");
        }
    }

    #[test]
    fn captcha_score_range_enforced() {
        assert_eq!(parse_captcha_score("0"), Some(0.0));
        assert_eq!(parse_captcha_score("0.5"), Some(0.5));
        assert_eq!(parse_captcha_score("1"), Some(1.0));
        assert_eq!(parse_captcha_score("1.0001"), None);
        assert_eq!(parse_captcha_score("-0.1"), None);
        assert_eq!(parse_captcha_score("NaN"), None);
        assert_eq!(parse_captcha_score("inf"), None);
        assert_eq!(parse_captcha_score(""), None);
    }

    #[test]
    fn screen_resolution_validates_shape_and_bounds() {
        assert_eq!(parse_screen_resolution("1920x1080"), Some("1920x1080".into()));
        assert_eq!(parse_screen_resolution("1920X1080"), Some("1920x1080".into()));
        assert_eq!(parse_screen_resolution("  800 x 600 "), Some("800x600".into()));
        assert_eq!(parse_screen_resolution("0x600"), None);
        assert_eq!(parse_screen_resolution("100000x100"), None);
        assert_eq!(parse_screen_resolution("1920"), None);
        assert_eq!(parse_screen_resolution("1920x"), None);
        assert_eq!(parse_screen_resolution("1920xabc"), None);
    }

    #[test]
    fn fingerprint_hash_requires_hex_and_bounded_length() {
        assert_eq!(parse_fingerprint_hash("DEADBEEF"), Some("deadbeef".into()));
        assert_eq!(parse_fingerprint_hash(" abc123 "), Some("abc123".into()));
        assert_eq!(parse_fingerprint_hash("zzz"), None);
        assert_eq!(parse_fingerprint_hash(""), None);
        let too_long = "a".repeat(MAX_FINGERPRINT_LEN + 1);
        assert_eq!(parse_fingerprint_hash(&too_long), None);
    }

    #[test]
    fn ja3_accepts_raw_and_hashed_forms() {
        assert!(parse_ja3("e7d705a3286e19ea42f587b344ee6865").is_some());
        assert!(parse_ja3("771,4865-4866,0-23-65281,29-23-24,0").is_some());
        assert_eq!(parse_ja3(""), None);
        assert_eq!(parse_ja3("has spaces"), None);
        assert_eq!(parse_ja3("control\x01char"), None);
        let too_long = "a".repeat(MAX_JA3_LEN + 1);
        assert_eq!(parse_ja3(&too_long), None);
    }

    #[test]
    fn from_headers_populates_expected_fields() {
        use std::collections::HashMap;
        let map: HashMap<&str, &str> = [
            ("x-client-webdriver", "true"),
            ("x-client-captcha-score", "0.9"),
            ("x-client-screen", "1920x1080"),
            ("x-client-touch", "false"),
            ("x-client-fingerprint", "abc123"),
            ("x-tls-ja3", "e7d705a3286e19ea42f587b344ee6865"),
        ]
        .into_iter()
        .collect();

        let sig = ClientSdkSignals::from_headers(|k| map.get(k).copied());

        assert_eq!(sig.webdriver_detected, Some(true));
        assert_eq!(sig.captcha_score, Some(0.9));
        assert_eq!(sig.screen_resolution.as_deref(), Some("1920x1080"));
        assert_eq!(sig.touch_capable, Some(false));
        assert_eq!(sig.device_fingerprint_hash.as_deref(), Some("abc123"));
        assert_eq!(
            sig.ja3_fingerprint.as_deref(),
            Some("e7d705a3286e19ea42f587b344ee6865")
        );
    }

    #[test]
    fn from_headers_tolerates_missing_and_malformed() {
        let sig = ClientSdkSignals::from_headers(|k| match k {
            "x-client-webdriver" => Some("garbage"),
            "x-client-captcha-score" => Some("1.5"),
            _ => None,
        });

        assert_eq!(sig.webdriver_detected, None);
        assert_eq!(sig.captcha_score, None);
        assert_eq!(sig.screen_resolution, None);
    }
}
