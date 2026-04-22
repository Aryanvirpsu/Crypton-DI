use chrono::Utc;

use crate::decision::Decision;
use crate::store::EscalationEntry;

/// Outcome of escalation analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EscalationOutcome {
    /// No escalation warranted.
    NoChange,
    /// Decision should be promoted to `Hold`.
    EscalateTo(Decision),
}

/// Examine the last N decisions recorded for this user and determine
/// whether the current decision should be escalated.
///
/// Rules:
/// 1. ≥3 `Challenge` decisions within the last 5 minutes → escalate to `Hold`
/// 2. ≥2 `Hold` decisions within the last 5 minutes → escalate to `Deny`
///
/// These thresholds encode the observation that repeated friction indicates
/// either a persistent attacker (who keeps completing challenges) or a
/// genuine account takeover that step-up auth is not stopping.
///
/// `entries` must be ordered **newest first** (as returned from Redis LRANGE).
pub fn check_escalation(
    current_decision: &Decision,
    entries: &[EscalationEntry],
) -> EscalationOutcome {
    let now = Utc::now().timestamp();
    let window_secs: i64 = 300; // 5 minutes

    // Only entries within the time window count.
    let recent: Vec<&EscalationEntry> = entries
        .iter()
        .filter(|e| now - e.ts_unix <= window_secs)
        .collect();

    // Count by decision type in recent window
    let recent_challenges = recent
        .iter()
        .filter(|e| e.decision == Decision::Challenge)
        .count();
    let recent_holds = recent
        .iter()
        .filter(|e| e.decision == Decision::Hold)
        .count();

    // Rule 2: repeated holds → deny (higher priority, check first)
    if recent_holds >= 2
        && matches!(current_decision, Decision::Challenge | Decision::Hold)
    {
        return EscalationOutcome::EscalateTo(Decision::Deny);
    }

    // Rule 1: repeated challenges → hold
    if recent_challenges >= 3 && *current_decision == Decision::Challenge {
        return EscalationOutcome::EscalateTo(Decision::Hold);
    }

    EscalationOutcome::NoChange
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::Decision;
    use crate::store::EscalationEntry;
    use chrono::Utc;

    fn entry(decision: Decision, seconds_ago: i64) -> EscalationEntry {
        EscalationEntry {
            score: 45,
            decision,
            ts_unix: Utc::now().timestamp() - seconds_ago,
        }
    }

    #[test]
    fn three_recent_challenges_escalates_to_hold() {
        let entries = vec![
            entry(Decision::Challenge, 30),
            entry(Decision::Challenge, 60),
            entry(Decision::Challenge, 90),
        ];
        let outcome = check_escalation(&Decision::Challenge, &entries);
        assert_eq!(outcome, EscalationOutcome::EscalateTo(Decision::Hold));
    }

    #[test]
    fn two_recent_holds_escalates_to_deny() {
        let entries = vec![
            entry(Decision::Hold, 30),
            entry(Decision::Hold, 60),
        ];
        let outcome = check_escalation(&Decision::Challenge, &entries);
        assert_eq!(outcome, EscalationOutcome::EscalateTo(Decision::Deny));
    }

    #[test]
    fn old_challenges_outside_window_do_not_escalate() {
        let entries = vec![
            entry(Decision::Challenge, 400), // >300s ago, outside window
            entry(Decision::Challenge, 500),
            entry(Decision::Challenge, 600),
        ];
        let outcome = check_escalation(&Decision::Challenge, &entries);
        assert_eq!(outcome, EscalationOutcome::NoChange);
    }

    #[test]
    fn allow_decision_never_escalated() {
        let entries = vec![
            entry(Decision::Challenge, 10),
            entry(Decision::Challenge, 20),
            entry(Decision::Challenge, 30),
        ];
        // Even with 3 recent challenges, if the *current* decision is Allow,
        // no escalation applies.
        let outcome = check_escalation(&Decision::Allow, &entries);
        assert_eq!(outcome, EscalationOutcome::NoChange);
    }

    #[test]
    fn deny_decision_never_further_escalated() {
        let entries = vec![
            entry(Decision::Hold, 10),
            entry(Decision::Hold, 20),
        ];
        let outcome = check_escalation(&Decision::Deny, &entries);
        assert_eq!(outcome, EscalationOutcome::NoChange);
    }
}
