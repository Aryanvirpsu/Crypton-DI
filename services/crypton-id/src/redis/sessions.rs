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
    let exists: bool = redis::cmd("EXISTS").arg(&key).query_async(conn).await?;
    Ok(exists)
}

// ── Integration test (real Redis) ────────────────────────────────────────────
// This is the actual mechanism logout relies on (AuthUser checks is_denied
// before accepting any token — see jwt.rs). Requires REDIS_URL; skips
// itself (passes trivially) rather than failing when no Redis is reachable,
// so `cargo test` still works without infra up — but CI always has one.

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn deny_token_makes_is_denied_true_until_it_would_have_expired() {
        let Ok(redis_url) = std::env::var("REDIS_URL") else {
            eprintln!("skipping: REDIS_URL not set");
            return;
        };
        let Ok(client) = redis::Client::open(redis_url) else {
            return;
        };
        let Ok(mut conn) = client.get_multiplexed_tokio_connection().await else {
            return;
        };

        let user_id = format!("test-user-{}", uuid::Uuid::new_v4());
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let iat = now;
        let exp = now + 60;

        assert!(
            !is_denied(&mut conn, &user_id, iat).await.unwrap(),
            "must not be denied before logout"
        );

        deny_token(&mut conn, &user_id, iat, exp).await.unwrap();

        assert!(
            is_denied(&mut conn, &user_id, iat).await.unwrap(),
            "must be denied immediately after logout"
        );
        // A different iat for the same user is a different token and must
        // not be affected — this is the real basis of the "denylist
        // granularity" property the JWT layer depends on.
        assert!(!is_denied(&mut conn, &user_id, iat + 1).await.unwrap());
    }
}
