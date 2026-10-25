use chrono::Timelike as _;

use crate::types::{PolicyContext, PolicyDecision, PolicyEvaluator};

/// Triggers step-up authentication for logins outside configured business hours.
///
/// Returns `StepUp` rather than `Deny` — the user can proceed after additional
/// verification. Use with a step-up UI that prompts for a secondary factor or
/// admin confirmation.
///
/// Config:
/// ```json
/// { "start_hour": 8, "end_hour": 18, "timezone": "America/New_York" }
/// ```
/// Both hours are inclusive on the start, exclusive on the end (i.e. [8, 18)).
/// A login at hour 18 is considered off-hours.
///
/// `timezone` must be a valid IANA timezone name understood by `chrono-tz`.
/// If omitted or unrecognised, UTC is used (P2-8).
pub struct OffHoursEvaluator;

impl PolicyEvaluator for OffHoursEvaluator {
    fn key(&self) -> &'static str { "off_hours" }

    fn evaluate(&self, ctx: &PolicyContext, config: &serde_json::Value) -> PolicyDecision {
        let start_raw = config.get("start_hour").and_then(|v| v.as_u64()).unwrap_or(8);
        let end_raw   = config.get("end_hour").and_then(|v| v.as_u64()).unwrap_or(18);

        // Clamp to [0, 23] and warn on obviously wrong config values.
        let start_hour = start_raw.min(23) as u8;
        let end_hour   = end_raw.min(23) as u8;
        if start_raw > 23 || end_raw > 23 {
            tracing::warn!(
                start = start_raw, end = end_raw,
                "off_hours: hour value > 23 clamped — check policy config"
            );
        }
        if start_hour >= end_hour {
            tracing::warn!(
                start = start_hour, end = end_hour,
                "off_hours: start_hour >= end_hour — entire day treated as off-hours"
            );
        }

        // P2-8: convert login timestamp to configured timezone before comparing hours.
        let tz_str = config.get("timezone")
            .and_then(|v| v.as_str())
            .unwrap_or("UTC");

        let local_hour: u8 = if tz_str == "UTC" {
            ctx.login_at.hour() as u8
        } else {
            match tz_str.parse::<chrono_tz::Tz>() {
                Ok(tz) => ctx.login_at.with_timezone(&tz).hour() as u8,
                Err(_) => {
                    tracing::warn!(
                        timezone = tz_str,
                        "off_hours: unrecognised timezone, falling back to UTC"
                    );
                    ctx.login_at.hour() as u8
                }
            }
        };

        let within_hours = local_hour >= start_hour && local_hour < end_hour;
        if within_hours {
            PolicyDecision::Allow
        } else {
            // Reason is intentionally terse: do not leak the local hour or the
            // internal timezone string to the client — those are config internals
            // that would help an attacker time their login attempts.
            PolicyDecision::StepUp("off_hours_login".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone as _;
    use uuid::Uuid;

    /// Build a `PolicyContext` where `login_at` is set to `hour:00 UTC` on
    /// an arbitrary fixed date (2024-01-15) so tests are deterministic.
    fn ctx(hour: u8) -> PolicyContext {
        crate::types::PolicyContext {
            user_id:                 Uuid::nil(),
            username:                "test".into(),
            ip:                      None,
            country_code:            None,
            last_login_country:      None,
            is_tor_exit:             false,
            device_id:               Uuid::nil(),
            credential_status:       "active".to_string(),
            risk_score:              0,
            last_login_at:           None,
            last_credential_used_at: None,
            login_hour:              hour,
            login_at:                chrono::Utc
                .with_ymd_and_hms(2024, 1, 15, hour as u32, 0, 0)
                .unwrap(),
            failed_attempts_count:   0,
            failed_window_minutes:   15,
        }
    }

    #[test]
    fn during_hours_allows() {
        let ev = OffHoursEvaluator;
        let config = serde_json::json!({"start_hour": 8, "end_hour": 18});
        assert_eq!(ev.evaluate(&ctx(9),  &config), PolicyDecision::Allow);
        assert_eq!(ev.evaluate(&ctx(8),  &config), PolicyDecision::Allow);
        assert_eq!(ev.evaluate(&ctx(17), &config), PolicyDecision::Allow);
    }

    #[test]
    fn off_hours_steps_up() {
        let ev = OffHoursEvaluator;
        let config = serde_json::json!({"start_hour": 8, "end_hour": 18});
        // Reason must be the opaque "off_hours_login" — no hour or tz leakage.
        assert_eq!(
            ev.evaluate(&ctx(2), &config),
            PolicyDecision::StepUp("off_hours_login".to_string()),
        );
        assert_eq!(
            ev.evaluate(&ctx(18), &config),
            PolicyDecision::StepUp("off_hours_login".to_string()),
        );
        assert_eq!(
            ev.evaluate(&ctx(23), &config),
            PolicyDecision::StepUp("off_hours_login".to_string()),
        );
    }

    #[test]
    fn default_config_8_to_18() {
        let ev = OffHoursEvaluator;
        assert_eq!(ev.evaluate(&ctx(12), &serde_json::json!({})), PolicyDecision::Allow);
        assert_eq!(
            ev.evaluate(&ctx(3), &serde_json::json!({})),
            PolicyDecision::StepUp("off_hours_login".to_string()),
        );
    }

    #[test]
    fn timezone_aware_new_york() {
        // 13:00 UTC = 08:00 EST (UTC-5) on a winter date → within 8–18 → Allow
        let ev = OffHoursEvaluator;
        let config = serde_json::json!({"start_hour": 8, "end_hour": 18, "timezone": "America/New_York"});
        assert_eq!(ev.evaluate(&ctx(13), &config), PolicyDecision::Allow);

        // 01:00 UTC = 20:00 EST → off-hours → StepUp
        assert_eq!(
            ev.evaluate(&ctx(1), &config),
            PolicyDecision::StepUp("off_hours_login".to_string()),
        );
    }

    #[test]
    fn invalid_timezone_falls_back_to_utc() {
        let ev = OffHoursEvaluator;
        let config = serde_json::json!({"start_hour": 8, "end_hour": 18, "timezone": "Not/AReal_Tz"});
        // UTC fallback: hour 12 is within 8–18 → Allow
        assert_eq!(ev.evaluate(&ctx(12), &config), PolicyDecision::Allow);
    }

    #[test]
    fn out_of_range_hours_clamped() {
        // start=25 → clamped to 23; end=23 → start >= end warning, entire day is off-hours
        let ev = OffHoursEvaluator;
        let config = serde_json::json!({"start_hour": 25, "end_hour": 23});
        // Any hour should trigger StepUp since start(23) >= end(23)
        assert_eq!(
            ev.evaluate(&ctx(12), &config),
            PolicyDecision::StepUp("off_hours_login".to_string()),
        );
    }
}
