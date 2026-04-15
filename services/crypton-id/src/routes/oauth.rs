/// OAuth 2.0 Authorization Code flow — minimal implementation
///
/// Routes:
///   GET  /authorize             — validate client, store session, redirect to React login
///   POST /auth/oauth/complete   — called by React after WebAuthn; issues auth code, returns redirect_url
///   POST /token                 — exchange code for opaque access token (server-to-server)
///   GET  /userinfo              — return {sub, username} for valid access token
///
/// Storage (Redis):
///   oauth:session:{nonce}  TTL 600s  → {client_id, redirect_uri, state}
///   oauth:code:{code}      TTL 120s  → {user_id, username, client_id, redirect_uri}
///   oauth:token:{token}    TTL 3600s → {user_id, username}
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Json, Router,
};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::env;
use uuid::Uuid;

use crate::{error::AppJson, state::AppState};

// ── Config helpers (read env at call time; no startup overhead) ───────────────

fn cfg_client_id() -> String {
    env::var("OAUTH_CLIENT_ID").unwrap_or_else(|_| "demo-site".into())
}
fn cfg_client_secret() -> String {
    match env::var("OAUTH_CLIENT_SECRET") {
        Ok(s) if !s.is_empty() => s,
        _ => {
            tracing::warn!("OAUTH_CLIENT_SECRET not set — using insecure default. DO NOT deploy to production.");
            "insecure-change-me".into()
        }
    }
}
fn cfg_redirect_uris() -> Vec<String> {
    env::var("OAUTH_REDIRECT_URIS")
        .unwrap_or_else(|_| "http://localhost:4000/callback".into())
        .split(',')
        .map(|s| s.trim().to_string())
        .collect()
}
fn cfg_frontend() -> String {
    env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "http://localhost:3000".into())
}

// ── GET /authorize ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AuthorizeParams {
    response_type: String,
    client_id: String,
    redirect_uri: String,
    state: String,
    #[allow(dead_code)]
    scope: Option<String>,
}

pub async fn authorize(
    Query(p): Query<AuthorizeParams>,
    State(st): State<AppState>,
) -> Response {
    if p.response_type != "code" {
        return (StatusCode::BAD_REQUEST, "unsupported_response_type").into_response();
    }
    if p.client_id != cfg_client_id() {
        tracing::warn!("oauth/authorize: unknown client_id={}", p.client_id);
        return (StatusCode::BAD_REQUEST, "unknown_client").into_response();
    }
    if !cfg_redirect_uris().contains(&p.redirect_uri) {
        tracing::warn!("oauth/authorize: disallowed redirect_uri={}", p.redirect_uri);
        return (StatusCode::BAD_REQUEST, "redirect_uri_not_allowed").into_response();
    }

    let nonce = Uuid::new_v4().to_string();
    let payload = serde_json::json!({
        "client_id": p.client_id,
        "redirect_uri": p.redirect_uri,
        "state": p.state,
    })
    .to_string();

    match st.redis_conn().await {
        Ok(mut conn) => {
            let key = format!("oauth:session:{}", nonce);
            let _: Result<(), _> = conn.set_ex(&key, &payload, 600u64).await;
            tracing::info!(
                "oauth/authorize: session stored nonce={} client={}",
                nonce,
                p.client_id
            );
        }
        Err(e) => {
            tracing::error!("oauth/authorize: redis error: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "storage_error").into_response();
        }
    }

    let login_url = format!("{}/login?oauth={}", cfg_frontend(), nonce);
    tracing::info!("oauth/authorize: → {}", login_url);
    Redirect::temporary(&login_url).into_response()
}

// ── POST /auth/oauth/complete ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CompleteBody {
    nonce: String,
    token: String, // JWT from WebAuthn login finish
}

#[derive(Serialize)]
pub struct CompleteResponse {
    redirect_url: String,
}

