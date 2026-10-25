//! Bearer-token authentication against the `crypton-id` token contract.
//!
//! Trust boundary: `crypton-id` is the only issuer. It signs EdDSA (Ed25519)
//! tokens whose header carries `kid = "v1"` and whose claims are
//! `{sub (user UUID), username, cred_id (credential UUID), iat, exp}`
//! (see `services/crypton-id/src/jwt.rs`). The gateway holds only the PUBLIC
//! key (`JWT_PUBLIC_KEY_PEM` / `_FILE`) and can verify but never mint.
//!
//! The contract has no `iss`/`aud` claim, so issuer/audience are not checked;
//! trust rests on the signature under the dedicated identity key. Anything
//! other than EdDSA (HS256, `none`, RS*) is rejected before signature
//! verification; there is deliberately no shared-secret or HS256 fallback.
//!
//! After the signature, the gateway applies the same revocation semantics
//! identity does on every request: the Redis logout denylist
//! (`jwt:deny:{sub}:{iat}`) and "credential `cred_id` still `active` for
//! `sub`" in Postgres. Unlike identity, an unavailable Redis/Postgres here
//! fails CLOSED (503), never open.

use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, decode_header, errors::ErrorKind, Algorithm, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::middleware::decision::PrincipalSlot;
use crate::middleware::public_routes::{is_public_route, PublicRoute};
use crate::middleware::reject::Reject;
use crate::state::{AppState, Metrics};

/// Key id carried in every identity token header (`crypton-id` `JWT_KID`).
/// Anything else is an unknown key and is rejected until rotation support is
/// added deliberately.
pub const JWT_KID: &str = "v1";

/// Claims exactly as `crypton-id` issues them. Every field is required; a token
/// missing any of them fails deserialisation and is rejected.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Claims {
    /// User id (UUID string).
    pub sub: String,
    pub username: String,
    /// Credential/device id (UUID string) the token was issued for.
    pub cred_id: String,
    pub iat: u64,
    pub exp: u64,
}

pub async fn validate_jwt(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, Reject> {
    let public = is_public_route(req.method(), req.uri().path());

    let claims = match req.headers().get(AUTHORIZATION) {
        None if public => {
            req.extensions_mut().insert(PublicRoute);
            return Ok(next.run(req).await);
        }
        None => return Err(deny(&state, "missing_authorization_header")),
        Some(h) => {
            // A presented token must always be valid, even on a public route:
            // identity may use it (e.g. enrolling a device for an existing
            // user), so a bad/revoked token is never silently downgraded to
            // anonymous.
            let token = h
                .to_str()
                .ok()
                .and_then(|h| h.strip_prefix("Bearer "))
                .ok_or_else(|| deny(&state, "invalid_authorization_scheme"))?;
            authenticate(&state, token).await?
        }
    };

    if public {
        req.extensions_mut().insert(PublicRoute);
    }
    if let Some(slot) = req.extensions().get::<PrincipalSlot>() {
        slot.set(&claims);
    }
    tracing::debug!(user = %claims.sub, "auth: token accepted");
    Metrics::inc(&state.metrics.auth_jwt_accepted);
    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}

fn deny(state: &AppState, reason: &'static str) -> Reject {
    tracing::warn!(reason, "auth: rejected");
    Metrics::inc(&state.metrics.auth_rejected);
    Reject::new(StatusCode::UNAUTHORIZED, reason)
}

fn unavailable(state: &AppState, reason: &'static str) -> Reject {
    tracing::error!(reason, "auth: dependency unavailable, failing closed");
    Metrics::inc(&state.metrics.auth_rejected);
    Reject::new(StatusCode::SERVICE_UNAVAILABLE, reason)
}

/// Verify signature and claims, then apply revocation.
async fn authenticate(state: &AppState, token: &str) -> Result<Claims, Reject> {
    let header = decode_header(token).map_err(|_| deny(state, "token_malformed"))?;
    if header.alg != Algorithm::EdDSA {
        return Err(deny(state, "token_algorithm_not_allowed"));
    }
    if header.kid.as_deref() != Some(JWT_KID) {
        return Err(deny(state, "token_unknown_key_id"));
    }

    let mut validation = Validation::new(Algorithm::EdDSA);
    // Pin explicitly: jsonwebtoken's default would otherwise be HS256-capable.
    validation.algorithms = vec![Algorithm::EdDSA];
    validation.set_required_spec_claims(&["exp", "sub"]);

    let claims = decode::<Claims>(token, &state.jwt_decoding_key, &validation)
        .map(|d| d.claims)
        .map_err(|e| {
            deny(
                state,
                match e.kind() {
                    ErrorKind::ExpiredSignature => "token_expired",
                    ErrorKind::InvalidSignature => "token_signature_invalid",
                    ErrorKind::MissingRequiredClaim(_) => "token_missing_claim",
                    ErrorKind::Json(_) => "token_claims_invalid",
                    _ => "token_invalid",
                },
            )
        })?;

    let user_id =
        Uuid::parse_str(&claims.sub).map_err(|_| deny(state, "token_subject_not_uuid"))?;
    let cred_id =
        Uuid::parse_str(&claims.cred_id).map_err(|_| deny(state, "token_credential_not_uuid"))?;
    if claims.iat == 0 {
        return Err(deny(state, "token_missing_claim"));
    }

    check_not_revoked(state, &claims).await?;
    check_credential_active(state, cred_id, user_id).await?;
    Ok(claims)
}

/// Logout denylist and role-invalidation cutoff in the Redis identity shares.
/// Fails closed: if Redis cannot answer, the token is not accepted.
async fn check_not_revoked(state: &AppState, claims: &Claims) -> Result<(), Reject> {
    let mut conn = state.redis.as_ref().clone();

    let deny_key = format!("jwt:deny:{}:{}", claims.sub, claims.iat);
    match redis::cmd("EXISTS")
        .arg(&deny_key)
        .query_async::<bool>(&mut conn)
        .await
    {
        Ok(true) => return Err(deny(state, "token_revoked")),
        Ok(false) => {}
        Err(_) => return Err(unavailable(state, "revocation_check_unavailable")),
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
                    return Err(deny(state, "token_invalidated"));
                }
            }
        }
        Ok(None) => {}
        Err(_) => return Err(unavailable(state, "revocation_check_unavailable")),
    }
    Ok(())
}

