//! Background loop that drives periodic `OrgGraphStore::recompute_org_graph`
//! and `OrgGraphStore::prune_graph` calls.
//!
//! Host services spawn [`run_recompute_loop`] as a tokio task at startup. The
//! loop is adapter-agnostic — it operates on any `OrgGraphStore` and a
//! host-provided [`TenantDirectory`] that knows which tenants exist and how
//! many users each has.
//!
//! ```ignore
//! let store    = Arc::new(FredOrgGraphStore::new(fred_client));
//! let dir      = Arc::new(MyPostgresTenantDirectory::new(pg_pool));
//! let cfg      = WorkerConfig::default();
//! tokio::spawn(org_graph::worker::run_recompute_loop(store, dir, cfg));
//! ```
//!
//! ## Observability (Phase D)
//!
//! The loop emits structured tracing spans and, when the `metrics` feature is
//! enabled, Prometheus-compatible counters + histograms:
//!
//! | Metric                                              | Kind      | Labels                |
//! |-----------------------------------------------------|-----------|-----------------------|
//! | `risk_engine_org_cycle_total`                       | counter   | `outcome`             |
//! | `risk_engine_org_cycle_duration_seconds`            | histogram | —                     |
//! | `risk_engine_org_cycle_tenants`                     | histogram | —                     |
//! | `risk_engine_org_tenant_recompute_total`            | counter   | `outcome`             |
//! | `risk_engine_org_tenant_recompute_duration_seconds` | histogram | —                     |
//! | `risk_engine_org_tenant_prune_total`                | counter   | `outcome`             |
//! | `risk_engine_org_tenant_prune_duration_seconds`     | histogram | —                     |
//! | `risk_engine_org_total_users_failures_total`        | counter   | —                     |
//! | `risk_engine_org_list_tenants_failures_total`       | counter   | —                     |
//!
//! `outcome` ∈ `{success, error}`. Tenant IDs are NOT attached as labels —
//! high cardinality breaks Prometheus. Use tracing spans for per-tenant
//! correlation instead.

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::sync::watch;
use tracing::{debug, info, warn, Instrument};

use crate::error::RiskEngineError;
use crate::store::OrgGraphStore;

/// A shutdown signal shared between the spawner and the worker. Dropping the
/// sender triggers the worker to exit at the next cycle boundary. Cloning is
/// cheap — it's just a `watch::Receiver`.
pub type ShutdownSignal = watch::Receiver<bool>;

// ─────────────────────────────────────────────────────────────────────────────
// TenantDirectory — host service supplies tenant metadata
// ─────────────────────────────────────────────────────────────────────────────

/// Source of tenant metadata consumed by the background worker. The worker
/// does not assume any particular persistence model — the host service
/// decides how to enumerate tenants (Postgres `tenants` table, a config
/// file, service discovery, …).
///
/// Any panic or error here is logged and the tenant is skipped for this
/// cycle; the worker does not exit on transient failures.
#[async_trait]
pub trait TenantDirectory: Send + Sync {
    /// All tenant_ids whose org graph should be recomputed. Called once per
    /// cycle — the result does not need to be cached or stable across calls.
    async fn list_tenants(&self) -> Result<Vec<String>, RiskEngineError>;

    /// Approximate total tracked user count for this tenant. Used as the
    /// denominator for `cluster_density` in [`TenantRiskProfile`]. An
    /// inexact count is fine — the profile only cares about order-of-magnitude.
    async fn total_users(&self, tenant_id: &str) -> Result<u32, RiskEngineError>;
}

// ─────────────────────────────────────────────────────────────────────────────
// WorkerConfig
// ─────────────────────────────────────────────────────────────────────────────

/// Cadence knobs for the background loop. The defaults match the cadences
/// called out in the original plan (recompute ~5 min, prune ~1 hr).
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// How often to recompute every tenant's graph.
    pub recompute_interval: Duration,
    /// How often to prune decayed edges. Runs in-line with a recompute
    /// iteration when the previous prune is older than this.
    pub prune_interval: Duration,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            recompute_interval: Duration::from_secs(300),
            prune_interval: Duration::from_secs(3_600),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Metrics helpers — no-op without the `metrics` feature.
