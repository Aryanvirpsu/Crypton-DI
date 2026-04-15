/// Redis-backed JWT denylist for logout invalidation.
///
/// When a user logs out, their JWT's unique `iat` (issued-at) claim is added
/// to a Redis set with a TTL matching the JWT's remaining lifetime. The
/// `AuthUser` extractor checks this denylist before accepting a token.
///
/// This gives server-side session revocation without full session migration.

/// Redis key for a denylisted JWT.
/// Format: `jwt:deny:<user_id>:<iat>`
pub fn deny_key(user_id: &str, iat: u64) -> String {
    format!("jwt:deny:{}:{}", user_id, iat)
}

/// Add a JWT to the denylist. TTL = remaining seconds until expiry.
pub async fn deny_token(
    conn: &mut redis::aio::MultiplexedConnection,
    user_id: &str,
    iat: u64,
    exp: u64,
) -> Result<(), redis::RedisError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let ttl = if exp > now { exp - now } else { 1 };
    let key = deny_key(user_id, iat);
    let _: () = redis::cmd("SETEX")
        .arg(&key)
        .arg(ttl)
        .arg("1")
        .query_async(conn)
        .await?;
    Ok(())
}

/// Check if a JWT has been denylisted (returns true if denied).
pub async fn is_denied(
    conn: &mut redis::aio::MultiplexedConnection,
    user_id: &str,
    iat: u64,
) -> Result<bool, redis::RedisError> {
    let key = deny_key(user_id, iat);
    let exists: bool = redis::cmd("EXISTS")
        .arg(&key)
        .query_async(conn)
        .await?;
    Ok(exists)
}
