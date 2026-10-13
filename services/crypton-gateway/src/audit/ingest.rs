//! `POST /internal/audit/ingest` — service-to-service security events.
//!
//! Each accepted event is (1) recorded in `audit_logs` (the table
//! `crypton-id`'s migrations create; there is no second audit table) and
//! (2) applied to the Redis signals the policy engine reads (login failure
//! counters, account lock, recovery/device flags).
//!
//! Consistency: both happen or neither is reported as done. The audit row is
//! inserted inside a transaction, the Redis effect is applied as ONE atomic
//! operation (a Lua script or MULTI/EXEC), and the transaction commits only if
//! that succeeded. A Redis failure rolls the row back and returns 503; a
//! Postgres failure returns 500 before any Redis change. The one residual
//! window is a commit failure after a successful Redis write (the signal is
//! applied but no row is stored); it is logged and returns 500.

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

/// Constant-time equality so the token check does not leak a prefix length.
fn tokens_match(provided: &str, expected: &str) -> bool {
    let (a, b) = (provided.as_bytes(), expected.as_bytes());
    let mut diff = (a.len() ^ b.len()) as u8 as u32;
    for i in 0..a.len().max(b.len()) {
        let x = *a.get(i).unwrap_or(&0);
        let y = *b.get(i).unwrap_or(&0);
        diff |= (x ^ y) as u32;
    }
    diff == 0 && a.len() == b.len()
}

