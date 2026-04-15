use axum::{extract::State, routing::post, Json, Router};
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

const ALLOWED_ACTIONS: &[&str] = &["add_admin", "rotate_api_key", "export_data", "delete_resource"];

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
    if !ALLOWED_ACTIONS.contains(&req.action.as_str()) {
        return Err(AppError::bad_request("invalid_action"));
    }

    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let rows = sqlx::query(
        "SELECT passkey FROM credentials WHERE user_id = $1 AND status = 'active'",
    )
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
    ).await?;

    Ok(Json(ApiResponse::success(ActionChallengeResp { challenge_id, rcr })))
}

// ── POST /actions/execute ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ActionExecuteReq {
    challenge_id: Uuid,
    action: String,
    assertion: serde_json::Value,
    resource_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct ActionExecuteResp {
    status: &'static str,
    result: serde_json::Value,
}

async fn action_execute(
    auth: AuthUser,
    State(state): State<AppState>,
    AppJson(req): AppJson<ActionExecuteReq>,
) -> Result<Json<ApiResponse<ActionExecuteResp>>, AppError> {
    if !ALLOWED_ACTIONS.contains(&req.action.as_str()) {
        return Err(AppError::bad_request("invalid_action"));
    }

    // delete_resource requires resource_id before consuming the challenge
    if req.action == "delete_resource" && req.resource_id.as_deref().unwrap_or("").is_empty() {
        return Err(AppError::bad_request("resource_id_required"));
    }

    let key = action_key(req.challenge_id);
    let challenge = take_auth_challenge(&state, &key).await?
        .ok_or_else(|| AppError::bad_request("invalid_or_expired_challenge"))?;

    // Ownership guard: challenge must belong to the authenticated user
    if challenge.user_id != auth.user_id {
        return Err(AppError::bad_request("challenge_user_mismatch"));
    }

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

    // Resolve credential UUID for audit
    let cred_id_bytes: Vec<u8> = auth_result.cred_id().to_vec();
    let cred_uuid: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM credentials WHERE user_id = $1 AND credential_id = $2",
    )
    .bind(auth.user_id)
    .bind(&cred_id_bytes)
    .fetch_optional(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    // ── Action dispatch ──────────────────────────────────────────
    // WebAuthn verification above is REAL. Action results below are
    // demo stubs unless otherwise noted. Each stub is clearly labeled.

    let (result, is_demo) = match req.action.as_str() {
        "add_admin" => (serde_json::json!({
            "message": "[DEMO] Admin add simulated — not persisted",
            "user": "new-admin@crypton.io",
            "role": "Admin",
            "demo": true
        }), true),
        "rotate_api_key" => {
            let hex: String = (0..16)
                .map(|_| format!("{:02x}", rand_byte()))
                .collect();
            (serde_json::json!({
                "message": "[DEMO] API key generated — not persisted",
                "api_key": format!("ck_demo_{hex}"),
                "demo": true
            }), true)
        }
        "export_data" => (serde_json::json!({
            "message": "Export ready",
            "download_url": "/export/audit-logs",
            "demo": false
        }), false),
        "delete_resource" => {
            let rid = req.resource_id.as_deref().unwrap_or("");
            (serde_json::json!({
                "message": "[DEMO] Resource delete simulated — not persisted",
                "resource_id": rid,
                "demo": true
            }), true)
        }
        _ => (serde_json::json!({ "message": "unknown_action" }), true),
    };

    let outcome = if is_demo { "demo" } else { "success" };

    // Audit — action name carries demo: prefix when is_demo so log queries
    // can distinguish real operations from showcase flows without joining outcome.
    let prefix = if is_demo { "demo:" } else { "" };
    let (audit_action, audit_detail) = if req.action == "delete_resource" {
        (
            format!("{}action_executed", prefix),
            serde_json::json!({
                "action": "delete_resource",
                "resource_id": req.resource_id.as_deref().unwrap_or("")
            }),
        )
    } else {
        (format!("{}action:{}", prefix, req.action), result.clone())
    };

    audit::log_event(
        db,
        Some(auth.user_id),
        &auth.username,
        cred_uuid,
        &audit_action,
        audit_detail,
        outcome,
    )
    .await;

    tracing::info!(
        user_id = %auth.user_id,
        action = %req.action,
        demo = is_demo,
        "protected action executed"
    );

    Ok(Json(ApiResponse::success(ActionExecuteResp {
        status: "ok",
        result,
    })))
}

/// Simple deterministic-enough byte for demo hex keys.
fn rand_byte() -> u8 {
    use std::time::SystemTime;
    let t = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (t % 256) as u8
}
