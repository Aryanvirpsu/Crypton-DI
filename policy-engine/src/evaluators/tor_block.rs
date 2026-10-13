use crate::types::{PolicyContext, PolicyDecision, PolicyEvaluator};

/// Denies login from TOR exit nodes or known VPN IPs.
///
/// The check is pre-computed in `build_policy_context` by looking up the
/// client IP in the Redis SET `tor:exit_nodes`. Operators populate this set
/// via their preferred TOR exit-node feed (e.g. via a cron job calling
/// `SADD tor:exit_nodes <ip> ...`).
///
/// Config: `{}` (no config — presence in the Redis set is the signal)
pub struct TorBlockEvaluator;

impl PolicyEvaluator for TorBlockEvaluator {
    fn key(&self) -> &'static str { "tor_block" }

    fn evaluate(&self, ctx: &PolicyContext, _config: &serde_json::Value) -> PolicyDecision {
        if ctx.is_tor_exit {
            PolicyDecision::Deny("tor_exit_node".to_string())
        } else {
            PolicyDecision::Allow
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn ctx(is_tor: bool) -> PolicyContext {
        PolicyContext {
            user_id:                 Uuid::nil(),
            username:                "test".into(),
            ip:                      Some("1.2.3.4".into()),
            country_code:            None,
            last_login_country:      None,
            is_tor_exit:             is_tor,
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
    fn tor_exit_node_denies() {
        let ev = TorBlockEvaluator;
        assert_eq!(
            ev.evaluate(&ctx(true), &serde_json::json!({})),
            PolicyDecision::Deny("tor_exit_node".to_string()),
        );
    }

    #[test]
    fn normal_ip_allows() {
        let ev = TorBlockEvaluator;
        assert_eq!(ev.evaluate(&ctx(false), &serde_json::json!({})), PolicyDecision::Allow);
    }
}
