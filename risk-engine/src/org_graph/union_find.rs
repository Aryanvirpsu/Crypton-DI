use std::collections::HashMap;

use crate::org_graph::graph::canonical_edge_key;

// ─────────────────────────────────────────────────────────────────────────────
// Union-Find with path compression and union-by-rank
// ─────────────────────────────────────────────────────────────────────────────
//
// This is the core clustering primitive. It answers "are these two nodes in
// the same cluster?" in O(α(n)) amortised time (nearly constant in practice).
//
// Design decisions:
// - Path compression is applied eagerly on every `find()` call.
// - Union-by-rank keeps trees shallow.
// - `edge_weights` tracks the maximum decayed weight of any edge linking two
//   cluster roots. This is used for pruning: if the best edge between two
//   merged clusters decays below threshold, they are treated as separate.
// - The structure is entirely in-memory. Serialising it to/from Redis JSON
//   is the adapter's responsibility.
//
// Memory bound: O(N) where N = total distinct nodes. The graph is bounded by
// the `MAX_NODES_PER_TENANT` constant enforced in the adapter before calling
// `union()`.

/// Maximum nodes per tenant. Prevents unbounded memory growth if an attacker
/// registers millions of throwaway accounts.
pub const MAX_NODES_PER_TENANT: usize = 50_000;

/// Edges below this combined weight are not unioned.
const UNION_WEIGHT_THRESHOLD: f32 = 0.25;

#[derive(Debug, Clone)]
pub struct UnionFind {
    /// Maps node_key → parent_key. Root nodes map to themselves.
    parent: HashMap<String, String>,
    /// Maps node_key → rank (upper bound on subtree height).
    rank: HashMap<String, u32>,
    /// Maps canonical edge key (`"{root_a}|{root_b}"`, sorted) to the
    /// maximum decayed weight observed for any edge linking those two roots.
    /// Updated after each `union()`.
    edge_weights: HashMap<String, f32>,
}

impl Default for UnionFind {
    fn default() -> Self {
        Self::new()
    }
}

impl UnionFind {
    pub fn new() -> Self {
        UnionFind {
            parent: HashMap::new(),
            rank: HashMap::new(),
            edge_weights: HashMap::new(),
        }
    }

    /// Total number of nodes tracked.
    pub fn node_count(&self) -> usize {
        self.parent.len()
    }

    /// Ensure a node exists in the structure (no-op if already present).
    pub fn add_node(&mut self, key: &str) {
        if !self.parent.contains_key(key) {
            self.parent.insert(key.to_string(), key.to_string());
            self.rank.insert(key.to_string(), 0);
        }
    }

    /// Find the root (cluster representative) for a node, applying path
    /// compression. If the node doesn't exist, it is implicitly added.
    pub fn find(&mut self, x: &str) -> String {
        self.add_node(x);
        // Iterative path compression to avoid stack overflow on deep chains.
        // Collect path first, then flatten.
        let mut path = vec![x.to_string()];
        let mut current = x.to_string();
        loop {
            let parent = self.parent.get(&current).cloned().unwrap_or(current.clone());
            if parent == current {
                break;
            }
            path.push(parent.clone());
            current = parent;
        }
        let root = current.clone();
        // Path compression: point all nodes on the path directly to root.
        for node in &path {
            self.parent.insert(node.clone(), root.clone());
        }
        root
    }

    /// Union two nodes with an associated edge weight. Returns true if they
    /// were in different clusters (a merge actually happened).
    ///
    /// Edges below `UNION_WEIGHT_THRESHOLD` do not trigger a merge — the
    /// connection is too weak to be meaningful.
    pub fn union(&mut self, x: &str, y: &str, weight: f32) -> bool {
        if weight < UNION_WEIGHT_THRESHOLD {
            // Still register nodes, but don't merge clusters.
            self.add_node(x);
            self.add_node(y);
            return false;
        }

        let rx = self.find(x);
        let ry = self.find(y);

        if rx == ry {
            // Already in the same cluster. Update weight if higher.
            let key = edge_weight_key(&rx, &ry);
            let current = self.edge_weights.get(&key).copied().unwrap_or(0.0);
            if weight > current {
                self.edge_weights.insert(key, weight);
            }
            return false;
        }

        // Union by rank: attach smaller tree to larger tree.
        let rank_x = *self.rank.get(&rx).unwrap_or(&0);
        let rank_y = *self.rank.get(&ry).unwrap_or(&0);

        let (new_root, child) = if rank_x >= rank_y { (&rx, &ry) } else { (&ry, &rx) };
        self.parent.insert(child.to_string(), new_root.to_string());

        if rank_x == rank_y {
            *self.rank.entry(new_root.to_string()).or_insert(0) += 1;
        }

        // Record the edge weight between the two roots.
        let key = edge_weight_key(new_root, child);
        let existing = self.edge_weights.get(&key).copied().unwrap_or(0.0);
        self.edge_weights.insert(key, weight.max(existing));

        true
    }

    /// True if two nodes are in the same cluster.
    pub fn same_cluster(&mut self, x: &str, y: &str) -> bool {
        self.find(x) == self.find(y)
    }

    /// Returns all clusters as a map from root node key → member node keys.
    /// Does NOT include singleton nodes (single-node clusters) by default.
    pub fn all_clusters(&mut self) -> HashMap<String, Vec<String>> {
        // Collect all roots first (find() mutates via path compression)
        let keys: Vec<String> = self.parent.keys().cloned().collect();
        let mut roots: HashMap<String, Vec<String>> = HashMap::new();

        for key in keys {
            let root = self.find(&key);
            roots.entry(root).or_default().push(key);
        }

        // Filter out singletons
        roots.retain(|_, members| members.len() > 1);
        roots
    }

