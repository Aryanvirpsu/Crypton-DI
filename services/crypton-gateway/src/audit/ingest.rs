use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct AuditEvent {
    pub event_type: String,
    pub user_id: Option<String>,
    pub details: serde_json::Value,
}

fn verify_internal_token(headers: &HeaderMap, expected: &str) -> Result<(), StatusCode> {
    let provided = headers
        .get("x-internal-token")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if provided != expected {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(())
}

pub async fn ingest(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(event): Json<AuditEvent>,
) -> impl IntoResponse {
    if let Err(status) = verify_internal_token(&headers, &state.internal_token) {
        return (status, "").into_response();
    }

    // Reject non-UUID user_ids before they can be interpolated into Redis keys.
    // The identity service always sends valid UUIDs; anything else is malformed
    // or a spoofing attempt.
    if let Some(ref uid_str) = event.user_id {
        if Uuid::parse_str(uid_str).is_err() {
            tracing::warn!(user_id = %uid_str, "audit: rejected event with non-UUID user_id");
            return (StatusCode::BAD_REQUEST, "").into_response();
        }
    }

    tracing::info!(event_type = %event.event_type, "audit: received event");

    {
        let mut conn = state.redis.as_ref().clone();
        match event.event_type.as_str() {
            "login_failed" => {
                if let Some(user) = &event.user_id {
                    // Simple lockout counter (legacy — checked by diagnostics + old policy rules).
                    let key = format!("login:failed:{}", user);
                    let count: i32 = redis::cmd("INCR")
                        .arg(&key)
                        .query_async(&mut conn)
                        .await
                        .unwrap_or(0);
                    if count == 1 {
                        let _: () = redis::cmd("EXPIRE")
                            .arg(&key)
                            .arg(300u32)
                            .query_async(&mut conn)
                            .await
                            .unwrap_or(());
                    }
                    if count > 5 {
                        let lock = format!("login:locked:{}", user);
                        let _: () = redis::cmd("SET")
                            .arg(&lock)
                            .arg(1)
                            .arg("EX")
                            .arg(600u32)
                            .query_async(&mut conn)
                            .await
                            .unwrap_or(());
                    }

                    // Also update the risk-engine velocity key so brute-force
                    // scoring sees live data. Applies when user_id is a UUID.
                    if let Ok(uid) = Uuid::parse_str(user) {
                        let vkey = format!("velocity:failed:{}:1h", uid);
                        let vcnt: i32 = redis::cmd("INCR")
                            .arg(&vkey)
                            .query_async(&mut conn)
                            .await
                            .unwrap_or(0);
                        if vcnt == 1 {
                            let _: () = redis::cmd("EXPIRE")
                                .arg(&vkey)
                                .arg(3600u32)
                                .query_async(&mut conn)
                                .await
                                .unwrap_or(());
                        }
                    }
                }
            }
            "login_success" => {
                if let Some(user) = &event.user_id {
                    for key in [
                        format!("login:failed:{}", user),
                        format!("login:locked:{}", user),
                    ] {
                        let _: () = redis::cmd("DEL")
                            .arg(&key)
                            .query_async(&mut conn)
                            .await
                            .unwrap_or(());
                    }
                }
            }
            "device_added" => {
                if let Some(device_id) =
                    event.details.get("device_id").and_then(|v| v.as_str())
                {
                    let key = format!("device:added:{}", device_id);
                    let _: () = redis::cmd("SET")
                        .arg(&key)
                        .arg(1)
                        .arg("EX")
                        .arg(600u32)
                        .query_async(&mut conn)
                        .await
                        .unwrap_or(());
                }
            }
            "recovery_started" => {
                if let Some(user) = &event.user_id {
                    let key = format!("recovery:pending:{}", user);
                    let _: () = redis::cmd("SET")
                        .arg(&key)
                        .arg(1)
                        .arg("EX")
                        .arg(86400u32)
                        .query_async(&mut conn)
                        .await
                        .unwrap_or(());
                    // Also update risk-engine recovery velocity key.
                    if let Ok(uid) = Uuid::parse_str(user) {
                        let vkey = format!("velocity:recovery:{}:24h", uid);
                        let vcnt: i32 = redis::cmd("INCR")
                            .arg(&vkey)
                            .query_async(&mut conn)
                            .await
                            .unwrap_or(0);
                        if vcnt == 1 {
                            let _: () = redis::cmd("EXPIRE")
                                .arg(&vkey)
                                .arg(86400u32)
                                .query_async(&mut conn)
                                .await
                                .unwrap_or(());
                        }
                    }
                }
            }
            "recovery_completed" => {
                if let Some(user) = &event.user_id {
                    let key = format!("recovery:pending:{}", user);
                    let _: () = redis::cmd("DEL")
                        .arg(&key)
                        .query_async(&mut conn)
                        .await
                        .unwrap_or(());
                }
            }
            _ => {}
        }
    }

    let id = Uuid::new_v4();
    let res = sqlx::query(
        "INSERT INTO audit_events (id, event_type, user_id, details) VALUES ($1, $2, $3, $4)",
    )
    .bind(id)
    .bind(&event.event_type)
    .bind(&event.user_id)
    .bind(&event.details)
    .execute(&state.pg)
    .await;

    match res {
        Ok(_) => (StatusCode::ACCEPTED, "").into_response(),
        Err(e) => {
            tracing::error!(error = ?e, "audit: failed to persist event");
            (StatusCode::INTERNAL_SERVER_ERROR, "").into_response()
        }
    }
}
