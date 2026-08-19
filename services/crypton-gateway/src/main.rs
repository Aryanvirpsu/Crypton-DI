//! crypton-gateway
//!
//! A small, honest reverse proxy that sits between frontends and
//! crypton-identity. This file used to sit alongside ~800 lines of
//! auth/policy/rate-limit/nonce/audit/metrics modules that were never
//! wired into the router (no `mod` declarations existed for them) and,
//! independently, could not have compiled if they had been (they used
//! crates — redis, sqlx, jsonwebtoken, serde, uuid, hyper — that weren't
//! even in Cargo.toml). Rather than resurrect unreviewed dead code, this
//! rewrite keeps the gateway to what it actually does today and documents
//! where the next layer attaches.
//!
//! What this gateway does:
//!   - GET /health — its own liveness check (does not proxy to identity)
//!   - Everything else — forwarded verbatim to IDENTITY_URL, headers and
//!     cookies included (default: http://127.0.0.1:8080)
//!   - Assigns/propagates an `x-request-id` correlation ID across the hop
//!     to identity, and includes it in every log line for that request
//!   - CORS, from a configurable origin allowlist
//!
//! What it deliberately does NOT do yet — these are real future work, not
//! implemented here, see `docs/architecture.md`:
//!   - Verify or introspect JWTs (identity does this; the gateway is not
//!     yet a policy enforcement point)
//!   - Rate limiting / replay protection (would need a shared store —
//!     Redis — which the gateway doesn't currently depend on)
//!   - Agent authorization / delegation checks — this is the extension
//!     point the upcoming Agent Lab work will need: a place to ask
//!     "is this agent allowed to call this route, on this human's behalf,
//!     with this delegated scope?" before the request reaches identity.

use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderName, HeaderValue, Method, Response, StatusCode},
    routing::get,
    Router,
};
use bytes::Bytes;
use reqwest::Client;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const REQUEST_ID_HEADER: &str = "x-request-id";

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct GatewayState {
    client: Client,
    identity_base: String,
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    // Prefer Railway's PORT first
    let port: u16 = std::env::var("PORT")
        .or_else(|_| std::env::var("APP_PORT"))
        .unwrap_or_else(|_| "8090".to_string())
        .parse()
        .map_err(|_| anyhow::anyhow!("PORT (or APP_PORT) must be a valid port number"))?;

    let identity_base =
        std::env::var("IDENTITY_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());

    tracing::info!("crypton-gateway listening on 0.0.0.0:{port} → identity at {identity_base}");

    let state = Arc::new(GatewayState {
        // no_proxy() prevents reqwest from routing gateway→identity traffic
        // through any system HTTP_PROXY env var that might be set
        client: Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()?,
        identity_base,
    });

    let request_id_header = HeaderName::from_static(REQUEST_ID_HEADER);

    let app = Router::new()
        // Gateway's own health endpoint — does NOT proxy to identity
        .route("/health", get(gateway_health))
        // Everything else is forwarded transparently.
        //
        // ── Future extension point ──────────────────────────────────────
        // Agent-identity authorization middleware belongs here, between
        // routing and the proxy fallback — e.g.
        //   .layer(middleware::from_fn_with_state(state.clone(), agent_authz))
        // so it can inspect (and reject) a request before it ever reaches
        // identity. Nothing agent-specific is implemented yet.
        .fallback(proxy_handler)
        .with_state(state)
        .layer(build_cors())
        .layer(
            ServiceBuilder::new()
                // Assign x-request-id first so everything downstream can see it.
                .layer(SetRequestIdLayer::new(
                    request_id_header.clone(),
                    MakeRequestUuid,
                ))
                .layer(
                    TraceLayer::new_for_http().make_span_with(|req: &Request<Body>| {
                        let request_id = req
                            .headers()
                            .get(REQUEST_ID_HEADER)
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("-")
                            .to_string();
                        tracing::info_span!(
                            "http_request",
                            request_id = %request_id,
                            method = %req.method(),
                            path = %req.uri().path(),
                        )
                    }),
                )
                // Copy x-request-id onto the outgoing response.
                .layer(PropagateRequestIdLayer::new(request_id_header)),
        );

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Structured-log-friendly by default (every event carries request_id via
/// the span above). Set LOG_FORMAT=json for machine-parseable output —
/// useful in any environment that ships logs to a collector; plain text
/// stays the default because it's what a developer reads most often.
fn init_tracing() {
    let filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());

    if std::env::var("LOG_FORMAT").as_deref() == Ok("json") {
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer())
            .init();
    }
}

/// CORS origin allowlist — configurable via ALLOWED_ORIGINS (comma-separated),
/// falling back to the local dev origins if unset. There is exactly one
/// frontend now (crypton-console); production deployments should set
/// ALLOWED_ORIGINS explicitly rather than rely on this default.
fn build_cors() -> CorsLayer {
    let origins: Vec<HeaderValue> = std::env::var("ALLOWED_ORIGINS")
        .unwrap_or_else(|_| {
            "http://localhost:3000,http://127.0.0.1:3000,http://localhost:3001,http://127.0.0.1:3001"
                .to_string()
        })
        .split(',')
        .filter_map(|s| {
            let s = s.trim();
            if s.is_empty() {
                None
            } else {
                s.parse().ok()
            }
        })
        .collect();

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            HeaderName::from_static(REQUEST_ID_HEADER),
        ])
}

