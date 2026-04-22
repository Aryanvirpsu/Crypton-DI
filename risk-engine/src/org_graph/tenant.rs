use serde::{Deserialize, Serialize};

use crate::org_graph::cluster::AttackCluster;

// ─────────────────────────────────────────────────────────────────────────────
// Thresholds and constants for tenant risk scoring
// ─────────────────────────────────────────────────────────────────────────────

/// Attack pressure threshold at which the tenant is considered under severe attack.
const SEVERE_ATTACK_PRESSURE: f32 = 0.7;
/// Attack pressure threshold for moderate attack.
const MODERATE_ATTACK_PRESSURE: f32 = 0.5;
/// Attack pressure threshold for minor attack.
const MINOR_ATTACK_PRESSURE: f32 = 0.3;
/// Cluster density threshold for additional shift.
const CLUSTER_DENSITY_THRESHOLD: f32 = 0.15;
/// Stability score threshold for considering the tenant stable.
const STABILITY_THRESHOLD: f32 = 0.9;
/// Low attack pressure threshold for relaxation.
const LOW_ATTACK_PRESSURE: f32 = 0.1;
/// Attack pressure threshold above which the tenant is under active attack.
const ACTIVE_ATTACK_PRESSURE: f32 = 0.6;
/// Number of active clusters above which the tenant is considered under attack.
const ACTIVE_ATTACK_CLUSTER_COUNT: u32 = 5;

// ─────────────────────────────────────────────────────────────────────────────
// TenantRiskProfile — per-tenant adaptive state
// ─────────────────────────────────────────────────────────────────────────────

/// Per-tenant risk posture derived from cluster intelligence.
///
/// The `threshold_shift` field drives adaptive thresholds in the engine:
/// - Positive shift → effective score increases → stricter decisions
/// - Negative shift → effective score decreases → more lenient (fewer false positives)
///
/// Stored at `org:risk:{tenant_id}` in Redis. Recomputed by the background task.
/// Never computed per-request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantRiskProfile {
    pub tenant_id: String,

    /// Fraction of requests in the last hour exhibiting attack signals.
    /// Derived from: blocked_requests / total_requests. Range: 0.0–1.0.
    pub attack_pressure: f32,

    /// Inverse of decision volatility. High stability = few DENY/HOLD swings.
    /// Range: 0.0–1.0.
    pub stability_score: f32,

    /// Fraction of tracked users currently in an active attack cluster.
    /// Range: 0.0–1.0.
    pub cluster_density: f32,

    /// Active attack cluster count (non-zero confidence clusters).
    pub active_cluster_count: u32,

    /// Computed threshold shift. Applied in engine.rs to effective_score.
    /// Bounded: [-15, +20].
    pub threshold_shift: i8,

    /// Unix timestamp of the last profile recomputation.
    pub updated_at: i64,
}

impl TenantRiskProfile {
    /// Construct a neutral profile for a tenant with no history.
    pub fn neutral(tenant_id: &str, now: i64) -> Self {
        TenantRiskProfile {
            tenant_id: tenant_id.to_string(),
            attack_pressure: 0.0,
            stability_score: 1.0,
            cluster_density: 0.0,
            active_cluster_count: 0,
            threshold_shift: 0,
            updated_at: now,
        }
    }

