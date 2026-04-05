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
    let allowed_origins = [
        "https://crypton-di-s2ay.vercel.app",
        "https://crypton-di.vercel.app",
        "https://app.cryptonid.tech",
        "https://cryptonid.tech",
        "https://auth.cryptonid.tech",
    ];

    let origins: Vec<HeaderValue> = allowed_origins
        .iter()
        .map(|o| o.parse().expect("invalid CORS origin"))
        .collect();

    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PATCH, Method::OPTIONS])
        .allow_headers([CONTENT_TYPE, AUTHORIZATION]);

    Router::new()
        .merge(routes::router())
        .with_state(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}
