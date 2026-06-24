use crate::types::{PolicyContext, PolicyDecision, PolicyEvaluator};

/// Denies login from countries in the configured block list.
///
/// Config: `{ "blocked_countries": ["CN", "RU", "KP", "IR"] }`
///
/// Country codes are ISO 3166-1 alpha-2 (as supplied by Cloudflare's
/// `CF-IPCountry` header or a GeoIP lookup). Fails open when no country
/// code is available — do not use as the sole control for high-security contexts.
pub struct GeoBlockEvaluator;

impl PolicyEvaluator for GeoBlockEvaluator {
    fn key(&self) -> &'static str { "geo_block" }

    fn evaluate(&self, ctx: &PolicyContext, config: &serde_json::Value) -> PolicyDecision {
        let Some(ref country) = ctx.country_code else {
            // No country data — fail open
            return PolicyDecision::Allow;
        };

        let is_blocked = config
            .get("blocked_countries")
            .and_then(|v| v.as_array())
            .map_or(false, |arr| {
                arr.iter().any(|v| {
                    v.as_str()
                        .map_or(false, |s| s.eq_ignore_ascii_case(country))
                })
            });

        if is_blocked {
            PolicyDecision::Deny(format!("country_blocked:{}", country.to_uppercase()))
        } else {
            PolicyDecision::Allow
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn ctx(country: Option<&str>) -> PolicyContext {
        PolicyContext {
            user_id:                 Uuid::nil(),
            username:                "test".into(),
            ip:                      None,
            country_code:            country.map(String::from),
            last_login_country:      None,
            is_tor_exit:             false,
            device_id:               Uuid::nil(),
            credential_status:       "active".to_string(),
            risk_score:              0,
            last_login_at:           None,
            last_credential_used_at: None,
            login_hour:              12,
            login_at:                chrono::Utc::now(),
            failed_attempts_count:   0,
            failed_window_minutes:   15,
        }
    }

    #[test]
    fn blocked_country_denies() {
        let ev = GeoBlockEvaluator;
        let config = serde_json::json!({"blocked_countries": ["CN", "RU"]});
        match ev.evaluate(&ctx(Some("CN")), &config) {
            PolicyDecision::Deny(r) => assert_eq!(r, "country_blocked:CN"),
            other => panic!("expected Deny, got {:?}", other),
        }
    }

    #[test]
    fn blocked_country_case_insensitive() {
        let ev = GeoBlockEvaluator;
        let config = serde_json::json!({"blocked_countries": ["CN"]});
        match ev.evaluate(&ctx(Some("cn")), &config) {
            PolicyDecision::Deny(_) => {}
            other => panic!("expected Deny, got {:?}", other),
        }
    }

    #[test]
    fn allowed_country_passes() {
        let ev = GeoBlockEvaluator;
        let config = serde_json::json!({"blocked_countries": ["CN", "RU"]});
        assert_eq!(ev.evaluate(&ctx(Some("US")), &config), PolicyDecision::Allow);
    }

    #[test]
    fn missing_country_fails_open() {
        let ev = GeoBlockEvaluator;
        let config = serde_json::json!({"blocked_countries": ["CN"]});
        assert_eq!(ev.evaluate(&ctx(None), &config), PolicyDecision::Allow);
    }

    #[test]
    fn empty_block_list_allows_all() {
        let ev = GeoBlockEvaluator;
        assert_eq!(ev.evaluate(&ctx(Some("RU")), &serde_json::json!({})), PolicyDecision::Allow);
    }
}
