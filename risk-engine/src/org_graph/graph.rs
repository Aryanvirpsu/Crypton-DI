use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use uuid::Uuid;

// ─────────────────────────────────────────────────────────────────────────────
// Node types
// ─────────────────────────────────────────────────────────────────────────────

/// Every entity in the attack graph is a typed node.
/// The `NodeId::key()` method produces the canonical Redis field name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NodeType {
    User,
    Credential,
    IpAddress,
    Device,
}

/// A typed, stable identifier for a graph node.
/// Format: `"{kind}:{value}"` — e.g., `"user:550e8400-…"` or `"ip:1.2.3.4"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId {
    pub kind: NodeType,
    pub value: String,
}

impl NodeId {
    pub fn user(id: Uuid) -> Self {
        NodeId { kind: NodeType::User, value: id.to_string() }
    }

    pub fn credential(id: Uuid) -> Self {
        NodeId { kind: NodeType::Credential, value: id.to_string() }
    }

    pub fn ip(addr: IpAddr) -> Self {
        NodeId { kind: NodeType::IpAddress, value: addr.to_string() }
    }

    pub fn device(fingerprint: impl Into<String>) -> Self {
        NodeId { kind: NodeType::Device, value: fingerprint.into() }
    }

    /// Canonical string key used as Redis hash field and union-find node id.
    pub fn key(&self) -> String {
        let prefix = match self.kind {
            NodeType::User => "user",
            NodeType::Credential => "cred",
            NodeType::IpAddress => "ip",
            NodeType::Device => "dev",
        };
        format!("{}:{}", prefix, self.value)
    }

