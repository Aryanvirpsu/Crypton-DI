use axum::Router;
use tower_http::trace::TraceLayer;

use crate::{routes, state::AppState};

pub fn app(state: AppState) -> Router {
    Router::<AppState>::new()
        .merge(routes::router())
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}
