//! Shadow / dry-run evaluation (Phase 5).
//!
//! Runs two [`PolicyConfig`] values against the same [`RiskContext`] and returns
//! the **active** decision as the authoritative verdict. The shadow decision is
//! computed side-effect-free (no counter increments, no escalation recording,
//! no account lock, no IP block) so the shadow policy can be tuned against live
//! traffic without affecting state.
//!
//! Divergence between active and shadow is logged via [`tracing`] and — when the
//! `metrics` feature is enabled — also emitted as a counter:
//!
//! ```text
//! risk_engine_shadow_divergence_total{
//!     active_version, shadow_version, active_decision, shadow_decision
//! }
//! ```
//!
//! Intended flow: pin `active` to the deployed policy, point `shadow` at the
//! candidate tuning, sample 100% of traffic (the cost is a second synchronous
//! scoring pass — the hard-gate + factor computation is cheap), and diff. When
//! divergences stabilise at an acceptable rate, promote `shadow` to `active`.
//!
//! The shadow path calls [`crate::evaluate`] with a [`ReadOnlyStore`] wrapper
//! that forwards reads to the real store and swallows writes. Escalation memory
//! still influences the shadow decision — the wrapper reads the current
//! history and computes the would-be escalation purely, without recording
//! anything.

use std::net::IpAddr;
use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::context::RiskContext;
use crate::decision::{Decision, RiskDecision};
use crate::engine::escalation::{check_escalation, EscalationOutcome};
use crate::engine::evaluate;
use crate::error::RiskEngineError;
use crate::policy_config::PolicyConfig;
use crate::store::{EscalationEntry, SignalStore, VelocityCounters};

/// Result of a shadow evaluation. The caller should return `active` to the
/// client; `shadow` and `diverged` exist for logging, debugging, and bench
/// comparisons.
#[derive(Debug, Clone)]
pub struct ShadowEvaluation {
    pub active: RiskDecision,
    pub shadow: RiskDecision,
    /// True when `active.decision != shadow.decision`. Score-only drift is
    /// tracked by comparing the scores on the returned struct — it is not a
    /// divergence for policy-gating purposes.
    pub diverged: bool,
}

/// Evaluate `ctx` against both `active` and `shadow` policies.
///
/// The `active` run is identical to calling [`evaluate`] directly: side effects
/// fire, escalation writes, and the decision is authoritative. The `shadow` run
/// uses [`ReadOnlyStore`] so it never writes.
///
/// This function takes `ctx` by value (to preserve the `evaluate` signature)
/// and clones it once for the shadow pass.
pub async fn evaluate_with_shadow<S: SignalStore>(
    ctx: RiskContext,
    store: &S,
    active: Arc<PolicyConfig>,
    shadow: Arc<PolicyConfig>,
) -> ShadowEvaluation {
    let mut ctx_shadow = ctx.clone();
    ctx_shadow.policy = shadow.clone();

    let mut ctx_active = ctx;
    ctx_active.policy = active.clone();

    // Run active first so its writes (escalation, lock, block_ip) land before
    // the shadow pass reads them. The shadow then sees a consistent world.
    let active_decision = evaluate(ctx_active, store).await;

    let ro_store = ReadOnlyStore { inner: store };
    let shadow_decision = evaluate(ctx_shadow, &ro_store).await;

    let diverged = active_decision.decision != shadow_decision.decision;

    if diverged {
        tracing::warn!(
            active_version = %active.version,
            shadow_version = %shadow.version,
            active_decision = ?active_decision.decision,
            shadow_decision = ?shadow_decision.decision,
            active_score = active_decision.score,
            shadow_score = shadow_decision.score,
            "shadow policy diverged from active"
        );

        #[cfg(feature = "metrics")]
        {
            metrics::counter!(
                "risk_engine_shadow_divergence_total",
                "active_version" => active.version.clone(),
                "shadow_version" => shadow.version.clone(),
                "active_decision" => decision_label(&active_decision.decision),
                "shadow_decision" => decision_label(&shadow_decision.decision),
            )
            .increment(1);
        }
    } else {
        tracing::debug!(
            active_version = %active.version,
            shadow_version = %shadow.version,
            decision = ?active_decision.decision,
            active_score = active_decision.score,
            shadow_score = shadow_decision.score,
            "shadow policy agreed with active"
        );
    }

    ShadowEvaluation {
        active: active_decision,
        shadow: shadow_decision,
        diverged,
    }
}

