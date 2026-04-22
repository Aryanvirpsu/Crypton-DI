pub mod device;
pub mod network;
pub mod session;
pub mod velocity;
pub mod correlation;

use crate::context::RiskAction;
use crate::decision::{Factor, ScoreComponent};
use crate::policy_config::PolicyConfig;

// ─────────────────────────────────────────────────────────────────────────────
// Action multiplier table
// ─────────────────────────────────────────────────────────────────────────────

/// M_action: sensitivity multiplier keyed by action type.
pub fn action_multiplier(action: &RiskAction, policy: &PolicyConfig) -> f32 {
    match action {
        RiskAction::DeviceList => 1.0,
        RiskAction::Login => policy.multiplier_login,
        RiskAction::Register => policy.multiplier_register,
        RiskAction::OauthComplete => policy.multiplier_oauth_complete,
        RiskAction::ActionChallenge => policy.multiplier_action_challenge,
        RiskAction::DeviceMarkLost => policy.multiplier_device_mark_lost,
        RiskAction::DeviceRevoke => policy.multiplier_device_revoke,
        RiskAction::RecoveryStart => policy.multiplier_recovery_start,
        RiskAction::AuditLogExport => policy.multiplier_audit_log_export,
        RiskAction::RecoveryApprove => policy.multiplier_recovery_approve,
        RiskAction::RecoveryComplete => policy.multiplier_recovery_complete,
        RiskAction::ActionExecute { action_name } => match action_name.as_str() {
            "export_data"     => policy.multiplier_execute_export_data,
            "rotate_api_key"  => policy.multiplier_execute_rotate_api_key,
            "delete_resource" => policy.multiplier_execute_delete_resource,
            "add_admin"       => policy.multiplier_execute_add_admin,
            _                 => policy.multiplier_execute_default,
        },
        RiskAction::Unknown { .. } => 1.0,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Factor builder helper (used by scoring sub-modules)
// ─────────────────────────────────────────────────────────────────────────────

pub fn factor(
    name: &str,
    component: ScoreComponent,
    contribution: u8,
    description: impl Into<String>,
) -> Factor {
    Factor {
        name: name.to_string(),
        component,
        contribution,
        description: description.into(),
    }
}

/// Capped scoring helper: adds `raw` to `current` without exceeding `cap`.
/// Returns the effective contribution actually counted, ensuring factor
/// reports match the real score impact.
pub fn capped_add(current: &mut u8, cap: u8, raw: u8) -> u8 {
    let effective = raw.min(cap.saturating_sub(*current));
    *current += effective;
    effective
}
