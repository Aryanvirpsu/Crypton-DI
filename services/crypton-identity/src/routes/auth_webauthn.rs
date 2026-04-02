use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;
use webauthn_rs::prelude::*;

use crate::{
    audit,
    error::AppError,
    jwt::{issue_jwt, AuthUser},
    redis::challenges::{challenge_key, login_key},
    state::{AppState, AuthChallenge, RegChallenge, CHALLENGE_TTL_SECS},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/register/start", post(register_start))
        .route("/auth/register/finish", post(register_finish))
        .route("/auth/login/start", post(login_start))
        .route("/auth/login/finish", post(login_finish))
}

// ── Challenge helpers ─────────────────────────────────────────────────────────

fn store_reg_challenge(state: &AppState, key: String, challenge: RegChallenge) {
    let expiry = std::time::Instant::now()
        + std::time::Duration::from_secs(CHALLENGE_TTL_SECS);
    state
        .reg_challenges
        .lock()
        .unwrap()
        .insert(key, (challenge, expiry));
}

fn take_reg_challenge(state: &AppState, key: &str) -> Option<RegChallenge> {
    let entry = state.reg_challenges.lock().unwrap().remove(key)?;
    let (challenge, expiry) = entry;
    if std::time::Instant::now() >= expiry {
        return None;
    }
    Some(challenge)
}

pub fn store_auth_challenge(state: &AppState, key: String, challenge: AuthChallenge) {
    let expiry = std::time::Instant::now()
        + std::time::Duration::from_secs(CHALLENGE_TTL_SECS);
    state
        .auth_challenges
        .lock()
        .unwrap()
        .insert(key, (challenge, expiry));
}

pub fn take_auth_challenge(state: &AppState, key: &str) -> Option<AuthChallenge> {
    let entry = state.auth_challenges.lock().unwrap().remove(key)?;
    let (challenge, expiry) = entry;
    if std::time::Instant::now() >= expiry {
        return None;
    }
    Some(challenge)
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

    store_reg_challenge(
        &state,
        key,
        RegChallenge {
            user_id,
            username: req.username,
            reg_state,
        },
    );

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
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<String>,
}

async fn register_finish(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<RegisterFinishReq>,
) -> Result<Json<RegisterFinishResp>, AppError> {
    // Extract user-agent for device metadata
    let user_agent = headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    tracing::info!(challenge_id = %req.challenge_id, "register_finish");

    let key = challenge_key(req.challenge_id);
    let challenge = take_reg_challenge(&state, &key)
        .ok_or_else(|| AppError::bad_request("invalid_or_expired_challenge"))?;

    let rpk: RegisterPublicKeyCredential = serde_json::from_value(req.attestation)
        .map_err(|_| AppError::bad_request("invalid_attestation"))?;

    let passkey = state
        .webauthn
        .finish_passkey_registration(&rpk, &challenge.reg_state)
        .map_err(|_| AppError::bad_request("webauthn_verification_failed"))?;

    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let passkey_json = serde_json::to_string(&passkey)
        .map_err(|e| AppError::internal(e.to_string()))?;
    let cred_id_bytes: Vec<u8> = passkey.cred_id().to_vec();

    // Upsert user
    sqlx::query(
        "INSERT INTO users (id, username) VALUES ($1, $2) ON CONFLICT (username) DO NOTHING",
    )
    .bind(challenge.user_id)
    .bind(&challenge.username)
    .execute(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    // Resolve canonical user_id
    let user_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE username = $1")
        .bind(&challenge.username)
        .fetch_one(db)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    // Persist credential
    sqlx::query(
        "INSERT INTO credentials (user_id, credential_id, sign_count, passkey, user_agent) \
         VALUES ($1, $2, 0, $3, $4)",
    )
    .bind(user_id)
    .bind(&cred_id_bytes)
    .bind(&passkey_json)
    .bind(&user_agent)
    .execute(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    // Fetch the new credential's DB UUID for JWT binding
    let cred_uuid: Uuid = sqlx::query_scalar(
        "SELECT id FROM credentials WHERE user_id = $1 AND credential_id = $2",
    )
    .bind(user_id)
    .bind(&cred_id_bytes)
    .fetch_one(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    // Issue JWT for auto-login
    let token = issue_jwt(user_id, &challenge.username, cred_uuid, &state.jwt_secret)?;

    // Audit
    audit::log_event(
        db,
        Some(user_id),
        &challenge.username,
        Some(cred_uuid),
        "register",
        serde_json::json!({}),
        "success",
    )
    .await;

    tracing::info!(user_id = %user_id, username = %challenge.username, "registration complete");
    Ok(Json(RegisterFinishResp {
        status: "ok",
        user_id,
        token: Some(token),
    }))
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
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

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

    store_auth_challenge(
        &state,
        key,
        AuthChallenge {
            user_id,
            username: req.username,
            auth_state,
        },
    );

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
    let challenge = take_auth_challenge(&state, &key)
        .ok_or_else(|| AppError::bad_request("invalid_or_expired_challenge"))?;

    let auth_cred: PublicKeyCredential = serde_json::from_value(req.assertion)
        .map_err(|_| AppError::bad_request("invalid_assertion"))?;

    let auth_result = state
        .webauthn
        .finish_passkey_authentication(&auth_cred, &challenge.auth_state)
        .map_err(|_| AppError::bad_request("webauthn_verification_failed"))?;

    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let cred_id_bytes: Vec<u8> = auth_result.cred_id().to_vec();
    let new_counter = auth_result.counter() as i64;

    let cred_row = sqlx::query(
        "SELECT id, status FROM credentials WHERE user_id = $1 AND credential_id = $2",
    )
    .bind(challenge.user_id)
    .bind(&cred_id_bytes)
    .fetch_optional(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    let (cred_uuid, status) = match cred_row {
        None => return Err(AppError::bad_request("device_revoked")),
        Some(r) => {
            let id: Uuid = r.try_get("id").map_err(|e| AppError::internal(e.to_string()))?;
            let s: String = r.try_get("status").map_err(|e| AppError::internal(e.to_string()))?;
            (id, s)
        }
    };

    if status != "active" {
        return Err(AppError::bad_request("device_revoked"));
    }

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

    let token = issue_jwt(challenge.user_id, &challenge.username, cred_uuid, &state.jwt_secret)?;

    // Audit
    audit::log_event(
        db,
        Some(challenge.user_id),
        &challenge.username,
        Some(cred_uuid),
        "login",
        serde_json::json!({}),
        "success",
    )
    .await;

    tracing::info!(
        user_id = %challenge.user_id,
        username = %challenge.username,
        cred_id = %cred_uuid,
        "login complete"
    );
    Ok(Json(LoginFinishResp { status: "ok", token }))
}
