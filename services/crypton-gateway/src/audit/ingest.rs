use axum::{extract::Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct AuditEvent {
    pub event_type: String,
    pub user_id: Option<String>,
    pub details: serde_json::Value,
}

pub async fn ingest(
    State(state): State<AppState>,
    Json(event): Json<AuditEvent>,
) -> impl IntoResponse {
    tracing::info!(?event, "received audit event");

    // update Redis caches used by the policy engine
    {
        let mut conn = state.redis.clone();
        match event.event_type.as_str() {
            "login_failed" => {
                if let Some(user) = &event.user_id {
                    let key = format!("login:failed:{}", user);
                    let count: i32 = redis::cmd("INCR").arg(&key).query_async(&mut *conn).await.unwrap_or(0);
                    if count == 1 {
                        let _: () = redis::cmd("EXPIRE").arg(&key).arg(300).query_async(&mut *conn).await.ok();
                    }
                    if count > 5 {
                        let lock = format!("login:locked:{}", user);
                        let _: () = redis::cmd("SET").arg(&lock).arg(1).arg("EX").arg(600).query_async(&mut *conn).await.ok();
                    }
                }
            }
            "login_success" => {
                if let Some(user) = &event.user_id {
                    let key = format!("login:failed:{}", user);
                    let _: () = redis::cmd("DEL").arg(&key).query_async(&mut *conn).await.ok();
                    let lock = format!("login:locked:{}", user);
                    let _: () = redis::cmd("DEL").arg(&lock).query_async(&mut *conn).await.ok();
                }
            }
            "device_added" => {
                if let Some(device_id) = event.details.get("device_id").and_then(|v| v.as_str()) {
                    let key = format!("device:added:{}", device_id);
                    let _: () = redis::cmd("SET").arg(&key).arg(1).arg("EX").arg(600).query_async(&mut *conn).await.ok();
                }
            }
            "recovery_started" => {
                if let Some(user) = &event.user_id {
                    let key = format!("recovery:pending:{}", user);
                    let _: () = redis::cmd("SET").arg(&key).arg(1).arg("EX").arg(600).query_async(&mut *conn).await.ok();
                }
            }
            "recovery_completed" => {
                if let Some(user) = &event.user_id {
                    let key = format!("recovery:pending:{}", user);
                    let _: () = redis::cmd("DEL").arg(&key).query_async(&mut *conn).await.ok();
                }
            }
            _ => {}
        }
    }

    let id = Uuid::new_v4();
    let res = sqlx::query!(
        "INSERT INTO audit_events (id, event_type, user_id, details) VALUES ($1, $2, $3, $4)",
        id,
        event.event_type,
        event.user_id,
        event.details,
    )
    .execute(&state.pg)
    .await;

    match res {
        Ok(_) => (StatusCode::ACCEPTED, ""),
        Err(e) => {
            tracing::error!(error = ?e, "failed to insert audit event");
            (StatusCode::INTERNAL_SERVER_ERROR, "")
        }
    }
}