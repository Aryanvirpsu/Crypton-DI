//! # disposable_email
//!
//! Static lookup for disposable / temporary email providers.
//!
//! Populates `RiskContext::email_domain_disposable`. The adapter extracts the
//! domain from the user's email address and calls [`is_disposable`].
//!
//! ## Scope
//!
//! The bundled list covers the highest-volume disposable providers observed
//! in account-abuse data. It is not exhaustive — it deliberately stays small
//! enough to review in-tree. For production deployments that require
//! comprehensive coverage, load a larger list at runtime via
//! [`DisposableDomains::from_iter`] and wrap calls through that struct.
//!
//! ## Non-goals
//!
//! - **Forwarder detection** (SimpleLogin, 33Mail, DuckDuckGo Email Protection)
//!   is intentionally NOT in this list. Forwarders are legitimate privacy
//!   tools and blocking them harms users.
//! - **Sub-addressing** (`user+tag@gmail.com`) is not disposable — Gmail is
//!   not flagged.

use std::collections::HashSet;

/// Bundled baseline list of disposable email domains. All lowercase, sorted.
///
/// Sources reviewed: <https://github.com/disposable-email-domains/disposable-email-domains>
/// and the top providers by volume in our own abuse telemetry.
pub const BUILTIN_DISPOSABLE_DOMAINS: &[&str] = &[
    "10minutemail.com",
    "anonaddy.me",
    "armyspy.com",
    "cuvox.de",
    "dayrep.com",
    "deadaddress.com",
    "dispostable.com",
    "einrot.com",
    "fakeinbox.com",
    "fleckens.hu",
    "getairmail.com",
    "guerrillamail.biz",
    "guerrillamail.com",
    "guerrillamail.de",
    "guerrillamail.info",
    "guerrillamail.net",
    "guerrillamail.org",
    "guerrillamailblock.com",
    "harakirimail.com",
    "inboxbear.com",
    "jetable.org",
    "mailcatch.com",
    "maildrop.cc",
    "mailforspam.com",
    "mailinator.com",
    "mailnesia.com",
    "mintemail.com",
    "mohmal.com",
    "mytrashmail.com",
    "nowmymail.com",
    "rhyta.com",
    "sharklasers.com",
    "spam4.me",
    "spambox.us",
    "spamgourmet.com",
    "spamherelots.com",
    "superrito.com",
    "teleworm.us",
    "temp-mail.org",
    "tempail.com",
    "tempinbox.com",
    "tempmail.com",
    "tempmail.plus",
    "tempmailaddress.com",
    "tempmailo.com",
    "throwawaymail.com",
    "trashmail.com",
    "trashmail.de",
    "tutanota.com",
    "yopmail.com",
    "yopmail.fr",
    "yopmail.net",
];

/// Quick check against the built-in list. Domain comparison is
/// case-insensitive.
///
/// For dynamic or larger lists, use [`DisposableDomains`].
pub fn is_disposable(domain: &str) -> bool {
    let lower = domain.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return false;
    }
    BUILTIN_DISPOSABLE_DOMAINS
        .binary_search(&lower.as_str())
        .is_ok()
}

/// Extract the domain portion (everything after the last `@`) from an email
/// address, normalised to lowercase. Returns `None` if the input has no `@`
/// or the local/domain part is empty.
pub fn extract_domain(email: &str) -> Option<String> {
    let trimmed = email.trim();
    let at = trimmed.rfind('@')?;
    let (local, rest) = trimmed.split_at(at);
    if local.is_empty() {
        return None;
    }
    let domain = &rest[1..];
    if domain.is_empty() {
        return None;
    }
    Some(domain.to_ascii_lowercase())
}

/// Runtime-configurable disposable-domain set. Construct from an iterator of
/// domains (e.g., loaded from a config file or a larger upstream list) and
/// query via [`Self::contains`].
///
/// Intended use: the host adapter builds one at startup, wraps it in `Arc`,
/// and calls `contains` during request enrichment.
pub struct DisposableDomains {
    set: HashSet<String>,
}

