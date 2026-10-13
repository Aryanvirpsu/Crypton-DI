use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::org_graph::graph::{EdgeType, GraphSnapshot};
use crate::org_graph::union_find::UnionFind;

// ─────────────────────────────────────────────────────────────────────────────
// Classification thresholds for attack type detection
// ─────────────────────────────────────────────────────────────────────────────

/// Recovery edges threshold: >= 2 edges → recovery abuse cluster
const MIN_RECOVERY_EDGES: u32 = 2;
/// Shared device threshold: >= 2 edges → session hijack cluster
const MIN_SHARED_DEVICE_EDGES: u32 = 2;
/// Shared IP threshold (>3 = 4+) for credential stuffing detection
const MIN_SHARED_IP_FOR_CREDS: u32 = 4;
/// Login sequence threshold (>1 = 2+) for credential stuffing detection
const MIN_LOGIN_SEQUENCES_FOR_CREDS: u32 = 2;
/// Shared IP threshold (>5 = 6+) to classify as proxy network
const MIN_SHARED_IP_FOR_PROXY: u32 = 6;
/// Login sequence threshold (>2 = 3+) for sybil farm detection
const MIN_LOGIN_SEQUENCES_FOR_SYBIL: u32 = 3;
/// Action link threshold (>= 1) for sybil farm detection
const MIN_ACTION_LINKS_FOR_SYBIL: u32 = 1;
/// Shared IP fallback threshold (>= 2) for general credential stuffing
const MIN_SHARED_IP_FALLBACK: u32 = 2;
/// Login sequence fallback threshold (>= 2) for general credential stuffing
const MIN_LOGIN_SEQUENCES_FALLBACK: u32 = 2;

// ─────────────────────────────────────────────────────────────────────────────
// AttackType
// ─────────────────────────────────────────────────────────────────────────────

/// The category of coordinated attack represented by a cluster.
/// Determined deterministically from edge-type distribution within the cluster.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttackType {
    /// Many distinct users/IPs attempting login with similar credentials.
    CredentialStuffing,
    /// Large number of freshly registered accounts sharing IPs or devices.
    SybilFarm,
    /// Coordinated recovery requests across multiple accounts.
    RecoveryAbuse,
    /// Multiple sessions hijacked from the same device/IP.
    SessionHijackCluster,
    /// Many users sharing the same proxy/VPN exit node.
    ProxyNetwork,
    /// Statistically anomalous cluster without a clear pattern.
    UnknownAnomaly,
}

impl AttackType {
    /// Severity weight used in org risk score computation.
    pub fn severity(&self) -> f32 {
        match self {
            AttackType::SessionHijackCluster => 1.5,
            AttackType::RecoveryAbuse        => 1.4,
            AttackType::CredentialStuffing   => 1.2,
            AttackType::SybilFarm            => 1.0,
            AttackType::ProxyNetwork         => 0.7,
            AttackType::UnknownAnomaly       => 0.6,
        }
    }

