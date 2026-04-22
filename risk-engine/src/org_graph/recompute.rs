//! Pure recompute step for the org attack graph.
//!
//! The adapter's background job reads raw state from Redis, calls
//! [`recompute_tenant`], and writes the outputs back. All graph/clustering
//! math is in this module so it can be unit-tested without a Redis instance.
//!
//! ```text
//! RecomputeInputs
//!   ├── prev_edges      (map of edge_key → GraphEdge)
//!   ├── updates         (Vec<OrgGraphUpdate> from org:updates:{tenant})
//!   ├── prev_shift      (from prior TenantRiskProfile, 0 if fresh)
//!   ├── total_users
//!   ├── blocked_1h / total_1h  (from org:meta:{tenant}:stats)
//!   ├── now             (unix seconds)
//!   └── prune           (drop sub-threshold edges before clustering)
//!         ↓
//!   recompute_tenant()
//!         ↓
//! RecomputeOutputs
//!   ├── edges          (post-merge, post-prune)
//!   ├── pruned_keys    (edge_keys that were removed — for adapter HDEL)
//!   ├── clusters       (AttackCluster list)
//!   ├── profile        (TenantRiskProfile)
//!   ├── snapshot       (OrgRiskSnapshot — what the engine reads)
//!   └── memberships    (node_key → ClusterMembership, for per-node writes)
//! ```

use std::collections::HashMap;

use crate::org_graph::cluster::{AttackCluster, ClusterDetector, ClusterMembership};
use crate::org_graph::graph::{GraphEdge, GraphSnapshot, OrgGraphUpdate};
use crate::org_graph::tenant::{OrgRiskSnapshot, TenantRiskProfile};

// ─────────────────────────────────────────────────────────────────────────────
// Inputs / Outputs
// ─────────────────────────────────────────────────────────────────────────────

/// All the state the background worker hands to the pure recompute step.
#[derive(Debug, Clone)]
pub struct RecomputeInputs {
    pub tenant_id: String,
    /// Previously persisted edges keyed by `GraphEdge::edge_key()`. Empty on
    /// the first ever recompute for a tenant.
    pub prev_edges: HashMap<String, GraphEdge>,
    /// Updates pushed since the last recompute (order does not matter — each
    /// update is either reinforcing an existing edge or creating a new one).
    pub updates: Vec<OrgGraphUpdate>,
    /// Previous `TenantRiskProfile::threshold_shift` for inertia. Zero if no
    /// prior profile exists.
    pub prev_shift: i8,
    /// Approximate tracked-user count for this tenant. Feeds cluster density.
    pub total_users: u32,
    /// Denied + held decisions in the last hour (from `record_decision_stats`).
    pub blocked_requests_1h: u32,
    /// All decisions in the last hour (from `record_decision_stats`).
    pub total_requests_1h: u32,
    /// Unix seconds — used for decay, classification `now`, and snapshot ts.
    pub now: i64,
    /// If true, edges whose decayed weight falls below
    /// [`graph::MIN_EDGE_WEIGHT`] are dropped before clustering. The worker's
    /// prune loop always passes true; the recompute loop usually passes true
    /// too because keeping dead edges in Redis is pure waste.
    pub prune: bool,
}

/// Everything the adapter needs to persist after a recompute.
#[derive(Debug, Clone)]
pub struct RecomputeOutputs {
    /// Edges the adapter should persist to `org:graph:{tenant}:edges`.
    /// Key = `GraphEdge::edge_key()`, value = edge.
    pub edges: HashMap<String, GraphEdge>,
    /// Edge keys that were present in `prev_edges` but pruned this cycle.
    /// The adapter should `HDEL` these from `org:graph:{tenant}:edges`.
    pub pruned_keys: Vec<String>,
    /// Detected clusters, written to `org:clusters:{tenant}` (if used) or
    /// kept only implicitly via per-node memberships.
    pub clusters: Vec<AttackCluster>,
    /// Tenant-level derived profile. Written to `org:tenant:{tenant}`.
    pub profile: TenantRiskProfile,
    /// Snapshot the engine reads from `org:risk:{tenant}`.
    pub snapshot: OrgRiskSnapshot,
    /// Per-node cluster memberships. The adapter writes each as
    /// `org:membership:{tenant}:{node_key}` (JSON of `ClusterMembership`).
    pub memberships: HashMap<String, ClusterMembership>,
}

// ─────────────────────────────────────────────────────────────────────────────
// recompute_tenant — the pure step
// ─────────────────────────────────────────────────────────────────────────────

