use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;
use sqlx::{postgres::PgRow, Row};
use uuid::Uuid;

use crate::{error::AppError, jwt::AuthUser, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new().route("/audit-logs", get(list_audit_logs))
}

// ── response types ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct AuditLogEntry {
    id: Uuid,
    actor: Option<String>,
    event_type: String,
    status: String,
    credential_id: Option<Uuid>,
    metadata: serde_json::Value,
    created_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct AuditLogsResp {
    logs: Vec<AuditLogEntry>,
}

// ── GET /audit-logs ───────────────────────────────────────────────────────────

async fn list_audit_logs(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<AuditLogsResp>, AppError> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let rows = sqlx::query(
        "SELECT id, actor, credential_id, \
                action  AS event_type, \
                outcome AS status, \
                detail  AS metadata, \
                created_at::text AS created_at \
         FROM audit_logs \
         WHERE user_id = $1 \
         ORDER BY created_at DESC \
         LIMIT 50",
    )
    .bind(auth.user_id)
    .fetch_all(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    let logs = rows.iter().filter_map(row_to_entry).collect();
    Ok(Json(AuditLogsResp { logs }))
}

fn row_to_entry(row: &PgRow) -> Option<AuditLogEntry> {
    Some(AuditLogEntry {
        id:            row.try_get("id").ok()?,
        actor:         row.try_get("actor").ok(),
        event_type:    row.try_get("event_type").ok()?,
        status:        row.try_get("status").ok()?,
        credential_id: row.try_get("credential_id").ok().flatten(),
        metadata:      row.try_get("metadata").ok().unwrap_or(serde_json::json!({})),
        created_at:    row.try_get("created_at").ok().flatten(),
    })
}
