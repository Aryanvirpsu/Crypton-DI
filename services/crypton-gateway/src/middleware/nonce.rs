use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::state::{AppState, Metrics};

const TIMESTAMP_WINDOW_SECS: u64 = 60;

fn timestamp_in_window(req_ts: u64, now: u64) -> bool {
    let diff = if req_ts > now { req_ts - now } else { now - req_ts };
    diff <= TIMESTAMP_WINDOW_SECS
}

pub async fn enforce_nonce(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if let Some(ts_str) = req
        .headers()
        .get("x-timestamp")
        .and_then(|v| v.to_str().ok())
    {
        let req_ts: u64 = ts_str.parse().map_err(|_| {
            tracing::warn!(timestamp = %ts_str, "nonce: invalid timestamp format");
            Metrics::inc(&state.metrics.nonce_timestamp_invalid);
            StatusCode::BAD_REQUEST
        })?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .as_secs();

        if !timestamp_in_window(req_ts, now) {
            tracing::warn!(
                request_ts = req_ts,
                current_ts = now,
                window = TIMESTAMP_WINDOW_SECS,
                "nonce: timestamp outside acceptable window"
            );
            return Err(StatusCode::BAD_REQUEST);
        }
    } else {
        tracing::warn!("nonce: missing x-timestamp header");
        Metrics::inc(&state.metrics.nonce_timestamp_missing);
        return Err(StatusCode::BAD_REQUEST);
    }

    let nonce = match req
        .headers()
        .get("x-nonce")
        .and_then(|v| v.to_str().ok())
    {
        Some(n) => n.to_owned(),
        None => {
            tracing::warn!("nonce: missing x-nonce header");
            Metrics::inc(&state.metrics.nonce_replay_blocked);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    let mut conn = state.redis.as_ref().clone();
    let key = format!("nonce:{}", nonce);

    // Atomic SET NX EX: sets the key only if it does not yet exist.
    // Returns "OK" on success, or nil if the key was already present
    // (replay attack). This is a single round-trip — no EXISTS+SET race.
    let inserted: Option<String> = match redis::cmd("SET")
        .arg(&key)
        .arg(1_u8)
        .arg("NX")
        .arg("EX")
        .arg(300u32)
        .query_async(&mut conn)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            // Can't verify nonce uniqueness — fail closed, but report it as a
            // dependency outage (503), not a replay (409), so operators and
            // clients don't misread a Redis blip as an attack.
            tracing::warn!(error = %e, "nonce: redis unavailable, rejecting request");
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    };

    if inserted.is_none() {
        tracing::warn!(nonce = %nonce, "nonce: replayed nonce rejected");
        Metrics::inc(&state.metrics.nonce_replay_blocked);
        return Err(StatusCode::CONFLICT);
    }

    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_boundary_is_accepted() {
        assert!(timestamp_in_window(100, 100));
        assert!(timestamp_in_window(100, 160));  // diff = 60, at boundary
        assert!(timestamp_in_window(160, 100));  // future ts at boundary
    }

    #[test]
    fn one_second_past_boundary_is_rejected() {
        assert!(!timestamp_in_window(100, 161));  // diff = 61
        assert!(!timestamp_in_window(161, 100));  // future ts over boundary
    }

    #[test]
    fn well_within_window_is_accepted() {
        assert!(timestamp_in_window(1000, 1030));
        assert!(timestamp_in_window(1030, 1000));
    }

    #[test]
    fn far_past_is_rejected() {
        assert!(!timestamp_in_window(0, 3600));
    }
}
