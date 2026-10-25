use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};

use crate::middleware::reject::Reject;
use crate::state::{AppState, Metrics};

// Atomic INCR + conditional EXPIRE via Lua script.
// This eliminates the INCR → TTL → EXPIRE race that would allow a key to
// accumulate counts indefinitely if the process crashed between INCR and EXPIRE.
const INCR_EXPIRE_LUA: &str = "
local cnt = redis.call('INCR', KEYS[1])
if cnt == 1 then
  redis.call('EXPIRE', KEYS[1], tonumber(ARGV[1]))
end
return cnt
";

async fn incr_and_check(
    conn: &mut redis::aio::ConnectionManager,
    key: &str,
    limit: i64,
    window_secs: usize,
) -> redis::RedisResult<bool> {
    let cnt: i64 = redis::Script::new(INCR_EXPIRE_LUA)
        .key(key)
        .arg(window_secs)
        .invoke_async(conn)
        .await?;
    Ok(cnt > limit)
}

fn is_sensitive_path(path: &str) -> bool {
    let p = path.strip_prefix("/api").unwrap_or(path);
    p.starts_with("/auth/") || p.starts_with("/recovery/") || p.contains("/devices/revoke")
}

fn ip_from_request(req: &Request) -> Option<String> {
    // Delegate to the shared trusted-peer helper so rate limiting and risk
    // scoring agree on the client IP and the X-Forwarded-For trust model.
    crate::middleware::client_ip(req).map(|ip| ip.to_string())
}

pub async fn limit(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, Reject> {
    let path = req.uri().path();
    if !is_sensitive_path(path) {
        return Ok(next.run(req).await);
    }

    let mut conn = state.redis.as_ref().clone();

    if let Some(ip) = ip_from_request(&req) {
        let key = format!("ratelimit:ip:{}", ip);
        if incr_and_check(&mut conn, &key, 100, 60)
            .await
            .unwrap_or(false)
        {
            tracing::warn!(%ip, "rate limit exceeded for ip");
            Metrics::inc(&state.metrics.rate_limit_block_ip);
            return Err(Reject::new(
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited_ip",
            ));
        }
    }

    if let Some(claims) = req.extensions().get::<crate::middleware::auth::Claims>() {
        let key = format!("ratelimit:user:{}", claims.sub);
        if incr_and_check(&mut conn, &key, 60, 60)
            .await
            .unwrap_or(false)
        {
            tracing::warn!(user = %claims.sub, "rate limit exceeded for user");
            Metrics::inc(&state.metrics.rate_limit_block_user);
            return Err(Reject::new(
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited_user",
            ));
        }
    }

    if let Some(device) = req
        .headers()
        .get("x-device-id")
        .and_then(|h| h.to_str().ok())
    {
        let key = format!("ratelimit:device:{}", device);
        if incr_and_check(&mut conn, &key, 50, 60)
            .await
            .unwrap_or(false)
        {
            tracing::warn!(device = %device, "rate limit exceeded for device");
            Metrics::inc(&state.metrics.rate_limit_block_device);
            return Err(Reject::new(
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited_device",
            ));
        }
    }

    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitive_path_detection() {
        assert!(is_sensitive_path("/auth/login"));
        assert!(is_sensitive_path("/auth/register"));
        assert!(is_sensitive_path("/api/auth/login"));
        assert!(is_sensitive_path("/recovery/start"));
        assert!(is_sensitive_path("/recovery/complete"));
        assert!(is_sensitive_path("/user/devices/revoke"));

        assert!(!is_sensitive_path("/health"));
        assert!(!is_sensitive_path("/devices"));
        assert!(!is_sensitive_path("/profile"));
        assert!(!is_sensitive_path("/actions/do-this"));
        assert!(!is_sensitive_path("/api/profile/update"));
    }

    #[test]
    fn ip_from_request_returns_none_when_no_connect_info_and_no_xff() {
        let req = axum::http::Request::builder()
            .uri("/")
            .body(axum::body::Body::empty())
            .unwrap();
        // No ConnectInfo and no XFF — should return None.
        assert!(ip_from_request(&req).is_none());
    }

    #[test]
    fn ip_from_request_ignores_xff_without_trusted_peer() {
        // XFF is set but ConnectInfo is absent (peer unknown) — returns None,
        // not the spoofed XFF value.
        let req = axum::http::Request::builder()
            .uri("/")
            .header("x-forwarded-for", "1.2.3.4")
            .body(axum::body::Body::empty())
            .unwrap();
        assert!(ip_from_request(&req).is_none());
    }
}
