use axum::{extract::State, http::StatusCode, middleware::Next, Request, response::Response};
use crate::state::AppState;
use crate::state::Metrics;

async fn incr_and_check(conn: &mut redis::aio::ConnectionManager, key: &str, limit: i32, window_secs: usize) -> redis::RedisResult<bool> {
    // increment the counter, set TTL if new, return true if over limit
    let cnt: i32 = redis::cmd("INCR").arg(key).query_async(conn).await?;
    let ttl: i32 = redis::cmd("TTL").arg(key).query_async(conn).await.unwrap_or(-1);
    if ttl < 0 {
        let _: () = redis::cmd("EXPIRE").arg(key).arg(window_secs).query_async(conn).await?;
    }
    Ok(cnt > limit)
}

fn is_sensitive_path(path: &str) -> bool {
    let p = path.strip_prefix("/api").unwrap_or(path);
    p.starts_with("/auth/")
        || p.starts_with("/recovery/")
        || p.contains("/devices/revoke")
}

fn ip_from_request<B>(req: &Request<B>) -> Option<String> {
    // Prefer reverse-proxy header if present.
    if let Some(ip) = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        return Some(ip.to_string());
    }

    // Fall back to ConnectInfo if the router is configured for it.
    req.extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip().to_string())
}

pub async fn limit<B>(
    State(state): State<AppState>,
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    // Apply rate limiting only to sensitive endpoints (MVP).
    let path = req.uri().path();
    if !is_sensitive_path(path) {
        return Ok(next.run(req).await);
    }

    let conn = state.redis.as_ref().clone();
    let mut conn = conn;

    // IP limit
    if let Some(ip) = ip_from_request(&req) {
        let key = format!("ratelimit:ip:{}", ip);
        if incr_and_check(&mut conn, &key, 100, 60).await.unwrap_or(false) {
            tracing::warn!(%ip, key = %key, "rate limit exceeded for ip");
            Metrics::inc(&state.metrics.rate_limit_block_ip);
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
    }

    // user limit (if authenticated)
    if let Some(claims) = req.extensions().get::<crate::middleware::auth::Claims>() {
        let key = format!("ratelimit:user:{}", claims.sub);
        if incr_and_check(&mut conn, &key, 60, 60).await.unwrap_or(false) {
            tracing::warn!(user = %claims.sub, key = %key, "rate limit exceeded for user");
            Metrics::inc(&state.metrics.rate_limit_block_user);
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
    }

    // device limit (look for header or query param)
    if let Some(device) = req
        .headers()
        .get("x-device-id")
        .and_then(|h| h.to_str().ok())
    {
        let key = format!("ratelimit:device:{}", device);
        if incr_and_check(&mut conn, &key, 50, 60).await.unwrap_or(false) {
            tracing::warn!(device = %device, key = %key, "rate limit exceeded for device");
            Metrics::inc(&state.metrics.rate_limit_block_device);
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
    }

    Ok(next.run(req).await)
}