use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    audit,
    error::{ApiResponse, AppError},
    jwt::AuthUser,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/devices", get(list_devices))
        .route("/devices/:id/revoke", post(revoke_device))
        .route("/devices/:id/mark-lost", post(mark_lost))
}

// ── GET /devices ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct DeviceInfo {
    id: Uuid,
    nickname: Option<String>,
    status: String,
    user_agent: Option<String>,
    created_at: Option<String>,
    last_used_at: Option<String>,
}

async fn list_devices(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<DeviceInfo>>>, AppError> {
    let db = match state.db.as_ref() {
        Some(db) => db,
        None => return Ok(Json(ApiResponse::success(vec![]))),
    };

    let rows = sqlx::query(
        "SELECT id, nickname, status, \
                user_agent, \
                created_at::text AS created_at, \
                last_used_at::text AS last_used_at \
         FROM credentials \
         WHERE user_id = $1 AND status IN ('active', 'lost') \
         ORDER BY created_at",
    )
    .bind(auth.user_id)
    .fetch_all(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    let devices = rows
        .iter()
        .filter_map(|r| {
            Some(DeviceInfo {
                id: r.try_get("id").ok()?,
                nickname: r.try_get("nickname").ok().flatten(),
                status: r.try_get("status").ok()?,
                user_agent: r.try_get("user_agent").ok().flatten(),
                created_at: r.try_get("created_at").ok().flatten(),
                last_used_at: r.try_get("last_used_at").ok().flatten(),
            })
        })
        .collect();

    Ok(Json(ApiResponse::success(devices)))
}

// ── POST /devices/:id/revoke ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct RevokeResp {
    status: &'static str,
}

async fn revoke_device(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<RevokeResp>>, AppError> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let result = sqlx::query(
        "UPDATE credentials SET status = 'revoked' \
         WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(auth.user_id)
    .execute(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("credential_not_found"));
    }

    audit::log_event(
        db,
        Some(auth.user_id),
        &auth.username,
        Some(id),
        "device_revoke",
        serde_json::json!({}),
        "success",
    )
    .await;
    crate::signals::record_revocation(&state, auth.user_id).await;

    tracing::info!(user_id = %auth.user_id, device_id = %id, "device revoked");
    Ok(Json(ApiResponse::success(RevokeResp { status: "revoked" })))
}

// ── POST /devices/:id/mark-lost ───────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct MarkLostResp {
    status: &'static str,
}

async fn mark_lost(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<MarkLostResp>>, AppError> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let result = sqlx::query(
        "UPDATE credentials SET status = 'lost' \
         WHERE id = $1 AND user_id = $2 AND status = 'active'",
    )
    .bind(id)
    .bind(auth.user_id)
    .execute(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("credential_not_found_or_not_active"));
    }

    audit::log_event(
        db,
        Some(auth.user_id),
        &auth.username,
        Some(id),
        "device_mark_lost",
        serde_json::json!({}),
        "success",
    )
    .await;
    crate::signals::record_revocation(&state, auth.user_id).await;

    tracing::info!(user_id = %auth.user_id, device_id = %id, "device marked lost");
    Ok(Json(ApiResponse::success(MarkLostResp { status: "lost" })))
}

// ── Integration tests ────────────────────────────────────────────────────────
// Real Postgres via `#[sqlx::test]` (spins up a throwaway DB per test and
// runs migrations/ automatically) — not mocked. Requires DATABASE_URL to
// point at a Postgres server with CREATEDB privilege; CI provides one (see
// .github/workflows/ci.yml). Handlers are called directly (bypassing HTTP)
// since AuthUser/Path/State are plain structs with public fields — this
// exercises the real SQL and real authorization checks without needing a
// running server or a simulated WebAuthn ceremony.

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

    async fn seed_user_with_device(pool: &PgPool, status: &str) -> (Uuid, Uuid) {
        let user_id: Uuid =
            sqlx::query_scalar("INSERT INTO users (username) VALUES ($1) RETURNING id")
                .bind(format!("user-{}", Uuid::new_v4()))
                .fetch_one(pool)
                .await
                .unwrap();
        let cred_id: Uuid = sqlx::query_scalar(
            "INSERT INTO credentials (user_id, credential_id, status, nickname) \
             VALUES ($1, $2, $3, 'Test Device') RETURNING id",
        )
        .bind(user_id)
        .bind(Uuid::new_v4().as_bytes().to_vec())
        .bind(status)
        .fetch_one(pool)
        .await
        .unwrap();
        (user_id, cred_id)
    }

    #[sqlx::test]
    async fn list_devices_returns_only_the_caller_own_devices(pool: PgPool) {
        let (user_a, _) = seed_user_with_device(&pool, "active").await;
        let (_user_b, _) = seed_user_with_device(&pool, "active").await;
        let state = test_state(pool);

        let auth = AuthUser {
            user_id: user_a,
            username: "a".into(),
            cred_id: Uuid::new_v4(),
        };
        let Json(resp) = list_devices(auth, State(state)).await.unwrap();
        let devices = resp.data.unwrap();

        assert_eq!(devices.len(), 1, "must not see another user's device");
    }

    #[sqlx::test]
    async fn revoke_device_sets_status_and_is_idempotent_safe(pool: PgPool) {
        let (user_id, cred_id) = seed_user_with_device(&pool, "active").await;
        let state = test_state(pool.clone());
        let auth = AuthUser {
            user_id,
            username: "a".into(),
            cred_id: Uuid::new_v4(),
        };

        let Json(resp) = revoke_device(auth, State(state.clone()), Path(cred_id))
            .await
            .unwrap();
        assert_eq!(resp.data.unwrap().status, "revoked");

        let status: String = sqlx::query_scalar("SELECT status FROM credentials WHERE id = $1")
            .bind(cred_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status, "revoked");

        // A different user attempting to revoke this same credential ID
        // must be rejected (the UPDATE's WHERE clause requires user_id
        // to match, so a mismatched caller affects zero rows -> 404).
        let auth_wrong_user = AuthUser {
            user_id: Uuid::new_v4(),
            username: "eve".into(),
            cred_id: Uuid::new_v4(),
        };
        let result = revoke_device(auth_wrong_user, State(state), Path(cred_id)).await;
        assert!(
            result.is_err(),
            "revoking someone else's device must fail, not silently succeed"
        );
    }

    #[sqlx::test]
    async fn mark_lost_refuses_when_not_currently_active(pool: PgPool) {
        let (user_id, cred_id) = seed_user_with_device(&pool, "revoked").await;
        let state = test_state(pool.clone());
        let auth = AuthUser {
            user_id,
            username: "a".into(),
            cred_id: Uuid::new_v4(),
        };

        let result = mark_lost(auth, State(state), Path(cred_id)).await;
        assert!(
            result.is_err(),
            "marking a revoked device as lost must be rejected"
        );

        let status: String = sqlx::query_scalar("SELECT status FROM credentials WHERE id = $1")
            .bind(cred_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            status, "revoked",
            "status must be unchanged after the rejected transition"
        );
    }

    #[sqlx::test]
    async fn mark_lost_succeeds_from_active(pool: PgPool) {
        let (user_id, cred_id) = seed_user_with_device(&pool, "active").await;
        let state = test_state(pool.clone());
        let auth = AuthUser {
            user_id,
            username: "a".into(),
            cred_id: Uuid::new_v4(),
        };

        let Json(resp) = mark_lost(auth, State(state), Path(cred_id)).await.unwrap();
        assert_eq!(resp.data.unwrap().status, "lost");
    }
}