    /// Compute a fresh profile from cluster data and request counters.
    ///
    /// This is called by the background recompute task, not per-request.
    ///
    /// Arguments:
    /// - `clusters`: all active clusters for this tenant
    /// - `total_users`: approximate user count (for cluster_density denominator)
    /// - `blocked_requests_1h`: DENY + HOLD decisions in last hour
    /// - `total_requests_1h`: all decisions in last hour
    /// - `prev_shift`: previous threshold_shift (provides inertia)
    /// - `now`: Unix timestamp
    pub fn compute(
        tenant_id: &str,
        clusters: &[AttackCluster],
        total_users: u32,
        blocked_requests_1h: u32,
        total_requests_1h: u32,
        prev_shift: i8,
        now: i64,
    ) -> Self {
        // ── Attack pressure ──────────────────────────────────────────────────
        let attack_pressure = if total_requests_1h == 0 {
            0.0
        } else {
            (blocked_requests_1h as f32 / total_requests_1h as f32).min(1.0)
        };

        // ── Cluster density ──────────────────────────────────────────────────
        // Count distinct user nodes in all clusters
        let clustered_users: std::collections::HashSet<&str> = clusters
            .iter()
            .flat_map(|c| c.nodes.iter())
            .filter(|n| n.starts_with("user:"))
            .map(|n| n.as_str())
            .collect();
        let cluster_density = if total_users == 0 {
            0.0
        } else {
            (clustered_users.len() as f32 / total_users as f32).min(1.0)
        };

        // ── Stability score (inverse of volatility) ──────────────────────────
        // Higher attack_pressure and cluster_density = lower stability
        let instability = (attack_pressure * 0.6 + cluster_density * 0.4).min(1.0);
        let stability_score = (1.0 - instability).max(0.0);

        // ── Threshold shift ──────────────────────────────────────────────────
        let target_shift = compute_target_shift(attack_pressure, cluster_density, stability_score);

        // Apply inertia: shift moves at most 3 points per recomputation cycle.
        // This prevents sudden lockouts from transient spikes.
        let delta = (target_shift as i16 - prev_shift as i16).clamp(-3, 3);
        let threshold_shift = (prev_shift as i16 + delta).clamp(-15, 20) as i8;

        TenantRiskProfile {
            tenant_id: tenant_id.to_string(),
            attack_pressure,
            stability_score,
            cluster_density,
            active_cluster_count: clusters.len() as u32,
            threshold_shift,
            updated_at: now,
        }
    }

    /// True if the tenant is currently considered to be under active attack.
    pub fn is_under_attack(&self) -> bool {
        self.attack_pressure > ACTIVE_ATTACK_PRESSURE || self.active_cluster_count >= ACTIVE_ATTACK_CLUSTER_COUNT
    }

    /// True if the tenant is stable enough to allow slight threshold relaxation.
    pub fn is_stable(&self) -> bool {
        self.stability_score > STABILITY_THRESHOLD && self.active_cluster_count == 0
    }
}

/// Compute the target threshold shift from pressure, density, and stability.
///
/// Rules (applied in order, highest pressure wins):
/// - attack_pressure > 0.7 → +15 (severe attack)
/// - attack_pressure > 0.5 → +10
/// - attack_pressure > 0.3 → +6
/// - cluster_density > 0.15 → +4 additional
/// - stability > 0.9 AND attack_pressure < 0.1 → -5 (proven stable tenant)
/// - Otherwise → 0
fn compute_target_shift(
    attack_pressure: f32,
    cluster_density: f32,
    stability_score: f32,
) -> i8 {
    let pressure_shift: i8 = if attack_pressure > SEVERE_ATTACK_PRESSURE {
        15 // Severe attack: lock down threshold
    } else if attack_pressure > MODERATE_ATTACK_PRESSURE {
        10 // Moderate attack: elevate threshold
    } else if attack_pressure > MINOR_ATTACK_PRESSURE {
        6 // Minor attack: slight elevation
    } else if stability_score > STABILITY_THRESHOLD && attack_pressure < LOW_ATTACK_PRESSURE {
        -5 // Long-term stable tenant: relax slightly to reduce friction
    } else {
        0
    };

    let density_extra: i8 = if cluster_density > CLUSTER_DENSITY_THRESHOLD { 4 } else { 0 };

    (pressure_shift as i16 + density_extra as i16).clamp(-15, 20) as i8
}

// ─────────────────────────────────────────────────────────────────────────────
// OrgRiskSnapshot — read by adapter, injected into RiskContext
// ─────────────────────────────────────────────────────────────────────────────

/// Lightweight snapshot of org-level risk, pre-fetched from Redis.
/// Stored at `org:risk:{tenant_id}`. TTL: 5 minutes (refreshed by background task).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgRiskSnapshot {
    pub tenant_id: String,
    /// Aggregate org risk score (0–100). See `scoring::compute_org_risk_score`.
    pub org_risk_score: u8,
    /// Pre-computed threshold shift from TenantRiskProfile.
    pub threshold_shift: i8,
    /// How many active clusters exist.
    pub active_cluster_count: u32,
    /// Distribution: [low(0–25), medium(26–50), high(51–75), critical(76–100)]
    pub cluster_severity_distribution: [u32; 4],
    /// Unix timestamp of the last background recomputation.
    pub computed_at: i64,
}

