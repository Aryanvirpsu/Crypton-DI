use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sqlx::{Row, postgres::PgRow};
use uuid::Uuid;

use crate::{
    audit,
    error::{ApiResponse, AppError, AppJson},
    jwt::AuthUser,
    redis::challenges::recovery_enrollment_key,
    routes::auth_webauthn::RecoveryEnrollmentGrant,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/recovery", get(get_recovery))
        .route("/recovery/start", post(start_recovery))
        .route("/recovery/approve", post(approve_recovery))
        .route("/recovery/reject", post(reject_recovery))
        .route("/recovery/complete", post(complete_recovery))
        .route("/recovery/public/start", post(public_start_recovery))
        .route("/recovery/public/status", get(public_get_recovery))
        .route("/recovery/public/complete", post(public_complete_recovery))
}

// ── shared response type ──────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct RecoveryRequest {
    id: Uuid,
    user_id: Uuid,
    status: String,
    method: String,
    approved_by_credential_id: Option<Uuid>,
    created_at: String,
    expires_at: String,
}

fn row_to_recovery(row: &PgRow) -> Result<RecoveryRequest, AppError> {
    Ok(RecoveryRequest {
        id: row
            .try_get("id")
            .map_err(|e| AppError::internal(e.to_string()))?,
        user_id: row
            .try_get("user_id")
            .map_err(|e| AppError::internal(e.to_string()))?,
        status: row
            .try_get("status")
            .map_err(|e| AppError::internal(e.to_string()))?,
        method: row
            .try_get("method")
            .map_err(|e| AppError::internal(e.to_string()))?,
        approved_by_credential_id: row
            .try_get("approved_by_credential_id")
            .map_err(|e| AppError::internal(e.to_string()))?,
        created_at: row
            .try_get("created_at")
            .map_err(|e| AppError::internal(e.to_string()))?,
        expires_at: row
            .try_get("expires_at")
            .map_err(|e| AppError::internal(e.to_string()))?,
    })
}

#[derive(Debug, Serialize)]
struct PublicRecoveryRequest {
    status: String,
    method: String,
    expires_at: String,
}

#[derive(Debug, Serialize)]
struct PublicStartResp {
    request: PublicRecoveryRequest,
    #[serde(skip_serializing_if = "Option::is_none")]
    recovery_token: Option<String>,
}

#[derive(Debug, Serialize)]
struct PublicCompleteResp {
    request: PublicRecoveryRequest,
    enrollment_token: String,
}

fn public_recovery(resp: &RecoveryRequest) -> PublicRecoveryRequest {
    PublicRecoveryRequest {
        status: resp.status.clone(),
        method: resp.method.clone(),
        expires_at: resp.expires_at.clone(),
    }
}

fn new_secret_token() -> String {
    format!("{}.{}", Uuid::new_v4(), Uuid::new_v4())
}

async fn store_recovery_enrollment_grant(
    state: &AppState,
    token: &str,
    grant: &RecoveryEnrollmentGrant,
) -> Result<(), AppError> {
    let mut conn = state
        .redis_conn()
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
    let json = serde_json::to_string(grant).map_err(|e| AppError::internal(e.to_string()))?;
    let _: () = redis::cmd("SETEX")
        .arg(recovery_enrollment_key(token))
        .arg(600)
        .arg(json)
        .query_async(&mut conn)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
    Ok(())
}

// ── POST /recovery/start ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct StartReq {
    #[serde(default = "default_method")]
    method: String,
}

fn default_method() -> String {
    "trusted_device".to_string()
}