/// Mirror of `crypton-id`'s `verify_device_active`: the credential the token
/// was issued for must still exist, belong to `sub`, and be `active`
/// (revoked / lost / unknown are all the same 401).
async fn check_credential_active(
    state: &AppState,
    cred_id: Uuid,
    user_id: Uuid,
) -> Result<(), Reject> {
    let row: Result<Option<(String,)>, _> =
        sqlx::query_as("SELECT status FROM credentials WHERE id = $1 AND user_id = $2")
            .bind(cred_id)
            .bind(user_id)
            .fetch_optional(&state.pg)
            .await;
    match row {
        Ok(Some((status,))) if status == "active" => Ok(()),
        Ok(_) => Err(deny(state, "credential_not_active")),
        Err(e) => {
            tracing::error!(error = %e, "auth: credential lookup failed");
            Err(unavailable(state, "credential_check_unavailable"))
        }
    }
}

/// Guard for the internal diagnostics router. Must run after [`validate_jwt`].
///
/// Identity tokens carry no role claim (role lives in `users.role`), so the
/// role is read from Postgres, the same source `crypton-id`'s `AdminUser`
/// extractor uses. Unknown user / DB error => no access.
pub async fn require_admin(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, Reject> {
    let claims = req
        .extensions()
        .get::<Claims>()
        .cloned()
        .ok_or_else(|| Reject::new(StatusCode::UNAUTHORIZED, "no_authenticated_principal"))?;
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| Reject::new(StatusCode::UNAUTHORIZED, "token_subject_not_uuid"))?;

    let role: Result<Option<String>, _> =
        sqlx::query_scalar("SELECT role FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&state.pg)
            .await;
    match role {
        Ok(Some(r)) if is_admin_role(&r) => Ok(next.run(req).await),
        Ok(_) => {
            tracing::warn!(user = %claims.sub, "auth: admin route rejected (insufficient role)");
            Err(Reject::new(StatusCode::FORBIDDEN, "admin_role_required"))
        }
        Err(e) => {
            tracing::error!(error = %e, "auth: admin role lookup failed");
            Err(Reject::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "role_check_unavailable",
            ))
        }
    }
}

fn is_admin_role(role: &str) -> bool {
    // Both admin and super_admin are administrative roles (`crypton-id`
    // `AdminUser`).
    matches!(role, "admin" | "super_admin")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claims_require_every_identity_field() {
        let full = r#"{"sub":"u","username":"n","cred_id":"c","iat":1,"exp":2}"#;
        assert!(serde_json::from_str::<Claims>(full).is_ok());
        for missing in ["sub", "username", "cred_id", "iat", "exp"] {
            let mut v: serde_json::Value = serde_json::from_str(full).unwrap();
            v.as_object_mut().unwrap().remove(missing);
            assert!(
                serde_json::from_value::<Claims>(v).is_err(),
                "claims without {missing} must not deserialize"
            );
        }
    }

    #[test]
    fn admin_roles() {
        assert!(is_admin_role("admin"));
        assert!(is_admin_role("super_admin"));
        assert!(!is_admin_role("user"));
        assert!(!is_admin_role("moderator"));
        assert!(!is_admin_role(""));
    }
}
