use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use async_trait::async_trait;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use sqlx::{self, Row};

use crate::{error::AppError, state::AppState};
use sqlx::PgPool;

// ── Claims ────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,      // user_id
    pub username: String,
    pub cred_id: String,  // credential/device uuid
    pub iat: u64,
    pub exp: u64,
}

pub async fn verify_device_active(pool: &PgPool, cred_uuid: Uuid, user_id: Uuid) -> Result<(), AppError> {
    let row = sqlx::query(
        "SELECT status FROM credentials WHERE id = $1 AND user_id = $2",
    )
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

pub fn issue_jwt(user_id: Uuid, username: &str, credential_id: Uuid, secret: &str) -> Result<String, AppError> {
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

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, AppError> {
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
            ).await {
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