#[cfg(feature = "metrics")]
fn decision_label(d: &Decision) -> &'static str {
    match d {
        Decision::Allow => "allow",
        Decision::Challenge => "challenge",
        Decision::Hold => "hold",
        Decision::Deny => "deny",
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ReadOnlyStore — forwards reads, swallows writes
// ─────────────────────────────────────────────────────────────────────────────

/// Transparent `SignalStore` wrapper that forwards every read to the inner
/// store and turns every write into a no-op `Ok(())`. Used by
/// [`evaluate_with_shadow`] so the shadow scoring pass cannot pollute Redis,
/// double-count velocity, or record a second escalation entry.
///
/// `check_and_record_escalation` is handled specially: the wrapper reads the
/// inner store's escalation history and computes the promotion purely via
/// [`check_escalation`], then returns the would-be decision **without
/// recording** it. This keeps the shadow decision honest (escalation memory
/// does influence it) while avoiding the double-write.
pub struct ReadOnlyStore<'a, S: SignalStore + ?Sized> {
    inner: &'a S,
}

impl<'a, S: SignalStore + ?Sized> ReadOnlyStore<'a, S> {
    pub fn new(inner: &'a S) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<'a, S: SignalStore + ?Sized> SignalStore for ReadOnlyStore<'a, S> {
    async fn get_velocity_counters(
        &self,
        user_id: Uuid,
        request_ip: Option<IpAddr>,
    ) -> Result<VelocityCounters, RiskEngineError> {
        self.inner.get_velocity_counters(user_id, request_ip).await
    }

    async fn get_recent_decisions(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<EscalationEntry>, RiskEngineError> {
        self.inner.get_recent_decisions(user_id).await
    }

    async fn record_decision(
        &self,
        _user_id: Uuid,
        _entry: EscalationEntry,
    ) -> Result<(), RiskEngineError> {
        Ok(())
    }

    async fn check_and_record_escalation(
        &self,
        user_id: Uuid,
        current_decision: &Decision,
        _score: u8,
        _ts_unix: i64,
    ) -> Result<(Decision, bool), RiskEngineError> {
        let recent = self.inner.get_recent_decisions(user_id).await?;
        match check_escalation(current_decision, &recent) {
            EscalationOutcome::NoChange => Ok((current_decision.clone(), false)),
            EscalationOutcome::EscalateTo(promoted) => Ok((promoted, true)),
        }
    }

    async fn lock_account(&self, _user_id: Uuid, _ttl_seconds: u64) -> Result<(), RiskEngineError> {
        Ok(())
    }

    async fn block_ip(&self, _ip: IpAddr, _ttl_seconds: u64) -> Result<(), RiskEngineError> {
        Ok(())
    }

    async fn get_jwt_fingerprint(
        &self,
        fingerprint_key: &str,
    ) -> Result<Option<String>, RiskEngineError> {
        self.inner.get_jwt_fingerprint(fingerprint_key).await
    }

    async fn set_jwt_fingerprint(
        &self,
        _fingerprint_key: &str,
        _fingerprint_value: &str,
        _ttl_seconds: u64,
    ) -> Result<(), RiskEngineError> {
        Ok(())
    }

    async fn is_nonce_used(&self, nonce: &str) -> Result<bool, RiskEngineError> {
        self.inner.is_nonce_used(nonce).await
    }

    async fn consume_nonce(&self, _nonce: &str) -> Result<(), RiskEngineError> {
        Ok(())
    }
}