impl OrgRiskSnapshot {
    /// A safe default snapshot when Redis is unavailable.
    /// The engine fails open: no org bias, no threshold shift.
    pub fn unavailable(tenant_id: &str) -> Self {
        OrgRiskSnapshot {
            tenant_id: tenant_id.to_string(),
            org_risk_score: 0,
            threshold_shift: 0,
            active_cluster_count: 0,
            cluster_severity_distribution: [0, 0, 0, 0],
            computed_at: 0,
        }
    }

    /// Compute a snapshot from clusters and the tenant profile.
    pub fn from_clusters(
        tenant_id: &str,
        clusters: &[AttackCluster],
        profile: &TenantRiskProfile,
        now: i64,
    ) -> Self {
        let org_risk_score = crate::org_graph::scoring::compute_org_risk_score(clusters, profile);

        let mut dist = [0u32; 4];
        for c in clusters {
            let bucket = match c.risk_score {
                0..=25  => 0,
                26..=50 => 1,
                51..=75 => 2,
                _       => 3,
            };
            dist[bucket] += 1;
        }

        OrgRiskSnapshot {
            tenant_id: tenant_id.to_string(),
            org_risk_score,
            threshold_shift: profile.threshold_shift,
            active_cluster_count: clusters.len() as u32,
            cluster_severity_distribution: dist,
            computed_at: now,
        }
    }

    /// Whether this snapshot is stale and should trigger a background refresh.
    /// Default staleness threshold: 5 minutes.
    pub fn is_stale(&self, now: i64) -> bool {
        now - self.computed_at > 300
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::org_graph::cluster::{AttackCluster, AttackType};

    fn fake_cluster(risk: u8, count: u32) -> AttackCluster {
        AttackCluster {
            cluster_id: "test:cluster:root".into(),
            tenant_id: "test".into(),
            risk_score: risk,
            nodes: (0..count).map(|i| format!("user:{:04}", i)).collect(),
            attack_type: AttackType::CredentialStuffing,
            confidence: 0.8,
            first_seen: 0,
            last_updated: 0,
        }
    }

    #[test]
    fn neutral_profile_has_zero_shift() {
        let p = TenantRiskProfile::neutral("t1", 1000);
        assert_eq!(p.threshold_shift, 0);
        assert_eq!(p.attack_pressure, 0.0);
    }

    #[test]
    fn high_attack_pressure_produces_positive_shift() {
        let clusters = vec![fake_cluster(70, 10)];
        let profile = TenantRiskProfile::compute(
            "t1", &clusters, 100, 80, 100, 0, 1000,
        );
        assert!(profile.threshold_shift > 0,
            "high pressure should raise threshold: shift={}",
            profile.threshold_shift
        );
    }

    #[test]
    fn stable_tenant_gets_negative_shift() {
        let profile = TenantRiskProfile::compute(
            "t1", &[], 100, 0, 100, 0, 1000,
        );
        assert!(profile.threshold_shift <= 0,
            "stable tenant should not be penalised: shift={}",
            profile.threshold_shift
        );
    }

    #[test]
    fn shift_inertia_prevents_sudden_spikes() {
        // Previous shift = 0, target = +15, inertia limits to +3 per cycle
        let clusters = vec![fake_cluster(90, 50)];
        let profile = TenantRiskProfile::compute(
            "t1", &clusters, 100, 90, 100, 0, 1000,
        );
        assert!(profile.threshold_shift <= 3,
            "inertia should prevent jump from 0 to 15 in one cycle: {}",
            profile.threshold_shift
        );
    }

    #[test]
    fn shift_bounded_to_max() {
        let target = compute_target_shift(0.9, 0.5, 0.0);
        assert!(target <= 20, "threshold shift must not exceed +20");
        assert!(target >= -15, "threshold shift must not go below -15");
    }

    #[test]
    fn snapshot_staleness_detection() {
        let now = 10_000i64;
        let mut snap = OrgRiskSnapshot::unavailable("t1");
        snap.computed_at = now - 301; // 301 seconds old
        assert!(snap.is_stale(now));

        snap.computed_at = now - 60; // 60 seconds old
        assert!(!snap.is_stale(now));
    }
}
