use crate::types::{PolicyContext, PolicyDecision, PolicyEvaluator};

/// Denies login attempts by credentials that are awaiting admin approval.
///
/// This evaluator runs as the **first** step in the policy chain, before any
/// other checks.  It is defence-in-depth: `verify_device_active` in
/// `oauth_complete` already blocks non-active credentials, but placing the
/// check here means:
///
/// 1. The block is **audited** — a `policy_evaluations` row is written for
///    every attempt, giving operators visibility into unapproved-device usage.
/// 2. Future evolution can promote this to a `StepUp` (e.g. requiring the
///    user to re-confirm via email) by changing only this evaluator.
///
/// Config: `{}` (no tuneable parameters)
pub struct NewDeviceEvaluator;

impl PolicyEvaluator for NewDeviceEvaluator {
    fn key(&self) -> &'static str { "new_device" }

    fn evaluate(&self, ctx: &PolicyContext, _config: &serde_json::Value) -> PolicyDecision {
        if ctx.credential_status == "pending_approval" {
            PolicyDecision::Deny("pending_admin_approval".to_string())
        } else {
            PolicyDecision::Allow
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn ctx(status: &str) -> PolicyContext {
        PolicyContext {
            user_id:                 Uuid::nil(),
            username:                "test".into(),
            ip:                      None,
            country_code:            None,
            last_login_country:      None,
            is_tor_exit:             false,
            device_id:               Uuid::nil(),
            credential_status:       status.to_string(),
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
    fn active_credential_allows() {
        let ev = NewDeviceEvaluator;
        assert_eq!(ev.evaluate(&ctx("active"), &serde_json::json!({})), PolicyDecision::Allow);
    }

    #[test]
    fn pending_approval_denies() {
        let ev = NewDeviceEvaluator;
        assert_eq!(
            ev.evaluate(&ctx("pending_approval"), &serde_json::json!({})),
            PolicyDecision::Deny("pending_admin_approval".to_string()),
        );
    }

    #[test]
    fn revoked_credential_allows_from_evaluator_perspective() {
        // `verify_device_active` handles revoked — this evaluator only gates pending_approval.
        let ev = NewDeviceEvaluator;
        assert_eq!(ev.evaluate(&ctx("revoked"), &serde_json::json!({})), PolicyDecision::Allow);
    }
}
