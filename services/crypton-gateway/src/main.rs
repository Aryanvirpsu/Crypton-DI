//! crypton-gateway
//!
//! A minimal transparent reverse-proxy that sits between the React frontend
//! and the crypton-identity service.
//!
//! Request path (local dev):
//!   Browser → React CRA :3000 → CRA proxy → gateway :8090 → identity :8080
//!
//! Request path (cloudflared tunnel):
//!   Browser → tunnel → React CRA :3000 → CRA proxy → gateway :8090 → identity :8080
//!
//! The gateway:
//!   - Exposes GET /health for its own health check
//!   - Forwards every other request verbatim to IDENTITY_URL (default: http://127.0.0.1:8080)
//!   - Passes all request headers through (including Authorization: Bearer <jwt>)
//!   - Returns the upstream status code, headers, and body unchanged

use axum::{
    body::Body,
    extract::State,
    http::{header, Method, Request, Response, StatusCode},
    routing::get,
    Router,
};
use bytes::Bytes;
use reqwest::Client;
use std::sync::Arc;
use tower_http::{cors::{Any, CorsLayer}, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let port: u16 = std::env::var("APP_PORT")
        .unwrap_or_else(|_| "8090".to_string())
        .parse()
        .map_err(|_| anyhow::anyhow!("APP_PORT must be a valid port number"))?;

    let identity_base = std::env::var("IDENTITY_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());

    tracing::info!(
        "crypton-gateway listening on 0.0.0.0:{port} → identity at {identity_base}"
    );

    let state = Arc::new(GatewayState {
        client: Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()?,
        identity_base,
    });

   let cors = CorsLayer::new()
    .allow_origin(Any)
    .allow_methods(Any)
    .allow_headers(Any);

    let app = Router::new()
        // Gateway's own health endpoint — does NOT proxy to identity
        .route("/health", get(gateway_health))
        // Everything else is forwarded transparently
        .fallback(proxy_handler)
        .with_state(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    axum::serve(listener, app).await?;

    Ok(())
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

async fn proxy_handler(
    State(state): State<Arc<GatewayState>>,
    req: Request<Body>,
) -> Result<Response<Body>, StatusCode> {
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
    let body_bytes: Bytes = axum::body::to_bytes(body, 8 * 1024 * 1024)
        .await
        .map_err(|e| {
            tracing::error!("failed to read request body: {e}");
            StatusCode::BAD_REQUEST
        })?;

    // Build the upstream reqwest request
    let mut builder = state.client.request(method, &upstream_url);

    // Forward all non-hop-by-hop request headers
    for (name, value) in &parts.headers {
        if !HOP_BY_HOP.contains(&name.as_str()) {
            builder = builder.header(name.as_str(), value.as_bytes());
        }
    }

    // Only attach a body if there is one (avoids adding Content-Length: 0 on GETs)
    if !body_bytes.is_empty() {
        builder = builder.body(body_bytes);
    }

    // Execute upstream request
    let upstream_resp = builder.send().await.map_err(|e| {
        tracing::error!("upstream error proxying {upstream_url}: {e}");
        StatusCode::BAD_GATEWAY
    })?;

    // Map upstream status
    let status = StatusCode::from_u16(upstream_resp.status().as_u16())
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    // Collect upstream response headers before consuming the body
    let resp_headers = upstream_resp.headers().clone();

    // Read upstream response body
    let resp_body: Bytes = upstream_resp.bytes().await.map_err(|e| {
        tracing::error!("failed to read upstream response body: {e}");
        StatusCode::BAD_GATEWAY
    })?;

    // Build the downstream response
    let mut resp_builder = Response::builder().status(status);

    for (name, value) in &resp_headers {
        if !HOP_BY_HOP.contains(&name.as_str()) {
            resp_builder = resp_builder.header(name.as_str(), value.as_bytes());
        }
    }

    resp_builder.body(Body::from(resp_body)).map_err(|e| {
        tracing::error!("failed to build downstream response: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}
