//! # org_graph
//!
//! Multi-tenant adversarial graph intelligence layer for the Crypton risk engine.
//!
//! ## Architecture
//!
//! ```text
//! Background recompute task (NOT per-request):
//!   GraphSnapshot (from Redis)
//!     └── ClusterDetector::detect()
//!           └── UnionFind (union-by-rank, path compression)
//!                 └── Vec<AttackCluster>
//!                       └── TenantRiskProfile::compute()
//!                             └── OrgRiskSnapshot → Redis (org:risk:{tenant_id})
//!
//! Per-request hot path (read-only, < 5ms):
//!   Adapter reads org:risk:{tenant_id}     → OrgRiskSnapshot
//!   Adapter reads org:clusters:{tenant_id} → ClusterMembership (if user in cluster)
//!   Adapter injects into RiskContext:
//!     ctx.org_risk_score        = snapshot.org_risk_score
//!     ctx.cluster_membership    = membership
//!     ctx.threshold_shift       = snapshot.threshold_shift
//!   engine::evaluate() calls scoring::compute_org_cluster_bias()
//!     → OrgClusterBias { org_bias, cluster_bias, threshold_shift, factors }
//!   adjusted_score = (final_score + org_bias + cluster_bias).min(100)
//!   effective_score = (adjusted_score + threshold_shift).clamp(0, 100)
//!   decision ← score_to_decision(effective_score)
//! ```
//!
//! ## Redis key schema
//!
//! | Key | Type | TTL | Description |
//! |-----|------|-----|-------------|
//! | `org:risk:{tenant_id}` | String (JSON) | 5m | OrgRiskSnapshot |
//! | `org:clusters:{tenant_id}` | Hash `{cluster_id → JSON}` | 1h | AttackCluster map |
//! | `org:uf:{tenant_id}` | String (JSON) | 24h | RedisUnionFindRepr |
//! | `org:graph:{tenant_id}:edges` | Hash `{edge_key → JSON}` | 24h | GraphEdge map |
//! | `org:graph:{tenant_id}:nodes` | Set | 24h | All node keys |
//! | `org:meta:{tenant_id}:stats` | Hash | 1h | blocked/total request counters |
//!
//! ## Memory bounds
//!
//! - Maximum nodes per tenant: `union_find::MAX_NODES_PER_TENANT` (50,000)
//! - Edges are pruned by time decay (< 0.1 weight) on background recompute
//! - Clusters with confidence < 0.2 are dropped on detection

pub mod cluster;
pub mod graph;
pub mod recompute;
pub mod scoring;
pub mod tenant;
pub mod union_find;
pub mod worker;

// ── Primary public re-exports ─────────────────────────────────────────────────

pub use cluster::{
    AttackCluster, AttackType, ClusterContext, ClusterDetector, ClusterFactor, ClusterMembership,
};
pub use graph::{
    canonical_edge_key, EdgeType, GraphEdge, GraphSnapshot, NodeId, NodeType, OrgGraphUpdate,
};
pub use recompute::{recompute_tenant, RecomputeInputs, RecomputeOutputs};
pub use scoring::{compute_org_cluster_bias, OrgClusterBias, ORG_LEVEL_CLUSTER_ID};
pub use tenant::{OrgRiskSnapshot, TenantRiskProfile};
pub use union_find::{RedisUnionFindRepr, UnionFind, MAX_NODES_PER_TENANT};
