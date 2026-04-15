//! Stub implementations for demo.
//! Returns plausible hardcoded data for every frontend-expected endpoint
//! that doesn't have a real DB implementation yet.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post},
    Json, Router,
};
use serde::Deserialize;
use sqlx::Row;
use uuid;

use crate::{error::{AppJson, ApiResponse}, jwt::AuthUser, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        // Dashboard
        .route("/dashboard/stats",    get(dashboard_stats))
        .route("/dashboard/activity", get(dashboard_activity))
        // Sessions
        .route("/sessions",      get(list_sessions).delete(delete_all_sessions))
        .route("/sessions/:id",  delete(delete_session))
        // Risk
        .route("/risk/users", get(risk_users))
        .route("/risk/feed",  get(risk_feed))
        .route("/risk/scan",  post(risk_scan))
        // Users & Roles
        .route("/users",          get(list_users))
        .route("/users/:id/role", patch(patch_user_role))
        // Passkeys
        .route("/passkeys",     get(list_passkeys))
        .route("/passkeys/:id", delete(delete_passkey))
        // Policies
        .route("/policies",     get(list_policies))
        .route("/policies/:id", patch(patch_policy))
        // Org
        .route("/org", get(get_org).patch(patch_org))
        // Audit
        .route("/export/audit-logs", get(export_audit_logs))
}

#[derive(serde::Serialize)]
struct StatusResp {
    status: &'static str,
}

// ── Dashboard ─────────────────────────────────────────────────────────────────

/// No auth required — always returns stub stats so the dashboard never 500s.
/// When the DB is available and the user is logged in, the real device count
/// comes from GET /devices (which IS auth-gated).
async fn dashboard_stats(State(state): State<AppState>) -> Json<ApiResponse<serde_json::Value>> {
    let active_devices: i64 = if let Some(db) = state.db.as_ref() {
        sqlx::query_scalar::<_, Option<i64>>(
            "SELECT COUNT(*) FROM credentials WHERE status = 'active'",
        )
        .fetch_one(db)
        .await
        .unwrap_or(Some(2))
        .unwrap_or(2)
    } else {
        2
    };

    Json(ApiResponse::success(serde_json::json!({
        "activeDevices":  active_devices,
        "authEvents24h":  142,
        "securityScore":  94
    })))
}

/// No auth required — returns activity feed in the shape the frontend expects.
/// Shape: Array<{ id, ico, type, title, meta, time, link }>
async fn dashboard_activity() -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!([
        { "id": "a1", "ico": "✓", "type": "s",
          "title": "Authentication successful",
          "meta": "MacBook Pro · Chrome · San Francisco, CA", "time": "1m ago", "link": "auditlogs" },
        { "id": "a2", "ico": "📱", "type": "i",
          "title": "New device enrolled",
          "meta": "iPhone 15 Pro · Passkey created", "time": "5m ago", "link": "devices" },
        { "id": "a3", "ico": "✓", "type": "s",
          "title": "Authentication successful",
          "meta": "iPad Air · Safari · New York, NY", "time": "12m ago", "link": "auditlogs" },
        { "id": "a4", "ico": "⚠", "type": "w",
          "title": "Unrecognized device blocked",
          "meta": "Unknown · Tokyo, JP · Request denied", "time": "1h ago", "link": "risk" }
    ])))
}

// ── Sessions ──────────────────────────────────────────────────────────────────

async fn list_sessions(_auth: AuthUser) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!([
        { "id": "sess_001", "user": "alice@example.com", "ip": "192.168.1.10",
          "device": "MacBook Pro", "created_at": "2026-03-24T10:00:00Z",
          "last_active": "2026-03-24T11:42:00Z", "status": "active" },
        { "id": "sess_002", "user": "bob@example.com",   "ip": "10.0.0.44",
          "device": "iPhone 15",  "created_at": "2026-03-24T09:00:00Z",
          "last_active": "2026-03-24T11:30:00Z", "status": "active" }
    ])))
}

async fn delete_session(_auth: AuthUser, Path(_id): Path<String>) -> Json<ApiResponse<StatusResp>> {
    Json(ApiResponse::success(StatusResp { status: "revoked" }))
}

async fn delete_all_sessions(_auth: AuthUser) -> Json<ApiResponse<StatusResp>> {
    Json(ApiResponse::success(StatusResp { status: "all_revoked" }))
}

// ── Risk ──────────────────────────────────────────────────────────────────────

async fn risk_users(_auth: AuthUser) -> Json<ApiResponse<serde_json::Value>> {
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

async fn risk_feed(_auth: AuthUser) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!([
        { "id": "feed_001", "type": "anomaly",     "user": "alice@example.com",
          "message": "Login from new country",              "time": "2m ago",  "severity": "high" },
        { "id": "feed_002", "type": "brute_force", "user": "unknown",
          "message": "15 failed attempts from 198.51.100.42", "time": "8m ago", "severity": "high" }
    ])))
}

async fn risk_scan(_auth: AuthUser) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({ "status": "scan_complete", "updated": 3, "high": 1, "medium": 1, "low": 1 })))
}

// ── Users & Roles ─────────────────────────────────────────────────────────────

async fn list_users(_auth: AuthUser) -> Json<ApiResponse<serde_json::Value>> {
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
    _auth: AuthUser,
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

async fn list_policies(_auth: AuthUser) -> Json<ApiResponse<serde_json::Value>> {
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
    _auth: AuthUser,
    Path(_id): Path<String>,
    AppJson(_body): AppJson<PolicyPatch>,
) -> Json<ApiResponse<StatusResp>> {
    Json(ApiResponse::success(StatusResp { status: "updated" }))
}

// ── Org ───────────────────────────────────────────────────────────────────────

async fn get_org(_auth: AuthUser) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({
        "name": "Crypton Demo Org",
        "domain": "example.com",
        "mfa_required": true,
        "allowed_countries": ["US", "CA", "GB"],
        "plan": "growth"
    })))
}

async fn patch_org(
    _auth: AuthUser,
    AppJson(_body): AppJson<serde_json::Value>,
) -> Json<ApiResponse<StatusResp>> {
    Json(ApiResponse::success(StatusResp { status: "updated" }))
}

// ── Audit ─────────────────────────────────────────────────────────────────────

async fn export_audit_logs(_auth: AuthUser, State(state): State<AppState>) -> impl IntoResponse {
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
