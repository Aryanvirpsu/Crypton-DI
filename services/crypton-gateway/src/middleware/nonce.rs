use axum::{extract::State, http::StatusCode, middleware::Next, Request, response::Response};
use crate::state::AppState;
use crate::state::Metrics;
use std::time::{SystemTime, UNIX_EPOCH};

const TIMESTAMP_WINDOW_SECS: u64 = 60; // requests valid within ±60 seconds

pub async fn enforce_nonce<B>(
    State(state): State<AppState>,
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    // validate timestamp freshness (required)
    if let Some(ts_str) = req.headers().get("x-timestamp").and_then(|v| v.to_str().ok()) {
        let req_ts: u64 = ts_str.parse().map_err(|_| {
            tracing::warn!(timestamp = %ts_str, "invalid timestamp format");
            Metrics::inc(&state.metrics.nonce_timestamp_invalid);
            StatusCode::BAD_REQUEST
        })?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .as_secs();

        let diff = if req_ts > now { req_ts - now } else { now - req_ts };
        if diff > TIMESTAMP_WINDOW_SECS {
            tracing::warn!(request_ts = req_ts, current_ts = now, window = TIMESTAMP_WINDOW_SECS, "timestamp outside acceptable window");
            return Err(StatusCode::BAD_REQUEST);
        }
    } else {
        // timestamp is required for replay protection
        tracing::warn!("nonce: missing x-timestamp header");
        Metrics::inc(&state.metrics.nonce_timestamp_missing);
        return Err(StatusCode::BAD_REQUEST);
    }

    // check nonce (optional but if present, must not be replayed)
    if let Some(nonce) = req.headers().get("x-nonce").and_then(|v| v.to_str().ok()) {
        let conn = state.redis.as_ref().clone();
        let mut conn = conn;
        let key = format!("nonce:{}", nonce);
        // if exists --> replay
        let exists: bool = redis::cmd("EXISTS")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .unwrap_or(false);
        if exists {
            tracing::warn!(nonce = %nonce, "nonce: replayed nonce rejected");
            Metrics::inc(&state.metrics.nonce_replay_blocked);
            return Err(StatusCode::CONFLICT);
        }
        // insert with TTL 300s (5 minutes)
        let _: () = redis::cmd("SET")
            .arg(&key)
            .arg(1)
            .arg("EX")
            .arg(300)
            .query_async(&mut conn)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    Ok(next.run(req).await)
}