use axum::{routing::get, Router};
use crate::state::AppState;


pub fn router() -> Router<AppState> {
    Router::new().route("/api/secure", get(secure))
}

async fn secure() -> &'static str {
    "secure endpoint (auth later)"
}
