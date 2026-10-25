pub mod device_trust;
pub mod failed_attempts;
pub mod geo_block;
pub mod geo_velocity;
pub mod new_device;
pub mod off_hours;
pub mod stepup_risk;
pub mod tor_block;

use crate::{
    cache::PolicyCache,
    types::{PolicyContext, PolicyDecision, PolicyEvaluator},
};
use device_trust::DeviceTrustEvaluator;
use failed_attempts::FailedAttemptsEvaluator;
use geo_block::GeoBlockEvaluator;
use geo_velocity::GeoVelocityEvaluator;
use new_device::NewDeviceEvaluator;
use off_hours::OffHoursEvaluator;
use stepup_risk::StepUpRiskEvaluator;
use tor_block::TorBlockEvaluator;

/// Run all enabled evaluators against `ctx`.
///
/// Evaluation order matters:
///   1. Device approval gate first (blocks pending_approval with audit trail)
///   2. Hard denies (account lock, TOR, geo block, impossible travel, device inactivity)
///   3. Step-up triggers last (risk score, off-hours)
///
/// First Deny wins and short-circuits. **All** StepUp reasons are accumulated
/// and joined with "," so operators see every triggered policy, not just the
/// last one.  No heap allocation — evaluators are zero-sized structs referenced
/// from a stack-local slice.
pub fn evaluate_all(ctx: &PolicyContext, cache: &PolicyCache) -> PolicyDecision {
    // Stack slice of references to ZST evaluators — zero heap allocation.
    let evaluators: &[&dyn PolicyEvaluator] = &[
        &NewDeviceEvaluator,       // device approval gate — always first
        &FailedAttemptsEvaluator,
        &TorBlockEvaluator,
        &GeoBlockEvaluator,
        &GeoVelocityEvaluator,
        &DeviceTrustEvaluator,
        &StepUpRiskEvaluator,
        &OffHoursEvaluator,
    ];

    let mut step_up_reasons: Vec<String> = Vec::new();

    for ev in evaluators {
        let record = match cache.get(ev.key()) {
            Some(r) if r.enabled => r,
            _ => continue,
        };

        match ev.evaluate(ctx, &record.config) {
            PolicyDecision::Allow => {}
            PolicyDecision::Deny(r) => return PolicyDecision::Deny(r),
            // Accumulate all step-up reasons rather than overwriting — operators
            // and the audit log should see every policy that fired.
            PolicyDecision::StepUp(r) => step_up_reasons.push(r),
        }
    }

    if step_up_reasons.is_empty() {
        PolicyDecision::Allow
    } else {
        PolicyDecision::StepUp(step_up_reasons.join(","))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cache::PolicyCache, types::PolicyRecord};
    use uuid::Uuid;

    fn make_ctx() -> PolicyContext {
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
            failed_attempts_count:   0,
            failed_window_minutes:   15,
        }
    }

    fn enabled_record(key: &str, config: serde_json::Value) -> PolicyRecord {
        PolicyRecord {
            key: key.into(),
            display_name: key.into(),
            description: String::new(),
            enabled: true,
            config,
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn all_disabled_allows() {
        let cache = PolicyCache::empty();
        assert_eq!(evaluate_all(&make_ctx(), &cache), PolicyDecision::Allow);
    }

    #[test]
    fn deny_wins_over_stepup() {
        let cache = PolicyCache::empty();
        cache.upsert(enabled_record("failed_attempts", serde_json::json!({"max_failures": 1})));
        cache.upsert(enabled_record("stepup_risk", serde_json::json!({"threshold": 0})));

        let mut ctx = make_ctx();
        ctx.failed_attempts_count = 5;
        ctx.risk_score = 100;

        assert_eq!(evaluate_all(&ctx, &cache), PolicyDecision::Deny("account_locked".to_string()));
    }

    #[test]
    fn stepup_without_deny() {
        let cache = PolicyCache::empty();
        cache.upsert(enabled_record("stepup_risk", serde_json::json!({"threshold": 0})));

        let mut ctx = make_ctx();
        ctx.risk_score = 100;

        assert_eq!(evaluate_all(&ctx, &cache), PolicyDecision::StepUp("high_risk_score".to_string()));
    }

    #[test]
    fn disabled_evaluator_skipped() {
        let cache = PolicyCache::empty();
        cache.upsert(PolicyRecord {
            key:          "failed_attempts".into(),
            display_name: "test".into(),
            description:  String::new(),
            enabled:      false,  // disabled
            config:       serde_json::json!({"max_failures": 1}),
            updated_at:   chrono::Utc::now(),
        });

        let mut ctx = make_ctx();
        ctx.failed_attempts_count = 99;

        assert_eq!(evaluate_all(&ctx, &cache), PolicyDecision::Allow);
    }
}
