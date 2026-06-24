use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::middleware::policy::DeviceContext;
use crate::state::AppState;

/// Populate [`DeviceContext`] for the current request.
///
/// Extracts the device UUID from the `x-device-id` header, looks up the
/// credential row in Postgres for its status, and checks the Redis
/// `device:added:{id}` flag to determine whether the device was recently
/// enrolled.  The context is injected as a request extension for policy to
/// consume.
///
/// If the header is absent, the UUID is malformed, or the lookup finds no
/// credential **owned by the authenticated user**, no extension is inserted and
/// the engine treats the credential as `Active` (coarse pre-filter semantics).
/// The identity service is authoritative for revoked/lost enforcement; this
/// gateway check is defence-in-depth, not the sole gate.
pub async fn inject_device_context(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    let device_id = match req
        .headers()
        .get("x-device-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
    {
        Some(id) => id,
        None => return next.run(req).await,
    };

    // The device id is client-supplied, so bind the lookup to the authenticated
    // user. Without `AND user_id = $2` a caller could present any active device's
    // id to satisfy the revoked/lost credential gates with someone else's status.
    // Claims are guaranteed present here — auth middleware runs earlier in the stack.
    let user_id = match req
        .extensions()
        .get::<crate::middleware::auth::Claims>()
        .and_then(|c| Uuid::parse_str(&c.sub).ok())
    {
        Some(uid) => uid,
        None => return next.run(req).await,
    };

    // Runtime query (no compile-time DB required) so `cargo check` works
    // without a live database.
    let row: Option<(String,)> =
        sqlx::query_as("SELECT status FROM credentials WHERE id = $1 AND user_id = $2")
            .bind(device_id)
            .bind(user_id)
            .fetch_optional(&state.pg)
            .await
            .unwrap_or(None);

    let status = match row {
        Some((s,)) => s,
        None => {
            tracing::debug!(%device_id, "device_context: credential not found in db");
            return next.run(req).await;
        }
    };

    let mut conn = state.redis.as_ref().clone();
    let recently_added: bool = redis::cmd("EXISTS")
        .arg(format!("device:added:{}", device_id))
        .query_async(&mut conn)
        .await
        .unwrap_or(false);

    req.extensions_mut().insert(DeviceContext {
        device_id: device_id.to_string(),
        status,
        recently_added,
    });

    next.run(req).await
}
