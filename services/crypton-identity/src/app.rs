use axum::Router;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use crate::{routes, state::AppState};

pub fn app(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::<AppState>::new()
        .merge(routes::router())
        .with_state(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}