async fn start_recovery(
    auth: AuthUser,
    State(state): State<AppState>,
    AppJson(req): AppJson<StartReq>,
) -> Result<Json<ApiResponse<RecoveryRequest>>, AppError> {
    if req.method != "trusted_device" && req.method != "admin" {
        return Err(AppError::bad_request("invalid_method"));
    }

    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    // Return the existing unexpired pending request if one exists.
    if let Some(row) = sqlx::query(
        "SELECT id, user_id, status, method, approved_by_credential_id, \
                created_at::text AS created_at, expires_at::text AS expires_at \
         FROM recovery_requests \
         WHERE user_id = $1 AND status = 'pending' AND expires_at > now() \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(auth.user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
    {
        let resp = row_to_recovery(&row)?;
        audit::log_event(
            db,
            Some(auth.user_id),
            &auth.username,
            Some(auth.cred_id),
            "recovery_start_deduplicated",
            serde_json::json!({ "request_id": resp.id, "method": resp.method }),
            "success",
        )
        .await;
        return Ok(Json(ApiResponse::success(resp)));
    }

    // Create a new pending request expiring in 24 h.
    let row = sqlx::query(
        "INSERT INTO recovery_requests (user_id, method, expires_at) \
         VALUES ($1, $2, now() + interval '24 hours') \
         RETURNING id, user_id, status, method, approved_by_credential_id, \
                   created_at::text AS created_at, expires_at::text AS expires_at",
    )
    .bind(auth.user_id)
    .bind(&req.method)
    .fetch_one(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    let resp = row_to_recovery(&row)?;

    audit::log_event(
        db,
        Some(auth.user_id),
        &auth.username,
        Some(auth.cred_id),
        "recovery_started",
        serde_json::json!({ "request_id": resp.id, "method": resp.method }),
        "success",
    )
    .await;
    crate::signals::record_recovery_started(&state, auth.user_id).await;

    tracing::info!(user_id = %auth.user_id, request_id = %resp.id, "recovery started");
    Ok(Json(ApiResponse::success(resp)))
}

// ── GET /recovery ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct GetRecoveryResp {
    request: Option<RecoveryRequest>,
}

#[derive(Debug, Serialize)]
struct PublicGetRecoveryResp {
    request: Option<PublicRecoveryRequest>,
}

async fn get_recovery(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<GetRecoveryResp>>, AppError> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let row = sqlx::query(
        "SELECT id, user_id, status, method, approved_by_credential_id, \
                created_at::text AS created_at, expires_at::text AS expires_at \
         FROM recovery_requests \
         WHERE user_id = $1 AND status IN ('pending', 'approved') AND expires_at > now() \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(auth.user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    let request = row.map(|r| row_to_recovery(&r)).transpose()?;
    Ok(Json(ApiResponse::success(GetRecoveryResp { request })))
}

// ── POST /recovery/approve ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ApproveReq {
    request_id: Uuid,
}

async fn approve_recovery(
    auth: AuthUser,
    State(state): State<AppState>,
    AppJson(req): AppJson<ApproveReq>,
) -> Result<Json<ApiResponse<RecoveryRequest>>, AppError> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let row = sqlx::query(
        "UPDATE recovery_requests \
         SET status = 'approved', approved_by_credential_id = $1 \
         WHERE id = $2 AND user_id = $3 AND status = 'pending' AND expires_at > now() \
         RETURNING id, user_id, status, method, approved_by_credential_id, \
                   created_at::text AS created_at, expires_at::text AS expires_at",
    )
    .bind(auth.cred_id)
    .bind(req.request_id)
    .bind(auth.user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
    .ok_or_else(|| AppError::not_found("request_not_found_or_not_pending"))?;

    let resp = row_to_recovery(&row)?;

    audit::log_event(
        db,
        Some(auth.user_id),
        &auth.username,
        Some(auth.cred_id),
        "recovery_approved",
        serde_json::json!({ "request_id": resp.id }),
        "success",
    )
    .await;

    tracing::info!(user_id = %auth.user_id, request_id = %resp.id, "recovery approved");
    Ok(Json(ApiResponse::success(resp)))
}

// ── POST /recovery/complete ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CompleteReq {
    request_id: Uuid,
}

async fn complete_recovery(
    auth: AuthUser,
    State(state): State<AppState>,
    AppJson(req): AppJson<CompleteReq>,
) -> Result<Json<ApiResponse<RecoveryRequest>>, AppError> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let row = sqlx::query(
        "UPDATE recovery_requests \
         SET status = 'completed' \
         WHERE id = $1 AND user_id = $2 AND status = 'approved' AND expires_at > now() \
         RETURNING id, user_id, status, method, approved_by_credential_id, \
                   created_at::text AS created_at, expires_at::text AS expires_at",
    )
    .bind(req.request_id)
    .bind(auth.user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
    .ok_or_else(|| AppError::not_found("request_not_found_or_not_approved"))?;

    let resp = row_to_recovery(&row)?;

    audit::log_event(
        db,
        Some(auth.user_id),
        &auth.username,
        Some(auth.cred_id),
        "recovery_completed",
        serde_json::json!({ "request_id": resp.id }),
        "success",
    )
    .await;
    crate::signals::record_recovery_completed(&state, auth.user_id).await;

    tracing::info!(user_id = %auth.user_id, request_id = %resp.id, "recovery completed");
    Ok(Json(ApiResponse::success(resp)))
}

