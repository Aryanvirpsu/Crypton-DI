//! Metrics-instrumented wrapper around [`crate::evaluate`] (Phase 4).
//!
//! This module is gated on the `metrics` feature. It adds zero overhead to the
//! default build and emits through the [`metrics`] facade so the host binary
//! can install any compatible recorder (e.g. `metrics-exporter-prometheus`).
//!
//! ## Full metric taxonomy
//!
//! ### Engine (this module)
//!
//! | Name                                     | Kind      | Labels                  |
//! |------------------------------------------|-----------|-------------------------|
//! | `risk_engine_decision_total`             | counter   | `decision`, `action`    |
//! | `risk_engine_gate_fired_total`           | counter   | `gate`                  |
//! | `risk_engine_factor_fired_total`         | counter   | `factor`, `component`   |
//! | `risk_engine_signals_degraded_total`     | counter   | `source`                |
//! | `risk_engine_escalation_promoted_total`  | counter   | `decision`              |
//! | `risk_engine_evaluate_duration_seconds`  | histogram | `action`                |
//!
//! ### HTTP adapter (`adapters::axum_server`)
//!
//! | Name                                         | Kind      | Labels     |
//! |----------------------------------------------|-----------|------------|
//! | `risk_engine_http_evaluate_total`            | counter   | `outcome`  |
//! | `risk_engine_http_evaluate_duration_seconds` | histogram | —          |
//!
//! ### Rate limiter (`rate_limit`)
//!
//! | Name                                         | Kind      | Labels     |
//! |----------------------------------------------|-----------|------------|
//! | `risk_engine_rate_limit_degraded_total`      | counter   | —          |
//!
//! Emitted from `evaluate_with_rate_limit_fail_open` when the underlying
//! limiter returns a non-`RateLimited` error (Redis down, etc.) and the
//! wrapper admits the request anyway. An alert on this counter tells
//! operators the rate-limiter is effectively off.
//!
//! ### Org-graph background worker (`org_graph::worker`)
//!
//! | Name                                                | Kind      | Labels     |
//! |-----------------------------------------------------|-----------|------------|
//! | `risk_engine_org_cycle_total`                       | counter   | `outcome`  |
//! | `risk_engine_org_cycle_duration_seconds`            | histogram | —          |
//! | `risk_engine_org_cycle_tenants`                     | histogram | —          |
//! | `risk_engine_org_tenant_recompute_total`            | counter   | `outcome`  |
//! | `risk_engine_org_tenant_recompute_duration_seconds` | histogram | —          |
//! | `risk_engine_org_tenant_prune_total`                | counter   | `outcome`  |
//! | `risk_engine_org_tenant_prune_duration_seconds`     | histogram | —          |
//! | `risk_engine_org_total_users_failures_total`        | counter   | —          |
//! | `risk_engine_org_list_tenants_failures_total`       | counter   | —          |
//!
//! ### Enrichment (`signals::metrics`)
//!
//! | Name                                             | Kind      | Labels                  |
//! |--------------------------------------------------|-----------|-------------------------|
//! | `risk_engine_enrichment_classification_total`    | counter   | `source`, `result`      |
//! | `risk_engine_enrichment_duration_seconds`        | histogram | `source`                |
//! | `risk_engine_enrichment_errors_total`            | counter   | `source`                |
//!
//! ## Label hygiene
//!
//! No metric in this taxonomy carries `tenant_id` or `user_id` as a label —
//! both are unbounded-cardinality keys that will crater Prometheus. Use
//! tracing spans (`request_id`, `tenant_id`, `user_id`) for per-entity
//! correlation instead.
//!
//! The wrapper preserves [`evaluate`]'s contract — same input, same output —
//! so callers can swap it in without touching downstream code.

use std::time::Instant;

use metrics::{counter, histogram};

use crate::context::RiskContext;
use crate::decision::{Decision, HardGate, RiskDecision, ScoreComponent};
use crate::engine::evaluate;
use crate::store::SignalStore;

/// Drop-in replacement for [`crate::evaluate`] that also emits metrics.
pub async fn evaluate_with_metrics<S: SignalStore>(
    ctx: RiskContext,
    store: &S,
) -> RiskDecision {
    let action_label = ctx.action.label().to_string();
    let start = Instant::now();

    let decision = evaluate(ctx, store).await;

    let elapsed = start.elapsed().as_secs_f64();
    histogram!("risk_engine_evaluate_duration_seconds", "action" => action_label.clone())
        .record(elapsed);

    counter!(
        "risk_engine_decision_total",
        "decision" => decision_label(&decision.decision),
        "action" => action_label.clone(),
    )
    .increment(1);

    for gate in &decision.triggered_gates {
        counter!(
            "risk_engine_gate_fired_total",
            "gate" => gate_label(gate),
        )
        .increment(1);
    }

    for factor in &decision.contributing_factors {
        counter!(
            "risk_engine_factor_fired_total",
            "factor" => factor.name.clone(),
            "component" => component_label(&factor.component),
        )
        .increment(1);
    }

    if decision.signals_degraded {
        counter!("risk_engine_signals_degraded_total", "source" => "any").increment(1);
    }

    if decision.escalated {
        counter!(
            "risk_engine_escalation_promoted_total",
            "decision" => decision_label(&decision.decision),
        )
        .increment(1);
    }

    decision
}

fn decision_label(d: &Decision) -> &'static str {
    match d {
        Decision::Allow => "allow",
        Decision::Challenge => "challenge",
        Decision::Hold => "hold",
        Decision::Deny => "deny",
    }
}

fn component_label(c: &ScoreComponent) -> &'static str {
    match c {
        ScoreComponent::Device => "device",
        ScoreComponent::Session => "session",
        ScoreComponent::Network => "network",
        ScoreComponent::Behavioral => "behavioral",
        ScoreComponent::Correlation => "correlation",
        ScoreComponent::Absolute => "absolute",
        ScoreComponent::Gate => "gate",
    }
}

fn gate_label(g: &HardGate) -> &'static str {
    match g {
        HardGate::RevokedCredential => "revoked_credential",
        HardGate::LostCredential => "lost_credential",
        HardGate::AccountLocked => "account_locked",
        HardGate::NonceReplay => "nonce_replay",
        HardGate::ImpossibleTravelViaTor => "impossible_travel_via_tor",
        HardGate::TorOnSensitiveAction => "tor_on_sensitive_action",
        HardGate::RecoveryCompletedWithNoPending => "recovery_completed_with_no_pending",
        HardGate::OauthNonceIpMismatchOnTor => "oauth_nonce_ip_mismatch_on_tor",
        HardGate::OauthIpMismatch => "oauth_ip_mismatch",
        HardGate::SignCountRollback => "sign_count_rollback",
        HardGate::SanctionedCountry => "sanctioned_country",
        HardGate::BreachedCredentialOnSensitive => "breached_credential_on_sensitive",
        HardGate::VpnOnCriticalAction => "vpn_on_critical_action",
        HardGate::TorOnRecovery => "tor_on_recovery",
        HardGate::AutomationOnRegister => "automation_on_register",
        HardGate::SensitiveActionOnMismatch => "sensitive_action_on_mismatch",
        HardGate::PrivilegedActionOnFreshCredential => "privileged_action_on_fresh_credential",
        HardGate::ExpiredJwtOnSensitive => "expired_jwt_on_sensitive",
    }
}
