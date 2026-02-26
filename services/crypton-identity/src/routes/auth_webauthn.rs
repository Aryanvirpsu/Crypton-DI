use axum::{extract::State, routing::post, Json, Router};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::AppError,
    redis::challenges::challenge_key,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/webauthn/register/start", post(register_start))
        .route("/auth/webauthn/register/finish", post(register_finish))
}

// ── /register/start ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RegisterStartReq {
    username: String,
    display_name: Option<String>,
}

#[derive(Debug, Serialize)]
struct RegisterStartResp {
    challenge_id: Uuid,
    status: &'static str,
}

async fn register_start(
    State(state): State<AppState>,
    Json(req): Json<RegisterStartReq>,
) -> Result<Json<RegisterStartResp>, AppError> {
    tracing::info!(username = %req.username, "register_start");

    let challenge_id = Uuid::new_v4();
    let key = challenge_key(challenge_id);

    let mut conn = state.redis_conn().await?;
    // Store a placeholder value with a 5-minute TTL.
    let _: () = conn.set_ex(&key, "1", 300).await?;

    Ok(Json(RegisterStartResp {
        challenge_id,
        status: "stored",
    }))
}

// ── /register/finish ─────────────────────────────────────────────────────────

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

    // One-time use: check existence then immediately delete.
    let exists: bool = conn.exists(&key).await?;
    if !exists {
        return Err(AppError::bad_request("invalid_or_expired_challenge"));
    }

    let _: () = conn.del(&key).await?;
    let _ = req.attestation; // ignored until Week 3 (real attestation verification)

    Ok(Json(RegisterFinishResp { status: "consumed" }))
}
