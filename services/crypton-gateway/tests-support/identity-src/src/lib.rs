//! See Cargo.toml. Everything under `error`, `jwt`, `redis::sessions` and
//! `me` is crypton-id's own source file, included unmodified.
#![allow(dead_code, unused_imports)]

#[path = "../../../../crypton-id/src/error.rs"]
pub mod error;
#[path = "../../../../crypton-id/src/jwt.rs"]
pub mod jwt;
pub mod redis;
#[path = "../../../../crypton-id/src/routes/me.rs"]
pub mod me;
pub mod state;

use axum::{Json, Router, extract::State, routing::post};
use serde::Serialize;

pub use jwt::{Claims, JWT_KID, issue_jwt, verify_token};
pub use state::AppState;

#[derive(Serialize)]
struct LogoutResp {
    status: &'static str,
}

/// Same steps as `crypton-id`'s `/auth/logout` handler (auth_webauthn.rs) minus
/// the audit-log write: authenticate, then denylist the token in Redis with the
/// real `deny_token`.
async fn logout(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    _auth: jwt::AuthUser,
) -> Result<Json<error::ApiResponse<LogoutResp>>, error::AppError> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| error::AppError::unauthorized("missing_token"))?;
    let claims = jwt::verify_token(&state, token)?;
    let mut conn = state
        .redis_conn()
        .await
        .map_err(|e| error::AppError::internal(e.to_string()))?;
    redis::sessions::deny_token(&mut conn, &claims.sub, claims.iat, claims.exp)
        .await
        .map_err(|e| error::AppError::internal(e.to_string()))?;
    Ok(Json(error::ApiResponse::success(LogoutResp { status: "ok" })))
}

/// The slice of crypton-id the gateway talks to for authenticated routes: the
/// real `/me` handler and a logout equivalent, behind the real `AuthUser`.
pub fn identity_router(state: AppState) -> Router {
    me::router()
        .route("/auth/logout", post(logout))
        .with_state(state)
}
