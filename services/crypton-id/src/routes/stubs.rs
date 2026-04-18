//! Route handlers: real dashboard implementations + stub handlers for features
//! not yet backed by a real DB implementation.
//!
//! Real handlers (auth-gated, live DB):
//!   GET /dashboard/stats          — AuthUser, user-scoped
//!   GET /dashboard/activity       — AuthUser, user-scoped
//!   GET /admin/dashboard/stats    — AdminUser, global
//!   GET /admin/dashboard/activity — AdminUser, global
//!
//! Stubs (hardcoded, for demo):
//!   sessions, risk, users, policies, org, export

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    error::{ApiResponse, AppError, AppJson},
    jwt::{AdminUser, AuthUser},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        // ── Real dashboard (user-scoped) ──────────────────────────────────
        .route("/dashboard/stats",    get(dashboard_stats))
        .route("/dashboard/activity", get(dashboard_activity))
        // ── Real dashboard (admin, global) ────────────────────────────────
        .route("/admin/dashboard/stats",    get(admin_dashboard_stats))
        .route("/admin/dashboard/activity", get(admin_dashboard_activity))
        // ── Sessions ──────────────────────────────────────────────────────
        .route("/sessions",      get(list_sessions).delete(delete_all_sessions))
        .route("/sessions/:id",  delete(delete_session))
        // ── Risk ──────────────────────────────────────────────────────────
        .route("/risk/users", get(risk_users))
        .route("/risk/feed",  get(risk_feed))
        .route("/risk/scan",  post(risk_scan))
        // ── Users & Roles ─────────────────────────────────────────────────
        .route("/users",          get(list_users))
        .route("/users/:id/role", patch(patch_user_role))
        // ── Passkeys ──────────────────────────────────────────────────────
        .route("/passkeys",     get(list_passkeys))
        .route("/passkeys/:id", delete(delete_passkey))
        // ── Policies ──────────────────────────────────────────────────────
        .route("/policies",     get(list_policies))
        .route("/policies/:id", patch(patch_policy))
        // ── Org ───────────────────────────────────────────────────────────
        .route("/org", get(get_org).patch(patch_org))
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
    id:            Uuid,
    ico:           String,
    #[serde(rename = "type")]
    activity_type: String,
    title:         String,
    meta:          String,
    time:          String,
    link:          String,
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
    log_id:   Uuid,
    action:   &str,
    outcome:  &str,
    actor:    &str,
    age_secs: i64,
) -> ActivityItem {
    let (ico, act_type, title, link): (&str, &str, &str, &str) =
        match (action, outcome) {
            ("login",              "success") => ("✓",  "s", "Authentication successful", "auditlogs"),
            ("login",              _)         => ("⚠",  "w", "Authentication failed",     "auditlogs"),
            ("register",           _)         => ("📱", "i", "New device enrolled",       "devices"),
            ("logout",             _)         => ("🔒", "i", "Session ended",             "auditlogs"),
            ("recovery_started",   _)         => ("🔑", "w", "Recovery started",          "recovery"),
            ("recovery_completed", _)         => ("✓",  "i", "Recovery completed",        "recovery"),
            _                                 => ("·",  "i", action,                      "auditlogs"),
        };

    ActivityItem {
        id:            log_id,
        ico:           ico.to_string(),
        activity_type: act_type.to_string(),
        title:         title.to_string(),
        meta:          actor.to_string(),
        time:          relative_time_secs(age_secs),
        link:          link.to_string(),
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
    let lost_devices:   i64 = row.try_get("lost").unwrap_or(0);
    let auth_24h:       i64 = row.try_get("auth_24h").unwrap_or(0);
    let fail_24h:       i64 = row.try_get("fail_24h").unwrap_or(0);

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
            let id:       Uuid   = r.try_get("id").ok()?;
            let actor:    String = r.try_get("actor").unwrap_or_default();
            let action:   String = r.try_get("action").ok()?;
            let outcome:  String = r.try_get("outcome").ok()?;
            let age_secs: i64    = r.try_get("age_seconds").unwrap_or(0);
            Some(map_audit_to_activity(id, &action, &outcome, &actor, age_secs))
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

    let total_users:          i64 = row.try_get("total").unwrap_or(0);
    let active_credentials:   i64 = row.try_get("active").unwrap_or(0);
    let lost_credentials:     i64 = row.try_get("lost").unwrap_or(0);
    let revoked_credentials:  i64 = row.try_get("revoked").unwrap_or(0);
    let auth_24h:             i64 = row.try_get("auth_24h").unwrap_or(0);
    let failed_auths_24h:     i64 = row.try_get("fail_24h").unwrap_or(0);

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
            let id:       Uuid   = r.try_get("id").ok()?;
            let actor:    String = r.try_get("actor").unwrap_or_default();
            let action:   String = r.try_get("action").ok()?;
            let outcome:  String = r.try_get("outcome").ok()?;
            let age_secs: i64    = r.try_get("age_seconds").unwrap_or(0);
            Some(map_audit_to_activity(id, &action, &outcome, &actor, age_secs))
        })
        .collect();

    Ok(Json(ApiResponse::success(items)))
}

