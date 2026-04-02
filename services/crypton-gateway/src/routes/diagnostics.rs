use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use redis::AsyncCommands;

use crate::state::AppState;

async fn get_i32(conn: &mut redis::aio::ConnectionManager, key: &str) -> Option<i32> {
    conn.get::<_, Option<i32>>(key).await.ok().flatten()
}

async fn ttl(conn: &mut redis::aio::ConnectionManager, key: &str) -> Option<i64> {
    conn.ttl::<_, i64>(key).await.ok()
}

async fn exists(conn: &mut redis::aio::ConnectionManager, key: &str) -> bool {
    conn.exists::<_, bool>(key).await.unwrap_or(false)
}

pub async fn user(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let conn = state.redis.as_ref().clone();
    let mut conn = conn;

    let failed_key = format!("login:failed:{}", user_id);
    let locked_key = format!("login:locked:{}", user_id);
    let recovery_key = format!("recovery:pending:{}", user_id);
    let ratelimit_user_key = format!("ratelimit:user:{}", user_id);

    let out = serde_json::json!({
        "user_id": user_id,
        "login_failed": {
            "count": get_i32(&mut conn, &failed_key).await,
            "ttl_seconds": ttl(&mut conn, &failed_key).await,
        },
        "login_locked": {
            "locked": exists(&mut conn, &locked_key).await,
            "ttl_seconds": ttl(&mut conn, &locked_key).await,
        },
        "recovery_pending": {
            "pending": exists(&mut conn, &recovery_key).await,
            "ttl_seconds": ttl(&mut conn, &recovery_key).await,
        },
        "ratelimit": {
            "user": {
                "count": get_i32(&mut conn, &ratelimit_user_key).await,
                "ttl_seconds": ttl(&mut conn, &ratelimit_user_key).await,
            }
        }
    });

    Ok(Json(out))
}

pub async fn device(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let conn = state.redis.as_ref().clone();
    let mut conn = conn;

    let added_key = format!("device:added:{}", device_id);
    let ratelimit_device_key = format!("ratelimit:device:{}", device_id);

    let out = serde_json::json!({
        "device_id": device_id,
        "recently_added_flag": {
            "set": exists(&mut conn, &added_key).await,
            "ttl_seconds": ttl(&mut conn, &added_key).await,
        },
        "ratelimit": {
            "device": {
                "count": get_i32(&mut conn, &ratelimit_device_key).await,
                "ttl_seconds": ttl(&mut conn, &ratelimit_device_key).await,
            }
        }
    });

    Ok(Json(out))
}

pub async fn ip(
    State(state): State<AppState>,
    Path(ip): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let conn = state.redis.as_ref().clone();
    let mut conn = conn;

    let ratelimit_ip_key = format!("ratelimit:ip:{}", ip);
    let out = serde_json::json!({
        "ip": ip,
        "ratelimit": {
            "ip": {
                "count": get_i32(&mut conn, &ratelimit_ip_key).await,
                "ttl_seconds": ttl(&mut conn, &ratelimit_ip_key).await,
            }
        }
    });

    Ok(Json(out))
}

