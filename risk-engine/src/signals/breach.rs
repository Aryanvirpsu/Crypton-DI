//! # breach
//!
//! Credential-breach lookup via HaveIBeenPwned's k-anonymity API, or any
//! compatible local breach set.
//!
//! Populates `RiskContext::breached_credential`. Called by the adapter at
//! registration, password-reset, and (optionally) login when the user
//! supplies a password. For WebAuthn-only flows, this trait is not called.
//!
//! ## Protocol — what's sent over the wire
//!
//! The HIBP *Pwned Passwords* API implements k-anonymity: the client sends
//! only the first 5 hex chars of the SHA-1 of the password. The API returns
//! every full SHA-1 hash with that prefix, plus an occurrence count. The
//! client checks locally whether its full hash appears in the response.
//! **The full password hash never leaves the host**, which is why HIBP is a
//! safe upstream for plaintext-handling paths.
//!
//! ```text
//!   password = "hunter2"
//!   sha1     = f3bbbd66a63d4bf1747940578ec3d0103530e21d
//!   prefix   = "f3bbb"
//!   GET https://api.pwnedpasswords.com/range/f3bbb
//!   body →   D66A63D4BF1747940578EC3D0103530E21D:3         ← match, 3 breaches
//!            D67A9C8F7B2E1D0E3B4A5C6D7E8F9A0B1C2:1
//!            …
//! ```
//!
//! ## Trait boundary
//!
//! We deliberately do NOT pull in an HTTP client here. The engine stays
//! dependency-light; the host service (which already has `reqwest` or
//! `hyper` configured for its other integrations) supplies the transport.
//! The [`sha1_prefix_and_suffix`] helper does the hashing so every impl
//! shares the same k-anonymity invariant.
//!
//! Two zero-dep impls are shipped in-crate:
//! - [`NoopBreachChecker`] — always returns `Ok(false)`.
//! - [`StaticBreachChecker`] — in-memory set of known-compromised password
//!   hashes, useful for tests and air-gapped deployments.

use async_trait::async_trait;
use sha1::{Digest, Sha1};
use std::collections::HashSet;

use crate::error::RiskEngineError;

/// Hash a password with SHA-1 and split the hex digest into `(prefix, suffix)`
/// where `prefix` is the first 5 uppercase hex chars (the only part sent to
/// HIBP) and `suffix` is the remaining 35 uppercase hex chars (compared
/// locally against the API response body).
///
/// Returning uppercase matches HIBP's wire format byte-for-byte so callers
/// can `.contains(&suffix)` over `\r\n`-delimited response lines without
/// case-folding on the hot path.
pub fn sha1_prefix_and_suffix(password: &str) -> (String, String) {
    let mut hasher = Sha1::new();
    hasher.update(password.as_bytes());
    let digest = hasher.finalize();
    let hex: String = digest
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect();
    debug_assert_eq!(hex.len(), 40);
    let (prefix, suffix) = hex.split_at(5);
    (prefix.to_string(), suffix.to_string())
}

/// Parse a HIBP `/range/{prefix}` response body and check whether the given
/// suffix is present. Body format: one `SUFFIX:COUNT\r\n` entry per line.
///
/// Returns `Some(count)` when the suffix matches, `None` otherwise.
/// Whitespace and `\r` are tolerated.
pub fn hibp_body_contains(body: &str, suffix: &str) -> Option<u64> {
    let suffix_upper = suffix.trim().to_ascii_uppercase();
    for line in body.lines() {
        let line = line.trim();
        let (hash, count) = match line.split_once(':') {
            Some(p) => p,
            None => continue,
        };
        if hash.eq_ignore_ascii_case(&suffix_upper) {
            return count.trim().parse::<u64>().ok();
        }
    }
    None
}

// ─────────────────────────────────────────────────────────────────────────────
// Trait
// ─────────────────────────────────────────────────────────────────────────────

