use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;

use crate::{audit, error::{AppError, ApiResponse}, jwt::AuthUser, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/devices", get(list_devices))
        .route("/devices/:id", delete(revoke_device))
        .route("/devices/:id/revoke", post(revoke_device_post))
        .route("/devices/:id/mark-lost", post(mark_lost))
}

// ── GET /devices ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct DeviceInfo {
    id: Uuid,
    nickname: Option<String>,
    status: String,
    user_agent: Option<String>,
    created_at: Option<String>,
    last_used_at: Option<String>,
}

async fn list_devices(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<DeviceInfo>>>, AppError> {
    let db = match state.db.as_ref() {
        Some(db) => db,
        None => return Ok(Json(ApiResponse::success(vec![]))),
    };

    let rows = sqlx::query(
        "SELECT id, nickname, status, \
                user_agent, \
                created_at::text AS created_at, \
                last_used_at::text AS last_used_at \
         FROM credentials \
         WHERE user_id = $1 AND status IN ('active', 'lost') \
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
                user_agent: r.try_get("user_agent").ok().flatten(),
                created_at: r.try_get("created_at").ok().flatten(),
                last_used_at: r.try_get("last_used_at").ok().flatten(),
            })
        })
        .collect();

    Ok(Json(ApiResponse::success(devices)))
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
) -> Result<Json<ApiResponse<RevokeResp>>, AppError> {
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

    audit::log_event(
        db,
        Some(auth.user_id),
        &auth.username,
        Some(id),
        "device_revoke",
        serde_json::json!({}),
        "success",
    )
    .await;

    tracing::info!(user_id = %auth.user_id, credential_id = %id, "device revoked");
    Ok(Json(ApiResponse::success(RevokeResp { status: "revoked" })))
}

// ── POST /devices/:id/revoke ──────────────────────────────────────────────────

async fn revoke_device_post(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<RevokeResp>>, AppError> {
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

    audit::log_event(
        db,
        Some(auth.user_id),
        &auth.username,
        Some(id),
        "device_revoke",
        serde_json::json!({}),
        "success",
    )
    .await;

    tracing::info!(user_id = %auth.user_id, device_id = %id, "device revoked via POST");
    Ok(Json(ApiResponse::success(RevokeResp { status: "revoked" })))
}

// ── POST /devices/:id/mark-lost ───────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct MarkLostResp {
    status: &'static str,
}

async fn mark_lost(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<MarkLostResp>>, AppError> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let result = sqlx::query(
        "UPDATE credentials SET status = 'lost' \
         WHERE id = $1 AND user_id = $2 AND status = 'active'",
    )
    .bind(id)
    .bind(auth.user_id)
    .execute(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("credential_not_found_or_not_active"));
    }

    audit::log_event(
        db,
        Some(auth.user_id),
        &auth.username,
        Some(id),
        "device_mark_lost",
        serde_json::json!({}),
        "success",
    )
    .await;

    tracing::info!(user_id = %auth.user_id, device_id = %id, "device marked lost");
    Ok(Json(ApiResponse::success(MarkLostResp { status: "lost" })))
}
