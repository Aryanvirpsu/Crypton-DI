use axum::{extract::State, routing::post, Json, Router};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;
use webauthn_rs::prelude::*;

use crate::{
    error::AppError,
    redis::challenges::{challenge_key, login_key},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/webauthn/register/start", post(register_start))
        .route("/auth/webauthn/register/finish", post(register_finish))
        .route("/auth/webauthn/login/start", post(login_start))
        .route("/auth/webauthn/login/finish", post(login_finish))
}

// ── Redis state payloads ──────────────────────────────────────────────────────
// These are serialised to JSON and stored in Redis during start, then
// deserialised in finish for cryptographic verification.

#[derive(Serialize, Deserialize)]
struct RegChallengeState {
    user_id: Uuid,
    username: String,
    reg_state: PasskeyRegistration, // requires danger-allow-state-serialisation
}

#[derive(Serialize, Deserialize)]
struct AuthChallengeState {
    user_id: Uuid,
    username: String,
    auth_state: PasskeyAuthentication, // requires danger-allow-state-serialisation
}

// ── /register/start ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RegisterStartReq {
    username: String,
    display_name: Option<String>,
}

#[derive(Debug, Serialize)]
struct RegisterStartResp {
    challenge_id: Uuid,
    #[serde(flatten)]
    ccr: CreationChallengeResponse, // what the browser's navigator.credentials.create() needs
}

async fn register_start(
    State(state): State<AppState>,
    Json(req): Json<RegisterStartReq>,
) -> Result<Json<RegisterStartResp>, AppError> {
    tracing::info!(username = %req.username, "register_start");

    let user_id = Uuid::new_v4();
    let display_name = req.display_name.as_deref().unwrap_or(&req.username);

    // Ask webauthn-rs to create the challenge. Returns:
    //   ccr       → send to browser
    //   reg_state → store in Redis, needed to verify the attestation in finish
    let (ccr, reg_state) = state
        .webauthn
        .start_passkey_registration(user_id, &req.username, display_name, None)
        .map_err(|e| AppError::internal(e.to_string()))?;

    let challenge_id = Uuid::new_v4();
    let key = challenge_key(challenge_id);

    let payload = serde_json::to_string(&RegChallengeState {
        user_id,
        username: req.username,
        reg_state,
    })
    .map_err(|e| AppError::internal(e.to_string()))?;

    let mut conn = state.redis_conn().await?;
    let _: () = conn.set_ex(&key, payload, 300).await?;

    Ok(Json(RegisterStartResp { challenge_id, ccr }))
}

// ── /register/finish ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RegisterFinishReq {
    challenge_id: Uuid,
    attestation: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct RegisterFinishResp {
    status: &'static str,
}

async fn register_finish(
    State(state): State<AppState>,
    Json(req): Json<RegisterFinishReq>,
) -> Result<Json<RegisterFinishResp>, AppError> {
    tracing::info!(challenge_id = %req.challenge_id, "register_finish");

    let key = challenge_key(req.challenge_id);
    let mut conn = state.redis_conn().await?;

    let exists: bool = conn.exists(&key).await?;
    if !exists {
        return Err(AppError::bad_request("invalid_or_expired_challenge"));
    }

    let _: () = conn.del(&key).await?;
    let _ = req.attestation; // verified in Checkpoint 4

    Ok(Json(RegisterFinishResp { status: "consumed" }))
}

// ── /login/start ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct LoginStartReq {
    username: String,
}

#[derive(Debug, Serialize)]
struct LoginStartResp {
    challenge_id: Uuid,
    #[serde(flatten)]
    rcr: RequestChallengeResponse, // what the browser's navigator.credentials.get() needs
}

async fn login_start(
    State(state): State<AppState>,
    Json(req): Json<LoginStartReq>,
) -> Result<Json<LoginStartResp>, AppError> {
    tracing::info!(username = %req.username, "login_start");

    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database not configured"))?;

    // Look up user
    let user_row = sqlx::query("SELECT id FROM users WHERE username = $1")
        .bind(&req.username)
        .fetch_optional(db)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::bad_request("user_not_found"))?;

    let user_id: Uuid = user_row
        .try_get("id")
        .map_err(|e| AppError::internal(e.to_string()))?;

    // Load their active passkeys
    let rows = sqlx::query(
        "SELECT passkey FROM credentials WHERE user_id = $1 AND status = 'active'",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    if rows.is_empty() {
        return Err(AppError::bad_request("no_credentials"));
    }

    let passkeys: Vec<Passkey> = rows
        .iter()
        .filter_map(|r| {
            let json: &str = r.try_get("passkey").ok()?;
            serde_json::from_str(json).ok()
        })
        .collect();

    if passkeys.is_empty() {
        return Err(AppError::internal("failed_to_load_credentials"));
    }

    // Ask webauthn-rs to create the authentication challenge
    let (rcr, auth_state) = state
        .webauthn
        .start_passkey_authentication(&passkeys)
        .map_err(|e| AppError::internal(e.to_string()))?;

    let challenge_id = Uuid::new_v4();
    let key = login_key(challenge_id);

    let payload = serde_json::to_string(&AuthChallengeState {
        user_id,
        username: req.username,
        auth_state,
    })
    .map_err(|e| AppError::internal(e.to_string()))?;

    let mut conn = state.redis_conn().await?;
    let _: () = conn.set_ex(&key, payload, 300).await?;

    Ok(Json(LoginStartResp { challenge_id, rcr }))
}

// ── /login/finish ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct LoginFinishReq {
    challenge_id: Uuid,
    assertion: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct LoginFinishResp {
    status: &'static str,
}

async fn login_finish(
    State(state): State<AppState>,
    Json(req): Json<LoginFinishReq>,
) -> Result<Json<LoginFinishResp>, AppError> {
    tracing::info!(challenge_id = %req.challenge_id, "login_finish");

    let key = login_key(req.challenge_id);
    let mut conn = state.redis_conn().await?;

    let exists: bool = conn.exists(&key).await?;
    if !exists {
        return Err(AppError::bad_request("invalid_or_expired_challenge"));
    }

    let _: () = conn.del(&key).await?;
    let _ = req.assertion; // verified in Checkpoint 5

    Ok(Json(LoginFinishResp { status: "consumed" }))
}
