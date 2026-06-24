use crate::types::{PolicyContext, PolicyDecision, PolicyEvaluator};

pub struct StepUpRiskEvaluator;

impl PolicyEvaluator for StepUpRiskEvaluator {
    fn key(&self) -> &'static str { "stepup_risk" }

    fn evaluate(&self, ctx: &PolicyContext, config: &serde_json::Value) -> PolicyDecision {
        let threshold = config.get("threshold")
            .and_then(|v| v.as_u64())
            .unwrap_or(70) as u32;

        if ctx.risk_score >= threshold {
            PolicyDecision::StepUp("high_risk_score".to_string())
        } else {
            PolicyDecision::Allow
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn ctx(score: u32) -> PolicyContext {
        PolicyContext {
            user_id:                 Uuid::nil(),
            username:                "test".into(),
            ip:                      None,
            country_code:            None,
            last_login_country:      None,
            is_tor_exit:             false,
            device_id:               Uuid::nil(),
            credential_status:       "active".to_string(),
            risk_score:              score,
            last_login_at:           None,
            last_credential_used_at: None,
            login_hour:              12,
            login_at:                chrono::Utc::now(),
            failed_attempts_count:   0,
            failed_window_minutes:   15,
        }
    }

    #[test]
    fn below_threshold_allows() {
        let ev = StepUpRiskEvaluator;
        assert_eq!(ev.evaluate(&ctx(69), &serde_json::json!({"threshold": 70})), PolicyDecision::Allow);
    }

    #[test]
    fn at_threshold_steps_up() {
        let ev = StepUpRiskEvaluator;
        assert_eq!(
            ev.evaluate(&ctx(70), &serde_json::json!({"threshold": 70})),
            PolicyDecision::StepUp("high_risk_score".to_string()),
        );
    }

    #[test]
    fn default_threshold() {
        let ev = StepUpRiskEvaluator;
        assert_eq!(
            ev.evaluate(&ctx(70), &serde_json::json!({})),
            PolicyDecision::StepUp("high_risk_score".to_string()),
        );
        assert_eq!(ev.evaluate(&ctx(69), &serde_json::json!({})), PolicyDecision::Allow);
    }

    #[test]
    fn above_threshold_steps_up() {
        let ev = StepUpRiskEvaluator;
        assert_eq!(
            ev.evaluate(&ctx(95), &serde_json::json!({"threshold": 70})),
            PolicyDecision::StepUp("high_risk_score".to_string()),
        );
    }
}
