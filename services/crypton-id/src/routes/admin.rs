//! Real admin/dashboard route handlers, backed entirely by Postgres.
//!
//! This file used to be `stubs.rs` and shipped several hardcoded/fake
//! endpoints (sessions, risk scoring, org settings, policies, passkey
//! listing duplicate of /devices). Those were removed rather than kept
//! as demo filler — see git history for `stubs.rs` if they're needed as
//! a reference. Everything remaining here executes a real query.
//!
//! Handlers:
//!   GET   /dashboard/stats          — AuthUser, user-scoped
//!   GET   /dashboard/activity       — AuthUser, user-scoped
//!   GET   /admin/dashboard/stats    — AdminUser, global
//!   GET   /admin/dashboard/activity — AdminUser, global
//!   GET   /admin/users              — AdminUser, real user+role listing
//!   PATCH /admin/users/:id/role     — AdminUser, real role update (with
//!                                     a safeguard against demoting the
//!                                     last remaining admin)
//!   POST  /admin/bootstrap          — AuthUser, one-time-only: promotes
//!                                     the caller to super_admin iff no
//!                                     admin exists yet. This is how a
//!                                     fresh deployment gets its first
//!                                     admin without manual SQL.
//!   GET   /export/audit-logs        — AdminUser, real CSV export

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch, post},
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    audit,
    error::{ApiResponse, AppError, AppJson},
    jwt::{AdminUser, AuthUser},
    state::AppState,
};

const VALID_ROLES: &[&str] = &["user", "admin", "super_admin"];
const ADMIN_ROLES: &[&str] = &["admin", "super_admin"];

pub fn router() -> Router<AppState> {
    Router::new()
        // ── Real dashboard (user-scoped) ──────────────────────────────────
        .route("/dashboard/stats", get(dashboard_stats))
        .route("/dashboard/activity", get(dashboard_activity))
        // ── Real dashboard (admin, global) ────────────────────────────────
        .route("/admin/dashboard/stats", get(admin_dashboard_stats))
        .route("/admin/dashboard/activity", get(admin_dashboard_activity))
        // ── Users & Roles ─────────────────────────────────────────────────
        .route("/admin/users", get(list_users))
        .route("/admin/users/:id/role", patch(patch_user_role))
        .route("/admin/bootstrap", post(admin_bootstrap))
        // ── Audit ─────────────────────────────────────────────────────────
        .route("/export/audit-logs", get(export_audit_logs))
}

