//! crypton-gateway
//!
//! Two-listener Axum server:
//!
//!   External  0.0.0.0:8090   — proxy traffic with full auth/policy middleware
//!   Internal  127.0.0.1:9090 — metrics, diagnostics, audit ingest (never exposed publicly)
//!
//! Middleware order on the external listener (outermost → innermost):
//!   auth → nonce → rate_limit → device_context → step_up_context → policy → proxy
//!
//! Startup aborts if JWT_SECRET or INTERNAL_TOKEN are not set in the environment.

mod audit;
#[cfg(test)]
mod gateway_tests;
mod middleware;
mod routes;
mod state;

use std::net::SocketAddr;

use axum::{
    body::Body,
    extract::State,
    http::{HeaderName, Method, Request, Response, StatusCode},
    middleware::{from_fn, from_fn_with_state},
    routing::{get, post},
    Router,
};
use bytes::Bytes;
use state::{AppState, Metrics};
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use middleware::{
    auth::{require_admin, validate_jwt},
    device_context::inject_device_context,
    nonce::enforce_nonce,
    policy::enforce_policy,
    rate_limit::limit,
    step_up_context::inject_step_up_context,
};

const REQUEST_ID_HEADER: &str = "x-request-id";

/// Plain-text logs by default; set LOG_FORMAT=json for machine-parseable
/// output. Every request-scoped event carries `request_id` via the span in
/// [`with_request_id`].
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

/// Assign (or keep a caller-supplied) `x-request-id`, log it on the request
/// span, forward it to the upstream (the proxy copies non-hop-by-hop headers)
/// and echo it on the response. Applied outermost so requests rejected by the
/// security middleware also carry a request ID.
fn with_request_id(router: Router) -> Router {
    let header = HeaderName::from_static(REQUEST_ID_HEADER);
    router
        .layer(TraceLayer::new_for_http().make_span_with(|req: &Request<Body>| {
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
        }))
        .layer(PropagateRequestIdLayer::new(header.clone()))
        .layer(SetRequestIdLayer::new(header, MakeRequestUuid))
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    init_tracing();

    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let database_url =
        std::env::var("DATABASE_URL").map_err(|_| anyhow::anyhow!("DATABASE_URL is required"))?;

    // AppState::new validates JWT_SECRET and INTERNAL_TOKEN — startup aborts
    // here if either is missing, before any port is bound.
    let app_state = AppState::new(&redis_url, &database_url).await?;

    let ext_port: u16 = std::env::var("PORT")
        .or_else(|_| std::env::var("APP_PORT"))
        .unwrap_or_else(|_| "8090".to_string())
        .parse()
        .map_err(|_| anyhow::anyhow!("PORT (or APP_PORT) must be a valid port number"))?;

    let int_port: u16 = std::env::var("INTERNAL_PORT")
        .unwrap_or_else(|_| "9090".to_string())
        .parse()
        .map_err(|_| anyhow::anyhow!("INTERNAL_PORT must be a valid port number"))?;

    let external_app = build_external_router(app_state.clone());
    let internal_app = build_internal_router(app_state);

    let ext_addr = format!("0.0.0.0:{ext_port}");
    let int_addr = format!("127.0.0.1:{int_port}");

    tracing::info!("crypton-gateway external listener: {ext_addr}");
    tracing::info!("crypton-gateway internal listener: {int_addr}");

    let ext_listener = tokio::net::TcpListener::bind(&ext_addr).await?;
    let int_listener = tokio::net::TcpListener::bind(&int_addr).await?;

    // Both listeners run concurrently. If either exits the process exits.
    // External listener uses ConnectInfo so rate_limit middleware can read the
    // real peer address and decide whether to trust X-Forwarded-For.
    tokio::try_join!(
        axum::serve(
            ext_listener,
            external_app.into_make_service_with_connect_info::<SocketAddr>(),
        ),
        axum::serve(int_listener, internal_app),
    )?;

    Ok(())
}

// ── External router ───────────────────────────────────────────────────────────

fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin([
            "https://app.cryptonid.tech".parse().unwrap(),
            "https://demo.cryptonid.tech".parse().unwrap(),
            "http://localhost:3000".parse().unwrap(),
            "http://127.0.0.1:3000".parse().unwrap(),
        ])
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(Any)
}

fn build_external_router(state: AppState) -> Router {
    // ServiceBuilder applies layers outermost-first: the first .layer() runs
    // first on every incoming request. Order matches the stack doc above.
    let auth_stack = ServiceBuilder::new()
        .layer(from_fn_with_state(state.clone(), validate_jwt))
        .layer(from_fn_with_state(state.clone(), enforce_nonce))
        .layer(from_fn_with_state(state.clone(), limit))
        .layer(from_fn_with_state(state.clone(), inject_device_context))
        .layer(from_fn_with_state(state.clone(), inject_step_up_context))
        .layer(from_fn_with_state(state.clone(), enforce_policy));

    let proxy_router = Router::new()
        .fallback(proxy_handler)
        .layer(auth_stack)
        .with_state(state.clone());

    let router = Router::new()
        .route("/health", get(|| async { "crypton-gateway ok" }))
        .merge(proxy_router)
        .layer(cors_layer());
    with_request_id(router)
}

// ── Internal router ───────────────────────────────────────────────────────────

