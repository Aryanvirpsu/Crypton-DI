use crate::context::RiskContext;
use crate::decision::{Factor, ScoreComponent};
use crate::scoring::{capped_add, factor};


/// B component — behavioural velocity score.
/// Cap: 20. Derived entirely from Redis counters (no DB reads).
///
/// Uses overlapping time windows (1m, 5m, 1h, 24h) to catch both burst and
/// sustained attacks. The tightest-window match takes priority; wider windows
/// are checked only if narrower ones didn't fire.
pub fn score_velocity(ctx: &RiskContext, factors: &mut Vec<Factor>) -> u8 {
    let mut b: u8 = 0;
    let cap = ctx.policy.cap_behavioral;

    // ── Login attempt velocity (multi-window) ────────────────────────────────
    // Check tightest window first (1m), fall through to wider windows.
    let mut login_scored = false;

    if let Some(attempts) = ctx.login_attempts_1m {
        if attempts > 3 {
            let eff = capped_add(&mut b, cap, ctx.policy.b_login_burst);
            if eff > 0 {
                factors.push(factor(
                    "login_velocity_burst",
                    ScoreComponent::Behavioral,
                    eff,
                    format!("{} login attempts in last 1 minute (>3 threshold)", attempts),
                ));
            }
            login_scored = true;
        }
    }

    if !login_scored {
        if let Some(attempts) = ctx.login_attempts_5m {
            if attempts > 10 {
                let eff = capped_add(&mut b, cap, ctx.policy.b_login_burst);
                if eff > 0 {
                    factors.push(factor(
                        "login_velocity_critical",
                        ScoreComponent::Behavioral,
                        eff,
                        format!("{} login attempts in last 5 minutes (>10 threshold)", attempts),
                    ));
                }
                login_scored = true;
            } else if attempts > 5 {
                let eff = capped_add(&mut b, cap, ctx.policy.b_login_elevated);
                if eff > 0 {
                    factors.push(factor(
                        "login_velocity_elevated",
                        ScoreComponent::Behavioral,
                        eff,
                        format!("{} login attempts in last 5 minutes (>5 threshold)", attempts),
                    ));
                }
                login_scored = true;
            } else if attempts > 3 {
                let eff = capped_add(&mut b, cap, ctx.policy.b_login_moderate);
                if eff > 0 {
                    factors.push(factor(
                        "login_velocity_moderate",
                        ScoreComponent::Behavioral,
                        eff,
                        format!("{} login attempts in last 5 minutes (>3 threshold)", attempts),
                    ));
                }
                login_scored = true;
            }
        }
    }

    // Wider windows catch slow-and-low attacks (e.g., 3 attempts every 6 minutes).
    if !login_scored {
        if let Some(attempts) = ctx.login_attempts_1h {
            if attempts > 20 {
                let eff = capped_add(&mut b, cap, ctx.policy.b_login_hourly);
                if eff > 0 {
                    factors.push(factor(
                        "login_velocity_hourly",
                        ScoreComponent::Behavioral,
                        eff,
                        format!("{} login attempts in last 1 hour (>20 threshold)", attempts),
                    ));
                }
                login_scored = true;
            }
        }
    }

    if !login_scored {
        if let Some(attempts) = ctx.login_attempts_24h {
            if attempts > 50 {
                let eff = capped_add(&mut b, cap, ctx.policy.b_login_daily);
                if eff > 0 {
                    factors.push(factor(
                        "login_velocity_daily",
                        ScoreComponent::Behavioral,
                        eff,
                        format!("{} login attempts in last 24 hours (>50 threshold)", attempts),
                    ));
                }
            }
        }
    }

    // ── Failed attempt history ────────────────────────────────────────────────
    // Counts failures even when the attacker eventually succeeded.
    if let Some(failed) = ctx.failed_login_attempts_1h {
        if failed > 5 {
            let eff = capped_add(&mut b, cap, ctx.policy.b_failed_high);
            if eff > 0 {
                factors.push(factor(
                    "failed_attempts_high",
                    ScoreComponent::Behavioral,
                    eff,
                    format!("{} failed logins in last 1 hour (>5 threshold)", failed),
                ));
            }
        } else if failed > 3 {
            let eff = capped_add(&mut b, cap, ctx.policy.b_failed_elevated);
            if eff > 0 {
                factors.push(factor(
                    "failed_attempts_elevated",
                    ScoreComponent::Behavioral,
                    eff,
                    format!("{} failed logins in last 1 hour (>3 threshold)", failed),
                ));
            }
        }
    }

    // ── Recovery request frequency ────────────────────────────────────────────
    if let Some(recoveries) = ctx.recovery_requests_24h {
        if recoveries > 1 {
            let eff = capped_add(&mut b, cap, ctx.policy.b_recovery_velocity);
            if eff > 0 {
                factors.push(factor(
                    "recovery_velocity",
                    ScoreComponent::Behavioral,
                    eff,
                    format!(
                        "{} recovery requests in last 24 hours (>1 threshold)",
                        recoveries
                    ),
                ));
            }
        }
    }

    // ── Action execution frequency ────────────────────────────────────────────
    if let Some(actions) = ctx.actions_executed_5m {
        if actions > 3 {
            let eff = capped_add(&mut b, cap, ctx.policy.b_action_velocity);
            if eff > 0 {
                factors.push(factor(
                    "action_velocity",
                    ScoreComponent::Behavioral,
                    eff,
                    format!("{} protected actions in last 5 minutes (>3 threshold)", actions),
                ));
            }
        }
    }

    // ── Device revocation spree ───────────────────────────────────────────────
    if let Some(revocations) = ctx.device_revocations_1h {
        if revocations > 2 {
            let eff = capped_add(&mut b, cap, ctx.policy.b_revocation_spree);
            if eff > 0 {
                factors.push(factor(
                    "revocation_spree",
                    ScoreComponent::Behavioral,
                    eff,
                    format!(
                        "{} device revocations in last 1 hour (>2 threshold)",
                        revocations
                    ),
                ));
            }
        } else if revocations > 1 {
            let eff = capped_add(&mut b, cap, ctx.policy.b_revocation_elevated);
            if eff > 0 {
                factors.push(factor(
                    "revocation_elevated",
                    ScoreComponent::Behavioral,
                    eff,
                    format!(
                        "{} device revocations in last 1 hour (>1 threshold)",
                        revocations
                    ),
                ));
            }
        }
    }

    // ── Sybil registration from same IP ──────────────────────────────────────
    if let Some(reg_count) = ctx.registrations_from_ip_10m {
        if reg_count > 3 {
            let eff = capped_add(&mut b, cap, ctx.policy.b_sybil_registration);
            if eff > 0 {
                factors.push(factor(
                    "sybil_registration",
                    ScoreComponent::Behavioral,
                    eff,
                    format!(
                        "{} registrations from this IP in last 10 minutes (>3 threshold)",
                        reg_count
                    ),
                ));
            }
        }
    }

    // ── New account with no history ───────────────────────────────────────────
    if ctx.prior_audit_event_count == 0 {
        let eff = capped_add(&mut b, cap, ctx.policy.b_zero_audit_history);
        if eff > 0 {
            factors.push(factor(
                "zero_audit_history",
                ScoreComponent::Behavioral,
                eff,
                "No prior audit events on record for this user",
            ));
        }
    }

    // ── Disposable email domain ──────────────────────────────────────────────
    // Accounts registered with disposable/temporary email providers are a
    // strong sybil indicator. The adapter checks against a list of known
    // disposable domains (guerrillamail, tempmail, mailinator, etc.).
    if ctx.email_domain_disposable == Some(true) {
        let eff = capped_add(&mut b, cap, ctx.policy.b_disposable_email);
        if eff > 0 {
            factors.push(factor(
                "disposable_email",
                ScoreComponent::Behavioral,
                eff,
                "Email domain is a known disposable/temporary email provider",
            ));
        }
    }

    // ── Young account on sensitive action ────────────────────────────────────
    // Accounts younger than 24 hours performing sensitive actions are high risk.
    // Account age is separate from credential age — an account can be old but
    // have a new credential (re-registration), or new with an old credential
    // (imported from another service).
    if let Some(age_days) = ctx.account_age_days {
        if age_days == 0 && ctx.action.is_sensitive() {
            let eff = capped_add(&mut b, cap, ctx.policy.b_young_account_critical);
            if eff > 0 {
                factors.push(factor(
                    "young_account_sensitive",
                    ScoreComponent::Behavioral,
                    eff,
                    "Account created today is performing a sensitive action",
                ));
            }
        } else if age_days <= 3 && ctx.action.is_sensitive() {
            let eff = capped_add(&mut b, cap, ctx.policy.b_young_account_new);
            if eff > 0 {
                factors.push(factor(
                    "new_account_sensitive",
                    ScoreComponent::Behavioral,
                    eff,
                    format!(
                        "Account is only {} days old and performing a sensitive action",
                        age_days
                    ),
                ));
            }
        }
    }

    b
}