// ── Shared response types ─────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct StatusResp {
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct ActivityItem {
    id: Uuid,
    ico: String,
    #[serde(rename = "type")]
    activity_type: String,
    title: String,
    meta: String,
    time: String,
    link: String,
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Derive a 0-100 security score from real device and failure counts.
/// Penalises 20 points per lost device and 10 points per failed auth in 24h.
fn security_score(lost_devices: i64, failures_24h: i64) -> i64 {
    (100 - lost_devices * 20 - failures_24h * 10).max(0)
}

/// Convert an age in seconds to a human-readable relative string.
fn relative_time_secs(secs: i64) -> String {
    if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
}

/// Map a raw audit_log row into a frontend-shaped ActivityItem.
fn map_audit_to_activity(
    log_id: Uuid,
    action: &str,
    outcome: &str,
    actor: &str,
    age_secs: i64,
) -> ActivityItem {
    let (ico, act_type, title, link): (&str, &str, &str, &str) = match (action, outcome) {
        ("login", "success") => ("✓", "s", "Authentication successful", "auditlogs"),
        ("login", _) => ("⚠", "w", "Authentication failed", "auditlogs"),
        ("register", _) => ("📱", "i", "New device enrolled", "devices"),
        ("logout", _) => ("🔒", "i", "Session ended", "auditlogs"),
        ("recovery_started", _) => ("🔑", "w", "Recovery started", "recovery"),
        ("recovery_completed", _) => ("✓", "i", "Recovery completed", "recovery"),
        _ => ("·", "i", action, "auditlogs"),
    };

    ActivityItem {
        id: log_id,
        ico: ico.to_string(),
        activity_type: act_type.to_string(),
        title: title.to_string(),
        meta: actor.to_string(),
        time: relative_time_secs(age_secs),
        link: link.to_string(),
    }
}

// ── Real dashboard — user-scoped ──────────────────────────────────────────────

/// GET /dashboard/stats
/// Returns per-user device counts, 24h login events, and a computed security score.
async fn dashboard_stats(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let row = sqlx::query(
        "WITH
           devices AS (
             SELECT
               COUNT(*) FILTER (WHERE status = 'active') AS active,
               COUNT(*) FILTER (WHERE status = 'lost')   AS lost
             FROM credentials WHERE user_id = $1
           ),
           events AS (
             SELECT
               COUNT(*) FILTER (WHERE action = 'login')                           AS auth_24h,
               COUNT(*) FILTER (WHERE action = 'login' AND outcome != 'success')  AS fail_24h
             FROM audit_logs
             WHERE user_id = $1 AND created_at > now() - interval '24 hours'
           )
         SELECT d.active, d.lost, e.auth_24h, e.fail_24h FROM devices d, events e",
    )
    .bind(auth.user_id)
    .fetch_one(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    let active_devices: i64 = row.try_get("active").unwrap_or(0);
    let lost_devices: i64 = row.try_get("lost").unwrap_or(0);
    let auth_24h: i64 = row.try_get("auth_24h").unwrap_or(0);
    let fail_24h: i64 = row.try_get("fail_24h").unwrap_or(0);

    Ok(Json(ApiResponse::success(serde_json::json!({
        "activeDevices":  active_devices,
        "authEvents24h":  auth_24h,
        "securityScore":  security_score(lost_devices, fail_24h),
    }))))
}

/// GET /dashboard/activity
/// Returns the 10 most recent audit events for the authenticated user,
/// shaped as ActivityItems for the frontend feed.
async fn dashboard_activity(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<ActivityItem>>>, AppError> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let rows = sqlx::query(
        "SELECT id, actor, action, outcome,
                EXTRACT(EPOCH FROM now() - created_at)::bigint AS age_seconds
         FROM audit_logs
         WHERE user_id = $1
         ORDER BY created_at DESC
         LIMIT 10",
    )
    .bind(auth.user_id)
    .fetch_all(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    let items: Vec<ActivityItem> = rows
        .iter()
        .filter_map(|r| {
            let id: Uuid = r.try_get("id").ok()?;
            let actor: String = r.try_get("actor").unwrap_or_default();
            let action: String = r.try_get("action").ok()?;
            let outcome: String = r.try_get("outcome").ok()?;
            let age_secs: i64 = r.try_get("age_seconds").unwrap_or(0);
            Some(map_audit_to_activity(
                id, &action, &outcome, &actor, age_secs,
            ))
        })
        .collect();

    Ok(Json(ApiResponse::success(items)))
}

// ── Real dashboard — admin / global ──────────────────────────────────────────

/// GET /admin/dashboard/stats
/// Returns organisation-wide counters: total users, credential breakdown,
/// and 24h auth event summary across all users.
async fn admin_dashboard_stats(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let row = sqlx::query(
        "WITH
           users_count AS (
             SELECT COUNT(*) AS total FROM users
           ),
           creds AS (
             SELECT
               COUNT(*) FILTER (WHERE status = 'active')  AS active,
               COUNT(*) FILTER (WHERE status = 'lost')    AS lost,
               COUNT(*) FILTER (WHERE status = 'revoked') AS revoked
             FROM credentials
           ),
           events AS (
             SELECT
               COUNT(*) FILTER (WHERE action = 'login')                           AS auth_24h,
               COUNT(*) FILTER (WHERE action = 'login' AND outcome != 'success')  AS fail_24h
             FROM audit_logs
             WHERE created_at > now() - interval '24 hours'
           )
         SELECT u.total, c.active, c.lost, c.revoked, e.auth_24h, e.fail_24h
         FROM users_count u, creds c, events e",
    )
    .fetch_one(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    let total_users: i64 = row.try_get("total").unwrap_or(0);
    let active_credentials: i64 = row.try_get("active").unwrap_or(0);
    let lost_credentials: i64 = row.try_get("lost").unwrap_or(0);
    let revoked_credentials: i64 = row.try_get("revoked").unwrap_or(0);
    let auth_24h: i64 = row.try_get("auth_24h").unwrap_or(0);
    let failed_auths_24h: i64 = row.try_get("fail_24h").unwrap_or(0);

    Ok(Json(ApiResponse::success(serde_json::json!({
        "totalUsers":          total_users,
        "activeCredentials":   active_credentials,
        "lostCredentials":     lost_credentials,
        "revokedCredentials":  revoked_credentials,
        "authEvents24h":       auth_24h,
        "failedAuths24h":      failed_auths_24h,
    }))))
}

/// GET /admin/dashboard/activity
/// Returns the 20 most recent audit events across all users.
async fn admin_dashboard_activity(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<ActivityItem>>>, AppError> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let rows = sqlx::query(
        "SELECT id, actor, action, outcome,
                EXTRACT(EPOCH FROM now() - created_at)::bigint AS age_seconds
         FROM audit_logs
         ORDER BY created_at DESC
         LIMIT 20",
    )
    .fetch_all(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    let items: Vec<ActivityItem> = rows
        .iter()
        .filter_map(|r| {
            let id: Uuid = r.try_get("id").ok()?;
            let actor: String = r.try_get("actor").unwrap_or_default();
            let action: String = r.try_get("action").ok()?;
            let outcome: String = r.try_get("outcome").ok()?;
            let age_secs: i64 = r.try_get("age_seconds").unwrap_or(0);
            Some(map_audit_to_activity(
                id, &action, &outcome, &actor, age_secs,
            ))
        })
        .collect();

    Ok(Json(ApiResponse::success(items)))
}

// ── Users & Roles ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct UserSummary {
    id: Uuid,
    username: String,
    role: String,
    created_at: Option<String>,
    device_count: i64,
}

/// GET /admin/users — real user + role listing with a live device count per user.
async fn list_users(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<UserSummary>>>, AppError> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let rows = sqlx::query(
        "SELECT u.id, u.username, u.role, u.created_at::text AS created_at,
                COUNT(c.id) FILTER (WHERE c.status = 'active') AS device_count
         FROM users u
         LEFT JOIN credentials c ON c.user_id = u.id
         GROUP BY u.id, u.username, u.role, u.created_at
         ORDER BY u.created_at",
    )
    .fetch_all(db)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;

    let users = rows
        .iter()
        .filter_map(|r| {
            Some(UserSummary {
                id: r.try_get("id").ok()?,
                username: r.try_get("username").ok()?,
                role: r.try_get("role").ok()?,
                created_at: r.try_get("created_at").ok().flatten(),
                device_count: r.try_get("device_count").unwrap_or(0),
            })
        })
        .collect();

    Ok(Json(ApiResponse::success(users)))
}

#[derive(Deserialize)]
struct RolePatch {
    role: String,
}

/// PATCH /admin/users/:id/role — real role update.
/// Refuses to demote the last remaining admin/super_admin so a deployment
/// can never accidentally lock itself out of the admin surface.
async fn patch_user_role(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    AppJson(body): AppJson<RolePatch>,
) -> Result<Json<ApiResponse<StatusResp>>, AppError> {
    if !VALID_ROLES.contains(&body.role.as_str()) {
        return Err(AppError::bad_request("invalid_role"));
    }

    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    if !ADMIN_ROLES.contains(&body.role.as_str()) {
        let remaining_admins: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM users WHERE role IN ('admin','super_admin') AND id != $1",
        )
        .bind(id)
        .fetch_one(db)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

        if remaining_admins == 0 {
            return Err(AppError::bad_request("cannot_demote_last_admin"));
        }
    }

    let result = sqlx::query("UPDATE users SET role = $1 WHERE id = $2")
        .bind(&body.role)
        .bind(id)
        .execute(db)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("user_not_found"));
    }

    audit::log_event(
        db,
        Some(admin.user_id),
        &admin.username,
        None,
        "admin_role_updated",
        serde_json::json!({ "target_user_id": id, "role": body.role }),
        "success",
    )
    .await;

    tracing::info!(admin_id = %admin.user_id, target_user_id = %id, role = %body.role, "user role updated");
    Ok(Json(ApiResponse::success(StatusResp { status: "updated" })))
}

/// POST /admin/bootstrap — the only way to create the first admin.
/// Any authenticated user may call this; it only succeeds while the
/// `users` table has zero admin/super_admin rows, in which case the
/// caller becomes `super_admin`. Once any admin exists, this always
/// returns 403 — it is not a standing escalation path.
async fn admin_bootstrap(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<StatusResp>>, AppError> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AppError::internal("database_not_configured"))?;

    let existing_admins: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role IN ('admin','super_admin')")
            .fetch_one(db)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;

    if existing_admins > 0 {
        return Err(AppError::forbidden("admin_already_bootstrapped"));
    }

    sqlx::query("UPDATE users SET role = 'super_admin' WHERE id = $1")
        .bind(auth.user_id)
        .execute(db)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    audit::log_event(
        db,
        Some(auth.user_id),
        &auth.username,
        Some(auth.cred_id),
        "admin_bootstrap",
        serde_json::json!({}),
        "success",
    )
    .await;

    tracing::warn!(user_id = %auth.user_id, "admin bootstrap: promoted to super_admin");
    Ok(Json(ApiResponse::success(StatusResp {
        status: "promoted",
    })))
}

