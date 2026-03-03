use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use async_trait::async_trait;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::{error::AppError, state::AppState};

// ── Claims ────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,     // user_id
    pub username: String,
    pub iat: u64,
    pub exp: u64,
}

pub fn issue_jwt(user_id: Uuid, username: &str, secret: &str) -> Result<String, AppError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AppError::internal(e.to_string()))?
        .as_secs();

    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_owned(),
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

        Ok(AuthUser {
            user_id,
            username: token_data.claims.username,
        })
    }
}
