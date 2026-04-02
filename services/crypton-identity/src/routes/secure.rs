use axum::{routing::get, Json, Router};
use serde::Serialize;

use crate::{jwt::AuthUser, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new().route("/api/secure", get(secure))
}

#[derive(Serialize)]
struct SecureResp {
    message: &'static str,
    user_id: String,
    username: String,
}

async fn secure(auth: AuthUser) -> Json<SecureResp> {
    Json(SecureResp {
        message: "authenticated",
        user_id: auth.user_id.to_string(),
        username: auth.username,
    })
}