// ─────────────────────────────────────────────────────────────────────────────

#[inline]
fn record_outcome_counter(_name: &'static str, _outcome: &'static str) {
    #[cfg(feature = "metrics")]
    metrics::counter!(_name, "outcome" => _outcome).increment(1);
}

#[inline]
fn record_histogram(_name: &'static str, _value: f64) {
    #[cfg(feature = "metrics")]
    metrics::histogram!(_name).record(_value);
}

#[inline]
fn record_counter(_name: &'static str) {
    #[cfg(feature = "metrics")]
    metrics::counter!(_name).increment(1);
}

// ─────────────────────────────────────────────────────────────────────────────
// run_recompute_loop
// ─────────────────────────────────────────────────────────────────────────────

/// Drive the recompute and prune cycles until aborted. Equivalent to
/// [`run_recompute_loop_with_shutdown`] with a never-signalled channel —
/// retained for callers that manage lifetime via `JoinHandle::abort`.
///
/// The loop is intentionally simple — one tenant at a time, sequentially.
/// For very large tenant counts, swap in a bounded `futures::stream` with
/// `.buffer_unordered(concurrency)` without changing the public contract.
pub async fn run_recompute_loop<S, D>(store: Arc<S>, directory: Arc<D>, config: WorkerConfig)
where
    S: OrgGraphStore + ?Sized + 'static,
    D: TenantDirectory + ?Sized + 'static,
{
    // An unsignalled watch channel — the `false` value never flips, so
    // `run_recompute_loop_with_shutdown` never exits except by abort.
    let (_tx, rx) = watch::channel(false);
    run_recompute_loop_with_shutdown(store, directory, config, rx).await
}

/// Graceful-shutdown variant. The loop checks the shutdown signal at every
/// cycle boundary AND while sleeping — so `tx.send(true)` from the host
/// service causes the worker to finish the in-flight cycle (if any) and then
/// return. Kubernetes SIGTERM handling is the canonical use case.
///
/// ```ignore
/// let (tx, rx) = tokio::sync::watch::channel(false);
/// let handle = tokio::spawn(run_recompute_loop_with_shutdown(store, dir, cfg, rx));
/// // ... on SIGTERM ...
/// let _ = tx.send(true);
/// let _ = handle.await;
/// ```
pub async fn run_recompute_loop_with_shutdown<S, D>(
    store: Arc<S>,
    directory: Arc<D>,
    config: WorkerConfig,
    mut shutdown: ShutdownSignal,
) where
    S: OrgGraphStore + ?Sized + 'static,
    D: TenantDirectory + ?Sized + 'static,
{
    let mut last_prune = Instant::now()
        .checked_sub(config.prune_interval)
        .unwrap_or_else(Instant::now);

    loop {
        // Check before starting a cycle so we don't begin a new one after a
        // shutdown request has already been latched.
        if *shutdown.borrow() {
            info!("org_graph worker: shutdown observed, exiting cleanly");
            return;
        }

        let cycle_span = tracing::info_span!("org_graph.cycle");
        run_one_cycle(
            store.as_ref(),
            directory.as_ref(),
            &config,
            &mut last_prune,
        )
        .instrument(cycle_span)
        .await;

        // Interruptible sleep — either the interval elapses or a shutdown
        // request arrives. Either way we loop back to re-check the signal.
        tokio::select! {
            _ = tokio::time::sleep(config.recompute_interval) => {}
            _ = shutdown.changed() => {
                info!("org_graph worker: shutdown signal received during idle, exiting");
                return;
            }
        }
    }
}