// ── /health ───────────────────────────────────────────────────────────────────

async fn gateway_health() -> &'static str {
    "crypton-gateway ok"
}

// ── Transparent proxy ─────────────────────────────────────────────────────────

/// Hop-by-hop headers that must NOT be forwarded upstream or downstream.
/// Per RFC 7230 §6.1 — these are connection-specific and must not be relayed.
const HOP_BY_HOP: &[&str] = &[
    "host",
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
];

/// A gateway-level failure (as opposed to identity returning a normal error
/// response, which is relayed unchanged). Shaped like crypton-id's error
/// envelope so a frontend doesn't need to special-case "which hop failed".
fn gateway_error(status: StatusCode, code: &str) -> Response<Body> {
    let body = serde_json::json!({
        "success": false,
        "error": { "code": code, "message": format!("Gateway error: {code}") }
    });
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap_or_else(|_| Response::new(Body::from("{}")))
}

async fn proxy_handler(
    State(state): State<Arc<GatewayState>>,
    req: Request<Body>,
) -> Response<Body> {
    // Destructure the incoming request so we can consume the body separately
    let (parts, body) = req.into_parts();

    // Build the upstream URL: identity_base + path?query
    let path_and_query = parts
        .uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    let upstream_url = format!("{}{}", state.identity_base, path_and_query);

    // Convert axum's Method to reqwest's Method (both wrap http 1.x)
    let method = reqwest::Method::from_bytes(parts.method.as_str().as_bytes())
        .unwrap_or(reqwest::Method::GET);

    // Read the full request body (cap at 8 MiB to prevent memory exhaustion)
    let body_bytes: Bytes = match axum::body::to_bytes(body, 8 * 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("failed to read request body: {e}");
            return gateway_error(StatusCode::BAD_REQUEST, "invalid_request_body");
        }
    };

    // Build the upstream reqwest request
    let mut builder = state.client.request(method, &upstream_url);

    // Forward all non-hop-by-hop request headers (includes Authorization,
    // cookies, and x-request-id set by SetRequestIdLayer above).
    for (name, value) in &parts.headers {
        if !HOP_BY_HOP.contains(&name.as_str()) {
            builder = builder.header(name.as_str(), value.as_bytes());
        }
    }

    if !body_bytes.is_empty() {
        builder = builder.body(body_bytes);
    }

    let upstream_resp = match builder.send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("upstream error proxying {upstream_url}: {e}");
            return gateway_error(StatusCode::BAD_GATEWAY, "upstream_unreachable");
        }
    };

    let status = StatusCode::from_u16(upstream_resp.status().as_u16())
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let resp_headers = upstream_resp.headers().clone();

    let resp_body: Bytes = match upstream_resp.bytes().await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("failed to read upstream response body: {e}");
            return gateway_error(StatusCode::BAD_GATEWAY, "upstream_response_unreadable");
        }
    };

    let mut resp_builder = Response::builder().status(status);
    for (name, value) in &resp_headers {
        if !HOP_BY_HOP.contains(&name.as_str()) {
            resp_builder = resp_builder.header(name.as_str(), value.as_bytes());
        }
    }

    resp_builder
        .body(Body::from(resp_body))
        .unwrap_or_else(|e| {
            tracing::error!("failed to build downstream response: {e}");
            gateway_error(StatusCode::INTERNAL_SERVER_ERROR, "response_build_failed")
        })
}
