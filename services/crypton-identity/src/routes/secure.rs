use axum::{routing::get, Router};

pub fn router() -> Router {
    Router::new().route("/api/secure", get(secure))
}

async fn secure() -> &'static str {
    "secure endpoint (auth later)"
}
