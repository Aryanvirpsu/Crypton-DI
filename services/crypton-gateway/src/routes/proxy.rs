use axum::{extract::{Path, State}, http::Request, response::Response};
use hyper::{Body, Client, Uri};

use crate::state::{AppState, Metrics};

// simple pass-through reverse proxy to identity-service
pub async fn handler(
    State(state): State<AppState>,
    Path(path): Path<String>,
    req: Request<Body>,
) -> Result<Response<Body>, axum::http::StatusCode> {
    tracing::info!(method = ?req.method(), path = ?req.uri(), "proxy request");
    let client = Client::new();
    // build target uri from config
    let base = std::env::var("IDENTITY_URL").unwrap_or_else(|_| "http://localhost:8081".into());
    let mut uri_string = format!("{}/{}", base.trim_end_matches('/'), path);
    if let Some(q) = req.uri().query() {
        uri_string.push('?');
        uri_string.push_str(q);
    }
    let uri: Uri = uri_string.parse().map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    let mut proxied = Request::builder()
        .method(req.method())
        .uri(uri)
        .body(req.into_body())
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    // copy headers
    *proxied.headers_mut() = req.headers().clone();

    let resp = client.request(proxied).await.map_err(|_| {
        Metrics::inc(&state.metrics.proxy_upstream_error);
        axum::http::StatusCode::BAD_GATEWAY
    })?;
    let sc = resp.status();
    if sc.is_success() {
        Metrics::inc(&state.metrics.proxy_upstream_2xx);
    } else if sc.is_client_error() {
        Metrics::inc(&state.metrics.proxy_upstream_4xx);
    } else if sc.is_server_error() {
        Metrics::inc(&state.metrics.proxy_upstream_5xx);
    }
    tracing::info!(upstream = %uri_string, status = %resp.status(), "proxy response");
    Ok(resp)
}