fn build_internal_router(state: AppState) -> Router {
    // Diagnostics require an authenticated admin JWT.
    // Layers run outermost-first: validate_jwt runs first, then require_admin
    // checks the role claim it inserted.
    let diag_router = Router::new()
        .route("/user/:user_id", get(routes::diagnostics::user))
        .route("/device/:device_id", get(routes::diagnostics::device))
        .route("/ip/:ip", get(routes::diagnostics::ip))
        .layer(from_fn(require_admin))
        .layer(from_fn_with_state(state.clone(), validate_jwt));

    let router = Router::new()
        .route("/internal/metrics", get(routes::metrics::handler))
        .nest("/internal/diagnostics", diag_router)
        .route("/internal/audit/ingest", post(audit::ingest))
        .with_state(state);
    with_request_id(router)
}

// ── Transparent proxy ─────────────────────────────────────────────────────────

/// Hop-by-hop headers that must not be forwarded (RFC 7230 §6.1).
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

/// Client-supplied "where did this request come from" headers. The gateway is
/// the trust boundary: it determines the real client IP from the TCP peer and
/// rewrites `x-forwarded-for` itself, so any inbound copies of these — which a
/// direct-to-gateway attacker can set freely — must be stripped before
/// proxying. Otherwise a caller could spoof their IP / country and defeat the
/// identity service's geo_block / tor_block / geo_velocity evaluators whenever
/// TRUST_PROXY_HEADERS is enabled there.
const CLIENT_SPOOFABLE_FORWARD_HEADERS: &[&str] = &[
    "x-forwarded-for",
    "x-real-ip",
    "true-client-ip",
    "cf-connecting-ip",
    "cf-ipcountry",
    "forwarded",
];

fn is_strippable_forward_header(name: &str) -> bool {
    HOP_BY_HOP.contains(&name) || CLIENT_SPOOFABLE_FORWARD_HEADERS.contains(&name)
}

async fn proxy_handler(
    State(st): State<AppState>,
    req: Request<Body>,
) -> Result<Response<Body>, StatusCode> {
    // Resolve the trusted client IP from the TCP peer (honouring XFF only from a
    // trusted private/loopback peer) BEFORE consuming the request.
    let client_ip = middleware::client_ip(&req);

    let (parts, body) = req.into_parts();

    let path_and_query = parts
        .uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    let upstream_url = format!("{}{}", st.identity_base, path_and_query);

    let method = reqwest::Method::from_bytes(parts.method.as_str().as_bytes())
        .unwrap_or(reqwest::Method::GET);

    let body_bytes: Bytes = axum::body::to_bytes(body, 8 * 1024 * 1024)
        .await
        .map_err(|e| {
            tracing::error!("proxy: failed to read request body: {e}");
            StatusCode::BAD_REQUEST
        })?;

    let mut builder = st.http_client.request(method, &upstream_url);

    for (name, value) in &parts.headers {
        if !is_strippable_forward_header(name.as_str()) {
            builder = builder.header(name.as_str(), value.as_bytes());
        }
    }

    // Set X-Forwarded-For to the gateway-resolved client IP. This is the only
    // forwarding header the upstream should trust — it replaces any spoofed
    // inbound copy stripped above. If we couldn't resolve a peer IP, we send
    // nothing rather than a guessable value.
    if let Some(ip) = client_ip {
        builder = builder.header("x-forwarded-for", ip.to_string());
    }

    if !body_bytes.is_empty() {
        builder = builder.body(body_bytes);
    }

    let upstream_resp = builder.send().await.map_err(|e| {
        tracing::error!("proxy: upstream error {upstream_url}: {e}");
        Metrics::inc(&st.metrics.proxy_upstream_error);
        StatusCode::BAD_GATEWAY
    })?;

    let status_u16 = upstream_resp.status().as_u16();
    let status = StatusCode::from_u16(status_u16).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    match status_u16 {
        200..=299 => Metrics::inc(&st.metrics.proxy_upstream_2xx),
        400..=499 => Metrics::inc(&st.metrics.proxy_upstream_4xx),
        500..=599 => Metrics::inc(&st.metrics.proxy_upstream_5xx),
        _ => {}
    }

    let resp_headers = upstream_resp.headers().clone();
    let resp_body: Bytes = upstream_resp.bytes().await.map_err(|e| {
        tracing::error!("proxy: failed to read upstream response: {e}");
        StatusCode::BAD_GATEWAY
    })?;

    let mut resp_builder = Response::builder().status(status);
    for (name, value) in &resp_headers {
        if !HOP_BY_HOP.contains(&name.as_str()) {
            resp_builder = resp_builder.header(name.as_str(), value.as_bytes());
        }
    }

    resp_builder.body(Body::from(resp_body)).map_err(|e| {
        tracing::error!("proxy: failed to build response: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[cfg(test)]
mod proxy_tests {
    use super::*;

    #[test]
    fn spoofable_forward_headers_are_stripped() {
        // Client-supplied IP/geo headers must never be forwarded verbatim.
        assert!(is_strippable_forward_header("x-forwarded-for"));
        assert!(is_strippable_forward_header("x-real-ip"));
        assert!(is_strippable_forward_header("true-client-ip"));
        assert!(is_strippable_forward_header("cf-connecting-ip"));
        assert!(is_strippable_forward_header("cf-ipcountry"));
        assert!(is_strippable_forward_header("forwarded"));
    }

    #[test]
    fn hop_by_hop_headers_are_stripped() {
        assert!(is_strippable_forward_header("connection"));
        assert!(is_strippable_forward_header("transfer-encoding"));
    }

    #[test]
    fn application_headers_pass_through() {
        // Legitimate app headers must survive proxying.
        assert!(!is_strippable_forward_header("authorization"));
        assert!(!is_strippable_forward_header("x-device-id"));
        assert!(!is_strippable_forward_header("x-nonce"));
        assert!(!is_strippable_forward_header("content-type"));
        assert!(!is_strippable_forward_header("user-agent"));
    }
}
