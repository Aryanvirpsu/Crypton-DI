use axum::{
    http::{
        header::{AUTHORIZATION, CONTENT_TYPE},
        HeaderValue, Method,
    },
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{routes, state::AppState};

pub fn app(state: AppState) -> Router {
    let cors_origin: HeaderValue = state
        .cors_origin
        .parse()
        .expect("WEBAUTHN_ORIGIN is not a valid HTTP header value");

    let cors = CorsLayer::new()
        .allow_origin(cors_origin)
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers([CONTENT_TYPE, AUTHORIZATION]);

    Router::<AppState>::new()
        .merge(routes::router())
        .with_state(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}
