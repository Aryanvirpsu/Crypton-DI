use axum::{extract::State, routing::post, Json, Router};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

// ── /register/start ──────────────────────────────────────────────────────────

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
    let _: () = conn.set_ex(&key, "1", 300).await?;

    Ok(Json(RegisterStartResp {
        challenge_id,
        status: "stored",
    }))
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
    let _ = req.attestation; // ignored until Week 3

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
    status: &'static str,
}

async fn login_start(
    State(state): State<AppState>,
    Json(req): Json<LoginStartReq>,
) -> Result<Json<LoginStartResp>, AppError> {
    tracing::info!(username = %req.username, "login_start");

    let challenge_id = Uuid::new_v4();
    let key = login_key(challenge_id);

    let mut conn = state.redis_conn().await?;
    let _: () = conn.set_ex(&key, "1", 300).await?;

    Ok(Json(LoginStartResp {
        challenge_id,
        status: "stored",
    }))
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
    let _ = req.assertion; // ignored until Week 3

    Ok(Json(LoginFinishResp { status: "consumed" }))
}
