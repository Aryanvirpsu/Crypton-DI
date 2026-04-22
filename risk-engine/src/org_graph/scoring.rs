use crate::org_graph::cluster::{AttackCluster, AttackType, ClusterFactor, ClusterMembership};
use crate::org_graph::tenant::TenantRiskProfile;
use crate::config::MAX_ORG_CLUSTER_BIAS;

/// Sentinel cluster_id used in the org-level ClusterFactor.
/// Not a real cluster — represents the aggregate tenant risk signal.
pub const ORG_LEVEL_CLUSTER_ID: &str = "org_level";

// ─────────────────────────────────────────────────────────────────────────────
// Org-level risk score (0–100)
// ─────────────────────────────────────────────────────────────────────────────

/// Compute the aggregate org risk score from all active clusters and the
/// tenant risk profile.
///
/// Formula:
///   org_score = (cluster_severity * 0.5)
///             + (cluster_density   * 0.25)
///             + (attack_pressure   * 0.25)
///   Scaled to 0–100 and clamped.
///
/// This is a pure function — deterministic given the same inputs.
/// Called only by the background recompute task.
pub fn compute_org_risk_score(clusters: &[AttackCluster], profile: &TenantRiskProfile) -> u8 {
    if clusters.is_empty() {
        // Stable state: small non-zero score if attack_pressure is nonzero
        let base = (profile.attack_pressure * 10.0) as u8;
        return base.min(10);
    }

    // ── Weighted severity across all clusters ────────────────────────────────
    // High-severity clusters contribute more. Uses severity multiplier from
    // AttackType.severity().
    let total_weight: f32 = clusters.iter()
        .map(|c| c.attack_type.severity() * c.confidence)
        .sum();
    let total_possible = (clusters.len() as f32) * 1.5; // max severity is 1.5
    let severity_factor = (total_weight / total_possible.max(1.0)).min(1.0);

    // ── Average cluster risk (normalised) ────────────────────────────────────
    let avg_cluster_risk: f32 = clusters.iter()
        .map(|c| c.risk_score as f32)
        .sum::<f32>() / clusters.len() as f32;
    let normalised_risk = avg_cluster_risk / 100.0;

    // ── Combine components ───────────────────────────────────────────────────
    let raw = normalised_risk * 0.5
        + severity_factor * 0.25
        + profile.cluster_density * 0.15
        + profile.attack_pressure * 0.10;

    (raw * 100.0).clamp(0.0, 100.0) as u8
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-user org bias (0–10 additive points)
// ─────────────────────────────────────────────────────────────────────────────

/// Compute additive score bias from the org-level risk score.
///
/// This is intentionally small — org risk is a background signal.
/// It nudges a borderline decision but cannot by itself push a clean request
/// from Allow to Deny.
///
/// Scale:
///   org_score  0– 30 → bias 0
///   org_score 31– 60 → bias 1–5
///   org_score 61– 80 → bias 5–8
///   org_score 81–100 → bias 8–10
pub fn compute_org_bias(org_risk_score: u8) -> u8 {
    match org_risk_score {
        0..=30  => 0,
        31..=60 => {
            // Linear: 0 at 30, 5 at 60
            ((org_risk_score - 30) as f32 / 6.0) as u8
        }
        61..=80 => {
            // 5 at 61, 8 at 80
            5 + ((org_risk_score - 60) as f32 / 7.0) as u8
        }
        _ => {
            // 8 at 81, 10 at 100
            (8 + ((org_risk_score.saturating_sub(80)) as f32 / 10.0) as u8).min(10)
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-user cluster bias (0–15 additive points)
// ─────────────────────────────────────────────────────────────────────────────

/// Compute additive score bias when the requesting user is a member of a
/// detected attack cluster.
///
/// Formula: `bias = (cluster_risk * confidence * severity_multiplier) / scale`
///
/// Capped at 15 to prevent cluster membership from being the sole cause of
/// a DENY. The request's own signals must also contribute.
pub fn compute_cluster_bias(membership: &ClusterMembership) -> u8 {
    let severity = membership.attack_type.severity();
    // Scale chosen so that maximum (risk=100, conf=1.0, severity=1.5) → 15
    let raw = (membership.cluster_risk_score as f32
        * membership.confidence
        * severity
        / 10.0) as u8;
    raw.min(15)
}

/// Build the `ClusterFactor` entry for `RiskDecision::cluster_factors`.
pub fn build_cluster_factor(membership: &ClusterMembership, bias: u8) -> ClusterFactor {
    ClusterFactor {
        cluster_id: membership.cluster_id.clone(),
        attack_type: membership.attack_type.clone(),
        contribution: bias,
        description: format!(
            "User is a member of {} cluster '{}' (risk={}, confidence={:.0}%, members={})",
            membership.attack_type.label(),
            &membership.cluster_id[..membership.cluster_id.len().min(16)],
            membership.cluster_risk_score,
            membership.confidence * 100.0,
            membership.member_count,
        ),
    }
}

/// Build an org-level `ClusterFactor` (when there's no direct membership
/// but the tenant is under elevated org risk).
pub fn build_org_factor(org_risk_score: u8, bias: u8, active_clusters: u32) -> ClusterFactor {
    ClusterFactor {
        cluster_id: ORG_LEVEL_CLUSTER_ID.to_string(),
        attack_type: AttackType::UnknownAnomaly,
        contribution: bias,
        description: format!(
            "Tenant under elevated risk (org_score={}, active_clusters={})",
            org_risk_score, active_clusters,
        ),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Aggregate: compute all org/cluster adjustments in one call
// ─────────────────────────────────────────────────────────────────────────────

/// Result of org/cluster bias computation. Consumed by `engine::evaluate()`.
#[derive(Debug, Default)]
pub struct OrgClusterBias {
    /// Points to add from org-level risk (0–10).
    pub org_bias: u8,
    /// Points to add from direct cluster membership (0–15).
    pub cluster_bias: u8,
    /// Total additive bias (org_bias + cluster_bias, max 25).
    pub total_bias: u8,
    /// Threshold shift from TenantRiskProfile (bounded to [-15, +20]).
    pub threshold_shift: i8,
    /// Factor records for RiskDecision::cluster_factors.
    pub factors: Vec<ClusterFactor>,
}

/// Compute org and cluster biases from context fields.
///
/// This is a pure synchronous function — all inputs come from `RiskContext`.
/// Returns a neutral `OrgClusterBias` (all zeros) if org data is unavailable,
/// implementing the fail-open guarantee.
pub fn compute_org_cluster_bias(
    org_risk_score: Option<u8>,
    cluster_membership: Option<&ClusterMembership>,
    threshold_shift: Option<i8>,
    active_cluster_count: u32,
) -> OrgClusterBias {
    let mut factors = Vec::new();

    // ── Org bias ─────────────────────────────────────────────────────────────
    let org_bias = match org_risk_score {
        Some(score) => {
            let bias = compute_org_bias(score);
            if bias > 0 {
                factors.push(build_org_factor(score, bias, active_cluster_count));
            }
            bias
        }
        None => 0, // fail open
    };

    // ── Cluster bias ──────────────────────────────────────────────────────────
    let cluster_bias = match cluster_membership {
        Some(m) => {
            let bias = compute_cluster_bias(m);
            if bias > 0 {
                factors.push(build_cluster_factor(m, bias));
            }
            bias
        }
        None => 0, // no cluster membership = no bias
    };

    let total_bias = org_bias.saturating_add(cluster_bias).min(MAX_ORG_CLUSTER_BIAS);
    let effective_shift = threshold_shift.unwrap_or(0);

    OrgClusterBias {
        org_bias,
        cluster_bias,
        total_bias,
        threshold_shift: effective_shift,
        factors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::org_graph::cluster::{AttackType, ClusterMembership};
    use crate::org_graph::tenant::TenantRiskProfile;

    fn profile(pressure: f32, density: f32) -> TenantRiskProfile {
        TenantRiskProfile {
            tenant_id: "t".into(),
            attack_pressure: pressure,
            stability_score: 1.0 - pressure,
            cluster_density: density,
            active_cluster_count: 1,
            threshold_shift: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn low_org_risk_produces_no_bias() {
        assert_eq!(compute_org_bias(0), 0);
        assert_eq!(compute_org_bias(30), 0);
    }

    #[test]
    fn max_org_risk_capped_at_10() {
        assert!(compute_org_bias(100) <= 10);
    }

    #[test]
    fn cluster_bias_capped_at_15() {
        let m = ClusterMembership {
            cluster_id: "c".into(),
            attack_type: AttackType::SessionHijackCluster,
            cluster_risk_score: 100,
            member_count: 50,
            confidence: 1.0,
        };
        assert!(compute_cluster_bias(&m) <= 15);
    }

    #[test]
    fn total_bias_capped_at_25() {
        let m = ClusterMembership {
            cluster_id: "c".into(),
            attack_type: AttackType::RecoveryAbuse,
            cluster_risk_score: 100,
            member_count: 50,
            confidence: 1.0,
        };
        let result = compute_org_cluster_bias(Some(100), Some(&m), Some(20), 10);
        assert!(result.total_bias <= 25, "total bias exceeded 25: {}", result.total_bias);
    }

    #[test]
    fn fail_open_on_missing_org_data() {
        let result = compute_org_cluster_bias(None, None, None, 0);
        assert_eq!(result.org_bias, 0);
        assert_eq!(result.cluster_bias, 0);
        assert_eq!(result.threshold_shift, 0);
        assert_eq!(result.total_bias, 0);
    }

    #[test]
    fn org_risk_score_zero_for_no_clusters() {
        let p = profile(0.0, 0.0);
        let score = compute_org_risk_score(&[], &p);
        assert_eq!(score, 0);
    }

    #[test]
    fn org_risk_score_increases_with_severity() {
        let p = profile(0.3, 0.1);
        let low_clusters = vec![crate::org_graph::cluster::AttackCluster {
            cluster_id: "c1".into(), tenant_id: "t".into(),
            risk_score: 40, nodes: vec!["user:a".into()],
            attack_type: AttackType::UnknownAnomaly, confidence: 0.5,
            first_seen: 0, last_updated: 0,
        }];
        let high_clusters = vec![crate::org_graph::cluster::AttackCluster {
            cluster_id: "c2".into(), tenant_id: "t".into(),
            risk_score: 85, nodes: vec!["user:b".into()],
            attack_type: AttackType::SessionHijackCluster, confidence: 0.9,
            first_seen: 0, last_updated: 0,
        }];
        let low_score = compute_org_risk_score(&low_clusters, &p);
        let high_score = compute_org_risk_score(&high_clusters, &p);
        assert!(high_score > low_score,
            "high severity cluster should produce higher org score: {} > {}",
            high_score, low_score
        );
    }
}