// ── POST /recovery/reject ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RejectReq {
    request_id: Uuid,
}

async fn reject_recovery(
    auth: AuthUser,
    State(state): State<AppState>,
    AppJson(req): AppJson<RejectReq>,
) -> Result<Json<ApiResponse<RecoveryRequest>>, AppError> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let row = sqlx::query(
        "UPDATE recovery_requests \
         SET status = 'rejected', approved_by_credential_id = $1 \
         WHERE id = $2 AND user_id = $3 AND status = 'pending' \
         RETURNING id, user_id, status, method, approved_by_credential_id, \
                   created_at::text AS created_at, expires_at::text AS expires_at",
    )
    .bind(auth.cred_id)
    .bind(req.request_id)
    .bind(auth.user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
    .ok_or_else(|| AppError::not_found("request_not_found_or_not_pending"))?;

    let resp = row_to_recovery(&row)?;

    audit::log_event(
        db,
        Some(auth.user_id),
        &auth.username,
        Some(auth.cred_id),
        "recovery_rejected",
        serde_json::json!({ "request_id": resp.id }),
        "success",
    )
    .await;

    tracing::info!(user_id = %auth.user_id, request_id = %resp.id, "recovery rejected");
    Ok(Json(ApiResponse::success(resp)))
}

// ── PUBLIC RECOVERY FLOW ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct PublicStartReq {
    username: String,
}

