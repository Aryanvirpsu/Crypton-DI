use crate::types::{PolicyContext, PolicyDecision, PolicyEvaluator};

/// Denies login if the user's country changed between the last successful login
/// and now, within a time window where physical travel would be impossible.
///
/// Config: `{ "window_hours": 4 }`
///
/// Requires `country_code` (current) and `last_login_country` + `last_login_at`
/// (previous) to be populated in `PolicyContext`. Silently allows if any field
/// is missing — fail open so legitimate users aren't blocked by missing data.
pub struct GeoVelocityEvaluator;

impl PolicyEvaluator for GeoVelocityEvaluator {
    fn key(&self) -> &'static str { "geo_velocity" }

    fn evaluate(&self, ctx: &PolicyContext, config: &serde_json::Value) -> PolicyDecision {
        let window_hours = config.get("window_hours")
            .and_then(|v| v.as_u64())
            .unwrap_or(4);

        // Fail open: missing data → skip the check
        let (Some(current_country), Some(last_country), Some(last_login_at)) = (
            ctx.country_code.as_deref(),
            ctx.last_login_country.as_deref(),
            ctx.last_login_at,
        ) else {
            return PolicyDecision::Allow;
        };

        // Same country → no velocity issue
        if current_country.eq_ignore_ascii_case(last_country) {
            return PolicyDecision::Allow;
        }

        // Use ctx.login_at as the authoritative "now" so all evaluators share the
        // same clock snapshot and tests can control the result deterministically.
        let elapsed_secs = ctx.login_at
            .signed_duration_since(last_login_at)
            .num_seconds()
            .max(0) as u64;

        let window_secs = window_hours * 3600;
        if elapsed_secs < window_secs {
            PolicyDecision::Deny(format!(
                "impossible_travel_{}h",
                elapsed_secs / 3600
            ))
        } else {
            // Country changed but enough time passed for plausible travel — allow
            PolicyDecision::Allow
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    fn ctx(
        current_country: Option<&str>,
        last_country: Option<&str>,
        last_login_at: Option<chrono::DateTime<Utc>>,
    ) -> PolicyContext {
        let now = Utc::now();
        PolicyContext {
            user_id:                 Uuid::nil(),
            username:                "test".into(),
            ip:                      None,
            country_code:            current_country.map(String::from),
            last_login_country:      last_country.map(String::from),
            is_tor_exit:             false,
            device_id:               Uuid::nil(),
            credential_status:       "active".to_string(),
            risk_score:              0,
            last_login_at,
            last_credential_used_at: None,
            login_hour:              12,
            login_at:                now,
            failed_attempts_count:   0,
            failed_window_minutes:   15,
        }
    }

    #[test]
    fn no_history_allows() {
        let ev = GeoVelocityEvaluator;
        assert_eq!(ev.evaluate(&ctx(Some("US"), None, None), &serde_json::json!({})), PolicyDecision::Allow);
    }

    #[test]
    fn same_country_allows() {
        let ev = GeoVelocityEvaluator;
        let recent = Utc::now() - Duration::hours(1);
        assert_eq!(
            ev.evaluate(&ctx(Some("US"), Some("US"), Some(recent)), &serde_json::json!({"window_hours": 4})),
            PolicyDecision::Allow,
        );
    }

    #[test]
    fn different_country_within_window_denies() {
        let ev = GeoVelocityEvaluator;
        // Build a context where login_at is "now" and last login was 1 hour ago.
        let now = Utc::now();
        let one_hour_ago = now - Duration::hours(1);
        let mut c = ctx(Some("GB"), Some("US"), Some(one_hour_ago));
        c.login_at = now;
        match ev.evaluate(&c, &serde_json::json!({"window_hours": 4})) {
            PolicyDecision::Deny(r) => assert!(r.starts_with("impossible_travel_")),
            other => panic!("expected Deny, got {:?}", other),
        }
    }

    #[test]
    fn different_country_outside_window_allows() {
        let ev = GeoVelocityEvaluator;
        let now = Utc::now();
        let six_hours_ago = now - Duration::hours(6);
        let mut c = ctx(Some("GB"), Some("US"), Some(six_hours_ago));
        c.login_at = now;
        assert_eq!(
            ev.evaluate(&c, &serde_json::json!({"window_hours": 4})),
            PolicyDecision::Allow,
        );
    }
}
