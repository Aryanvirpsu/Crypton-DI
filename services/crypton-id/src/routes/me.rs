//! GET /me — returns the authenticated caller's identity + role.
//!
//! This exists so a frontend can decide whether to show admin UI without
//! keeping a second, separately-issued "admin token" (the old crypton-demo/
//! crypton-admin split did this and it was a source of real bugs — one
//! JWT, one source of truth for role).

use axum::{Json, Router, extract::State, routing::get};
use serde::Serialize;

use crate::{
    error::{ApiResponse, AppError},
    jwt::AuthUser,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/me", get(me))
}

#[derive(Debug, Serialize)]
struct MeResp {
    user_id: String,
    username: String,
    role: String,
}

async fn me(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<MeResp>>, AppError> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let role: String = sqlx::query_scalar("SELECT role FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_optional(db)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?
        .unwrap_or_else(|| "user".to_string());

    Ok(Json(ApiResponse::success(MeResp {
        user_id: auth.user_id.to_string(),
        username: auth.username,
        role,
    })))
}
