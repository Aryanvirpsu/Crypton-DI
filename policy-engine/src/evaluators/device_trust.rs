use crate::types::{PolicyContext, PolicyDecision, PolicyEvaluator};

/// Triggers step-up authentication when a credential hasn't been used within
/// `inactivity_days` days, prompting the user to re-verify rather than locking
/// them out entirely (which would require full re-enrollment).
///
/// Config: `{ "inactivity_days": 30 }`
pub struct DeviceTrustEvaluator;

impl PolicyEvaluator for DeviceTrustEvaluator {
    fn key(&self) -> &'static str { "device_trust" }

    fn evaluate(&self, ctx: &PolicyContext, config: &serde_json::Value) -> PolicyDecision {
        let inactivity_days = config.get("inactivity_days")
            .and_then(|v| v.as_u64())
            .unwrap_or(30);

        let Some(last_used) = ctx.last_credential_used_at else {
            // Never used before — brand-new credential. Auth already succeeded so allow.
            return PolicyDecision::Allow;
        };

        // Use ctx.login_at as the authoritative "now" for determinism across evaluators.
        let elapsed_secs = ctx.login_at
            .signed_duration_since(last_used)
            .num_seconds()
            .max(0) as u64;

        let threshold_secs = inactivity_days * 86400;
        if elapsed_secs > threshold_secs {
            // StepUp (not Deny): the user should re-verify, not be permanently locked
            // out.  A Deny would require full device re-enrollment to recover.
            PolicyDecision::StepUp(format!(
                "device_inactive_{}d",
                elapsed_secs / 86400
            ))
        } else {
            PolicyDecision::Allow
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    fn ctx(last_used: Option<chrono::DateTime<Utc>>) -> PolicyContext {
        let now = Utc::now();
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
            last_credential_used_at: last_used,
            login_hour:              12,
            login_at:                now,
            failed_attempts_count:   0,
            failed_window_minutes:   15,
        }
    }

    #[test]
    fn never_used_allows() {
        let ev = DeviceTrustEvaluator;
        assert_eq!(ev.evaluate(&ctx(None), &serde_json::json!({})), PolicyDecision::Allow);
    }

    #[test]
    fn recently_used_allows() {
        let ev = DeviceTrustEvaluator;
        let recent = Utc::now() - Duration::days(10);
        assert_eq!(
            ev.evaluate(&ctx(Some(recent)), &serde_json::json!({"inactivity_days": 30})),
            PolicyDecision::Allow,
        );
    }

    #[test]
    fn stale_device_steps_up() {
        let ev = DeviceTrustEvaluator;
        let now = Utc::now();
        let stale = now - Duration::days(31);
        // Build context where login_at == now so elapsed = 31 days
        let mut c = ctx(Some(stale));
        c.login_at = now;
        match ev.evaluate(&c, &serde_json::json!({"inactivity_days": 30})) {
            PolicyDecision::StepUp(r) => assert!(r.starts_with("device_inactive_")),
            other => panic!("expected StepUp, got {:?}", other),
        }
    }
}