    /// Human-readable label for audit output.
    pub fn label(&self) -> &'static str {
        match self {
            AttackType::CredentialStuffing   => "credential_stuffing",
            AttackType::SybilFarm            => "sybil_farm",
            AttackType::RecoveryAbuse        => "recovery_abuse",
            AttackType::SessionHijackCluster => "session_hijack_cluster",
            AttackType::ProxyNetwork         => "proxy_network",
            AttackType::UnknownAnomaly       => "unknown_anomaly",
        }
    }

    /// Confidence weighting factors: `(w_size, w_density, w_weight)`.
    /// Used by `ClusterDetector::compute_confidence()`. Sum is always 1.0.
    pub fn confidence_weights(&self) -> (f32, f32, f32) {
        match self {
            AttackType::CredentialStuffing   => (0.4, 0.3, 0.3),
            AttackType::RecoveryAbuse        => (0.3, 0.4, 0.3),
            AttackType::SessionHijackCluster => (0.3, 0.3, 0.4),
            AttackType::SybilFarm            => (0.5, 0.3, 0.2),
            AttackType::ProxyNetwork         => (0.4, 0.4, 0.2),
            AttackType::UnknownAnomaly       => (0.3, 0.3, 0.4),
        }
    }

    /// Base risk score for a cluster of this attack type.
    /// Scaled by confidence and size in `ClusterDetector::compute_risk_score()`.
    pub fn base_risk_score(&self) -> f32 {
        match self {
            AttackType::SessionHijackCluster => 80.0,
            AttackType::RecoveryAbuse        => 75.0,
            AttackType::CredentialStuffing   => 70.0,
            AttackType::SybilFarm            => 60.0,
            AttackType::ProxyNetwork         => 45.0,
            AttackType::UnknownAnomaly       => 35.0,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// AttackCluster — the output of cluster detection
// ─────────────────────────────────────────────────────────────────────────────

/// A detected cluster of nodes exhibiting coordinated attack behaviour.
///
/// Stored at `org:clusters:{tenant_id}` as a JSON hash field keyed by
/// `cluster_id`. Updated by the background recompute task, read per-request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackCluster {
    /// Stable cluster identifier. Format: `"{tenant_id}:cluster:{root_node_key}"`.
    pub cluster_id: String,
    pub tenant_id: String,
    /// Aggregate risk score for this cluster (0–100).
    pub risk_score: u8,
    /// All node keys (user:, cred:, ip:, dev:) in this cluster.
    pub nodes: Vec<String>,
    pub attack_type: AttackType,
    /// Detection confidence 0.0–1.0. Drives per-user cluster_bias weighting.
    pub confidence: f32,
    /// Unix timestamp when the cluster was first detected.
    pub first_seen: i64,
    /// Unix timestamp of the last recomputation that included this cluster.
    pub last_updated: i64,
}

impl AttackCluster {
    pub fn member_count(&self) -> u32 {
        self.nodes.len() as u32
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ClusterMembership — per-user slice of cluster info (fits in RiskContext)
// ─────────────────────────────────────────────────────────────────────────────

/// Lightweight membership record injected into `RiskContext` by the adapter.
/// The adapter fetches this from Redis before calling `evaluate()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMembership {
    pub cluster_id: String,
    pub attack_type: AttackType,
    /// The cluster's aggregate risk score (0–100).
    pub cluster_risk_score: u8,
    /// How many nodes are in the cluster.
    pub member_count: u32,
    /// Detection confidence (0.0–1.0). Scales the per-user bias contribution.
    pub confidence: f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// ClusterDetector — pure deterministic cluster computation
// ─────────────────────────────────────────────────────────────────────────────

/// Stateless cluster detection logic. All methods are pure functions.
///
/// The detector operates on a `GraphSnapshot` (loaded from Redis by the
/// background recompute task) and produces `Vec<AttackCluster>`.
///
/// This is intentionally NOT called per-request. It runs in a background task
/// triggered when graph updates cross a threshold of significance.
pub struct ClusterDetector;

/// Grouped inputs for cluster scoring. Avoids parameter sprawl in
/// `compute_confidence()` and `compute_risk_score()`.
pub struct ClusterContext<'a> {
    pub member_count: u32,
    pub attack_type: &'a AttackType,
    pub edges: &'a [&'a crate::org_graph::graph::GraphEdge],
    pub now: i64,
}

/// Minimum confidence below which a cluster is pruned during detection.
const MIN_CLUSTER_CONFIDENCE: f32 = 0.2;
/// Minimum members for a cluster to be emitted.
const MIN_CLUSTER_SIZE: usize = 2;

impl ClusterDetector {
    /// Run full cluster detection on a graph snapshot.
    ///
    /// Algorithm:
    /// 1. Build union-find from all live edges (weight >= threshold).
    ///    Simultaneously group edges by cluster root for Step 3.
    /// 2. Extract all non-singleton clusters.
    /// 3. For each cluster, classify attack type and compute risk score.
    ///
    /// Time complexity: O(E * α(N)) where E = live edges, N = nodes.
    /// No O(N²) operations. `live_edges()` is iterated exactly once.
    pub fn detect(snapshot: &GraphSnapshot, tenant_id: &str, now: i64) -> Vec<AttackCluster> {
        let mut uf = UnionFind::new();

        // Collect live edges once to avoid evaluating current_weight() twice per edge.
        let live: Vec<&crate::org_graph::graph::GraphEdge> = snapshot.live_edges().collect();

        // Step 1: Build union-find from live edges
        for edge in &live {
            let weight = edge.current_weight(now);
            uf.union(&edge.from, &edge.to, weight);
        }

        // Step 2: Extract non-singleton clusters
        let clusters = uf.all_clusters();

        // Step 3: Group edges by cluster root (reuses the same `live` vec)
        let mut cluster_edges: HashMap<String, Vec<&crate::org_graph::graph::GraphEdge>> =
            HashMap::new();
        for edge in &live {
            let root = uf.find(&edge.from);
            cluster_edges.entry(root).or_default().push(edge);
        }

        // Step 4: Build AttackCluster for each detected cluster
        clusters
            .into_iter()
            .filter_map(|(root, nodes)| {
                let edges = cluster_edges.get(&root).map(|v| v.as_slice()).unwrap_or(&[]);
                let attack_type = Self::classify_cluster(edges);
                let cctx = ClusterContext {
                    member_count: nodes.len() as u32,
                    attack_type: &attack_type,
                    edges,
                    now,
                };
                let confidence = Self::compute_confidence(&cctx);
                let risk_score = Self::compute_risk_score(&cctx, confidence);

                // Prune low confidence or small clusters
                if confidence < MIN_CLUSTER_CONFIDENCE || nodes.len() < MIN_CLUSTER_SIZE {
                    return None;
                }

                let cluster_id = format!("{}:cluster:{}", tenant_id, root);
                let mut sorted_nodes = nodes;
                sorted_nodes.sort(); // deterministic ordering

                Some(AttackCluster {
                    cluster_id,
                    tenant_id: tenant_id.to_string(),
                    risk_score,
                    nodes: sorted_nodes,
                    attack_type,
                    confidence,
                    first_seen: now, // background task sets this correctly
                    last_updated: now,
                })
            })
            .collect()
    }

    /// Classify attack type from the edge-type distribution within a cluster.
    ///
    /// Uses deterministic classification rules based on edge-type counts.
    /// Higher-priority patterns (recovery abuse, session hijack) are checked first.
    pub fn classify_cluster(edges: &[&crate::org_graph::graph::GraphEdge]) -> AttackType {
        if edges.is_empty() {
            return AttackType::UnknownAnomaly;
        }

        // Count edge types
        let mut shared_ip = 0u32;
        let mut shared_device = 0u32;
        let mut login_seq = 0u32;
        let mut recovery = 0u32;
        let mut action = 0u32;

        for edge in edges {
            match edge.edge_type {
                EdgeType::SharedIp      => shared_ip += 1,
                EdgeType::SharedDevice  => shared_device += 1,
                EdgeType::LoginSequence => login_seq += 1,
                EdgeType::RecoveryChain => recovery += 1,
                EdgeType::ActionLink    => action += 1,
            }
        }

        // Classification rules (deterministic, explicit priority order)
        if recovery >= MIN_RECOVERY_EDGES {
            AttackType::RecoveryAbuse
        } else if shared_device >= MIN_SHARED_DEVICE_EDGES {
            AttackType::SessionHijackCluster
        } else if shared_ip >= MIN_SHARED_IP_FOR_CREDS && login_seq >= MIN_LOGIN_SEQUENCES_FOR_CREDS {
            AttackType::CredentialStuffing
        } else if shared_ip >= MIN_SHARED_IP_FOR_PROXY {
            AttackType::ProxyNetwork
        } else if login_seq >= MIN_LOGIN_SEQUENCES_FOR_SYBIL && action >= MIN_ACTION_LINKS_FOR_SYBIL {
            AttackType::SybilFarm
        } else if shared_ip >= MIN_SHARED_IP_FALLBACK || login_seq >= MIN_LOGIN_SEQUENCES_FALLBACK {
            AttackType::CredentialStuffing
        } else {
            AttackType::UnknownAnomaly
        }
    }

    /// Compute detection confidence (0.0–1.0) based on cluster size, edge
    /// density, and average edge weight.
    ///
    /// Formula: `confidence = w_size * size_factor + w_density * density + w_weight * avg_weight`
    /// All weights sum to 1.0.
    pub fn compute_confidence(ctx: &ClusterContext) -> f32 {
        if ctx.member_count < 2 || ctx.edges.is_empty() {
            return 0.0;
        }

        // Size factor: more members = more confident, saturates at 20
        let size_factor = (ctx.member_count as f32 / 20.0).min(1.0);

        // Edge density: edges / max_possible_edges for a complete graph
        let max_edges = (ctx.member_count * (ctx.member_count - 1)) / 2;
        let density = (ctx.edges.len() as f32 / max_edges.max(1) as f32).min(1.0);

        // Average edge weight
        let avg_weight = ctx.edges.iter()
            .map(|e| e.current_weight(ctx.now))
            .sum::<f32>()
            / ctx.edges.len() as f32;

        // Attack-type specific weighting (size, density, weight)
        let (w_size, w_density, w_weight) = ctx.attack_type.confidence_weights();

        let raw = w_size * size_factor + w_density * density + w_weight * avg_weight;
        raw.clamp(0.0, 1.0)
    }

    /// Compute aggregate risk score for a cluster (0–100).
    ///
    /// Formula: `risk = base_for_type * confidence * size_factor`
    pub fn compute_risk_score(ctx: &ClusterContext, confidence: f32) -> u8 {
        let base = ctx.attack_type.base_risk_score();
        // Size factor: log scale, saturates at 50 members
        let size_factor = (1.0 + (ctx.member_count as f32).ln()).min(4.5) / 4.5;
        let raw = base * confidence * size_factor;
        raw.clamp(0.0, 100.0) as u8
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ClusterFactor — for RiskDecision contributing_factors
// ─────────────────────────────────────────────────────────────────────────────

/// Explains how cluster membership contributed to the adjusted score.
/// Included in `RiskDecision::cluster_factors`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterFactor {
    pub cluster_id: String,
    pub attack_type: AttackType,
    /// Points added to the effective score due to this cluster membership.
    pub contribution: u8,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::org_graph::graph::{GraphEdge, GraphSnapshot};

    fn make_shared_ip_snapshot(tenant: &str, user_count: u32) -> GraphSnapshot {
        let now = 1_000_000i64;
        let mut nodes = vec!["ip:1.2.3.4".to_string()];
        let mut edges = vec![];

        for i in 0..user_count {
            let uid = format!("user:{:04}", i);
            nodes.push(uid.clone());
            edges.push(GraphEdge {
                from: "ip:1.2.3.4".to_string(),
                to: uid,
                edge_type: EdgeType::SharedIp,
                initial_weight: 0.8,
                last_seen: now - 60,
            });
        }

        GraphSnapshot::new(tenant.to_string(), nodes, edges, now)
    }

    #[test]
    fn detects_shared_ip_cluster() {
        let snap = make_shared_ip_snapshot("tenant_1", 5);
        let clusters = ClusterDetector::detect(&snap, "tenant_1", snap.snapshot_ts);
        assert!(!clusters.is_empty(), "should detect at least one cluster");
        assert!(clusters[0].member_count() >= 5);
    }

    #[test]
    fn recovery_abuse_classified_correctly() {
        let now = 1_000_000i64;

        // Two recovery edges → classified as recovery abuse
        let at = ClusterDetector::classify_cluster(
            &[
                &GraphEdge { from: "user:a".into(), to: "user:b".into(),
                    edge_type: EdgeType::RecoveryChain, initial_weight: 0.8,
                    last_seen: now },
                &GraphEdge { from: "user:b".into(), to: "user:c".into(),
                    edge_type: EdgeType::RecoveryChain, initial_weight: 0.7,
                    last_seen: now },
            ],
        );
        assert_eq!(at, AttackType::RecoveryAbuse);
    }

    #[test]
    fn risk_score_increases_with_size() {
        let at = AttackType::CredentialStuffing;
        let small_ctx = ClusterContext { member_count: 2, attack_type: &at, edges: &[], now: 0 };
        let large_ctx = ClusterContext { member_count: 50, attack_type: &at, edges: &[], now: 0 };
        let small = ClusterDetector::compute_risk_score(&small_ctx, 0.9);
        let large = ClusterDetector::compute_risk_score(&large_ctx, 0.9);
        assert!(large > small, "larger clusters should score higher");
    }

    #[test]
    fn confidence_zero_for_tiny_cluster() {
        let at = AttackType::UnknownAnomaly;
        let ctx = ClusterContext { member_count: 1, attack_type: &at, edges: &[], now: 0 };
        let c = ClusterDetector::compute_confidence(&ctx);
        assert_eq!(c, 0.0);
    }
}