// ── Audit export ──────────────────────────────────────────────────────────────

async fn export_audit_logs(_admin: AdminUser, State(state): State<AppState>) -> impl IntoResponse {
    let mut csv = String::from("id,actor,credential_id,action,outcome,time\n");

    if let Some(db) = state.db.as_ref() {
        let rows = sqlx::query(
            "SELECT id, actor, credential_id, action, outcome, \
                    created_at::text AS created_at \
             FROM audit_logs ORDER BY created_at DESC LIMIT 200",
        )
        .fetch_all(db)
        .await;

        if let Ok(rows) = rows {
            for r in &rows {
                let id: uuid::Uuid = r.try_get("id").unwrap_or_default();
                let actor: String = r.try_get("actor").unwrap_or_default();
                let cred: Option<uuid::Uuid> = r.try_get("credential_id").ok().flatten();
                let action: String = r.try_get("action").unwrap_or_default();
                let outcome: String = r.try_get("outcome").unwrap_or_default();
                let time: String = r
                    .try_get::<Option<String>, _>("created_at")
                    .ok()
                    .flatten()
                    .unwrap_or_default();
                csv.push_str(&format!(
                    "{},{},{},{},{},{}\n",
                    id,
                    actor,
                    cred.map(|u| u.to_string()).unwrap_or_default(),
                    action,
                    outcome,
                    time
                ));
            }
        }
    }

    (
        StatusCode::OK,
        [
            ("Content-Type", "text/csv"),
            (
                "Content-Disposition",
                "attachment; filename=\"audit-logs.csv\"",
            ),
        ],
        csv,
    )
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn security_score_perfect() {
        assert_eq!(security_score(0, 0), 100);
    }

    #[test]
    fn security_score_one_lost_device() {
        assert_eq!(security_score(1, 0), 80);
    }

    #[test]
    fn security_score_failures_only() {
        assert_eq!(security_score(0, 5), 50);
    }

    #[test]
    fn security_score_combined() {
        assert_eq!(security_score(2, 3), 30); // 100 - 40 - 30
    }

    #[test]
    fn security_score_clamps_at_zero() {
        assert_eq!(security_score(10, 10), 0);
    }

    #[test]
    fn relative_time_seconds() {
        assert_eq!(relative_time_secs(0), "0s ago");
        assert_eq!(relative_time_secs(45), "45s ago");
        assert_eq!(relative_time_secs(59), "59s ago");
    }

    #[test]
    fn relative_time_minutes() {
        assert_eq!(relative_time_secs(60), "1m ago");
        assert_eq!(relative_time_secs(90), "1m ago");
        assert_eq!(relative_time_secs(3599), "59m ago");
    }

    #[test]
    fn relative_time_hours() {
        assert_eq!(relative_time_secs(3600), "1h ago");
        assert_eq!(relative_time_secs(7200), "2h ago");
        assert_eq!(relative_time_secs(86399), "23h ago");
    }

    #[test]
    fn relative_time_days() {
        assert_eq!(relative_time_secs(86400), "1d ago");
        assert_eq!(relative_time_secs(172800), "2d ago");
    }

    #[test]
    fn map_login_success() {
        let id = Uuid::nil();
        let item = map_audit_to_activity(id, "login", "success", "alice", 120);
        assert_eq!(item.ico, "✓");
        assert_eq!(item.activity_type, "s");
        assert_eq!(item.title, "Authentication successful");
        assert_eq!(item.link, "auditlogs");
        assert_eq!(item.time, "2m ago");
        assert_eq!(item.meta, "alice");
    }

    #[test]
    fn map_login_failure() {
        let id = Uuid::nil();
        let item = map_audit_to_activity(id, "login", "failed", "bob", 30);
        assert_eq!(item.ico, "⚠");
        assert_eq!(item.activity_type, "w");
        assert_eq!(item.title, "Authentication failed");
    }

    #[test]
    fn map_register() {
        let id = Uuid::nil();
        let item = map_audit_to_activity(id, "register", "success", "carol", 5);
        assert_eq!(item.ico, "📱");
        assert_eq!(item.activity_type, "i");
        assert_eq!(item.title, "New device enrolled");
        assert_eq!(item.link, "devices");
    }

    #[test]
    fn map_recovery_started() {
        let id = Uuid::nil();
        let item = map_audit_to_activity(id, "recovery_started", "pending", "dave", 600);
        assert_eq!(item.ico, "🔑");
        assert_eq!(item.activity_type, "w");
        assert_eq!(item.link, "recovery");
    }

    #[test]
    fn map_unknown_action() {
        let id = Uuid::nil();
        let item = map_audit_to_activity(id, "some_new_action", "ok", "eve", 10);
        assert_eq!(item.ico, "·");
        assert_eq!(item.activity_type, "i");
        assert_eq!(item.title, "some_new_action");
    }

    // ── Integration tests (real Postgres via #[sqlx::test]) ────────────────
    // See devices.rs for why handlers are called directly instead of over HTTP.

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
            jwt_secret: "test-secret".into(),
            cors_origin: "http://localhost:3001".into(),
        }
    }

    async fn seed_user(pool: &PgPool, role: &str) -> Uuid {
        sqlx::query_scalar("INSERT INTO users (username, role) VALUES ($1, $2) RETURNING id")
            .bind(format!("user-{}", Uuid::new_v4()))
            .bind(role)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[sqlx::test]
    async fn admin_bootstrap_promotes_the_only_caller_on_a_fresh_deployment(pool: PgPool) {
        let user_id = seed_user(&pool, "user").await;
        let state = test_state(pool.clone());
        let auth = AuthUser {
            user_id,
            username: "first".into(),
            cred_id: Uuid::new_v4(),
        };

        let Json(resp) = admin_bootstrap(auth, State(state)).await.unwrap();
        assert_eq!(resp.data.unwrap().status, "promoted");

        let role: String = sqlx::query_scalar("SELECT role FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(role, "super_admin");
    }

    #[sqlx::test]
    async fn admin_bootstrap_refuses_once_any_admin_exists(pool: PgPool) {
        seed_user(&pool, "admin").await; // an admin already exists
        let second_user = seed_user(&pool, "user").await;
        let state = test_state(pool);
        let auth = AuthUser {
            user_id: second_user,
            username: "second".into(),
            cred_id: Uuid::new_v4(),
        };

        let result = admin_bootstrap(auth, State(state)).await;
        assert!(
            result.is_err(),
            "bootstrap must not be a standing escalation path"
        );
    }

    #[sqlx::test]
    async fn list_users_returns_real_rows_from_the_database(pool: PgPool) {
        seed_user(&pool, "user").await;
        seed_user(&pool, "admin").await;
        let state = test_state(pool);
        let admin = AdminUser {
            user_id: Uuid::new_v4(),
            username: "caller".into(),
            cred_id: Uuid::new_v4(),
            role: "admin".into(),
        };

        let Json(resp) = list_users(admin, State(state)).await.unwrap();
        let users = resp.data.unwrap();
        assert_eq!(users.len(), 2);
        assert!(users.iter().any(|u| u.role == "admin"));
    }

    #[sqlx::test]
    async fn patch_user_role_updates_the_role(pool: PgPool) {
        let admin_id = seed_user(&pool, "admin").await; // a second admin, so demote is legal
        let target = seed_user(&pool, "user").await;
        let state = test_state(pool.clone());
        let admin = AdminUser {
            user_id: admin_id,
            username: "caller".into(),
            cred_id: Uuid::new_v4(),
            role: "admin".into(),
        };

        let Json(resp) = patch_user_role(
            admin,
            State(state),
            Path(target),
            AppJson(RolePatch {
                role: "admin".into(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(resp.data.unwrap().status, "updated");

        let role: String = sqlx::query_scalar("SELECT role FROM users WHERE id = $1")
            .bind(target)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(role, "admin");
    }

    #[sqlx::test]
    async fn patch_user_role_refuses_to_demote_the_last_admin(pool: PgPool) {
        let only_admin = seed_user(&pool, "admin").await;
        let state = test_state(pool.clone());
        let admin = AdminUser {
            user_id: only_admin,
            username: "caller".into(),
            cred_id: Uuid::new_v4(),
            role: "admin".into(),
        };

        let result = patch_user_role(
            admin,
            State(state),
            Path(only_admin),
            AppJson(RolePatch {
                role: "user".into(),
            }),
        )
        .await;
        assert!(result.is_err(), "demoting the last admin must be refused");

        let role: String = sqlx::query_scalar("SELECT role FROM users WHERE id = $1")
            .bind(only_admin)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            role, "admin",
            "role must be unchanged after the refused demotion"
        );
    }

    #[sqlx::test]
    async fn patch_user_role_rejects_invalid_role_strings(pool: PgPool) {
        let admin_id = seed_user(&pool, "admin").await;
        let target = seed_user(&pool, "user").await;
        let state = test_state(pool);
        let admin = AdminUser {
            user_id: admin_id,
            username: "caller".into(),
            cred_id: Uuid::new_v4(),
            role: "admin".into(),
        };

        let result = patch_user_role(
            admin,
            State(state),
            Path(target),
            AppJson(RolePatch {
                role: "superuser".into(),
            }),
        )
        .await;
        assert!(
            result.is_err(),
            "a role outside VALID_ROLES must be rejected before touching the database"
        );
    }
}
