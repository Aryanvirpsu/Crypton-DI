//! Thin Axum wrapper exposing [`evaluate`] over HTTP.
//!
//! Two routes:
//!   - `POST /evaluate` — body: `RiskContext` JSON, response: `RiskDecision`
//!   - `GET  /health`   — always 200, body `{"status":"ok"}`
//!
//! The router holds an `AppState<S>` parameterized over a concrete
//! [`SignalStore`] (e.g. [`crate::adapters::fred_store::FredSignalStore`]).
//! Keeping `S` generic avoids a `?Sized` relaxation on [`evaluate`] and keeps
//! the hot path monomorphic.
//!
//! This is a **minimum viable HTTP surface**. Production deployments should
//! layer on auth (mTLS / bearer), rate-limiting, and request-size limits
//! before exposing the endpoint publicly.
//!
//! ## Observability (Phase D)
//!
//! - A `request_id` is extracted from the `X-Request-ID` header. Absent or
//!   malformed headers cause a fresh UUID v4 to be generated. The final
//!   value is written back to `RiskContext::request_id` (replacing the
//!   client-supplied one) AND returned on the response as `X-Request-ID` so
//!   downstream log correlation has a consistent key.
//! - The handler is instrumented with `tracing::info_span!("http_evaluate")`
//!   carrying `request_id`, `tenant_id`, `user_id`, `action`. Inner engine
//!   spans inherit it.
//! - When the `metrics` feature is enabled, the handler emits
//!   `risk_engine_http_evaluate_total{outcome}` (success|error) and
//!   `risk_engine_http_evaluate_duration_seconds`.

use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    extract::State,
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tracing::{info, Instrument};
use uuid::Uuid;

use crate::context::RiskContext;
use crate::store::SignalStore;

#[cfg(not(feature = "metrics"))]
use crate::engine::evaluate;

#[cfg(feature = "metrics")]
use crate::adapters::metrics::evaluate_with_metrics;

const X_REQUEST_ID: &str = "x-request-id";

/// Maximum accepted request body size for `/evaluate`. 64 KiB is generous for
/// a `RiskContext` JSON (typical payloads are under 4 KiB) while small enough
/// to make large-body DoS uninteresting.
pub const DEFAULT_BODY_LIMIT_BYTES: usize = 64 * 1024;

/// Default hard time budget for a single `/evaluate` request end-to-end. The
/// engine's own timeouts are tighter (20–50 ms per Redis op); this is a
/// backstop against a pathologically slow store or a stuck Tokio task.
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Liveness/readiness probe supplied by the host. Implementations should do
/// the minimum work that proves the dependency is reachable (a Redis `PING`,
/// a Postgres `SELECT 1`) and return quickly — the handler wraps the probe
/// in a 1-second budget.
///
/// A `None` probe on the router means `/ready` always returns 200 — use
/// only if you have external readiness (e.g. a proxy already checking
/// upstream health).
#[async_trait::async_trait]
pub trait ReadinessProbe: Send + Sync + 'static {
    /// Return `Ok(())` if the system is ready to serve traffic. Any `Err`
    /// value becomes a 503 response with the error's `Display` as the body.
    async fn check(&self) -> Result<(), String>;
}

/// Shared application state: the concrete `SignalStore` the engine talks to,
/// plus an optional readiness probe.
pub struct AppState<S: SignalStore + 'static> {
    pub store: Arc<S>,
    pub readiness: Option<Arc<dyn ReadinessProbe>>,
}

impl<S: SignalStore + 'static> AppState<S> {
    /// Convenience constructor for the common case (no readiness probe).
    pub fn new(store: Arc<S>) -> Self {
        Self { store, readiness: None }
    }

    /// Attach a readiness probe. Returns self for builder-style chaining.
    pub fn with_readiness(mut self, probe: Arc<dyn ReadinessProbe>) -> Self {
        self.readiness = Some(probe);
        self
    }
}

// Manual Clone so we don't require `S: Clone`.
impl<S: SignalStore + 'static> Clone for AppState<S> {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            readiness: self.readiness.clone(),
        }
    }
}

/// Build the router with default middleware (64 KiB body cap, 5 s timeout).
/// The caller mounts it on an `axum::serve(listener, router)`.
///
/// For a production deployment, layer additional middleware (auth, per-IP
/// rate-limit, request logging) with `.layer(...)` on the returned router.
pub fn router<S: SignalStore + 'static>(state: AppState<S>) -> Router {
    router_with_limits(state, DEFAULT_BODY_LIMIT_BYTES, DEFAULT_REQUEST_TIMEOUT)
}

/// Variant of [`router`] with explicit body-size and request-timeout budgets.
/// Intended for tests and deployments that need tighter or looser limits than
/// the defaults.
pub fn router_with_limits<S: SignalStore + 'static>(
    state: AppState<S>,
    body_limit_bytes: usize,
    request_timeout: Duration,
) -> Router {
    Router::new()
        .route("/evaluate", post(evaluate_handler::<S>))
        // `/health` is a pure liveness probe — it returns 200 whenever the
        // process is reachable. It does NOT check downstreams; use `/ready`
        // for that. Kubernetes should wire this to `livenessProbe`.
        .route("/health", get(health_handler))
        // `/ready` runs the host-supplied [`ReadinessProbe`] (if any) against
        // a 1 s budget. Failures return 503 with a short diagnostic body.
        // Wire to `readinessProbe` — a 503 will drop the pod from the
        // load-balancer until the downstream recovers.
        .route("/ready", get(ready_handler::<S>))
        // Order matters: the body limit must wrap the handler so oversized
        // payloads are rejected before deserialisation. The timeout layer
        // wraps the whole stack so even the parse step is time-bounded.
        .layer(RequestBodyLimitLayer::new(body_limit_bytes))
        // `with_status_code` returns 408 on elapsed budget (vs the deprecated
        // `new` which emits 500). 408 is the correct semantic for "client
        // request exceeded the server's time budget".
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            request_timeout,
        ))
        .with_state(state)
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
}