/// Abstraction over a breach-lookup backend. The host adapter implements
/// one concrete impl (HIBP HTTP, internal breach DB, local bloom filter)
/// and injects it into the enrichment pipeline.
///
/// `is_breached` returns `Ok(true)` when the password is known to have
/// appeared in at least one breach. Network/DB errors surface as
/// [`RiskEngineError::StoreError`]; the adapter should treat them as
/// "unknown" and NOT populate `RiskContext::breached_credential`, letting
/// the engine fail open on this signal alone.
#[async_trait]
pub trait BreachChecker: Send + Sync {
    async fn is_breached(&self, password: &str) -> Result<bool, RiskEngineError>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Noop
// ─────────────────────────────────────────────────────────────────────────────

/// Always returns `Ok(false)`. Use for tests and deployments without a
/// breach upstream. Does NOT log or error so it's safe to wire as a default.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopBreachChecker;

#[async_trait]
impl BreachChecker for NoopBreachChecker {
    async fn is_breached(&self, _password: &str) -> Result<bool, RiskEngineError> {
        Ok(false)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Static in-memory set
// ─────────────────────────────────────────────────────────────────────────────

/// In-memory checker backed by a set of full SHA-1 hex hashes (uppercase).
/// Intended for tests and air-gapped deployments that bundle a curated
/// "top N most common breached passwords" list. For production-scale HIBP
/// downloads (hundreds of millions of hashes), use a bloom-filter impl
/// in the host service instead of loading this into memory.
#[derive(Debug, Default, Clone)]
pub struct StaticBreachChecker {
    hashes: HashSet<String>,
}

impl StaticBreachChecker {
    /// Build from an iterator of SHA-1 hex strings. Each is trimmed and
    /// uppercased. Non-40-char entries are silently dropped.
    pub fn from_hashes<I, S>(iter: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let hashes = iter
            .into_iter()
            .map(|s| s.as_ref().trim().to_ascii_uppercase())
            .filter(|s| s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            .collect();
        Self { hashes }
    }

    /// Build from an iterator of plaintext passwords by hashing each one.
    /// Convenient for tests — do NOT hash a real breach corpus this way in
    /// production (it allocates a `String` per entry).
    pub fn from_plaintext<I, S>(iter: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let hashes = iter
            .into_iter()
            .map(|pw| {
                let (p, s) = sha1_prefix_and_suffix(pw.as_ref());
                format!("{p}{s}")
            })
            .collect();
        Self { hashes }
    }

    pub fn len(&self) -> usize {
        self.hashes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.hashes.is_empty()
    }
}

#[async_trait]
impl BreachChecker for StaticBreachChecker {
    async fn is_breached(&self, password: &str) -> Result<bool, RiskEngineError> {
        let (prefix, suffix) = sha1_prefix_and_suffix(password);
        let full = format!("{prefix}{suffix}");
        Ok(self.hashes.contains(&full))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SHA-1("hunter2") = F3BBBD66A63D4BF1747940578EC3D0103530E21D
    const HUNTER2_SHA1: &str = "F3BBBD66A63D4BF1747940578EC3D0103530E21D";

    #[test]
    fn sha1_prefix_suffix_well_known() {
        let (p, s) = sha1_prefix_and_suffix("hunter2");
        assert_eq!(p, "F3BBB");
        assert_eq!(s, "D66A63D4BF1747940578EC3D0103530E21D");
        assert_eq!(format!("{p}{s}"), HUNTER2_SHA1);
    }

    #[test]
    fn hibp_body_parse_matches_counts() {
        let body = "\
AAAA:1\r
D66A63D4BF1747940578EC3D0103530E21D:42\r
BBBB:9\r";
        let (_p, suffix) = sha1_prefix_and_suffix("hunter2");
        assert_eq!(hibp_body_contains(body, &suffix), Some(42));
        assert_eq!(hibp_body_contains(body, "ZZZZ"), None);
    }

    #[test]
    fn hibp_body_parse_skips_malformed_lines() {
        let body = "\
notakeyvalue
FOO:notanumber
F3BBBD66A63D4BF1747940578EC3D0103530E21D:7
";
        // NOTE: hibp_body_contains expects suffix (35 chars) or full-hash
        // comparison; we pass the suffix as HIBP does.
        let (_p, suffix) = sha1_prefix_and_suffix("hunter2");
        // The body line above has the FULL hash not the suffix, so the
        // suffix will NOT match — this exercises the "no false positives
        // on partial substring" invariant.
        assert_eq!(hibp_body_contains(body, &suffix), None);
    }

    #[tokio::test]
    async fn noop_always_returns_false() {
        let r = NoopBreachChecker.is_breached("hunter2").await.unwrap();
        assert!(!r);
    }

    #[tokio::test]
    async fn static_checker_via_plaintext_matches() {
        let c = StaticBreachChecker::from_plaintext(["password", "hunter2", "123456"]);
        assert!(c.is_breached("hunter2").await.unwrap());
        assert!(c.is_breached("password").await.unwrap());
        assert!(!c.is_breached("ferociously-long-unique-string").await.unwrap());
    }

    #[tokio::test]
    async fn static_checker_via_hex_matches() {
        let c = StaticBreachChecker::from_hashes([HUNTER2_SHA1, "not-valid", ""]);
        assert_eq!(c.len(), 1);
        assert!(c.is_breached("hunter2").await.unwrap());
    }
}