impl DisposableDomains {
    /// Build from the bundled [`BUILTIN_DISPOSABLE_DOMAINS`] list.
    pub fn builtin() -> Self {
        Self::from_iter(BUILTIN_DISPOSABLE_DOMAINS.iter().copied())
    }

    /// Build from any iterator of domains. Each entry is trimmed and
    /// lowercased.
    pub fn from_iter<I, S>(iter: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let set = iter
            .into_iter()
            .map(|s| s.as_ref().trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        Self { set }
    }

    /// Extend this set with additional domains — useful when merging a
    /// host-supplied list onto the built-in baseline.
    pub fn extend<I, S>(&mut self, iter: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for s in iter {
            let normalised = s.as_ref().trim().to_ascii_lowercase();
            if !normalised.is_empty() {
                self.set.insert(normalised);
            }
        }
    }

    /// Case-insensitive domain membership check.
    pub fn contains(&self, domain: &str) -> bool {
        let lower = domain.trim().to_ascii_lowercase();
        !lower.is_empty() && self.set.contains(&lower)
    }

    /// Convenience: extract the domain from an email and check membership in
    /// one call. Returns `false` for malformed input.
    pub fn contains_email_domain(&self, email: &str) -> bool {
        match extract_domain(email) {
            Some(d) => self.contains(&d),
            None => false,
        }
    }

    pub fn len(&self) -> usize {
        self.set.len()
    }

    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_list_is_sorted_and_lowercase() {
        let mut prev = "";
        for d in BUILTIN_DISPOSABLE_DOMAINS {
            assert_eq!(*d, d.to_lowercase(), "{d} must be lowercase");
            assert!(*d > prev, "{d} must come after {prev} (list must be sorted)");
            prev = d;
        }
    }

    #[test]
    fn known_disposable_domains_match() {
        assert!(is_disposable("mailinator.com"));
        assert!(is_disposable("yopmail.com"));
        assert!(is_disposable("guerrillamail.com"));
        assert!(is_disposable("tempmail.com"));
    }

    #[test]
    fn case_insensitive_match() {
        assert!(is_disposable("MAILINATOR.COM"));
        assert!(is_disposable("MailInator.com"));
    }

    #[test]
    fn legitimate_providers_do_not_match() {
        for d in ["gmail.com", "outlook.com", "proton.me", "icloud.com"] {
            assert!(!is_disposable(d));
        }
    }

    #[test]
    fn forwarders_not_in_list() {
        for d in ["simplelogin.io", "duck.com", "33mail.com"] {
            assert!(!is_disposable(d), "forwarder {d} should NOT be flagged");
        }
    }

    #[test]
    fn extract_domain_handles_common_shapes() {
        assert_eq!(extract_domain("a@b.com"), Some("b.com".into()));
        assert_eq!(extract_domain("Mixed@Case.COM"), Some("case.com".into()));
        assert_eq!(extract_domain("  u@host  "), Some("host".into()));
        assert_eq!(extract_domain("u+tag@gmail.com"), Some("gmail.com".into()));
        // The raw intent of last-@ handling is to tolerate quoted local parts.
        assert_eq!(
            extract_domain("\"weird@local\"@example.org"),
            Some("example.org".into())
        );
    }

    #[test]
    fn extract_domain_rejects_malformed() {
        assert_eq!(extract_domain(""), None);
        assert_eq!(extract_domain("noatsign"), None);
        assert_eq!(extract_domain("@no.local"), None);
        assert_eq!(extract_domain("no.domain@"), None);
    }

    #[test]
    fn runtime_set_matches_builtin_and_extends() {
        let mut set = DisposableDomains::builtin();
        assert!(set.contains("mailinator.com"));
        assert!(!set.contains("private-list.example"));

        set.extend(["private-list.example", "", "  SECOND.Example  "]);
        assert!(set.contains("private-list.example"));
        assert!(set.contains("second.example"));
    }

    #[test]
    fn runtime_set_contains_email_domain() {
        let set = DisposableDomains::builtin();
        assert!(set.contains_email_domain("evil@mailinator.com"));
        assert!(!set.contains_email_domain("user@gmail.com"));
        assert!(!set.contains_email_domain("malformed"));
    }
}
