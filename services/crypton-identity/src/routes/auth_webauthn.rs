use axum::{extract::State, routing::post, Json, Router};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;
use webauthn_rs::prelude::*;

use crate::{
    error::AppError,
    jwt::issue_jwt,
    redis::challenges::{challenge_key, login_key},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/register/start", post(register_start))
        .route("/auth/register/finish", post(register_finish))
        .route("/auth/login/start", post(login_start))
        .route("/auth/login/finish", post(login_finish))
}

// ── Redis state payloads ──────────────────────────────────────────────────────

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
    ccr: CreationChallengeResponse,
}

async fn register_start(
    State(state): State<AppState>,
    Json(req): Json<RegisterStartReq>,
) -> Result<Json<RegisterStartResp>, AppError> {
    tracing::info!(username = %req.username, "register_start");

    let user_id = Uuid::new_v4();
    let display_name = req.display_name.as_deref().unwrap_or(&req.username);

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

// ── /register/finish ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RegisterFinishReq {
    challenge_id: Uuid,
    attestation: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct RegisterFinishResp {
    status: &'static str,
    user_id: Uuid,
}

async fn register_finish(
    State(state): State<AppState>,
    Json(req): Json<RegisterFinishReq>,
) -> Result<Json<RegisterFinishResp>, AppError> {
    tracing::info!(challenge_id = %req.challenge_id, "register_finish");

    let key = challenge_key(req.challenge_id);
    let mut conn = state.redis_conn().await?;

    // Fetch and consume the challenge state atomically
    let raw: Option<String> = conn.get(&key).await?;
    let raw = raw.ok_or_else(|| AppError::bad_request("invalid_or_expired_challenge"))?;
    let _: () = conn.del(&key).await?;

    let challenge: RegChallengeState = serde_json::from_str(&raw)
        .map_err(|e| AppError::internal(format!("failed_to_parse_challenge: {e}")))?;

    // Parse the attestation from the browser
    let rpk: RegisterPublicKeyCredential = serde_json::from_value(req.attestation)
        .map_err(|e| AppError::bad_request(format!("invalid_attestation: {e}")))?;

    // Cryptographically verify the registration
    let passkey = state
        .webauthn
        .finish_passkey_registration(&rpk, &challenge.reg_state)
        .map_err(|e| AppError::bad_request(format!("webauthn_error: {e}")))?;

    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let passkey_json = serde_json::to_string(&passkey)
        .map_err(|e| AppError::internal(e.to_string()))?;
    let cred_id_bytes: Vec<u8> = passkey.cred_id().to_vec();

    // Upsert user — handles re-registration (adding a new device) gracefully
    sqlx::query(
        "INSERT INTO users (id, username) VALUES ($1, $2) ON CONFLICT (username) DO NOTHING",
    )
    .bind(challenge.user_id)
    .bind(&challenge.username)
    .execute(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    // Resolve the canonical user_id (may differ from challenge if username already existed)
    let user_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE username = $1")
        .bind(&challenge.username)
        .fetch_one(db)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    // Persist the credential — cred_id_bytes also fills the public_key placeholder
    // until we extract it separately; the passkey column is the source of truth.
    sqlx::query(
        "INSERT INTO credentials (user_id, credential_id, public_key, sign_count, passkey) \
         VALUES ($1, $2, $3, 0, $4)",
    )
    .bind(user_id)
    .bind(&cred_id_bytes)
    .bind(&cred_id_bytes)
    .bind(&passkey_json)
    .execute(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    tracing::info!(user_id = %user_id, username = %challenge.username, "registration complete");
    Ok(Json(RegisterFinishResp { status: "ok", user_id }))
}

// ── /login/start ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct LoginStartReq {
    username: String,
}

#[derive(Debug, Serialize)]
struct LoginStartResp {
    challenge_id: Uuid,
    #[serde(flatten)]
    rcr: RequestChallengeResponse,
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

    let user_row = sqlx::query("SELECT id FROM users WHERE username = $1")
        .bind(&req.username)
        .fetch_optional(db)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?
        .ok_or_else(|| AppError::bad_request("user_not_found"))?;

    let user_id: Uuid = user_row
        .try_get("id")
        .map_err(|e| AppError::internal(e.to_string()))?;

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
    token: String,
}

async fn login_finish(
    State(state): State<AppState>,
    Json(req): Json<LoginFinishReq>,
) -> Result<Json<LoginFinishResp>, AppError> {
    tracing::info!(challenge_id = %req.challenge_id, "login_finish");

    let key = login_key(req.challenge_id);
    let mut conn = state.redis_conn().await?;

    // Fetch and consume the challenge state atomically
    let raw: Option<String> = conn.get(&key).await?;
    let raw = raw.ok_or_else(|| AppError::bad_request("invalid_or_expired_challenge"))?;
    let _: () = conn.del(&key).await?;

    let challenge: AuthChallengeState = serde_json::from_str(&raw)
        .map_err(|e| AppError::internal(format!("failed_to_parse_challenge: {e}")))?;

    // Parse the assertion from the browser
    let auth_cred: PublicKeyCredential = serde_json::from_value(req.assertion)
        .map_err(|e| AppError::bad_request(format!("invalid_assertion: {e}")))?;

    // Cryptographically verify the authentication
    let auth_result = state
        .webauthn
        .finish_passkey_authentication(&auth_cred, &challenge.auth_state)
        .map_err(|e| AppError::bad_request(format!("webauthn_error: {e}")))?;

    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let cred_id_bytes: Vec<u8> = auth_result.cred_id().to_vec();
    let new_counter = auth_result.counter() as i64;

    // **new**: verify the credential is still active.  Although `login_start`
    // only returned active credentials, a device could be revoked between the
    // start and finish phases (e.g. via the web UI).  We look up the status by
    // ID and reject if it isn't active.
    let status: Option<String> = sqlx::query_scalar(
        "SELECT status FROM credentials WHERE user_id = $1 AND credential_id = $2",
    )
    .bind(challenge.user_id)
    .bind(&cred_id_bytes)
    .fetch_optional(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    if status.as_deref() != Some("active") {
        return Err(AppError::bad_request("device_revoked"));
    }

    // Update counter and last_used_at for replay protection
    sqlx::query(
        "UPDATE credentials SET sign_count = $1, last_used_at = now() \
         WHERE user_id = $2 AND credential_id = $3",
    )
    .bind(new_counter)
    .bind(challenge.user_id)
    .bind(&cred_id_bytes)
    .execute(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    // Keep the stored Passkey JSON in sync (counter may have changed, backup state, etc.)
    if auth_result.needs_update() {
        let row = sqlx::query(
            "SELECT passkey FROM credentials WHERE user_id = $1 AND credential_id = $2",
        )
        .bind(challenge.user_id)
        .bind(&cred_id_bytes)
        .fetch_optional(db)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

        if let Some(row) = row {
            let passkey_json: &str = row
                .try_get("passkey")
                .map_err(|e| AppError::internal(e.to_string()))?;
            let mut passkey: Passkey = serde_json::from_str(passkey_json)
                .map_err(|e| AppError::internal(format!("failed_to_parse_passkey: {e}")))?;

            passkey.update_credential(&auth_result);

            let updated_json = serde_json::to_string(&passkey)
                .map_err(|e| AppError::internal(e.to_string()))?;

            sqlx::query(
                "UPDATE credentials SET passkey = $1 \
                 WHERE user_id = $2 AND credential_id = $3",
            )
            .bind(&updated_json)
            .bind(challenge.user_id)
            .bind(&cred_id_bytes)
            .execute(db)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        }
    }

    let token = issue_jwt(challenge.user_id, &challenge.username, &state.jwt_secret)?;

    tracing::info!(user_id = %challenge.user_id, username = %challenge.username, "login complete");
    Ok(Json(LoginFinishResp { status: "ok", token }))
}
