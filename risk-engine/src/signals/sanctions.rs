//! # sanctions
//!
//! Static OFAC / EU sanctioned-country lookup.
//!
//! The adapter calls [`is_sanctioned`] with the ISO 3166-1 alpha-2 country code
//! resolved from GeoIP and populates `RiskContext::is_sanctioned_country`.
//!
//! ## Scope
//!
//! This is a conservative, comprehensive-sanctions list — countries subject to
//! broad OFAC embargoes or sectoral sanctions where most financial activity is
//! prohibited. Fine-grained SDN (Specially Designated Nationals) list matching
//! is a separate problem handled by a dedicated OFAC screening service, NOT
//! this module.
//!
//! ## Maintenance
//!
//! The list is intentionally short so that it can be reviewed and updated
//! in-tree. The authoritative sources are:
//!
//! - US Treasury OFAC:  <https://ofac.treasury.gov/sanctions-programs-and-country-information>
//! - EU consolidated:   <https://www.sanctionsmap.eu/>
//! - UK OFSI:           <https://www.gov.uk/government/publications/financial-sanctions-consolidated-list-of-targets>
//!
//! When policy changes, update [`SANCTIONED_COUNTRIES`] and bump the engine
//! version. Do NOT load this list from a file at runtime — the list is a
//! compliance input that must be code-reviewed.

/// Country codes (ISO 3166-1 alpha-2, uppercase) currently subject to
/// comprehensive or near-comprehensive sanctions. Sorted for binary search
/// and easy human review.
///
/// Last reviewed: 2025-08.
pub const SANCTIONED_COUNTRIES: &[&str] = &[
    "BY", // Belarus — EU/UK/US sectoral sanctions
    "CU", // Cuba — US comprehensive embargo
    "IR", // Iran — US/EU comprehensive
    "KP", // North Korea — UN/US/EU comprehensive
    "MM", // Myanmar — EU/UK/US targeted sanctions (broad scope post-2021)
    "RU", // Russia — G7/EU/UK/US sectoral sanctions
    "SY", // Syria — US/EU comprehensive
    "VE", // Venezuela — US sectoral sanctions
];

/// Return true if the given ISO 3166-1 alpha-2 country code is on the
/// sanctioned-country list. Case-insensitive input — `"ir"`, `"IR"`, and
/// `"Ir"` all resolve identically.
///
/// Returns `false` for empty, malformed, or unknown codes.
pub fn is_sanctioned(country_code: &str) -> bool {
    let trimmed = country_code.trim();
    if trimmed.len() != 2 {
        return false;
    }
    let upper = [
        trimmed.as_bytes()[0].to_ascii_uppercase(),
        trimmed.as_bytes()[1].to_ascii_uppercase(),
    ];
    let upper_str = match std::str::from_utf8(&upper) {
        Ok(s) => s,
        Err(_) => return false,
    };
    SANCTIONED_COUNTRIES.binary_search(&upper_str).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_is_sorted_uppercase_and_two_letter() {
        let mut prev = "";
        for code in SANCTIONED_COUNTRIES {
            assert_eq!(code.len(), 2, "{code} is not 2 chars");
            assert_eq!(*code, code.to_uppercase(), "{code} must be uppercase");
            assert!(*code > prev, "{code} must come after {prev}");
            prev = code;
        }
    }

    #[test]
    fn known_sanctioned_countries_match() {
        for code in ["IR", "KP", "CU", "RU", "SY", "BY", "VE", "MM"] {
            assert!(is_sanctioned(code), "{code} should be sanctioned");
        }
    }

    #[test]
    fn lowercase_and_mixed_case_match() {
        assert!(is_sanctioned("ir"));
        assert!(is_sanctioned("Kp"));
        assert!(is_sanctioned("cu"));
    }

    #[test]
    fn non_sanctioned_countries_do_not_match() {
        for code in ["US", "GB", "DE", "FR", "JP", "CA", "AU", "IN", "BR"] {
            assert!(!is_sanctioned(code), "{code} must NOT be flagged");
        }
    }

    #[test]
    fn malformed_input_is_not_sanctioned() {
        assert!(!is_sanctioned(""));
        assert!(!is_sanctioned("I"));
        assert!(!is_sanctioned("IRN"));
        assert!(!is_sanctioned("  "));
        assert!(!is_sanctioned("123"));
    }
}