    /// Parse a canonical key back into a NodeId. Returns None if malformed.
    pub fn from_key(key: &str) -> Option<Self> {
        let (prefix, value) = key.split_once(':')?;
        let kind = match prefix {
            "user" => NodeType::User,
            "cred" => NodeType::Credential,
            "ip"   => NodeType::IpAddress,
            "dev"  => NodeType::Device,
            _      => return None,
        };
        Some(NodeId { kind, value: value.to_string() })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Edge types
// ─────────────────────────────────────────────────────────────────────────────

/// Why two nodes are connected. Drives clustering classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    /// Same IP address seen authenticating multiple distinct users.
    SharedIp,
    /// Same device fingerprint seen for multiple users.
    SharedDevice,
    /// User A logged in, then User B from the same IP within a short window.
    LoginSequence,
    /// User A initiated recovery and User B approved it (or vice versa).
    RecoveryChain,
    /// Two users targeted the same resource in the same action window.
    ActionLink,
}

impl EdgeType {
    /// Base initial weight assigned when a new edge of this type is created.
    /// Higher weight = stronger signal. Decays over time.
    pub fn base_weight(&self) -> f32 {
        match self {
            EdgeType::SharedIp        => 0.6,
            EdgeType::SharedDevice    => 0.9,
            EdgeType::LoginSequence   => 0.5,
            EdgeType::RecoveryChain   => 0.8,
            EdgeType::ActionLink      => 0.4,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Graph edge with time-decay
// ─────────────────────────────────────────────────────────────────────────────

/// An undirected weighted edge stored in the org attack graph.
///
/// Edges decay exponentially over time. An edge with `initial_weight = 1.0`
/// reaches 0.5 after 6 hours, 0.25 after 12 hours, and is pruned below 0.1
/// after ~20 hours. This is implemented via:
///
/// `w(t) = initial_weight * 2^(-(elapsed_hours / 6))`
///
/// The decay is evaluated lazily (on read) — no background sweep needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Canonical key of the first node (lexicographically smaller).
    pub from: String,
    /// Canonical key of the second node (lexicographically larger).
    pub to: String,
    pub edge_type: EdgeType,
    /// Weight at the time the edge was last reinforced. Range: 0.0–1.0.
    pub initial_weight: f32,
    /// Unix timestamp when this edge was last reinforced (refreshes decay).
    /// Decay is evaluated lazily: `w(t) = initial_weight * 2^(-(elapsed_hours / 6))`.
    pub last_seen: i64,
}

/// Minimum weight before an edge is considered prunable.
pub const MIN_EDGE_WEIGHT: f32 = 0.1;

/// Decay half-life in hours. Weight halves every 6 hours.
const HALF_LIFE_HOURS: f64 = 6.0;

impl GraphEdge {
    /// Create a new edge with canonical ordering (from < to lexicographically).
    pub fn new(a: &str, b: &str, edge_type: EdgeType, now: i64) -> Self {
        let (from, to) = if a <= b { (a.to_string(), b.to_string()) }
                         else       { (b.to_string(), a.to_string()) };
        let weight = edge_type.base_weight();
        GraphEdge { from, to, edge_type, initial_weight: weight, last_seen: now }
    }

    /// Stable key for use as a Redis hash field or HashMap key.
    /// Uses canonical ordering (from <= to is maintained by constructor).
    pub fn edge_key(&self) -> String {
        // `from` is always <= `to` by construction, so no re-sorting needed.
        format!("{}|{}", self.from, self.to)
    }

    /// Current weight after applying exponential time decay.
    /// `now` is Unix seconds.
    pub fn current_weight(&self, now: i64) -> f32 {
        let elapsed_secs = (now - self.last_seen).max(0);
        let elapsed_hours = elapsed_secs as f64 / 3600.0;
        let decayed = self.initial_weight as f64 * (0.5_f64.powf(elapsed_hours / HALF_LIFE_HOURS));
        // Clamp to reasonable float range
        decayed.max(0.0).min(1.0) as f32
    }

    /// True if this edge should still be considered active.
    pub fn is_alive(&self, now: i64) -> bool {
        self.current_weight(now) >= MIN_EDGE_WEIGHT
    }

    /// Reinforce the edge: bump `last_seen` and raise initial_weight toward 1.0.
    /// Reinforcement uses a weighted average to avoid sudden jumps.
    pub fn reinforce(&mut self, now: i64) {
        let current = self.current_weight(now);
        // Blend: new_weight = 0.7 * base_type_weight + 0.3 * current_decayed
        self.initial_weight = 0.7 * self.edge_type.base_weight() + 0.3 * current;
        self.last_seen = now;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GraphSnapshot — in-memory view of a tenant's graph for batch computation
// ─────────────────────────────────────────────────────────────────────────────

/// An in-memory snapshot of a tenant's attack graph, used for cluster
/// recomputation. This is never held per-request — it is loaded from Redis
/// by the background recompute task.
///
/// Call `build_index()` after construction to enable O(1) edge lookups.
/// Without an index, `edge_weight()` falls back to O(E) linear scan.
#[derive(Debug, Default, Clone)]
pub struct GraphSnapshot {
    pub tenant_id: String,
    /// All nodes seen in this tenant's graph (canonical keys).
    pub nodes: Vec<String>,
    /// All edges, including potentially stale ones (caller must filter).
    pub edges: Vec<GraphEdge>,
    pub snapshot_ts: i64,
    /// O(1) edge lookup: `"from|to"` → index into `self.edges`.
    /// Built by `build_index()`. Empty until called.
    #[allow(clippy::type_complexity)]
    edge_index: HashMap<String, usize>,
}

impl GraphSnapshot {
    /// Construct a new snapshot. The edge index is not built until
    /// `build_index()` is called explicitly.
    pub fn new(tenant_id: String, nodes: Vec<String>, edges: Vec<GraphEdge>, snapshot_ts: i64) -> Self {
        GraphSnapshot {
            tenant_id,
            nodes,
            edges,
            snapshot_ts,
            edge_index: HashMap::new(),
        }
    }

    /// Build an edge-key → index map for O(1) lookups.
    /// Call once after loading edges from Redis.
    pub fn build_index(&mut self) {
        self.edge_index = self
            .edges
            .iter()
            .enumerate()
            .map(|(i, e)| (e.edge_key(), i))
            .collect();
    }

    /// Return only edges that are currently alive.
    pub fn live_edges(&self) -> impl Iterator<Item = &GraphEdge> {
        let now = self.snapshot_ts;
        self.edges.iter().filter(move |e| e.is_alive(now))
    }

    /// Return the current weight of an edge between two nodes (0.0 if absent).
    ///
    /// O(1) if `build_index()` was called; O(E) fallback otherwise.
    pub fn edge_weight(&self, a: &str, b: &str) -> f32 {
        let key = canonical_edge_key(a, b);

        // Fast path: use index if available
        if let Some(&idx) = self.edge_index.get(&key) {
            if let Some(edge) = self.edges.get(idx) {
                return edge.current_weight(self.snapshot_ts);
            }
        }

        // Slow path: linear scan (index not built or stale)
        if self.edge_index.is_empty() {
            self.edges
                .iter()
                .find(|e| e.edge_key() == key)
                .map(|e| e.current_weight(self.snapshot_ts))
                .unwrap_or(0.0)
        } else {
            0.0 // index exists but key not found → edge absent
        }
    }
}

/// Canonical edge key with lexicographic ordering.
/// Shared between GraphEdge::edge_key(), edge_weight(), and union_find.
pub fn canonical_edge_key(a: &str, b: &str) -> String {
    if a <= b {
        format!("{}|{}", a, b)
    } else {
        format!("{}|{}", b, a)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// OrgGraphUpdate — the write record pushed per request
// ─────────────────────────────────────────────────────────────────────────────

/// Describes what happened in a single request, for graph ingestion.
/// The adapter constructs this and pushes it to Redis after the decision is made.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgGraphUpdate {
    pub from_node: String,   // NodeId.key()
    pub to_node: String,     // NodeId.key()
    pub edge_type: EdgeType,
    pub ts_unix: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_id_round_trip() {
        let n = NodeId::user(Uuid::nil());
        let key = n.key();
        let parsed = NodeId::from_key(&key).unwrap();
        assert_eq!(n, parsed);
    }

    #[test]
    fn edge_canonical_ordering() {
        let a = GraphEdge::new("user:bbb", "user:aaa", EdgeType::SharedIp, 1000);
        assert_eq!(a.from, "user:aaa");
        assert_eq!(a.to, "user:bbb");
    }

    #[test]
    fn edge_decays_correctly() {
        let now = 1_000_000i64;
        let six_hours_ago = now - 6 * 3600;
        let edge = GraphEdge {
            from: "user:a".into(),
            to: "user:b".into(),
            edge_type: EdgeType::SharedIp,
            initial_weight: 1.0,
            last_seen: six_hours_ago,
        };
        let w = edge.current_weight(now);
        // Should be approximately 0.5 (half-life = 6h)
        assert!((w - 0.5).abs() < 0.01, "weight={}", w);
    }

    #[test]
    fn edge_prunable_after_decay() {
        let now = 1_000_000i64;
        let twenty_hours_ago = now - 20 * 3600;
        let edge = GraphEdge {
            from: "user:a".into(),
            to: "user:b".into(),
            edge_type: EdgeType::SharedDevice,
            initial_weight: 1.0,
            last_seen: twenty_hours_ago,
        };
        // After 20h, weight ≈ 0.063 < MIN_EDGE_WEIGHT
        assert!(!edge.is_alive(now));
    }

    #[test]
    fn reinforce_boosts_weight() {
        let t0 = 0i64;
        let t1 = 12 * 3600i64; // 12 hours later
        let mut edge = GraphEdge::new("user:a", "user:b", EdgeType::SharedDevice, t0);
        let before = edge.current_weight(t1);
        edge.reinforce(t1);
        let after = edge.current_weight(t1);
        assert!(after > before, "reinforce should boost weight: {} > {}", after, before);
    }
}
