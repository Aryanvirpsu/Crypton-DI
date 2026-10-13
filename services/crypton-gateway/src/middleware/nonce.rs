//! Replay protection: timestamp window + single-use nonce.
//!
//! Security scope of a nonce (the design, stated explicitly):
//!
//! * Authenticated request: unique per **authenticated user** (`sub`).
//!   Key `nonce:u:{sub}:{nonce}`. The same nonce value used by two different
//!   users is two different requests by two different principals, so it is
//!   accepted for each; one user can never burn or pre-empt another user's
//!   nonces. Replaying a captured request does NOT get around this by switching
//!   identity: the replay still carries the victim's token, so it lands in the
//!   victim's scope and is rejected. Authenticating as someone else requires a
//!   different valid token, which makes it a new request, not a replay.
//! * Anonymous bootstrap request (public auth routes, no principal yet):
//!   unique per `(client ip, method, path)`. Key `nonce:a:{ip}:{method}:{path}:{nonce}`.
//!
//! A nonce is single-use for `NONCE_TTL_SECS`, which must exceed the largest
//! age a request can have and still pass the timestamp check (2 x window), or a
//! nonce could expire while its request is still replayable.
//!
//! The nonce is claimed with one atomic `SET NX EX`, so concurrent reuse admits
//! exactly one request. If Redis cannot answer, the request is refused (503):
//! the store being down never silently disables replay protection.

use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};

use crate::middleware::auth::Claims;
use crate::middleware::reject::Reject;
use crate::state::{AppState, Metrics};

const TIMESTAMP_WINDOW_SECS: u64 = 60;
const NONCE_TTL_SECS: u32 = 300;
const MAX_NONCE_LEN: usize = 128;
const _: () = assert!(NONCE_TTL_SECS as u64 > 2 * TIMESTAMP_WINDOW_SECS);

fn timestamp_in_window(req_ts: u64, now: u64) -> bool {
    req_ts.abs_diff(now) <= TIMESTAMP_WINDOW_SECS
}

/// Nonces become part of a Redis key: restrict them to a conservative
/// charset and length so they cannot inject key structure or bloat memory.
fn nonce_is_well_formed(n: &str) -> bool {
    !n.is_empty()
        && n.len() <= MAX_NONCE_LEN
        && n.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':'))
}

fn nonce_key(scope: &str, nonce: &str) -> String {
    format!("nonce:{scope}:{nonce}")
}

fn scope_for(req: &Request) -> String {
    match req.extensions().get::<Claims>() {
        Some(c) => format!("u:{}", c.sub),
        None => {
            let ip = crate::middleware::client_ip(req)
                .map(|i| i.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            format!("a:{ip}:{}:{}", req.method(), req.uri().path())
        }
    }
}

pub async fn enforce_nonce(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, Reject> {
    let bad = |reason: &'static str| Reject::new(StatusCode::BAD_REQUEST, reason);

    let ts_str = match req
        .headers()
        .get("x-timestamp")
        .and_then(|v| v.to_str().ok())
    {
        Some(t) => t,
        None => {
            tracing::warn!("nonce: missing x-timestamp header");
            Metrics::inc(&state.metrics.nonce_timestamp_missing);
            return Err(bad("timestamp_missing"));
        }
    };
    let req_ts: u64 = ts_str.parse().map_err(|_| {
        tracing::warn!("nonce: invalid timestamp format");
        Metrics::inc(&state.metrics.nonce_timestamp_invalid);
        bad("timestamp_invalid")
    })?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Reject::new(StatusCode::INTERNAL_SERVER_ERROR, "clock_error"))?
        .as_secs();
    if !timestamp_in_window(req_ts, now) {
        tracing::warn!(
            request_ts = req_ts,
            current_ts = now,
            window = TIMESTAMP_WINDOW_SECS,
            "nonce: timestamp outside acceptable window"
        );
        Metrics::inc(&state.metrics.nonce_timestamp_invalid);
        return Err(bad("timestamp_out_of_window"));
    }

    let nonce = match req.headers().get("x-nonce").and_then(|v| v.to_str().ok()) {
        Some(n) => n.to_owned(),
        None => {
            tracing::warn!("nonce: missing x-nonce header");
            return Err(bad("nonce_missing"));
        }
    };
    if !nonce_is_well_formed(&nonce) {
        tracing::warn!("nonce: malformed x-nonce header");
        return Err(bad("nonce_malformed"));
    }

    let key = nonce_key(&scope_for(&req), &nonce);
    let mut conn = state.redis.as_ref().clone();

    // Atomic SET NX EX: sets the key only if absent. "OK" on success, nil if the
    // key already exists (replay). One round trip, no EXISTS+SET race.
    let inserted: Option<String> = match redis::cmd("SET")
        .arg(&key)
        .arg(1_u8)
        .arg("NX")
        .arg("EX")
        .arg(NONCE_TTL_SECS)
        .query_async(&mut conn)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            // Cannot verify uniqueness: fail closed, reported as an outage (503)
            // rather than a replay (409) so operators do not misread it.
            tracing::warn!(error = %e, "nonce: redis unavailable, rejecting request");
            return Err(Reject::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "replay_store_unavailable",
            ));
        }
    };

    if inserted.is_none() {
        tracing::warn!("nonce: replayed nonce rejected");
        Metrics::inc(&state.metrics.nonce_replay_blocked);
        return Err(Reject::new(StatusCode::CONFLICT, "nonce_replayed"));
    }

    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_boundary_is_accepted() {
        assert!(timestamp_in_window(100, 100));
        assert!(timestamp_in_window(100, 160));
        assert!(timestamp_in_window(160, 100));
    }

    #[test]
    fn one_second_past_boundary_is_rejected() {
        assert!(!timestamp_in_window(100, 161));
        assert!(!timestamp_in_window(161, 100));
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

    #[test]
    fn nonce_format() {
        assert!(nonce_is_well_formed("2f1c9a0e-8b7d-4c1e-9a55-3d2b1f0e7a11"));
        assert!(nonce_is_well_formed("abc_DEF.123:x"));
        assert!(!nonce_is_well_formed(""));
        assert!(!nonce_is_well_formed(&"a".repeat(MAX_NONCE_LEN + 1)));
        assert!(nonce_is_well_formed(&"a".repeat(MAX_NONCE_LEN)));
        assert!(!nonce_is_well_formed("has space"));
        assert!(!nonce_is_well_formed("new\nline"));
        assert!(!nonce_is_well_formed("emoji\u{1F600}"));
        assert!(!nonce_is_well_formed("a*b"));
    }

    #[test]
    fn keys_differ_by_scope() {
        assert_ne!(nonce_key("u:1", "n"), nonce_key("u:2", "n"));
        assert_eq!(nonce_key("u:1", "n"), nonce_key("u:1", "n"));
    }
}