/// Merge updates into the prior edge set, optionally prune decayed edges,
/// detect clusters, compute the tenant profile and snapshot, and build the
/// per-node membership map.
///
/// Deterministic: calling twice with identical inputs produces identical
/// outputs (modulo HashMap iteration order, which is internal to the types).
pub fn recompute_tenant(inputs: RecomputeInputs) -> RecomputeOutputs {
    let RecomputeInputs {
        tenant_id,
        mut prev_edges,
        updates,
        prev_shift,
        total_users,
        blocked_requests_1h,
        total_requests_1h,
        now,
        prune,
    } = inputs;

    // ── Step 1: merge updates into edges ─────────────────────────────────────
    for u in updates {
        let probe = GraphEdge::new(&u.from_node, &u.to_node, u.edge_type.clone(), u.ts_unix);
        let key = probe.edge_key();
        match prev_edges.get_mut(&key) {
            Some(existing) => existing.reinforce(u.ts_unix.max(existing.last_seen)),
            None => {
                prev_edges.insert(key, probe);
            }
        }
    }

    // ── Step 2: optionally prune dead edges ──────────────────────────────────
    let mut pruned_keys = Vec::new();
    if prune {
        prev_edges.retain(|k, edge| {
            let alive = edge.is_alive(now);
            if !alive {
                pruned_keys.push(k.clone());
            }
            alive
        });
    }

    // ── Step 3: gather nodes ──────────────────────────────────────────────────
    let mut node_set = std::collections::HashSet::<String>::new();
    for e in prev_edges.values() {
        node_set.insert(e.from.clone());
        node_set.insert(e.to.clone());
    }
    let nodes: Vec<String> = node_set.into_iter().collect();
    let edges_vec: Vec<GraphEdge> = prev_edges.values().cloned().collect();

    // ── Step 4: detect clusters ──────────────────────────────────────────────
    let mut snapshot_graph = GraphSnapshot::new(tenant_id.clone(), nodes, edges_vec, now);
    snapshot_graph.build_index();
    let clusters = ClusterDetector::detect(&snapshot_graph, &tenant_id, now);

    // ── Step 5: tenant profile & org snapshot ────────────────────────────────
    let profile = TenantRiskProfile::compute(
        &tenant_id,
        &clusters,
        total_users,
        blocked_requests_1h,
        total_requests_1h,
        prev_shift,
        now,
    );
    let snapshot = OrgRiskSnapshot::from_clusters(&tenant_id, &clusters, &profile, now);

    // ── Step 6: per-node memberships ─────────────────────────────────────────
    // One cluster can contribute membership for each of its nodes. If a node
    // somehow appears in multiple clusters (shouldn't happen — union-find
    // partitions the node set — but defensive), the highest-risk cluster wins.
    let mut memberships: HashMap<String, ClusterMembership> = HashMap::new();
    for c in &clusters {
        let m = ClusterMembership {
            cluster_id: c.cluster_id.clone(),
            attack_type: c.attack_type.clone(),
            cluster_risk_score: c.risk_score,
            member_count: c.member_count(),
            confidence: c.confidence,
        };
        for node_key in &c.nodes {
            memberships
                .entry(node_key.clone())
                .and_modify(|existing| {
                    if m.cluster_risk_score > existing.cluster_risk_score {
                        *existing = m.clone();
                    }
                })
                .or_insert_with(|| m.clone());
        }
    }

    RecomputeOutputs {
        edges: prev_edges,
        pruned_keys,
        clusters,
        profile,
        snapshot,
        memberships,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::org_graph::graph::EdgeType;

    fn update(from: &str, to: &str, kind: EdgeType, now: i64) -> OrgGraphUpdate {
        OrgGraphUpdate {
            from_node: from.to_string(),
            to_node: to.to_string(),
            edge_type: kind,
            ts_unix: now,
        }
    }

    fn base_inputs(now: i64) -> RecomputeInputs {
        RecomputeInputs {
            tenant_id: "t1".to_string(),
            prev_edges: HashMap::new(),
            updates: vec![],
            prev_shift: 0,
            total_users: 100,
            blocked_requests_1h: 0,
            total_requests_1h: 100,
            now,
            prune: true,
        }
    }

    #[test]
    fn empty_inputs_produce_neutral_snapshot() {
        let now = 1_000_000i64;
        let out = recompute_tenant(base_inputs(now));
        assert!(out.edges.is_empty());
        assert!(out.clusters.is_empty());
        assert_eq!(out.snapshot.active_cluster_count, 0);
        assert_eq!(out.snapshot.org_risk_score, 0);
        assert_eq!(out.memberships.len(), 0);
    }

    #[test]
    fn updates_create_edges_and_cluster() {
        let now = 1_000_000i64;
        let mut inputs = base_inputs(now);
        // A shared IP linking 5 distinct users triggers a proxy/stuffing cluster.
        for i in 0..5 {
            let uid = format!("user:{i:04}");
            inputs.updates.push(update("ip:1.2.3.4", &uid, EdgeType::SharedIp, now - 60));
        }

        let out = recompute_tenant(inputs);
        assert_eq!(out.edges.len(), 5, "one edge per user");
        assert!(!out.clusters.is_empty(), "a cluster should form");
        let cluster = &out.clusters[0];
        assert!(cluster.member_count() >= 5);

        // Every node in the cluster has a membership written.
        for node_key in &cluster.nodes {
            assert!(
                out.memberships.contains_key(node_key),
                "missing membership for {node_key}"
            );
        }
    }

    #[test]
    fn decayed_edges_are_pruned_when_prune_true() {
        let now = 1_000_000i64;
        let mut prev = HashMap::new();
        let stale = GraphEdge {
            from: "user:a".into(),
            to: "user:b".into(),
            edge_type: EdgeType::SharedIp,
            initial_weight: 1.0,
            last_seen: now - 25 * 3600, // ~25h → below MIN_EDGE_WEIGHT
        };
        prev.insert(stale.edge_key(), stale);

        let mut inputs = base_inputs(now);
        inputs.prev_edges = prev;
        let out = recompute_tenant(inputs);
        assert!(out.edges.is_empty(), "stale edge should have been pruned");
        assert_eq!(out.pruned_keys.len(), 1);
    }

    #[test]
    fn decayed_edges_are_kept_when_prune_false() {
        let now = 1_000_000i64;
        let mut prev = HashMap::new();
        let stale = GraphEdge {
            from: "user:a".into(),
            to: "user:b".into(),
            edge_type: EdgeType::SharedIp,
            initial_weight: 1.0,
            last_seen: now - 25 * 3600,
        };
        prev.insert(stale.edge_key(), stale);

        let mut inputs = base_inputs(now);
        inputs.prev_edges = prev;
        inputs.prune = false;
        let out = recompute_tenant(inputs);
        assert_eq!(out.edges.len(), 1, "stale edge kept when prune=false");
        assert_eq!(out.pruned_keys.len(), 0);
    }

    #[test]
    fn reinforcement_keeps_edge_alive() {
        let now = 1_000_000i64;
        let mut prev = HashMap::new();
        // Edge is 12h old — decayed but alive. last_seen old.
        let old = GraphEdge {
            from: "ip:1.2.3.4".into(),
            to: "user:x".into(),
            edge_type: EdgeType::SharedIp,
            initial_weight: 1.0,
            last_seen: now - 12 * 3600,
        };
        let edge_key = old.edge_key();
        prev.insert(edge_key.clone(), old);

        let mut inputs = base_inputs(now);
        inputs.prev_edges = prev;
        inputs.updates
            .push(update("ip:1.2.3.4", "user:x", EdgeType::SharedIp, now));

        let out = recompute_tenant(inputs);
        let refreshed = out.edges.get(&edge_key).expect("edge must survive");
        assert_eq!(refreshed.last_seen, now, "last_seen should be bumped");
    }

    #[test]
    fn profile_reflects_attack_pressure() {
        let now = 1_000_000i64;
        let mut inputs = base_inputs(now);
        inputs.blocked_requests_1h = 60;
        inputs.total_requests_1h = 100;
        // Also feed a device-share cluster so compute() sees real activity.
        for i in 0..3 {
            inputs.updates.push(update(
                "dev:fp-abc",
                &format!("user:{i}"),
                crate::org_graph::graph::EdgeType::SharedDevice,
                now - 60,
            ));
        }
        let out = recompute_tenant(inputs);
        assert!(out.profile.attack_pressure > 0.5);
        assert!(
            out.profile.threshold_shift > 0,
            "moderate pressure should produce positive shift (got {})",
            out.profile.threshold_shift,
        );
    }

    #[test]
    fn snapshot_matches_profile_shift() {
        let now = 1_000_000i64;
        let mut inputs = base_inputs(now);
        inputs.prev_shift = 5;
        let out = recompute_tenant(inputs);
        assert_eq!(out.snapshot.threshold_shift, out.profile.threshold_shift);
        assert_eq!(out.snapshot.computed_at, now);
    }

    #[test]
    fn determinism() {
        let now = 1_000_000i64;
        let mut a = base_inputs(now);
        let mut b = base_inputs(now);
        for i in 0..4 {
            let uid = format!("user:{i}");
            a.updates.push(update("ip:9.9.9.9", &uid, EdgeType::SharedIp, now - 60));
            b.updates.push(update("ip:9.9.9.9", &uid, EdgeType::SharedIp, now - 60));
        }
        let out_a = recompute_tenant(a);
        let out_b = recompute_tenant(b);
        assert_eq!(out_a.edges.len(), out_b.edges.len());
        assert_eq!(out_a.clusters.len(), out_b.clusters.len());
        assert_eq!(out_a.snapshot.org_risk_score, out_b.snapshot.org_risk_score);
        assert_eq!(out_a.profile.threshold_shift, out_b.profile.threshold_shift);
    }
}