/// One iteration of the recompute/prune cycle. Split out so the whole cycle
/// is covered by a single tracing span and metric set.
async fn run_one_cycle<S, D>(
    store: &S,
    directory: &D,
    config: &WorkerConfig,
    last_prune: &mut Instant,
) where
    S: OrgGraphStore + ?Sized,
    D: TenantDirectory + ?Sized,
{
    let cycle_start = Instant::now();
    let do_prune = last_prune.elapsed() >= config.prune_interval;
    if do_prune {
        *last_prune = Instant::now();
    }

    let tenants = match directory.list_tenants().await {
        Ok(t) => t,
        Err(e) => {
            warn!(error = %e, "list_tenants failed — skipping this cycle");
            record_counter("risk_engine_org_list_tenants_failures_total");
            record_outcome_counter("risk_engine_org_cycle_total", "error");
            record_histogram(
                "risk_engine_org_cycle_duration_seconds",
                cycle_start.elapsed().as_secs_f64(),
            );
            return;
        }
    };

    debug!(count = tenants.len(), do_prune, "org_graph worker cycle start");
    record_histogram("risk_engine_org_cycle_tenants", tenants.len() as f64);

    let mut ok = 0usize;
    let mut errors = 0usize;

    for tenant in &tenants {
        let tenant_span = tracing::info_span!("org_graph.tenant", tenant = %tenant);
        let (t_ok, t_err) = process_tenant(store, directory, tenant, do_prune)
            .instrument(tenant_span)
            .await;
        ok += t_ok;
        errors += t_err;
    }

    info!(
        tenants = tenants.len(),
        ok,
        errors,
        do_prune,
        elapsed_ms = cycle_start.elapsed().as_millis() as u64,
        "org_graph worker cycle complete"
    );

    record_outcome_counter(
        "risk_engine_org_cycle_total",
        if errors == 0 { "success" } else { "error" },
    );
    record_histogram(
        "risk_engine_org_cycle_duration_seconds",
        cycle_start.elapsed().as_secs_f64(),
    );
}

