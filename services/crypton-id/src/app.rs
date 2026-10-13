use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    extract::Request,
    http::{
        HeaderName, HeaderValue, Method,
        header::{AUTHORIZATION, CONTENT_TYPE},
    },
    middleware::Next,
    response::{IntoResponse, Response},
};
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

use crate::{routes, state::AppState};

const REQUEST_ID_HEADER: &str = "x-request-id";

/// Header the gateway adds to every request it forwards.
pub const GATEWAY_SECRET_HEADER: &str = "x-gateway-secret";

fn secrets_match(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut diff = (a.len() ^ b.len()) as u32;
    for i in 0..a.len().max(b.len()) {
        diff |= (*a.get(i).unwrap_or(&0) ^ *b.get(i).unwrap_or(&0)) as u32;
    }
    diff == 0
}

/// Defence in depth behind network isolation: when `GATEWAY_SHARED_SECRET` is set,
/// every request except `/health` (the platform liveness probe) must carry it in
/// `x-gateway-secret`, which only the gateway adds. A client that somehow reaches
/// this service directly, without going through the gateway, is refused. This is
/// NOT a substitute for keeping the service on a private network.
async fn require_gateway_secret(secret: Option<Arc<String>>, req: Request, next: Next) -> Response {
    if let Some(expected) = secret {
        let ok = req.uri().path() == "/health"
            || req
                .headers()
                .get(GATEWAY_SECRET_HEADER)
                .and_then(|v| v.to_str().ok())
                .map(|v| secrets_match(v, &expected))
                .unwrap_or(false);
        if !ok {
            tracing::warn!(path = %req.uri().path(), "refused request without the gateway secret");
            return (axum::http::StatusCode::FORBIDDEN, "forbidden").into_response();
        }
    }
    next.run(req).await
}

pub fn app(state: AppState) -> Router {
    let cors_origin: HeaderValue = state
        .cors_origin
        .parse()
        .expect("WEBAUTHN_ORIGIN is not a valid HTTP header value");

    let cors = CorsLayer::new()
        .allow_origin(cors_origin)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PATCH])
        .allow_headers([
            CONTENT_TYPE,
            AUTHORIZATION,
            HeaderName::from_static(REQUEST_ID_HEADER),
        ]);

    let request_id_header = HeaderName::from_static(REQUEST_ID_HEADER);

    let gateway_secret = std::env::var("GATEWAY_SHARED_SECRET")
        .ok()
        .filter(|s| !s.is_empty())
        .map(Arc::new);
    if gateway_secret.is_none() {
        tracing::warn!(
            "GATEWAY_SHARED_SECRET is not set: this service accepts requests that did not come through the gateway"
        );
    }

    Router::<AppState>::new()
        .merge(routes::router())
        .with_state(state)
        .layer(cors)
        .layer(
            ServiceBuilder::new()
                // If the gateway already assigned x-request-id, this is a
                // no-op — SetRequestIdLayer only generates one when the
                // header is absent (e.g. a direct call to :8080 in dev).
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
                .layer(PropagateRequestIdLayer::new(request_id_header)),
        )
        .layer(axum::middleware::from_fn(
            move |req: Request, next: Next| {
                require_gateway_secret(gateway_secret.clone(), req, next)
            },
        ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_comparison() {
        assert!(secrets_match("abc", "abc"));
        assert!(!secrets_match("abc", "abd"));
        assert!(!secrets_match("abc", "ab"));
        assert!(!secrets_match("", "x"));
    }
}

/// Production-intended deployments set `REQUIRE_GATEWAY_SECRET=true`: the service
/// then refuses to start unless `GATEWAY_SHARED_SECRET` is set to a value of at
/// least 16 characters, so a missing secret can never silently disable the
/// gateway-bypass defence. Without the flag the secret stays optional (local dev).
pub fn gateway_secret_policy(required: bool, secret: Option<&str>) -> Result<(), &'static str> {
    match (required, secret.map(str::trim).filter(|s| !s.is_empty())) {
        (true, None) => Err("REQUIRE_GATEWAY_SECRET=true but GATEWAY_SHARED_SECRET is not set"),
        (true, Some(s)) if s.len() < 16 => Err(
            "GATEWAY_SHARED_SECRET must be at least 16 characters when REQUIRE_GATEWAY_SECRET=true",
        ),
        _ => Ok(()),
    }
}

/// Reads the environment and applies [`gateway_secret_policy`]. Call before anything
/// else touches the network.
pub fn enforce_gateway_secret_policy() -> anyhow::Result<()> {
    let required = std::env::var("REQUIRE_GATEWAY_SECRET").as_deref() == Ok("true");
    let secret = std::env::var("GATEWAY_SHARED_SECRET").ok();
    gateway_secret_policy(required, secret.as_deref()).map_err(|e| anyhow::anyhow!(e))
}

#[cfg(test)]
mod secret_policy_tests {
    use super::gateway_secret_policy as p;

    #[test]
    fn optional_unless_required() {
        assert!(p(false, None).is_ok());
        assert!(p(false, Some("")).is_ok());
        assert!(p(false, Some("x")).is_ok());
    }

    #[test]
    fn required_means_present_and_long_enough() {
        assert!(p(true, None).is_err());
        assert!(p(true, Some("")).is_err());
        assert!(p(true, Some("   ")).is_err());
        assert!(p(true, Some("short")).is_err());
        assert!(p(true, Some("0123456789abcdef")).is_ok());
    }
}
