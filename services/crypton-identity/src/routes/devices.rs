use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;

use crate::{error::AppError, jwt::AuthUser, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/devices", get(list_devices))
        .route("/devices/:id", delete(revoke_device))
        .route("/devices/:id/revoke", post(revoke_device_post))
}

// ── GET /devices ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct DeviceInfo {
    id: Uuid,
    nickname: Option<String>,
    status: String,
}

async fn list_devices(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<DeviceInfo>>, AppError> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let rows = sqlx::query(
        "SELECT id, nickname, status \
         FROM credentials \
         WHERE user_id = $1 \
         ORDER BY created_at",
    )
    .bind(auth.user_id)
    .fetch_all(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    let devices = rows
        .iter()
        .filter_map(|r| {
            Some(DeviceInfo {
                id: r.try_get("id").ok()?,
                nickname: r.try_get("nickname").ok().flatten(),
                status: r.try_get("status").ok()?,
            })
        })
        .collect();

    Ok(Json(devices))
}

// ── DELETE /devices/:id ───────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct RevokeResp {
    status: &'static str,
}

async fn revoke_device(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<RevokeResp>, AppError> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let result = sqlx::query(
        "UPDATE credentials SET status = 'revoked' \
         WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(auth.user_id)
    .execute(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("credential_not_found"));
    }

    tracing::info!(
        user_id = %auth.user_id,
        credential_id = %id,
        "device revoked"
    );

    Ok(Json(RevokeResp { status: "revoked" }))
}

// ── POST /devices/:id/revoke ──────────────────────────────────────────────────
// Frontend calls this path+method. Same logic as DELETE /devices/:id.

async fn revoke_device_post(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<RevokeResp>, AppError> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let result = sqlx::query(
        "UPDATE credentials SET status = 'revoked' \
         WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(auth.user_id)
    .execute(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("credential_not_found"));
    }

    tracing::info!(user_id = %auth.user_id, device_id = %id, "device revoked via POST");
    Ok(Json(RevokeResp { status: "revoked" }))
}