// ── Sessions ──────────────────────────────────────────────────────────────────

async fn list_sessions(_admin: AdminUser) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!([
        { "id": "sess_001", "user": "alice@example.com", "ip": "192.168.1.10",
          "device": "MacBook Pro", "created_at": "2026-03-24T10:00:00Z",
          "last_active": "2026-03-24T11:42:00Z", "status": "active" },
        { "id": "sess_002", "user": "bob@example.com",   "ip": "10.0.0.44",
          "device": "iPhone 15",  "created_at": "2026-03-24T09:00:00Z",
          "last_active": "2026-03-24T11:30:00Z", "status": "active" }
    ])))
}

async fn delete_session(_admin: AdminUser, Path(_id): Path<String>) -> Json<ApiResponse<StatusResp>> {
    Json(ApiResponse::success(StatusResp { status: "revoked" }))
}

async fn delete_all_sessions(_admin: AdminUser) -> Json<ApiResponse<StatusResp>> {
    Json(ApiResponse::success(StatusResp { status: "all_revoked" }))
}

// ── Risk ──────────────────────────────────────────────────────────────────────

async fn risk_users(_admin: AdminUser) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!([
        { "id": "risk_001", "user": "alice@example.com", "score": 87, "level": "HIGH",
          "device": "MacBook Pro", "ip": "198.51.100.42", "loc": "San Francisco, CA",
          "time": "2m ago",  "reasons": ["Unusual login location", "New device registration"] },
        { "id": "risk_002", "user": "bob@example.com",   "score": 45, "level": "MEDIUM",
          "device": "iPhone 15",  "ip": "203.0.113.7",   "loc": "New York, NY",
          "time": "15m ago", "reasons": ["Multiple failed attempts"] },
        { "id": "risk_003", "user": "carol@example.com", "score": 12, "level": "LOW",
          "device": "Windows PC", "ip": "192.0.2.1",     "loc": "Austin, TX",
          "time": "1h ago",  "reasons": [] }
    ])))
}

async fn risk_feed(_admin: AdminUser) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!([
        { "id": "feed_001", "type": "anomaly",     "user": "alice@example.com",
          "message": "Login from new country",              "time": "2m ago",  "severity": "high" },
        { "id": "feed_002", "type": "brute_force", "user": "unknown",
          "message": "15 failed attempts from 198.51.100.42", "time": "8m ago", "severity": "high" }
    ])))
}

async fn risk_scan(_admin: AdminUser) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({ "status": "scan_complete", "updated": 3, "high": 1, "medium": 1, "low": 1 })))
}

// ── Users & Roles ─────────────────────────────────────────────────────────────

async fn list_users(_admin: AdminUser) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!([
        { "id": "u1", "email": "alice@example.com", "role": "admin",  "devices": 2, "status": "active" },
        { "id": "u2", "email": "bob@example.com",   "role": "member", "devices": 1, "status": "active" },
        { "id": "u3", "email": "carol@example.com", "role": "viewer", "devices": 1, "status": "active" }
    ])))
}

#[derive(Deserialize)]
struct RolePatch {
    #[allow(dead_code)]
    role: String,
}

async fn patch_user_role(
    _admin: AdminUser,
    Path(_id): Path<String>,
    AppJson(_body): AppJson<RolePatch>,
) -> Json<ApiResponse<StatusResp>> {
    Json(ApiResponse::success(StatusResp { status: "updated" }))
}

// ── Passkeys ──────────────────────────────────────────────────────────────────

async fn list_passkeys(_auth: AuthUser) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!([
        { "id": "pk_001", "name": "Touch ID — MacBook Pro", "created_at": "2026-03-01T00:00:00Z", "last_used": "1h ago" },
        { "id": "pk_002", "name": "Face ID — iPhone 15",   "created_at": "2026-03-10T00:00:00Z", "last_used": "2d ago" }
    ])))
}

