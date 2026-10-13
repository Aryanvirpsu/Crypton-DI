use crate::types::{PolicyContext, PolicyDecision, PolicyEvaluator};

pub struct FailedAttemptsEvaluator;

impl PolicyEvaluator for FailedAttemptsEvaluator {
    fn key(&self) -> &'static str { "failed_attempts" }

    fn evaluate(&self, ctx: &PolicyContext, config: &serde_json::Value) -> PolicyDecision {
        let max_failures = config.get("max_failures")
            .and_then(|v| v.as_u64())
            .unwrap_or(5) as u32;

        if ctx.failed_attempts_count >= max_failures {
            PolicyDecision::Deny("account_locked".to_string())
        } else {
            PolicyDecision::Allow
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn ctx(count: u32) -> PolicyContext {
        PolicyContext {
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
            login_hour:              12,
            login_at:                chrono::Utc::now(),
            failed_attempts_count:   count,
            failed_window_minutes:   15,
        }
    }

    #[test]
    fn below_threshold_allows() {
        let ev = FailedAttemptsEvaluator;
        assert_eq!(ev.evaluate(&ctx(4), &serde_json::json!({"max_failures": 5})), PolicyDecision::Allow);
    }

    #[test]
    fn at_threshold_denies() {
        let ev = FailedAttemptsEvaluator;
        assert_eq!(
            ev.evaluate(&ctx(5), &serde_json::json!({"max_failures": 5})),
            PolicyDecision::Deny("account_locked".to_string()),
        );
    }

    #[test]
    fn default_threshold_used_when_config_empty() {
        let ev = FailedAttemptsEvaluator;
        assert_eq!(
            ev.evaluate(&ctx(5), &serde_json::json!({})),
            PolicyDecision::Deny("account_locked".to_string()),
        );
    }

    #[test]
    fn custom_threshold() {
        let ev = FailedAttemptsEvaluator;
        assert_eq!(
            ev.evaluate(&ctx(3), &serde_json::json!({"max_failures": 3})),
            PolicyDecision::Deny("account_locked".to_string()),
        );
        assert_eq!(
            ev.evaluate(&ctx(2), &serde_json::json!({"max_failures": 3})),
            PolicyDecision::Allow,
        );
    }
}