pub async fn oauth_complete(
    State(st): State<AppState>,
    AppJson(body): AppJson<CompleteBody>,
) -> Response {
    // Verify JWT signature + expiry
    let claims = match jsonwebtoken::decode::<crate::jwt::Claims>(
        &body.token,
        &jsonwebtoken::DecodingKey::from_secret(st.jwt_secret.as_bytes()),
        &jsonwebtoken::Validation::default(),
    ) {
        Ok(d) => d.claims,
        Err(e) => {
            tracing::warn!("oauth/complete: invalid JWT: {}", e);
            return (StatusCode::UNAUTHORIZED, "invalid_token").into_response();
        }
    };

    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => return (StatusCode::UNAUTHORIZED, "invalid_token_sub").into_response(),
    };
    let cred_uuid = match Uuid::parse_str(&claims.cred_id) {
        Ok(id) => id,
        Err(_) => return (StatusCode::UNAUTHORIZED, "invalid_token_cred").into_response(),
    };

    if let Some(ref db) = st.db {
        if let Err(e) = crate::jwt::verify_device_active(db, cred_uuid, user_id).await {
            tracing::warn!("oauth/complete: device validation failed: {:?}", e);
            return (StatusCode::UNAUTHORIZED, "device_not_active").into_response();
        }
    }

    let mut conn = match st.redis_conn().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("oauth/complete: redis error: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "storage_error").into_response();
        }
    };

    let session_key = format!("oauth:session:{}", body.nonce);
    let raw: Option<String> = conn.get(&session_key).await.ok().flatten();
    let raw = match raw {
        Some(s) => s,
        None => {
            tracing::warn!("oauth/complete: unknown/expired nonce={}", body.nonce);
            return (StatusCode::BAD_REQUEST, "invalid_or_expired_session").into_response();
        }
    };

    // Consume nonce — one-time use
    let _: Result<(), _> = conn.del(&session_key).await;

    let session: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
    let redirect_uri = session["redirect_uri"].as_str().unwrap_or("").to_string();
    let oauth_state = session["state"].as_str().unwrap_or("").to_string();

    // Issue auth code
    let code = Uuid::new_v4().to_string();
    let code_payload = serde_json::json!({
        "user_id":    claims.sub,
        "username":   claims.username,
        "client_id":  cfg_client_id(),
        "redirect_uri": redirect_uri,
    })
    .to_string();

    let code_key = format!("oauth:code:{}", code);
    let _: Result<(), _> = conn.set_ex(&code_key, &code_payload, 120u64).await;

    tracing::info!("oauth/complete: code issued for user={}", claims.username);

    let redirect_url = format!("{}?code={}&state={}", redirect_uri, code, oauth_state);
    Json(CompleteResponse { redirect_url }).into_response()
}

// ── POST /token ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct TokenBody {
    grant_type: String,
    code: String,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

#[derive(Serialize)]
pub struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u64,
}

pub async fn token(
    State(st): State<AppState>,
    AppJson(body): AppJson<TokenBody>,
) -> Response {
    if body.grant_type != "authorization_code" {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"unsupported_grant_type"})),
        )
            .into_response();
    }
    if body.client_id != cfg_client_id() || body.client_secret != cfg_client_secret() {
        tracing::warn!("oauth/token: invalid client credentials client_id={}", body.client_id);
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error":"invalid_client"})),
        )
            .into_response();
    }

    let mut conn = match st.redis_conn().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("oauth/token: redis error: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "storage_error").into_response();
        }
    };

    let code_key = format!("oauth:code:{}", body.code);
    let raw: Option<String> = conn.get(&code_key).await.ok().flatten();
    let raw = match raw {
        Some(s) => s,
        None => {
            tracing::warn!("oauth/token: invalid or expired code");
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error":"invalid_grant"})),
            )
                .into_response();
        }
    };

    // Consume code — one-time use
    let _: Result<(), _> = conn.del(&code_key).await;

    let code_data: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();

    if code_data["client_id"].as_str() != Some(&body.client_id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"invalid_grant"})),
        )
            .into_response();
    }
    if code_data["redirect_uri"].as_str() != Some(&body.redirect_uri) {
        tracing::warn!("oauth/token: redirect_uri mismatch");
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"invalid_grant"})),
        )
            .into_response();
    }

    let user_id = code_data["user_id"].as_str().unwrap_or("").to_string();
    let username = code_data["username"].as_str().unwrap_or("").to_string();

    // Issue opaque access token stored in Redis
    let access_token = Uuid::new_v4().to_string();
    let token_payload =
        serde_json::json!({ "user_id": user_id, "username": username }).to_string();
    let token_key = format!("oauth:token:{}", access_token);
    let _: Result<(), _> = conn.set_ex(&token_key, &token_payload, 3600u64).await;

    tracing::info!("oauth/token: access token issued for user={}", username);

    Json(TokenResponse {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in: 3600,
    })
    .into_response()
}

// ── GET /userinfo ─────────────────────────────────────────────────────────────

pub async fn userinfo(State(st): State<AppState>, headers: HeaderMap) -> Response {
    let bearer = match headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
    {
        Some(t) => t.to_string(),
        None => return (StatusCode::UNAUTHORIZED, "missing_bearer").into_response(),
    };

    let mut conn = match st.redis_conn().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("oauth/userinfo: redis error: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "storage_error").into_response();
        }
    };

    let key = format!("oauth:token:{}", bearer);
    let raw: Option<String> = conn.get(&key).await.ok().flatten();
    let raw = match raw {
        Some(s) => s,
        None => {
            return (StatusCode::UNAUTHORIZED, "invalid_or_expired_token").into_response()
        }
    };

    let data: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
    tracing::info!(
        "oauth/userinfo: served for user={}",
        data["username"].as_str().unwrap_or("?")
    );

    Json(serde_json::json!({
        "sub":      data["user_id"],
        "username": data["username"],
    }))
    .into_response()
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/authorize", get(authorize))
        .route("/auth/oauth/complete", post(oauth_complete))
        .route("/token", post(token))
        .route("/userinfo", get(userinfo))
}
