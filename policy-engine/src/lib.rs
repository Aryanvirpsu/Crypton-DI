//! # policy-engine (standalone scaffold)
//!
//! An unwired preview of Crypton's adaptive policy engine. A set of composable
//! evaluators — geo-block, geo-velocity, off-hours, failed-attempts,
//! device-trust, new-device, tor-block, step-up risk — are run in priority
//! order by [`evaluators::evaluate_all`], producing an
//! [`types::PolicyDecision`] of `Allow` / `StepUp` / `Deny`. First `Deny` wins
//! and short-circuits; `StepUp` reasons accumulate.
//!
//! ## Status
//! This crate is intentionally **not** wired into any service and carries **no
//! database persistence layer** (see the README). It compiles standalone and
//! its evaluator logic tests pass on their own. Wiring it into the identity
//! service (cache hydration from Postgres, invocation from the auth flow) is a
//! follow-up.
pub mod cache;
pub mod evaluators;
pub mod types;
