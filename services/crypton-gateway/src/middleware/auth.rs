use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

use crate::state::{AppState, Metrics};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    /// Issued-at (unix seconds). Used to key the logout denylist
    /// (`jwt:deny:{sub}:{iat}`) and the role-invalidation cutoff. Defaults to 0
    /// for opaque tokens verified upstream, which carry no `iat`.
    #[serde(default)]
    pub iat: u64,
    #[serde(default)]
    pub role: Option<String>,
    /// JWT token ID — used to scope step-up verification to a single token.
    /// Populated when the identity service includes `jti` in the JWT payload.
    #[serde(default)]
    pub jti: Option<String>,
}

pub async fn validate_jwt(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    if let Some(h) = header {
        if let Some(token) = h.strip_prefix("Bearer ") {
            if let Ok(data) = decode::<Claims>(
                token,
                &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
                &Validation::default(),
            ) {
                // Defence-in-depth revocation enforcement. The identity service
                // writes the logout denylist and role-invalidation keys; the
                // gateway shares the same Redis, so a revoked token must not be
                // allowed to run policy here even though identity re-checks too.
                if is_token_revoked(&state, &data.claims).await {
                    tracing::warn!(user = %data.claims.sub, "auth: revoked/invalidated token rejected");
                    Metrics::inc(&state.metrics.auth_rejected);
                    return Err(StatusCode::UNAUTHORIZED);
                }
                req.extensions_mut().insert(data.claims.clone());
                tracing::debug!(user = %data.claims.sub, "auth: jwt accepted");
                Metrics::inc(&state.metrics.auth_jwt_accepted);
                return Ok(next.run(req).await);
            }

            // Fall back to opaque token verification via identity service.
            // Uses the shared http_client to avoid per-request connection overhead.
            if let Some(claims) =
                verify_opaque_token(&state.http_client, token, &state.identity_base).await
            {
                req.extensions_mut().insert(claims);
                tracing::debug!("auth: opaque token accepted");
                Metrics::inc(&state.metrics.auth_opaque_accepted);
                return Ok(next.run(req).await);
            }

            tracing::warn!("auth: bearer token rejected");
        }
    }

    tracing::warn!("auth: missing/invalid authorization header");
    Metrics::inc(&state.metrics.auth_rejected);
    Err(StatusCode::UNAUTHORIZED)
}

/// Check the identity service's Redis revocation state for this token.
///
/// Mirrors `crypton-id`'s `redis::sessions`:
///   * `jwt:deny:{sub}:{iat}`        — logout denylist (key present ⇒ revoked)
///   * `jwt:role_invalidate:{sub}`   — unix ts; tokens with `iat < ts` are stale
///
/// Fails open: a Redis error here defers to the identity service, which
/// re-checks with its own in-memory degradation cache. That keeps a Redis blip
/// from taking down all gateway auth while preserving end-to-end revocation.
async fn is_token_revoked(state: &AppState, claims: &Claims) -> bool {
    // iat == 0 ⇒ opaque token (no issued-at); identity is the authority for those.
    if claims.iat == 0 {
        return false;
    }

    let mut conn = state.redis.as_ref().clone();

    let deny_key = format!("jwt:deny:{}:{}", claims.sub, claims.iat);
    match redis::cmd("EXISTS")
        .arg(&deny_key)
        .query_async::<bool>(&mut conn)
        .await
    {
        Ok(true) => return true,
        Ok(false) => {}
        Err(e) => {
            tracing::warn!(error = %e, "auth: denylist check failed, deferring to identity");
            return false;
        }
    }

    let role_key = format!("jwt:role_invalidate:{}", claims.sub);
    match redis::cmd("GET")
        .arg(&role_key)
        .query_async::<Option<String>>(&mut conn)
        .await
    {
        Ok(Some(ts)) => {
            if let Ok(invalidated_at) = ts.parse::<u64>() {
                if claims.iat < invalidated_at {
                    return true;
                }
            }
        }
        Ok(None) => {}
        Err(e) => {
            tracing::warn!(error = %e, "auth: role-invalidation check failed, deferring to identity");
        }
    }

    false
}

/// Guard middleware — must run after [`validate_jwt`].
/// Returns 403 if the caller's JWT does not carry `"admin"` in the `role` claim.
pub async fn require_admin(req: Request, next: Next) -> Result<Response, StatusCode> {
    let claims = req
        .extensions()
        .get::<Claims>()
        .cloned()
        .ok_or(StatusCode::UNAUTHORIZED)?;
    check_admin_role(&claims)?;
    Ok(next.run(req).await)
}

fn check_admin_role(claims: &Claims) -> Result<(), StatusCode> {
    // Both admin and super_admin are administrative roles. The identity service
    // mints "super_admin" for top-tier operators; rejecting it here would lock
    // them out of diagnostics and push them to use a lower-privileged account.
    if !matches!(claims.role.as_deref(), Some("admin") | Some("super_admin")) {
        tracing::warn!(user = %claims.sub, "auth: admin route rejected (insufficient role)");
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_claims(role: Option<&str>, jti: Option<&str>) -> Claims {
        Claims {
            sub: "user-123".to_string(),
            exp: 9999999999,
            iat: 0,
            role: role.map(|s| s.to_string()),
            jti: jti.map(|s| s.to_string()),
        }
    }

    #[test]
    fn claims_deserializes_without_optional_fields() {
        let json = r#"{"sub":"u1","exp":9999}"#;
        let c: Claims = serde_json::from_str(json).unwrap();
        assert_eq!(c.sub, "u1");
        assert!(c.role.is_none());
        assert!(c.jti.is_none());
    }

    #[test]
    fn claims_deserializes_with_all_fields() {
        let json = r#"{"sub":"u1","exp":9999,"iat":1700000000,"role":"admin","jti":"tok-xyz"}"#;
        let c: Claims = serde_json::from_str(json).unwrap();
        assert_eq!(c.role.as_deref(), Some("admin"));
        assert_eq!(c.jti.as_deref(), Some("tok-xyz"));
        assert_eq!(c.iat, 1700000000);
    }

    #[test]
    fn claims_iat_defaults_to_zero_when_absent() {
        // Opaque tokens (verified upstream) carry no iat — must default to 0 so
        // the gateway skips the iat-keyed denylist check and defers to identity.
        let json = r#"{"sub":"u1","exp":9999}"#;
        let c: Claims = serde_json::from_str(json).unwrap();
        assert_eq!(c.iat, 0);
    }

    #[test]
    fn admin_role_is_allowed() {
        assert!(check_admin_role(&make_claims(Some("admin"), None)).is_ok());
    }

    #[test]
    fn super_admin_role_is_allowed() {
        assert!(check_admin_role(&make_claims(Some("super_admin"), None)).is_ok());
    }

    #[test]
    fn non_admin_role_is_rejected() {
        assert_eq!(
            check_admin_role(&make_claims(Some("user"), None)),
            Err(StatusCode::FORBIDDEN)
        );
        assert_eq!(
            check_admin_role(&make_claims(Some("moderator"), None)),
            Err(StatusCode::FORBIDDEN)
        );
    }

    #[test]
    fn missing_role_is_rejected() {
        assert_eq!(
            check_admin_role(&make_claims(None, None)),
            Err(StatusCode::FORBIDDEN)
        );
    }
}

async fn verify_opaque_token(
    client: &reqwest::Client,
    token: &str,
    identity_base: &str,
) -> Option<Claims> {
    let resp = client
        .post(format!("{}/verify", identity_base))
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .await
        .ok()?;
    if resp.status().is_success() {
        resp.json::<Claims>().await.ok()
    } else {
        None
    }
}
