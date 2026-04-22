pub mod escalation;

use chrono::Utc;
use tracing::{debug, info, warn};

use crate::context::{RequiredAction, RiskAction, RiskContext};
use crate::decision::{
    Decision, Factor, HardGate, RiskDecision, ScoreBreakdown, ScoreComponent, ENGINE_VERSION,
    SCHEMA_VERSION,
};
use crate::org_graph::scoring::compute_org_cluster_bias;
use crate::policy::hard_gates::{evaluate_hard_gates, GateOutcome};
use crate::scoring::{
    action_multiplier, capped_add, correlation::score_correlation, device::score_device,
    network::score_network, session::score_session, velocity::score_velocity,
};
use crate::store::{EscalationEntry, SignalStore};

// ─────────────────────────────────────────────────────────────────────────────
// Public entry point
// ─────────────────────────────────────────────────────────────────────────────

/// Primary engine evaluation function.
///
/// Evaluation order:
/// 1. Hard policy gates → immediate DENY if any fire
/// 2. Continuous score computation: R = (D+S+N+B) * M_action * M_org + A
/// 3. Handle degraded signals → force CHALLENGE on sensitive actions
/// 4. Org-graph bias + adaptive threshold adjustment
/// 5. Map effective score to initial Decision
/// 6. Atomic escalation memory check → may promote the decision
/// 7. Compute directives + apply side effects
///
/// This function is `async` only because escalation and side-effect store
/// calls require Redis I/O. All scoring logic is synchronous.
#[tracing::instrument(name = "risk_evaluate", skip(ctx, store), fields(
    request_id = %ctx.request_id,
    user_id = %ctx.user_id,
    tenant_id = %ctx.tenant_id,
    action = %ctx.action.label(),
))]
pub async fn evaluate<S: SignalStore>(ctx: RiskContext, store: &S) -> RiskDecision {
    let evaluated_at = Utc::now();

    // ── Step 1: Hard gates ────────────────────────────────────────────────────
    match evaluate_hard_gates(&ctx) {
        GateOutcome::Blocked(gates) => {
            let decision = build_gate_verdict(&ctx, gates, evaluated_at);
            let d_val = decision.decision.clone();
            let s_val = decision.score;
            if let Err(e) = store
                .record_decision(
                    ctx.user_id,
                    EscalationEntry {
                        score: s_val,
                        decision: d_val.clone(),
                        ts_unix: evaluated_at.timestamp(),
                    },
                )
                .await
            {
                warn!(error = %e, "failed to record gate verdict decision");
            }
            if decision.lock_account {
                if let Err(e) = store.lock_account(ctx.user_id, 3600).await {
                    warn!(error = %e, "failed to lock account after gate block");
                }
            }
            // Mirror the scoring-path side-effect: Register denies — whether
            // from hard gates or scoring — must block the originating IP so
            // sybil retries from the same source are rejected.
            if matches!(d_val, Decision::Deny) && matches!(ctx.action, RiskAction::Register) {
                if let Some(ip) = ctx.request_ip {
                    if let Err(e) = store.block_ip(ip, 3600).await {
                        warn!(error = %e, ip = %ip, "failed to block IP after hard-gate register deny");
                    }
                }
            }
            info!(
                decision = ?d_val,
                gate = true,
                triggered_gates = ?decision.triggered_gates,
                lock_account = decision.lock_account,
                "hard gate triggered - immediate block"
            );
            return decision;
        }
        GateOutcome::Passed(_) => {
            debug!("all hard gates passed");
        }
    }

    // ── Step 2: Compute continuous score ──────────────────────────────────────
    let mut factors: Vec<Factor> = Vec::new();

    let d = score_device(&ctx, &mut factors);
    let s = score_session(&ctx, &mut factors);
    let n = score_network(&ctx, &mut factors);
    let b = score_velocity(&ctx, &mut factors);
    let c = score_correlation(&ctx, &mut factors);

    let m_action = action_multiplier(&ctx.action, &ctx.policy);
    let m_org = ctx.org_risk_level.multiplier();

    // WeightedBase = D + S + N + B + C
    let base_raw = d as f32 + s as f32 + n as f32 + b as f32 + c as f32;

    // Diminishing returns after damping threshold (v3.0 Strategic Scoping)
    let damped_base = compute_damped_base(
        base_raw,
        ctx.policy.score_damping_threshold as f32,
        ctx.policy.score_damping_rate,
    );

    // Penalty = (Multiplier - 1.0) * (DampedBase / 2.0)
    // Shift from linear multiplication to proportional risk scaling
    let m_penalty = (m_action - 1.0) * (damped_base / 2.0);
    
    // Apply Org multiplier to the combined risk foundation
    let scaled_pre_bias = damped_base + m_penalty;
    let scaled = (scaled_pre_bias * m_org).clamp(0.0, 255.0) as u8;

    let a = compute_absolute_adder(&ctx, &mut factors);

    let final_score = scaled.saturating_add(a).min(100);

    let breakdown = ScoreBreakdown {
        d,
        s,
        n,
        b,
        c,
        base: base_raw as u8,
        m_action,
        m_org,
        a,
        final_score,
    };

    debug!(d, s, n, b, base = base_raw as u8, final_score, "score computed");

    // ── Step 3: Handle degraded signals ──────────────────────────────────────
    // If we could not collect signals and this is a sensitive action, we must
    // not silently allow. Upgrade to Challenge before score-based decision.
    // Covers BOTH Redis degradation (velocity counters, session state) and
    // DB degradation (credential integrity, login history, sign-count).
    let signals_degraded = ctx.redis_signals_degraded || ctx.db_signals_degraded;
    if signals_degraded && ctx.action.is_sensitive() {
        let degraded_rule = match (ctx.redis_signals_degraded, ctx.db_signals_degraded) {
            (true, true)  => "both_signals_degraded_on_sensitive_action",
            (true, false) => "redis_signals_degraded_on_sensitive_action",
            (false, true) => "db_signals_degraded_on_sensitive_action",
            (false, false) => unreachable!(),
        };
        warn!(
            action = %ctx.action.label(),
            rule = degraded_rule,
            "degraded signals on sensitive action, forcing challenge"
        );
        let decision = RiskDecision {
            decision: Decision::Challenge,
            score: final_score,
            required_action: Some(RequiredAction::StepUpWebAuthn),
            score_breakdown: breakdown.clone(),
            triggered_gates: vec![],
            triggered_rules: vec![degraded_rule.into()],
            contributing_factors: factors,
            lock_account: false,
            notify_user: false,
            notify_admin: true, // degraded signals warrant ops awareness
            hold_reason: None,
            signals_degraded: true,
            escalated: false,
            // org fields — safe defaults on degraded path
            base_score: final_score,
            adjusted_score: final_score,
            org_risk_score: ctx.org_risk_score,
            cluster_factors: vec![],
            applied_threshold_shift: 0,
            request_id: ctx.request_id,
            evaluated_at,
            engine_version: ENGINE_VERSION,
            schema_version: SCHEMA_VERSION,
        };
        if let Err(e) = store
            .record_decision(
                ctx.user_id,
                EscalationEntry {
                    score: final_score,
                    decision: Decision::Challenge,
                    ts_unix: evaluated_at.timestamp(),
                },
            )
            .await
        {
            warn!(error = %e, "failed to record degraded-signal decision");
        }
        return decision;
    }

    // ── Step 4: Org-graph bias + adaptive threshold adjustment ────────────────
    //
    // ADDITIVE ONLY and FAILS OPEN:
    // - If org data is absent, all biases are 0 and threshold_shift is 0.
    // - The base formula result (final_score) is never reduced by org signals.
    // - Maximum total org/cluster adder: 25 points.
    // - Maximum threshold shift: [-15, +20] bounded.
    let org_cluster = compute_org_cluster_bias(
        ctx.org_risk_score,
        ctx.cluster_membership.as_ref(),
        ctx.threshold_shift,
        ctx.org_active_cluster_count,
    );

    let after_bias = final_score
        .saturating_add(org_cluster.total_bias)
        .min(100);

    let effective_score = (after_bias as i16 + org_cluster.threshold_shift as i16)
        .clamp(0, 100) as u8;

    // ── Step 5: Map effective score to initial decision ────────────────────────
    let (mut decision, mut required_action) = score_to_decision(effective_score, &ctx);

    // ── Step 6: Atomic escalation memory check + record ───────────────────────
    // Uses check_and_record_escalation which atomically reads recent history,
    // checks escalation rules, and records the (possibly promoted) decision.
    // This prevents the TOCTOU race in the old get→check→record pattern.
    let mut escalated = false;
    match store
        .check_and_record_escalation(
            ctx.user_id,
            &decision,
            final_score,
            evaluated_at.timestamp(),
        )
        .await
    {
        Ok((final_decision, was_escalated)) => {
            if was_escalated {
                required_action = escalation_required_action(&final_decision);
                decision = final_decision;
                escalated = true;
                info!(escalated_to = ?decision, "decision escalated by escalation memory");
            }
        }
        Err(e) => {
            // Store error — proceed with current decision (best-effort).
            // The decision was NOT recorded, so the next request may not
            // escalate correctly. This is acceptable for a best-effort system.
            warn!(error = %e, "escalation check failed, proceeding without escalation");
        }
    }

    // ── Step 7: Directives ────────────────────────────────────────────────────
    let lock_account = should_lock_account(&decision, &ctx.action);
    let notify_user = decision.is_blocking();
    let notify_admin = matches!(decision, Decision::Hold | Decision::Deny);
    let hold_reason = if matches!(decision, Decision::Hold) {
        Some(build_hold_reason(&ctx, &factors, escalated))
    } else {
        None
    };

    // Sort factors by contribution descending for readability
    factors.sort_by(|a, b| b.contribution.cmp(&a.contribution));

    let triggered_rules = factors.iter().map(|f| f.name.clone()).collect();

    // ── Step 8: Apply side effects ────────────────────────────────────────────
    if lock_account {
        if let Err(e) = store.lock_account(ctx.user_id, 3600).await {
            warn!(error = %e, "failed to lock account");
        }
    }

    // Block IP on registration DENY to prevent sybil attacks from the same IP.
    if matches!(decision, Decision::Deny) && matches!(ctx.action, RiskAction::Register) {
        if let Some(ip) = ctx.request_ip {
            if let Err(e) = store.block_ip(ip, 3600).await {
                warn!(error = %e, ip = %ip, "failed to block IP after registration deny");
            }
        }
    }

    info!(
        decision = ?decision,
        score = final_score,
        adj_score = effective_score,
        d = breakdown.d,
        s = breakdown.s,
        n = breakdown.n,
        b = breakdown.b,
        c = breakdown.c,
        m_act = breakdown.m_action,
        rules = ?triggered_rules,
        req_action = ?required_action,
        hold_reason = ?hold_reason,
        escalated,
        "risk evaluation complete"
    );

    RiskDecision {
        decision,
        // `score` is always the RAW base score — backward compatible.
        score: final_score,
        required_action,
        score_breakdown: breakdown,
        triggered_gates: vec![],
        triggered_rules,
        contributing_factors: factors,
        lock_account,
        notify_user,
        notify_admin,
        hold_reason,
        signals_degraded: ctx.redis_signals_degraded || ctx.db_signals_degraded,
        escalated,
        // Org-graph fields — always populated for full observability.
        base_score: final_score,
        adjusted_score: effective_score,
        org_risk_score: ctx.org_risk_score,
        cluster_factors: org_cluster.factors,
        applied_threshold_shift: org_cluster.threshold_shift,
        request_id: ctx.request_id,
        evaluated_at,
        engine_version: ENGINE_VERSION,
        schema_version: SCHEMA_VERSION,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Absolute adder — A term in the formula
// ─────────────────────────────────────────────────────────────────────────────

fn compute_absolute_adder(ctx: &RiskContext, factors: &mut Vec<Factor>) -> u8 {
    let mut a: u8 = 0;
    const CAP: u8 = 30;

    // Brand-new credential on sensitive action
    let age_minutes = ctx
        .evaluated_at
        .signed_duration_since(ctx.credential_created_at)
        .num_minutes();
    if age_minutes < 5 && ctx.action.is_sensitive() {
        let eff = capped_add(&mut a, CAP, 20);
        if eff > 0 {
            factors.push(Factor {
                name: "fresh_credential_sensitive_action".into(),
                component: ScoreComponent::Absolute,
                contribution: eff,
                description: format!(
                    "Credential registered {}m ago attempting sensitive action",
                    age_minutes
                ),
            });
        }
    }

    // Active recovery request in flight
    if ctx.recovery_pending {
        let eff = capped_add(&mut a, CAP, 15);
        if eff > 0 {
            factors.push(Factor {
                name: "recovery_pending".into(),
                component: ScoreComponent::Absolute,
                contribution: eff,
                description: "An active recovery request exists for this user".into(),
            });
        }
    }

    // add_admin by low-history user
    if matches!(&ctx.action, RiskAction::ActionExecute { action_name } if action_name == "add_admin")
        && ctx.prior_audit_event_count < 10
    {
        let eff = capped_add(&mut a, CAP, 20);
        if eff > 0 {
            factors.push(Factor {
                name: "add_admin_low_history".into(),
                component: ScoreComponent::Absolute,
                contribution: eff,
                description: format!(
                    "add_admin attempted with only {} prior audit events",
                    ctx.prior_audit_event_count
                ),
            });
        }
    }

    // Token pre-dates fingerprinting on sensitive action
    if ctx.jwt_fingerprint_stored.is_none()
        && ctx.jwt_issued_at.is_some()
        && ctx.action.is_sensitive()
    {
        let eff = capped_add(&mut a, CAP, 5);
        if eff > 0 {
            factors.push(Factor {
                name: "unfingerprinted_jwt_sensitive".into(),
                component: ScoreComponent::Absolute,
                contribution: eff,
                description: "JWT predates fingerprint deployment; cannot verify client binding"
                    .into(),
            });
        }
    }

    // Breached credential (non-sensitive actions — sensitive is hard-gated)
    // Even on non-sensitive actions, a breached credential adds risk because
    // an attacker may be probing with the compromised credential before
    // attempting a sensitive operation.
    if ctx.breached_credential == Some(true) && !ctx.action.is_sensitive() {
        let eff = capped_add(&mut a, CAP, 15);
        if eff > 0 {
            factors.push(Factor {
                name: "breached_credential".into(),
                component: ScoreComponent::Absolute,
                contribution: eff,
                description:
                    "Credential found in breach database (non-sensitive action, not hard-gated)"
                        .into(),
            });
        }
    }

    // Disposable email + account under 7 days + sensitive action = strong sybil signal
    // This is an absolute adder because it combines multiple weak signals into
    // a strong composite indicator that crosses component boundaries.
    if ctx.email_domain_disposable == Some(true)
        && ctx.account_age_days.map_or(false, |d| d < 7)
        && ctx.action.is_sensitive()
    {
        let eff = capped_add(&mut a, CAP, 15);
        if eff > 0 {
            factors.push(Factor {
                name: "disposable_email_young_account_sensitive".into(),
                component: ScoreComponent::Absolute,
                contribution: eff,
                description:
                    "Disposable email domain + account <7 days old + sensitive action (sybil risk)"
                        .into(),
            });
        }
    }

    a
}

// ─────────────────────────────────────────────────────────────────────────────
// Score → Decision mapping
// ─────────────────────────────────────────────────────────────────────────────

/// Piecewise-linear damping of the weighted base score.
///
/// Below `threshold` the mapping is the identity (no damping). At and above
/// `threshold` each additional raw point contributes `rate` (typically < 1)
/// so extreme stacks of weak signals cannot dominate the verdict.
///
/// Pure; exposed for property testing.
pub fn compute_damped_base(base_raw: f32, threshold: f32, rate: f32) -> f32 {
    if base_raw < threshold {
        base_raw
    } else {
        threshold + (base_raw - threshold) * rate
    }
}

pub fn score_to_decision(score: u8, ctx: &RiskContext) -> (Decision, Option<RequiredAction>) {
    let base = if score <= ctx.policy.threshold_allow_max {
        (Decision::Allow, None)
    } else if score <= ctx.policy.threshold_challenge_max {
        (Decision::Challenge, Some(challenge_action_for(score, ctx)))
    } else if score <= ctx.policy.threshold_hold_max {
        (Decision::Hold, Some(RequiredAction::AdminApproval))
    } else {
        (Decision::Deny, None) // Safety fallback for everything else
    };

    // A fingerprint mismatch is a structural integrity failure. Sensitive
    // actions are already hard-gated via `SensitiveActionOnMismatch`; for
    // non-sensitive actions, still upgrade Allow → Challenge so a stolen
    // token can't be used silently for reconnaissance.
    if let (Some(stored), Some(current)) =
        (&ctx.jwt_fingerprint_stored, &ctx.jwt_fingerprint_current)
    {
        if stored != current && matches!(base.0, Decision::Allow) {
            return (Decision::Challenge, Some(RequiredAction::ReverifyDevice));
        }
    }

    base
}

/// Choose the most appropriate challenge type based on score bracket and action.
fn challenge_action_for(score: u8, ctx: &RiskContext) -> RequiredAction {
    // High-bracket challenge (60–64 in v2): re-login, not just step-up
    if score >= 60 {
        return RequiredAction::ReLogin;
    }
    // Credential/session anomalies → reverify device identity
    if ctx.jwt_fingerprint_stored.is_some()
        && ctx.jwt_fingerprint_current != ctx.jwt_fingerprint_stored
    {
        return RequiredAction::ReverifyDevice;
    }
    // Default: standard WebAuthn step-up assertion
    RequiredAction::StepUpWebAuthn
}

fn escalation_required_action(promoted: &Decision) -> Option<RequiredAction> {
    match promoted {
        Decision::Hold => Some(RequiredAction::AdminApproval),
        Decision::Deny => None,
        Decision::Challenge => Some(RequiredAction::StepUpWebAuthn),
        Decision::Allow => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Directive helpers
// ─────────────────────────────────────────────────────────────────────────────

fn should_lock_account(decision: &Decision, action: &RiskAction) -> bool {
    // Lock on DENY for any action where continued retry is itself the attack:
    //   - login-class (brute force, credential stuffing)
    //   - recovery-class (account takeover via recovery)
    //   - device-destructive (attacker pruning victim's other credentials)
    //   - critical ActionExecute names (add_admin, rotate_api_key, delete_resource)
    // Benign ActionExecute names (e.g., view_dashboard) must NOT lock, so a
    // deliberately-deniable request cannot be used as a DoS on the victim.
    if *decision != Decision::Deny {
        return false;
    }
    match action {
        RiskAction::Login
        | RiskAction::OauthComplete
        | RiskAction::Register
        | RiskAction::RecoveryStart
        | RiskAction::RecoveryApprove
        | RiskAction::RecoveryComplete
        | RiskAction::DeviceRevoke
        | RiskAction::DeviceMarkLost => true,
        RiskAction::ActionExecute { action_name } => matches!(
            action_name.as_str(),
            "add_admin" | "rotate_api_key" | "delete_resource"
        ),
        _ => false,
    }
}

fn build_hold_reason(ctx: &RiskContext, factors: &[Factor], escalated: bool) -> String {
    let top_factors: Vec<&str> = factors
        .iter()
        .take(3)
        .map(|f| f.name.as_str())
        .collect();

    if escalated {
        format!(
            "Escalated from repeated challenges. Top signals: {}. Action: {}.",
            top_factors.join(", "),
            ctx.action.label()
        )
    } else {
        format!(
            "Score threshold exceeded. Top signals: {}. Action: {}.",
            top_factors.join(", "),
            ctx.action.label()
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Hard gate deny builder
// ─────────────────────────────────────────────────────────────────────────────

fn build_gate_verdict(
    ctx: &RiskContext,
    gates: Vec<HardGate>,
    evaluated_at: chrono::DateTime<Utc>,
) -> RiskDecision {
    let gate_names: Vec<String> = gates.iter().map(|g| format!("{:?}", g)).collect();

    // Gates are classified into Deny-class and Hold-class. Deny always wins:
    // a Hold-class gate must never downgrade a Deny triggered in the same
    // request. The Hold-class overrides apply only when NO Deny-class gate
    // fired.
    let hold_only_gates: [HardGate; 2] = [
        HardGate::TorOnRecovery,
        HardGate::PrivilegedActionOnFreshCredential,
    ];
    let has_deny_gate = gates.iter().any(|g| !hold_only_gates.contains(g));

    let (decision, score, req_action, hold_reason) = if has_deny_gate {
        (Decision::Deny, 100u8, None, None)
    } else if gates.contains(&HardGate::PrivilegedActionOnFreshCredential) {
        (
            Decision::Hold,
            75u8,
            Some(RequiredAction::AdminApproval),
            Some(
                "Privileged action attempted on extremely fresh credential (trust spike protection)"
                    .to_string(),
            ),
        )
    } else if gates.contains(&HardGate::TorOnRecovery) {
        (
            Decision::Hold,
            80u8,
            Some(RequiredAction::AdminApproval),
            Some("Tor detected during account recovery (zero-tolerance policy)".to_string()),
        )
    } else {
        // Unreachable: gates non-empty implies deny or one of the hold variants.
        (Decision::Deny, 100u8, None, None)
    };

    // Lock semantics: mirror `should_lock_account` so hard-gate and score paths
    // agree. Revoked and Lost credentials are treated identically — both mean
    // the credential must not be usable for authentication.
    let compromised_credential = gates.contains(&HardGate::RevokedCredential)
        || gates.contains(&HardGate::LostCredential);
    let lock_account = gates.contains(&HardGate::AccountLocked)
        || (compromised_credential
            && matches!(&ctx.action, RiskAction::Login | RiskAction::OauthComplete))
        || should_lock_account(&decision, &ctx.action);

    RiskDecision {
        decision,
        score,
        required_action: req_action,
        score_breakdown: ScoreBreakdown::zeroed_max(),
        triggered_gates: gates,
        triggered_rules: gate_names,
        contributing_factors: vec![],
        lock_account,
        notify_user: true,
        notify_admin: true,
        hold_reason,
        signals_degraded: ctx.redis_signals_degraded || ctx.db_signals_degraded,
        escalated: false,
        base_score: score,
        adjusted_score: score,
        org_risk_score: ctx.org_risk_score,
        cluster_factors: vec![],
        applied_threshold_shift: 0,
        request_id: ctx.request_id,
        evaluated_at,
        engine_version: ENGINE_VERSION,
        schema_version: SCHEMA_VERSION,
    }
}