async fn public_start_recovery(
    State(state): State<AppState>,
    AppJson(req): AppJson<PublicStartReq>,
) -> Result<Json<ApiResponse<PublicStartResp>>, AppError> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let user_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM users WHERE username = $1")
        .bind(&req.username)
        .fetch_optional(db)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    let user_id = user_id.ok_or_else(|| AppError::bad_request("user_not_found"))?;

    // Check existing
    if let Some(row) = sqlx::query(
        "SELECT id, user_id, status, method, approved_by_credential_id, created_at::text AS created_at, expires_at::text AS expires_at \
         FROM recovery_requests \
         WHERE user_id = $1 AND status IN ('pending', 'approved') AND expires_at > now() \
         ORDER BY created_at DESC LIMIT 1"
    )
    .bind(user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
    {
        let resp = row_to_recovery(&row)?;
        return Ok(Json(ApiResponse::success(PublicStartResp {
            request: public_recovery(&resp),
            recovery_token: None,
        })));
    }

    let recovery_token = new_secret_token();
    let row = sqlx::query(
        "INSERT INTO recovery_requests (user_id, method, expires_at, public_token_hash) \
         VALUES ($1, 'trusted_device', now() + interval '24 hours', encode(digest($2, 'sha256'), 'hex')) \
         RETURNING id, user_id, status, method, approved_by_credential_id, created_at::text AS created_at, expires_at::text AS expires_at"
    )
    .bind(user_id)
    .bind(&recovery_token)
    .fetch_one(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    let resp = row_to_recovery(&row)?;

    audit::log_event(
        db,
        Some(user_id),
        &req.username,
        None,
        "recovery_started",
        serde_json::json!({ "request_id": resp.id }),
        "success",
    )
    .await;
    crate::signals::record_recovery_started(&state, user_id).await;

    Ok(Json(ApiResponse::success(PublicStartResp {
        request: public_recovery(&resp),
        recovery_token: Some(recovery_token),
    })))
}

#[derive(Debug, Deserialize)]
struct PublicStatusQuery {
    username: String,
}

async fn public_get_recovery(
    axum::extract::Query(query): axum::extract::Query<PublicStatusQuery>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<PublicGetRecoveryResp>>, AppError> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let row = sqlx::query(
        "SELECT id, user_id, status, method, approved_by_credential_id, created_at::text AS created_at, expires_at::text AS expires_at \
         FROM recovery_requests \
         WHERE user_id = (SELECT id FROM users WHERE username = $1) AND status IN ('pending', 'approved') AND expires_at > now() \
         ORDER BY created_at DESC LIMIT 1"
    )
    .bind(&query.username)
    .fetch_optional(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    let request = row
        .map(|r| row_to_recovery(&r).map(|req| public_recovery(&req)))
        .transpose()?;
    Ok(Json(ApiResponse::success(PublicGetRecoveryResp {
        request,
    })))
}

#[derive(Debug, Deserialize)]
struct PublicCompleteReq {
    username: String,
    recovery_token: String,
}

async fn public_complete_recovery(
    State(state): State<AppState>,
    AppJson(req): AppJson<PublicCompleteReq>,
) -> Result<Json<ApiResponse<PublicCompleteResp>>, AppError> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let user_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM users WHERE username = $1")
        .bind(&req.username)
        .fetch_optional(db)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
    let user_id = user_id.ok_or_else(|| AppError::bad_request("user_not_found"))?;

    let row = sqlx::query(
        "UPDATE recovery_requests \
         SET status = 'completed' \
         WHERE user_id = $1 \
           AND status = 'approved' \
           AND expires_at > now() \
           AND public_token_hash = encode(digest($2, 'sha256'), 'hex') \
         RETURNING id, user_id, status, method, approved_by_credential_id, created_at::text AS created_at, expires_at::text AS expires_at"
    )
    .bind(user_id)
    .bind(&req.recovery_token)
    .fetch_optional(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?
    .ok_or_else(|| AppError::not_found("request_not_found_or_not_approved"))?;

    let resp = row_to_recovery(&row)?;
    let enrollment_token = new_secret_token();
    store_recovery_enrollment_grant(
        &state,
        &enrollment_token,
        &RecoveryEnrollmentGrant {
            user_id,
            username: req.username.clone(),
            request_id: resp.id,
        },
    )
    .await?;

    audit::log_event(
        db,
        Some(user_id),
        &req.username,
        None,
        "recovery_completed",
        serde_json::json!({ "request_id": resp.id }),
        "success",
    )
    .await;
    crate::signals::record_recovery_completed(&state, user_id).await;

    Ok(Json(ApiResponse::success(PublicCompleteResp {
        request: public_recovery(&resp),
        enrollment_token,
    })))
}

// ── Integration tests (real Postgres via #[sqlx::test]) ─────────────────────
// See devices.rs for the pattern this follows. Coverage note: this covers
// every part of the state machine that only touches Postgres — the
// authenticated start/approve/reject/complete flow, and the public
// start/status endpoints. public_complete_recovery is NOT covered here
// because it also needs a live Redis (to consume the enrollment grant),
// which #[sqlx::test] doesn't provide; that one function is the one part
// of this file's logic without automated test coverage.

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;
    use webauthn_rs::prelude::WebauthnBuilder;

    fn test_state(pool: PgPool) -> AppState {
        let origin = url::Url::parse("http://localhost:8080").unwrap();
        let webauthn = WebauthnBuilder::new("localhost", &origin)
            .unwrap()
            .rp_name("Crypton Test")
            .build()
            .unwrap();
        AppState {
            db: Some(pool),
            redis: None,
            webauthn,
            jwt_encoding_key: jsonwebtoken::EncodingKey::from_ed_pem(
                crate::jwt::test_keys::SIGNING_KEY_PEM.as_bytes(),
            )
            .unwrap(),
            jwt_decoding_key: jsonwebtoken::DecodingKey::from_ed_pem(
                crate::jwt::test_keys::PUBLIC_KEY_PEM.as_bytes(),
            )
            .unwrap(),
            cors_origin: "http://localhost:3001".into(),
        }
    }

    async fn seed_user(pool: &PgPool, username: &str) -> Uuid {
        sqlx::query_scalar("INSERT INTO users (username) VALUES ($1) RETURNING id")
            .bind(username)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[sqlx::test]
    async fn full_authenticated_recovery_state_machine(pool: PgPool) {
        let user_id = seed_user(&pool, "alice").await;
        let trusted_cred = Uuid::new_v4();
        let state = test_state(pool.clone());

        let auth = AuthUser {
            user_id,
            username: "alice".into(),
            cred_id: trusted_cred,
        };
        let Json(resp) = start_recovery(
            auth,
            State(state.clone()),
            AppJson(StartReq {
                method: "trusted_device".into(),
            }),
        )
        .await
        .unwrap();
        let req = resp.data.unwrap();
        assert_eq!(req.status, "pending");

        // Starting again while one is already pending must return the same
        // request, not create a second one.
        let auth2 = AuthUser {
            user_id,
            username: "alice".into(),
            cred_id: trusted_cred,
        };
        let Json(resp2) = start_recovery(
            auth2,
            State(state.clone()),
            AppJson(StartReq {
                method: "trusted_device".into(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(
            resp2.data.unwrap().id,
            req.id,
            "must deduplicate to the existing pending request"
        );

        // Approve from a (simulated) trusted device.
        let approver = AuthUser {
            user_id,
            username: "alice".into(),
            cred_id: trusted_cred,
        };
        let Json(approved) = approve_recovery(
            approver,
            State(state.clone()),
            AppJson(ApproveReq { request_id: req.id }),
        )
        .await
        .unwrap();
        assert_eq!(approved.data.unwrap().status, "approved");

        // Complete.
        let completer = AuthUser {
            user_id,
            username: "alice".into(),
            cred_id: trusted_cred,
        };
        let Json(completed) = complete_recovery(
            completer,
            State(state),
            AppJson(CompleteReq { request_id: req.id }),
        )
        .await
        .unwrap();
        assert_eq!(completed.data.unwrap().status, "completed");
    }

    #[sqlx::test]
    async fn reject_recovery_cancels_a_pending_request(pool: PgPool) {
        let user_id = seed_user(&pool, "bob").await;
        let cred = Uuid::new_v4();
        let state = test_state(pool.clone());

        let auth = AuthUser {
            user_id,
            username: "bob".into(),
            cred_id: cred,
        };
        let Json(resp) = start_recovery(
            auth,
            State(state.clone()),
            AppJson(StartReq {
                method: "trusted_device".into(),
            }),
        )
        .await
        .unwrap();
        let req = resp.data.unwrap();

        let rejecter = AuthUser {
            user_id,
            username: "bob".into(),
            cred_id: cred,
        };
        let Json(rejected) = reject_recovery(
            rejecter,
            State(state),
            AppJson(RejectReq { request_id: req.id }),
        )
        .await
        .unwrap();
        assert_eq!(rejected.data.unwrap().status, "rejected");
    }

    #[sqlx::test]
    async fn complete_recovery_refuses_when_not_approved(pool: PgPool) {
        let user_id = seed_user(&pool, "carol").await;
        let cred = Uuid::new_v4();
        let state = test_state(pool.clone());

        let auth = AuthUser {
            user_id,
            username: "carol".into(),
            cred_id: cred,
        };
        let Json(resp) = start_recovery(
            auth,
            State(state.clone()),
            AppJson(StartReq {
                method: "trusted_device".into(),
            }),
        )
        .await
        .unwrap();
        let req = resp.data.unwrap();

        // Still pending, never approved.
        let completer = AuthUser {
            user_id,
            username: "carol".into(),
            cred_id: cred,
        };
        let result = complete_recovery(
            completer,
            State(state),
            AppJson(CompleteReq { request_id: req.id }),
        )
        .await;
        assert!(
            result.is_err(),
            "completing an unapproved request must be refused"
        );
    }

    #[sqlx::test]
    async fn public_start_recovery_issues_a_token_only_on_the_first_call(pool: PgPool) {
        seed_user(&pool, "dave").await;
        let state = test_state(pool.clone());

        let Json(first) = public_start_recovery(
            State(state.clone()),
            AppJson(PublicStartReq {
                username: "dave".into(),
            }),
        )
        .await
        .unwrap();
        let first_data = first.data.unwrap();
        assert!(
            first_data.recovery_token.is_some(),
            "the creating call must return the secret"
        );

        let Json(second) = public_start_recovery(
            State(state),
            AppJson(PublicStartReq {
                username: "dave".into(),
            }),
        )
        .await
        .unwrap();
        assert!(
            second.data.unwrap().recovery_token.is_none(),
            "a repeat call against an existing pending request must NOT reissue the secret"
        );
    }

    #[sqlx::test]
    async fn public_start_recovery_rejects_unknown_username(pool: PgPool) {
        let state = test_state(pool);
        let result = public_start_recovery(
            State(state),
            AppJson(PublicStartReq {
                username: "nobody".into(),
            }),
        )
        .await;
        assert!(
            result.is_err(),
            "must not reveal whether an unknown username 'succeeded' differently than a known one at the type level, but must still fail closed"
        );
    }

    #[sqlx::test]
    async fn public_get_recovery_reflects_status_without_leaking_the_id(pool: PgPool) {
        seed_user(&pool, "erin").await;
        let state = test_state(pool.clone());

        let _ = public_start_recovery(
            State(state.clone()),
            AppJson(PublicStartReq {
                username: "erin".into(),
            }),
        )
        .await
        .unwrap();

        let Json(resp) = public_get_recovery(
            axum::extract::Query(PublicStatusQuery {
                username: "erin".into(),
            }),
            State(state),
        )
        .await
        .unwrap();
        let status = resp.data.unwrap().request.unwrap();
        assert_eq!(status.status, "pending");
    }
}