async fn delete_passkey(_auth: AuthUser, Path(_id): Path<String>) -> Json<ApiResponse<StatusResp>> {
    Json(ApiResponse::success(StatusResp { status: "revoked" }))
}

// ── Policies ──────────────────────────────────────────────────────────────────

async fn list_policies(_admin: AdminUser) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!([
        { "id": "pol_001", "name": "Require MFA",             "active": true,  "description": "All users must use a second factor" },
        { "id": "pol_002", "name": "Device Trust Required",   "active": true,  "description": "Only enrolled devices can authenticate" },
        { "id": "pol_003", "name": "Geo-block high-risk IPs", "active": false, "description": "Block logins from known malicious IPs" }
    ])))
}

#[derive(Deserialize)]
struct PolicyPatch {
    #[allow(dead_code)]
    active: bool,
}

async fn patch_policy(
    _admin: AdminUser,
    Path(_id): Path<String>,
    AppJson(_body): AppJson<PolicyPatch>,
) -> Json<ApiResponse<StatusResp>> {
    Json(ApiResponse::success(StatusResp { status: "updated" }))
}

// ── Org ───────────────────────────────────────────────────────────────────────

async fn get_org(_admin: AdminUser) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({
        "name": "Crypton Demo Org",
        "domain": "example.com",
        "mfa_required": true,
        "allowed_countries": ["US", "CA", "GB"],
        "plan": "growth"
    })))
}

async fn patch_org(
    _admin: AdminUser,
    AppJson(_body): AppJson<serde_json::Value>,
) -> Json<ApiResponse<StatusResp>> {
    Json(ApiResponse::success(StatusResp { status: "updated" }))
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
            ("Content-Disposition", "attachment; filename=\"audit-logs.csv\""),
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
        assert_eq!(relative_time_secs(0),     "0s ago");
        assert_eq!(relative_time_secs(45),    "45s ago");
        assert_eq!(relative_time_secs(59),    "59s ago");
    }

    #[test]
    fn relative_time_minutes() {
        assert_eq!(relative_time_secs(60),    "1m ago");
        assert_eq!(relative_time_secs(90),    "1m ago");
        assert_eq!(relative_time_secs(3599),  "59m ago");
    }

    #[test]
    fn relative_time_hours() {
        assert_eq!(relative_time_secs(3600),  "1h ago");
        assert_eq!(relative_time_secs(7200),  "2h ago");
        assert_eq!(relative_time_secs(86399), "23h ago");
    }

    #[test]
    fn relative_time_days() {
        assert_eq!(relative_time_secs(86400),  "1d ago");
        assert_eq!(relative_time_secs(172800), "2d ago");
    }

    #[test]
    fn map_login_success() {
        let id = Uuid::nil();
        let item = map_audit_to_activity(id, "login", "success", "alice", 120);
        assert_eq!(item.ico,           "✓");
        assert_eq!(item.activity_type, "s");
        assert_eq!(item.title,         "Authentication successful");
        assert_eq!(item.link,          "auditlogs");
        assert_eq!(item.time,          "2m ago");
        assert_eq!(item.meta,          "alice");
    }

    #[test]
    fn map_login_failure() {
        let id = Uuid::nil();
        let item = map_audit_to_activity(id, "login", "failed", "bob", 30);
        assert_eq!(item.ico,           "⚠");
        assert_eq!(item.activity_type, "w");
        assert_eq!(item.title,         "Authentication failed");
    }

    #[test]
    fn map_register() {
        let id = Uuid::nil();
        let item = map_audit_to_activity(id, "register", "success", "carol", 5);
        assert_eq!(item.ico,           "📱");
        assert_eq!(item.activity_type, "i");
        assert_eq!(item.title,         "New device enrolled");
        assert_eq!(item.link,          "devices");
    }

    #[test]
    fn map_recovery_started() {
        let id = Uuid::nil();
        let item = map_audit_to_activity(id, "recovery_started", "pending", "dave", 600);
        assert_eq!(item.ico,           "🔑");
        assert_eq!(item.activity_type, "w");
        assert_eq!(item.link,          "recovery");
    }

    #[test]
    fn map_unknown_action() {
        let id = Uuid::nil();
        let item = map_audit_to_activity(id, "some_new_action", "ok", "eve", 10);
        assert_eq!(item.ico,           "·");
        assert_eq!(item.activity_type, "i");
        assert_eq!(item.title,         "some_new_action");
    }
}
