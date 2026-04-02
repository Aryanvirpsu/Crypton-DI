use axum::{response::Html, routing::get, Router};
use crate::state::AppState;

// Embed the test UI at compile time so it's served from the same origin as the API.
// This is critical for WebAuthn (same-origin) and for single-URL tunnel testing.
const TEST_UI: &str = include_str!("../../test-webauthn.html");

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/test", get(test_ui))
}

async fn root() -> &'static str {
    "crypton-identity up"
}

async fn health() -> &'static str {
    "ok"
}

async fn test_ui() -> Html<&'static str> {
    Html(TEST_UI)
}
