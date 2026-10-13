//! Step-up (re-authentication) verification.
//!
//! This is a real, minimal primitive: prove the caller still holds one of
//! their registered passkeys *right now*, for some caller-supplied intent
//! label, with the result audit-logged. It intentionally does not carry
//! any business logic (it used to gate three hardcoded "demo" results —
//! `add_admin`, `rotate_api_key`, `delete_resource` — that were removed
//! rather than kept as filler; see git history if they're wanted back as
//! a reference).
//!
//! This shape — human present, fresh cryptographic proof, audit trail —
//! is the building block the upcoming agent-delegation work will need
//! ("a human approves this specific agent action") and is preserved for
//! that reason.

use axum::{Json, Router, extract::State, routing::post};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;
use webauthn_rs::prelude::*;

use crate::{
    audit,
    error::{ApiResponse, AppError, AppJson},
    jwt::AuthUser,
    redis::challenges::action_key,
    routes::auth_webauthn::{store_auth_challenge, take_auth_challenge},
    state::{AppState, AuthChallenge},
};

/// Caller-supplied intent labels are not business logic — just a short,
/// audit-log-friendly tag for what the step-up was for. Constrained so an
/// arbitrary string can't pollute audit_logs.detail.
fn is_valid_intent(intent: &str) -> bool {
    !intent.is_empty()
        && intent.len() <= 64
        && intent
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':' || c == '-')
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/actions/challenge", post(action_challenge))
        .route("/actions/execute", post(action_execute))
}

// ── POST /actions/challenge ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ActionChallengeReq {
    action: String,
}

#[derive(Debug, Serialize)]
struct ActionChallengeResp {
    challenge_id: Uuid,
    #[serde(flatten)]
    rcr: RequestChallengeResponse,
}

async fn action_challenge(
    auth: AuthUser,
    State(state): State<AppState>,
    AppJson(req): AppJson<ActionChallengeReq>,
) -> Result<Json<ApiResponse<ActionChallengeResp>>, AppError> {
    if !is_valid_intent(&req.action) {
        return Err(AppError::bad_request("invalid_action"));
    }

    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let rows =
        sqlx::query("SELECT passkey FROM credentials WHERE user_id = $1 AND status = 'active'")
            .bind(auth.user_id)
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
    let key = action_key(challenge_id);

    store_auth_challenge(
        &state,
        key,
        AuthChallenge {
            user_id: auth.user_id,
            username: auth.username,
            auth_state,
        },
    )
    .await?;

    Ok(Json(ApiResponse::success(ActionChallengeResp {
        challenge_id,
        rcr,
    })))
}

// ── POST /actions/execute ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ActionExecuteReq {
    challenge_id: Uuid,
    action: String,
    assertion: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct ActionExecuteResp {
    status: &'static str,
    verified: bool,
    intent: String,
}

async fn action_execute(
    auth: AuthUser,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    AppJson(req): AppJson<ActionExecuteReq>,
) -> Result<Json<ApiResponse<ActionExecuteResp>>, AppError> {
    if !is_valid_intent(&req.action) {
        return Err(AppError::bad_request("invalid_action"));
    }

    let key = action_key(req.challenge_id);
    let challenge = take_auth_challenge(&state, &key)
        .await?
        .ok_or_else(|| AppError::bad_request("invalid_or_expired_challenge"))?;

    // Ownership guard: challenge must belong to the authenticated user
    if challenge.user_id != auth.user_id {
        return Err(AppError::bad_request("challenge_user_mismatch"));
    }

    let auth_cred: PublicKeyCredential = serde_json::from_value(req.assertion)
        .map_err(|_| AppError::bad_request("invalid_assertion"))?;

    let auth_result = match state
        .webauthn
        .finish_passkey_authentication(&auth_cred, &challenge.auth_state)
    {
        Ok(r) => r,
        Err(_) => {
            crate::signals::record_failed_assertion(&state, auth.user_id).await;
            return Err(AppError::bad_request("webauthn_verification_failed"));
        }
    };

    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    // Resolve credential UUID for audit
    let cred_id_bytes: Vec<u8> = auth_result.cred_id().to_vec();
    let cred_uuid: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM credentials WHERE user_id = $1 AND credential_id = $2")
            .bind(auth.user_id)
            .bind(&cred_id_bytes)
            .fetch_optional(db)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;

    // WebAuthn verification above is real and complete at this point — the
    // caller has proven fresh possession of a registered passkey for the
    // given intent. Record it; callers decide what "verified" authorizes.
    audit::log_event(
        db,
        Some(auth.user_id),
        &auth.username,
        cred_uuid,
        "step_up_verified",
        serde_json::json!({ "intent": req.action }),
        "success",
    )
    .await;

    // Record completion SERVER-SIDE, bound to this session token (user + iat), only
    // now that the assertion has been cryptographically verified above. The
    // gateway reads this key; nothing a client sends can create it. If it cannot
    // be stored the caller is told so rather than being told "verified".
    let bearer = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| AppError::unauthorized("missing_token"))?;
    let claims = crate::jwt::verify_token(&state, bearer)?;
    crate::signals::record_step_up_completed(&state, auth.user_id, claims.iat).await?;

    tracing::info!(user_id = %auth.user_id, intent = %req.action, "step-up verified");

    Ok(Json(ApiResponse::success(ActionExecuteResp {
        status: "ok",
        verified: true,
        intent: req.action,
    })))
}