#[derive(Serialize)]
struct ReadyBody {
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

async fn health_handler() -> Json<Health> {
    Json(Health { status: "ok" })
}

/// Readiness budget. Kept small because k8s calls this on an interval; a
/// slow readiness probe blocks node rollouts.
const READINESS_TIMEOUT: Duration = Duration::from_secs(1);

async fn ready_handler<S: SignalStore + 'static>(
    State(state): State<AppState<S>>,
) -> Response {
    // No probe configured → treat as always-ready. Matches the simplest
    // deployment mode where a proxy is doing upstream health checks.
    let probe = match &state.readiness {
        Some(p) => p.clone(),
        None => return (StatusCode::OK, Json(ReadyBody { status: "ok", error: None })).into_response(),
    };

    let result = tokio::time::timeout(READINESS_TIMEOUT, async move { probe.check().await }).await;

    match result {
        Ok(Ok(())) => (
            StatusCode::OK,
            Json(ReadyBody { status: "ok", error: None }),
        )
            .into_response(),
        Ok(Err(msg)) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ReadyBody { status: "unavailable", error: Some(msg) }),
        )
            .into_response(),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ReadyBody {
                status: "unavailable",
                error: Some(format!("readiness probe exceeded {}ms", READINESS_TIMEOUT.as_millis())),
            }),
        )
            .into_response(),
    }
}

/// Extract a `request_id` from the `X-Request-ID` header. If absent or
/// unparseable, generate a fresh UUID v4 so the request is still
/// trace-correlatable end-to-end.
fn extract_request_id(headers: &HeaderMap) -> Uuid {
    headers
        .get(X_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::from_str(s.trim()).ok())
        .unwrap_or_else(Uuid::new_v4)
}

#[inline]
fn record_outcome(_outcome: &'static str) {
    #[cfg(feature = "metrics")]
    metrics::counter!("risk_engine_http_evaluate_total", "outcome" => _outcome).increment(1);
}

#[inline]
fn record_duration(_value: f64) {
    #[cfg(feature = "metrics")]
    metrics::histogram!("risk_engine_http_evaluate_duration_seconds").record(_value);
}

async fn evaluate_handler<S: SignalStore + 'static>(
    State(state): State<AppState<S>>,
    headers: HeaderMap,
    Json(mut ctx): Json<RiskContext>,
) -> Result<Response, EngineHttpError> {
    let start = Instant::now();
    let request_id = extract_request_id(&headers);

    // Always override the client-supplied request_id with the one we trust.
    ctx.request_id = request_id;

    let span = tracing::info_span!(
        "http_evaluate",
        request_id = %request_id,
        tenant_id = %ctx.tenant_id,
        user_id = %ctx.user_id,
        action = %ctx.action.label(),
    );

    let decision = async {
        // When the `metrics` feature is on, route through the metrics
        // wrapper so engine-level counters/histograms fire on every HTTP
        // request. Otherwise call the bare engine.
        #[cfg(feature = "metrics")]
        let d = evaluate_with_metrics(ctx, state.store.as_ref()).await;
        #[cfg(not(feature = "metrics"))]
        let d = evaluate(ctx, state.store.as_ref()).await;
        info!(decision = ?d.decision, score = d.score, "http_evaluate complete");
        d
    }
    .instrument(span)
    .await;

    record_outcome("success");
    record_duration(start.elapsed().as_secs_f64());

    // Echo the request_id on the response so downstream systems can
    // correlate logs without re-parsing the body.
    let mut response = Json(decision).into_response();
    if let Ok(hv) = HeaderValue::from_str(&request_id.to_string()) {
        response
            .headers_mut()
            .insert(HeaderName::from_static(X_REQUEST_ID), hv);
    }
    Ok(response)
}

// ─────────────────────────────────────────────────────────────────────────────
// Errors
//
// `evaluate` itself never returns `Err` — it is infallible and always produces
// a decision (including Deny/Challenge on degraded signals). We still wrap the
// handler result in `Result` so future additions (e.g. a parse-policy-header
// step) can surface 4xx/5xx cleanly without changing the handler shape.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum EngineHttpError {
    BadRequest(String),
}

impl IntoResponse for EngineHttpError {
    fn into_response(self) -> Response {
        match self {
            EngineHttpError::BadRequest(msg) => {
                record_outcome("error");
                (StatusCode::BAD_REQUEST, Json(ErrBody { error: msg })).into_response()
            }
        }
    }
}

#[derive(Serialize)]
struct ErrBody {
    error: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue};

    #[test]
    fn extract_request_id_uses_header_when_valid() {
        let u = Uuid::new_v4();
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static(X_REQUEST_ID),
            HeaderValue::from_str(&u.to_string()).unwrap(),
        );
        assert_eq!(extract_request_id(&headers), u);
    }

    #[test]
    fn extract_request_id_generates_when_missing() {
        let a = extract_request_id(&HeaderMap::new());
        let b = extract_request_id(&HeaderMap::new());
        assert_ne!(a, b, "each fallback should produce a fresh UUID");
    }

    #[test]
    fn extract_request_id_generates_when_malformed() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static(X_REQUEST_ID),
            HeaderValue::from_static("not-a-uuid"),
        );
        // Must still return SOMETHING — not panic.
        let _ = extract_request_id(&headers);
    }
}