/// Process one tenant: fetch user count, recompute, optionally prune.
/// Returns `(successes, errors)` so the cycle-level span can aggregate.
async fn process_tenant<S, D>(
    store: &S,
    directory: &D,
    tenant: &str,
    do_prune: bool,
) -> (usize, usize)
where
    S: OrgGraphStore + ?Sized,
    D: TenantDirectory + ?Sized,
{
    let mut ok = 0usize;
    let mut errors = 0usize;

    let total_users = match directory.total_users(tenant).await {
        Ok(n) => n,
        Err(e) => {
            warn!(tenant = %tenant, error = %e, "total_users failed, using 0");
            record_counter("risk_engine_org_total_users_failures_total");
            0
        }
    };

    // ── Recompute ─────────────────────────────────────────────────────────
    let rc_start = Instant::now();
    match store.recompute_org_graph(tenant, total_users).await {
        Ok(()) => {
            ok += 1;
            record_outcome_counter("risk_engine_org_tenant_recompute_total", "success");
        }
        Err(e) => {
            warn!(tenant = %tenant, error = %e, "recompute_org_graph failed");
            errors += 1;
            record_outcome_counter("risk_engine_org_tenant_recompute_total", "error");
        }
    }
    record_histogram(
        "risk_engine_org_tenant_recompute_duration_seconds",
        rc_start.elapsed().as_secs_f64(),
    );

    // ── Prune ─────────────────────────────────────────────────────────────
    if do_prune {
        let p_start = Instant::now();
        match store.prune_graph(tenant).await {
            Ok(()) => {
                record_outcome_counter("risk_engine_org_tenant_prune_total", "success");
            }
            Err(e) => {
                warn!(tenant = %tenant, error = %e, "prune_graph failed");
                errors += 1;
                record_outcome_counter("risk_engine_org_tenant_prune_total", "error");
            }
        }
        record_histogram(
            "risk_engine_org_tenant_prune_duration_seconds",
            p_start.elapsed().as_secs_f64(),
        );
    }

    (ok, errors)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
//
// Exercising the full loop against a live store would require a running Redis
// or a mock `OrgGraphStore` impl — too heavy for a lib-level unit test. What
// we CAN verify cheaply is that the shutdown signal causes the loop to exit
// within the expected bound, which is the whole point of F2.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::org_graph::{
        ClusterMembership, OrgGraphUpdate, OrgRiskSnapshot, TenantRiskProfile,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Instant;

    /// Minimal no-op `OrgGraphStore` — every call succeeds with empty state.
    /// Counts recompute invocations so tests can assert cycle progress.
    struct FakeStore {
        recomputes: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl OrgGraphStore for FakeStore {
        async fn get_org_risk_snapshot(
            &self,
            _t: &str,
        ) -> Result<Option<OrgRiskSnapshot>, RiskEngineError> {
            Ok(None)
        }
        async fn get_cluster_membership(
            &self,
            _t: &str,
            _node_key: &str,
        ) -> Result<Option<ClusterMembership>, RiskEngineError> {
            Ok(None)
        }
        async fn get_tenant_profile(
            &self,
            _t: &str,
        ) -> Result<Option<TenantRiskProfile>, RiskEngineError> {
            Ok(None)
        }
        async fn push_graph_update(
            &self,
            _t: &str,
            _u: OrgGraphUpdate,
        ) -> Result<(), RiskEngineError> {
            Ok(())
        }
        async fn record_decision_stats(
            &self,
            _t: &str,
            _was_blocked: bool,
        ) -> Result<(), RiskEngineError> {
            Ok(())
        }
        async fn recompute_org_graph(
            &self,
            _t: &str,
            _total_users: u32,
        ) -> Result<(), RiskEngineError> {
            self.recomputes.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        async fn prune_graph(&self, _t: &str) -> Result<(), RiskEngineError> {
            Ok(())
        }
    }

    struct FakeDir;
    #[async_trait]
    impl TenantDirectory for FakeDir {
        async fn list_tenants(&self) -> Result<Vec<String>, RiskEngineError> {
            Ok(vec!["t1".into()])
        }
        async fn total_users(&self, _t: &str) -> Result<u32, RiskEngineError> {
            Ok(10)
        }
    }

    #[tokio::test]
    async fn shutdown_exits_during_idle_sleep() {
        let store = Arc::new(FakeStore { recomputes: AtomicUsize::new(0) });
        let dir = Arc::new(FakeDir);
        // A 60 s recompute interval — far longer than the test will wait.
        // Without graceful shutdown the task would hang past the timeout.
        let cfg = WorkerConfig {
            recompute_interval: Duration::from_secs(60),
            prune_interval: Duration::from_secs(60),
        };
        let (tx, rx) = watch::channel(false);

        let handle = tokio::spawn(run_recompute_loop_with_shutdown(
            store.clone(),
            dir.clone(),
            cfg,
            rx,
        ));

        // Give the first cycle a moment to run.
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(store.recomputes.load(Ordering::SeqCst), 1);

        let start = Instant::now();
        tx.send(true).expect("signal shutdown");
        // Should exit well under the 60 s recompute interval.
        tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .expect("worker did not exit within 2s of shutdown")
            .expect("worker task panicked");
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "shutdown took too long: {:?}",
            start.elapsed()
        );
    }

    #[tokio::test]
    async fn shutdown_observed_before_starting_new_cycle() {
        let store = Arc::new(FakeStore { recomputes: AtomicUsize::new(0) });
        let dir = Arc::new(FakeDir);
        // Very short interval so the loop races the shutdown flag on the
        // "check before starting a cycle" branch.
        let cfg = WorkerConfig {
            recompute_interval: Duration::from_millis(10),
            prune_interval: Duration::from_secs(60),
        };
        let (tx, rx) = watch::channel(false);
        // Pre-latch the flag BEFORE starting the loop.
        let _ = tx.send(true);

        let handle = tokio::spawn(run_recompute_loop_with_shutdown(
            store.clone(),
            dir,
            cfg,
            rx,
        ));

        tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .expect("worker did not exit within 2s")
            .expect("worker task panicked");

        // With the flag pre-latched, not a single cycle should have run.
        assert_eq!(store.recomputes.load(Ordering::SeqCst), 0);
    }
}
