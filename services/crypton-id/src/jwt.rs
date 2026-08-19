use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sqlx::{self, Row};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::{error::AppError, state::AppState};
use sqlx::PgPool;

// ── Claims ────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String, // user_id
    pub username: String,
    pub cred_id: String, // credential/device uuid
    pub iat: u64,
    pub exp: u64,
}

pub async fn verify_device_active(
    pool: &PgPool,
    cred_uuid: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    let row = sqlx::query("SELECT status FROM credentials WHERE id = $1 AND user_id = $2")
        .bind(cred_uuid)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| AppError::unauthorized("device_not_active"))?;

    let active = row
        .and_then(|r| r.try_get::<String, _>("status").ok())
        .map(|s| s == "active")
        .unwrap_or(false);

    if !active {
        return Err(AppError::unauthorized("device_not_active"));
    }
    Ok(())
}

pub fn issue_jwt(
    user_id: Uuid,
    username: &str,
    credential_id: Uuid,
    secret: &str,
) -> Result<String, AppError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AppError::internal(e.to_string()))?
        .as_secs();

    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_owned(),
        cred_id: credential_id.to_string(),
        iat: now,
        exp: now + 3600, // 1 hour
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::internal(format!("jwt_encode_error: {e}")))
}

// ── AuthUser extractor ────────────────────────────────────────────────────────
// Use this as a handler parameter on any route that requires authentication.
// Returns 401 if the header is missing, malformed, or the token is invalid/expired.

pub struct AuthUser {
    pub user_id: Uuid,
    pub username: String,
    pub cred_id: Uuid,
}

pub struct AdminUser {
    pub user_id: Uuid,
    pub username: String,
    pub cred_id: Uuid,
    pub role: String,
}

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::unauthorized("missing_authorization_header"))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AppError::unauthorized("invalid_authorization_scheme"))?;

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| AppError::unauthorized(format!("invalid_token: {e}")))?;

        let user_id = Uuid::parse_str(&token_data.claims.sub)
            .map_err(|_| AppError::unauthorized("malformed_subject_claim"))?;

        let cred_uuid = Uuid::parse_str(&token_data.claims.cred_id)
            .map_err(|_| AppError::unauthorized("malformed_cred_id"))?;

        // Check JWT denylist (logout invalidation) before any DB work.
        if let Ok(mut conn) = state.redis_conn().await {
            if let Ok(true) = crate::redis::sessions::is_denied(
                &mut conn,
                &token_data.claims.sub,
                token_data.claims.iat,
            )
            .await
            {
                return Err(AppError::unauthorized("session_invalidated"));
            }
        }

        // If a DB pool is available, verify the credential is still active.
        // Revoked and not-found are both reported as the same 401 to avoid
        // leaking whether a device was revoked vs never existed.
        if let Some(ref pool) = state.db {
            verify_device_active(pool, cred_uuid, user_id).await?;
        }

        Ok(AuthUser {
            user_id,
            username: token_data.claims.username,
            cred_id: cred_uuid,
        })
    }
}

#[async_trait]
impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        let auth = AuthUser::from_request_parts(parts, state).await?;

        let db = state
            .db
            .as_ref()
            .ok_or_else(|| AppError::unauthorized("admin_authorization_required"))?;

        let role: Option<String> = sqlx::query_scalar("SELECT role FROM users WHERE id = $1")
            .bind(auth.user_id)
            .fetch_optional(db)
            .await
            .map_err(|_| AppError::unauthorized("admin_authorization_required"))?;

        let role = role.ok_or_else(|| AppError::forbidden("admin_authorization_required"))?;
        if role != "admin" && role != "super_admin" {
            return Err(AppError::forbidden("admin_authorization_required"));
        }

        Ok(AdminUser {
            user_id: auth.user_id,
            username: auth.username,
            cred_id: auth.cred_id,
            role,
        })
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────
// Pure JWT issue/verify/expiry logic — no DB or Redis required, so these run
// anywhere `cargo test` runs, including plain `cargo test --lib` with no
// services up.

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "test-secret-do-not-use-in-prod";

    #[test]
    fn issue_jwt_roundtrips_claims() {
        let user_id = Uuid::new_v4();
        let cred_id = Uuid::new_v4();
        let token = issue_jwt(user_id, "alice", cred_id, SECRET).unwrap();

        let data = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(SECRET.as_bytes()),
            &Validation::default(),
        )
        .expect("token should decode with the correct secret");

        assert_eq!(data.claims.sub, user_id.to_string());
        assert_eq!(data.claims.username, "alice");
        assert_eq!(data.claims.cred_id, cred_id.to_string());
        assert_eq!(data.claims.exp, data.claims.iat + 3600);
    }

    #[test]
    fn decode_fails_with_wrong_secret() {
        let token = issue_jwt(Uuid::new_v4(), "bob", Uuid::new_v4(), SECRET).unwrap();

        let result = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(b"a-completely-different-secret"),
            &Validation::default(),
        );

        assert!(result.is_err(), "decoding with the wrong secret must fail");
    }

    #[test]
    fn decode_rejects_expired_token() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Hand-build claims that expired well outside jsonwebtoken's default
        // 60s leeway — issue_jwt always mints exp = iat + 3600, so we bypass
        // it here to exercise the expiry check itself. (A first version of
        // this test used `exp: now - 10`, which is INSIDE the default 60s
        // leeway and therefore validates successfully — that was a bug in
        // the test, not the code; caught by actually running `cargo test`.)
        let claims = Claims {
            sub: Uuid::new_v4().to_string(),
            username: "carol".into(),
            cred_id: Uuid::new_v4().to_string(),
            iat: now - 3720,
            exp: now - 120,
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(SECRET.as_bytes()),
        )
        .unwrap();

        let result = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(SECRET.as_bytes()),
            &Validation::default(),
        );

        assert!(result.is_err(), "an expired token must fail validation");
        assert_eq!(
            result.unwrap_err().kind(),
            &jsonwebtoken::errors::ErrorKind::ExpiredSignature
        );
    }

    #[test]
    fn decode_accepts_token_at_the_expiry_boundary() {
        // jsonwebtoken's default Validation allows a small leeway (60s), so
        // a token that expired a couple of seconds ago should still pass —
        // this pins that documented behaviour rather than assuming it.
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let claims = Claims {
            sub: Uuid::new_v4().to_string(),
            username: "dave".into(),
            cred_id: Uuid::new_v4().to_string(),
            iat: now - 3602,
            exp: now - 2,
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(SECRET.as_bytes()),
        )
        .unwrap();

        let result = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(SECRET.as_bytes()),
            &Validation::default(),
        );

        assert!(
            result.is_ok(),
            "default 60s leeway should cover a 2s-expired token"
        );
    }
}
