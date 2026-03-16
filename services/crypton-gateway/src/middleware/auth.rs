use axum::{extract::State, http::StatusCode, middleware::Next, Request, response::Response};
use axum::http::header::AUTHORIZATION;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

use crate::state::{AppState, Metrics};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    // additional fields (roles, device_id, etc.) can be added later
}

const JWT_SECRET_ENV: &str = "JWT_SECRET";

pub async fn validate_jwt<B>(
    State(state): State<AppState>,
    mut req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let header = req.headers().get(AUTHORIZATION).and_then(|h| h.to_str().ok());

    if let Some(h) = header {
        if let Some(token) = h.strip_prefix("Bearer ") {
            // first try JWT decode
            let secret = std::env::var(JWT_SECRET_ENV).unwrap_or_else(|_| "secret".into());
            if let Ok(data) = decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &Validation::default()) {
                // attach claims so later middleware/handlers can read
                req.extensions_mut().insert(data.claims.clone());
                tracing::debug!(user = %data.claims.sub, "auth: jwt accepted");
                Metrics::inc(&state.metrics.auth_jwt_accepted);
                return Ok(next.run(req).await);
            }

            // fall back to opaque token verification via identity service
            if let Some(claims) = verify_opaque_token(token).await {
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

async fn verify_opaque_token(token: &str) -> Option<Claims> {
    // call identity-service /verify (stub endpoint) to validate token
    let client = reqwest::Client::new();
    let url = std::env::var("IDENTITY_URL").unwrap_or_else(|_| "http://localhost:8081".into());
    let resp = client
        .post(format!("{}/verify", url))
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