fn verify_internal_token(headers: &HeaderMap, expected: &str) -> Result<(), StatusCode> {
    let provided = headers
        .get("x-internal-token")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if !tokens_match(provided, expected) {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(())
}

fn event_type_is_well_formed(t: &str) -> bool {
    !t.is_empty()
        && t.len() <= 64
        && t.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

// login_failed: bump the failure counters and set the lock past 5 failures,
// atomically. KEYS: failed, locked, velocity.
const LOGIN_FAILED_LUA: &str = "
local c = redis.call('INCR', KEYS[1])
if c == 1 then redis.call('EXPIRE', KEYS[1], 300) end
if c > 5 then redis.call('SET', KEYS[2], 1, 'EX', 600) end
local v = redis.call('INCR', KEYS[3])
if v == 1 then redis.call('EXPIRE', KEYS[3], 3600) end
return c
";

// recovery_started: set the pending flag and bump the 24h velocity counter.
// KEYS: pending, velocity.
const RECOVERY_STARTED_LUA: &str = "
redis.call('SET', KEYS[1], 1, 'EX', 86400)
local v = redis.call('INCR', KEYS[2])
if v == 1 then redis.call('EXPIRE', KEYS[2], 86400) end
return v
";

enum Effect {
    None,
    LoginFailed(Uuid),
    LoginSuccess(Uuid),
    DeviceAdded(Uuid),
    RecoveryStarted(Uuid),
    RecoveryCompleted(Uuid),
}

/// Parse the event into the Redis effect it implies, rejecting malformed input
/// before anything is written.
fn effect_for(event: &AuditEvent) -> Result<Effect, &'static str> {
    let user = match &event.user_id {
        Some(s) => Some(Uuid::parse_str(s).map_err(|_| "user_id_not_uuid")?),
        None => None,
    };
    Ok(match (event.event_type.as_str(), user) {
        ("login_failed", Some(u)) => Effect::LoginFailed(u),
        ("login_success", Some(u)) => Effect::LoginSuccess(u),
        ("recovery_started", Some(u)) => Effect::RecoveryStarted(u),
        ("recovery_completed", Some(u)) => Effect::RecoveryCompleted(u),
        ("device_added", _) => match event.details.get("device_id").and_then(|v| v.as_str()) {
            Some(d) => Effect::DeviceAdded(Uuid::parse_str(d).map_err(|_| "device_id_not_uuid")?),
            None => Effect::None,
        },
        _ => Effect::None,
    })
}

async fn apply_effect(
    conn: &mut redis::aio::ConnectionManager,
    effect: &Effect,
) -> Result<(), redis::RedisError> {
    match effect {
        Effect::None => {}
        Effect::LoginFailed(u) => {
            let _: i64 = redis::Script::new(LOGIN_FAILED_LUA)
                .key(format!("login:failed:{u}"))
                .key(format!("login:locked:{u}"))
                .key(format!("velocity:failed:{u}:1h"))
                .invoke_async(conn)
                .await?;
        }
        Effect::LoginSuccess(u) => {
            let _: () = redis::pipe()
                .atomic()
                .del(format!("login:failed:{u}"))
                .del(format!("login:locked:{u}"))
                .query_async(conn)
                .await?;
        }
        Effect::DeviceAdded(d) => {
            let _: () = redis::cmd("SET")
                .arg(format!("device:added:{d}"))
                .arg(1)
                .arg("EX")
                .arg(600u32)
                .query_async(conn)
                .await?;
        }
        Effect::RecoveryStarted(u) => {
            let _: i64 = redis::Script::new(RECOVERY_STARTED_LUA)
                .key(format!("recovery:pending:{u}"))
                .key(format!("velocity:recovery:{u}:24h"))
                .invoke_async(conn)
                .await?;
        }
        Effect::RecoveryCompleted(u) => {
            let _: () = redis::cmd("DEL")
                .arg(format!("recovery:pending:{u}"))
                .query_async(conn)
                .await?;
        }
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
    if !event_type_is_well_formed(&event.event_type) {
        return (StatusCode::BAD_REQUEST, "").into_response();
    }
    let effect = match effect_for(&event) {
        Ok(e) => e,
        Err(why) => {
            tracing::warn!(why, "audit: rejected malformed event");
            return (StatusCode::BAD_REQUEST, "").into_response();
        }
    };

    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_string();
    let user_uuid = event
        .user_id
        .as_deref()
        .and_then(|s| Uuid::parse_str(s).ok());
    let detail = serde_json::json!({
        "source": "internal_ingest",
        "request_id": request_id,
        "claimed_user_id": event.user_id,
        "event": event.details,
    });

    let mut tx = match state.pg.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!(error = %e, "audit: cannot begin transaction");
            return (StatusCode::INTERNAL_SERVER_ERROR, "").into_response();
        }
    };

    // `user_id` is a foreign key to users: resolve it through the table so an
    // unknown user yields NULL (the claimed id stays in `detail`) instead of a
    // constraint failure that would drop the event.
    if let Err(e) = sqlx::query(
        "INSERT INTO audit_logs (user_id, actor, credential_id, action, detail, outcome) \
         VALUES ((SELECT id FROM users WHERE id = $1), 'gateway-ingest', NULL, $2, $3, 'recorded')",
    )
    .bind(user_uuid)
    .bind(&event.event_type)
    .bind(&detail)
    .execute(&mut *tx)
    .await
    {
        tracing::error!(error = %e, "audit: failed to persist event");
        return (StatusCode::INTERNAL_SERVER_ERROR, "").into_response();
    }

    let mut conn = state.redis.as_ref().clone();
    if let Err(e) = apply_effect(&mut conn, &effect).await {
        tracing::error!(error = %e, "audit: redis effect failed, rolling back audit row");
        let _ = tx.rollback().await;
        return (StatusCode::SERVICE_UNAVAILABLE, "").into_response();
    }

    if let Err(e) = tx.commit().await {
        tracing::error!(error = %e, "audit: commit failed after redis effect was applied");
        return (StatusCode::INTERNAL_SERVER_ERROR, "").into_response();
    }

    (StatusCode::ACCEPTED, "").into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_comparison() {
        assert!(tokens_match("abc", "abc"));
        assert!(!tokens_match("abc", "abd"));
        assert!(!tokens_match("abc", "abcd"));
        assert!(!tokens_match("", "abc"));
        assert!(tokens_match("", ""));
    }

    #[test]
    fn event_type_charset() {
        assert!(event_type_is_well_formed("login_failed"));
        assert!(!event_type_is_well_formed(""));
        assert!(!event_type_is_well_formed("Login"));
        assert!(!event_type_is_well_formed("a b"));
        assert!(!event_type_is_well_formed(&"a".repeat(65)));
    }

    #[test]
    fn effects_validate_ids() {
        let ev = |t: &str, u: Option<&str>, d: serde_json::Value| AuditEvent {
            event_type: t.into(),
            user_id: u.map(String::from),
            details: d,
        };
        let id = Uuid::new_v4().to_string();
        assert!(matches!(
            effect_for(&ev("login_failed", Some(&id), serde_json::json!({}))),
            Ok(Effect::LoginFailed(_))
        ));
        assert!(effect_for(&ev("login_failed", Some("nope"), serde_json::json!({}))).is_err());
        assert!(effect_for(&ev(
            "device_added",
            None,
            serde_json::json!({"device_id": "not-a-uuid"})
        ))
        .is_err());
        assert!(matches!(
            effect_for(&ev("something_else", None, serde_json::json!({}))),
            Ok(Effect::None)
        ));
    }
}
