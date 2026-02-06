use axum::{routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/webauthn/register/start", post(register_start))
        .route("/auth/webauthn/register/finish", post(register_finish))
}

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

async fn register_start(Json(req): Json<RegisterStartReq>) -> Json<RegisterStartResp> {
    tracing::info!(username = %req.username, "register_start");
    Json(RegisterStartResp {
        challenge_id: Uuid::new_v4(),
        status: "stubbed",
    })
}

#[derive(Debug, Deserialize)]
struct RegisterFinishReq {
    challenge_id: Uuid,
    attestation: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct RegisterFinishResp {
    status: &'static str,
}

async fn register_finish(Json(req): Json<RegisterFinishReq>) -> Json<RegisterFinishResp> {
    tracing::info!(challenge_id = %req.challenge_id, "register_finish");
    let _ = req.attestation;
    Json(RegisterFinishResp { status: "stubbed" })
}