    /// Returns only the members of the cluster containing `node`.
    /// Returns an empty vec if the node is a singleton.
    pub fn cluster_members(&mut self, node: &str) -> Vec<String> {
        let target_root = self.find(node);
        let keys: Vec<String> = self.parent.keys().cloned().collect();
        let mut members: Vec<String> = keys
            .into_iter()
            .filter(|k| self.find(k) == target_root)
            .collect();
        members.sort(); // deterministic ordering
        if members.len() <= 1 { vec![] } else { members }
    }

    /// Returns the maximum edge weight linking the cluster of `x` to the
    /// cluster of `y`, or 0.0 if they are separate clusters or the same.
    pub fn inter_cluster_weight(&mut self, x: &str, y: &str) -> f32 {
        let rx = self.find(x);
        let ry = self.find(y);
        if rx == ry { return 0.0; }
        let key = edge_weight_key(&rx, &ry);
        self.edge_weights.get(&key).copied().unwrap_or(0.0)
    }

    /// Remove all nodes belonging to a specific cluster root from the structure.
    /// Used for pruning dead clusters.
    ///
    /// Uses `find()` for correct membership resolution — handles nodes that
    /// have not yet been path-compressed to point directly at `root`.
    pub fn remove_cluster(&mut self, root: &str) {
        let keys: Vec<String> = self.parent.keys().cloned().collect();
        let members: Vec<String> = keys
            .into_iter()
            .filter(|k| self.find(k) == root)
            .collect();
        for m in &members {
            self.parent.remove(m);
            self.rank.remove(m);
        }
        // Clean up edge weights involving this root.
        // Split on `|` and compare full components to avoid substring false positives
        // (e.g. root="ip:1" must NOT match key "ip:1.2.3.4|user:x").
        self.edge_weights.retain(|k, _| {
            match k.split_once('|') {
                Some((left, right)) => left != root && right != root,
                None => true, // malformed key — keep to avoid silent data loss
            }
        });
    }

    /// Serialize to a compact JSON-friendly representation for Redis storage.
    pub fn to_redis_repr(&self) -> RedisUnionFindRepr {
        RedisUnionFindRepr {
            parent: self.parent.clone(),
            rank: self.rank.clone(),
            edge_weights: self.edge_weights.clone(),
        }
    }

    /// Restore from the Redis representation.
    pub fn from_redis_repr(repr: RedisUnionFindRepr) -> Self {
        UnionFind {
            parent: repr.parent,
            rank: repr.rank,
            edge_weights: repr.edge_weights,
        }
    }
}

/// Serialisable form of UnionFind for Redis storage.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RedisUnionFindRepr {
    pub parent: HashMap<String, String>,
    pub rank: HashMap<String, u32>,
    pub edge_weights: HashMap<String, f32>,
}

/// Canonical key for an edge between two roots: sorted lexicographically.
/// Delegates to the shared `canonical_edge_key` to avoid duplicating the
/// ordering logic.
fn edge_weight_key(a: &str, b: &str) -> String {
    canonical_edge_key(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_union_and_find() {
        let mut uf = UnionFind::new();
        assert!(!uf.same_cluster("a", "b"));
        uf.union("a", "b", 0.8);
        assert!(uf.same_cluster("a", "b"));
    }

    #[test]
    fn low_weight_edge_does_not_merge() {
        let mut uf = UnionFind::new();
        uf.union("a", "b", 0.1); // below threshold
        assert!(!uf.same_cluster("a", "b"));
    }

    #[test]
    fn transitive_clustering() {
        let mut uf = UnionFind::new();
        uf.union("a", "b", 0.9);
        uf.union("b", "c", 0.8);
        assert!(uf.same_cluster("a", "c"));
    }

    #[test]
    fn all_clusters_excludes_singletons() {
        let mut uf = UnionFind::new();
        uf.add_node("loner");
        uf.union("a", "b", 0.9);
        let clusters = uf.all_clusters();
        assert!(!clusters.values().any(|v| v.contains(&"loner".to_string())));
        assert!(clusters.values().any(|v| v.len() == 2));
    }

    #[test]
    fn cluster_members_returns_empty_for_singleton() {
        let mut uf = UnionFind::new();
        uf.add_node("solo");
        assert!(uf.cluster_members("solo").is_empty());
    }

    #[test]
    fn serialization_round_trip() {
        let mut uf = UnionFind::new();
        uf.union("user:aaa", "ip:1.2.3.4", 0.9);
        uf.union("user:bbb", "ip:1.2.3.4", 0.7);

        let repr = uf.to_redis_repr();
        let json = serde_json::to_string(&repr).unwrap();
        let repr2: RedisUnionFindRepr = serde_json::from_str(&json).unwrap();
        let mut uf2 = UnionFind::from_redis_repr(repr2);

        assert!(uf2.same_cluster("user:aaa", "user:bbb"));
    }

    #[test]
    fn path_compression_keeps_correctness() {
        let mut uf = UnionFind::new();
        // Build a chain: a→b→c→d
        uf.union("a", "b", 0.9);
        uf.union("b", "c", 0.8);
        uf.union("c", "d", 0.7);
        // All should be in the same cluster
        assert!(uf.same_cluster("a", "d"));
        // After path compression, all should point to same root
        let root_a = uf.find("a");
        let root_d = uf.find("d");
        assert_eq!(root_a, root_d);
    }
}
