use axum::{
    Router,
    body::Body,
    extract::Request,
    http::{
        HeaderName, HeaderValue, Method,
        header::{AUTHORIZATION, CONTENT_TYPE},
    },
};
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

use crate::{routes, state::AppState};

const REQUEST_ID_HEADER: &str = "x-request-id";

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
}
