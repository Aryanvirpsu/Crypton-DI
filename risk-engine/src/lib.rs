//! # risk-engine
//!
//! Standalone, deterministic risk scoring and policy engine for Crypton-DI.
//!
//! ## Architecture
//!
//! ```text
//! evaluate()
//!   ├── evaluate_hard_gates()   → immediate DENY on known-bad conditions
//!   ├── compute_score()         → R = (D + S + N + B) * M_action * M_org + A
//!   ├── apply_escalation()      → promote decision based on recent history
//!   └── decide()                → map score to Decision + directives
//! ```
//!
//! ## Constraints
//!
//! - Zero dependencies on Crypton service code.
//! - All signal I/O abstracted behind traits (see `store` module).
//! - Fully synchronous scoring logic — async only at signal-fetch boundaries.
//! - No ML, no probabilistic models. Every outcome is traceable.

pub mod adapter;
pub mod adapters;
pub mod config;
pub mod context;
pub mod decision;
pub mod engine;
pub mod enrichment;
pub mod error;
pub mod org_graph;
pub mod policy;
pub mod policy_config;
pub mod policy_overrides;
pub mod rate_limit;
pub mod scoring;
pub mod shadow;
pub mod signals;
pub mod store;

// Re-export the primary public API surface.
pub use context::{
    AsnType, CredentialStatus, DeviceTrustLevel, GeoPoint, OrgRiskLevel, RedactedContext,
    RequiredAction, RiskAction, RiskContext,
};
pub use decision::{
    Decision, Factor, RiskDecision, ScoreBreakdown, ScoreComponent, ENGINE_VERSION, SCHEMA_VERSION,
};
pub use engine::evaluate;
pub use shadow::{evaluate_with_shadow, ReadOnlyStore, ShadowEvaluation};
pub use policy_config::PolicyConfig;
pub use policy_overrides::{CachedPolicyStore, PolicyOverrides, PolicyStore};
pub use rate_limit::{
    evaluate_with_rate_limit, evaluate_with_rate_limit_fail_open, InMemoryRateLimiter, RateLimiter,
};
pub use error::RiskEngineError;
pub use org_graph::{
    canonical_edge_key, AttackCluster, AttackType, ClusterContext, ClusterDetector,
    ClusterFactor, ClusterMembership, EdgeType, GraphEdge, GraphSnapshot, NodeId,
    NodeType, OrgGraphUpdate, OrgRiskSnapshot, TenantRiskProfile, UnionFind,
    MAX_NODES_PER_TENANT, ORG_LEVEL_CLUSTER_ID,
};
pub use store::{EscalationEntry, IncrementStore, OrgGraphStore, SignalStore, VelocityCounters};

// ── Enrichment signals (Phase C — host adapter callable) ─────────────────────
pub use signals::breach::{
    hibp_body_contains, sha1_prefix_and_suffix, BreachChecker, NoopBreachChecker,
    StaticBreachChecker,
};
pub use signals::client_sdk::ClientSdkSignals;
pub use signals::disposable_email::{DisposableDomains, BUILTIN_DISPOSABLE_DOMAINS};
pub use signals::ip_reputation::{
    IpReputation, IpReputationStore, NoopIpReputation, StaticIpReputation,
};
pub use signals::metrics::{ClassificationResult, EnrichmentSource};
// `signals::metrics::{record_classification, record_duration, record_error}`
// are not re-exported at crate root to avoid naming collisions; callers
// should import them via `risk_engine::signals::metrics::*`.
pub use signals::sanctions::{is_sanctioned, SANCTIONED_COUNTRIES};

pub use enrichment::{AsnResolver, Builder as PipelineBuilder, CountryResolver, Pipeline, PipelineConfig};

#[cfg(feature = "geoip")]
pub use signals::geoip::{classify_asn_org, GeoIpError, GeoIpReader};
