use axum::{extract::State, http::StatusCode, middleware::Next, Request, response::Response};
use crate::state::AppState;
use crate::state::Metrics;
use redis::AsyncCommands;

#[derive(Clone, Debug)]
pub struct PolicyContext {
    pub risk_score: u8,
}

// MVP policy engine enhanced with a few simple rules. In a real system these
// would be declarative and probably driven by a policy database or DSL.
// to keep everything in the gateway we rely on Redis keys populated by audit
// events.
pub async fn enforce_policy<B>(
    State(state): State<AppState>,
    mut req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    let path = path.strip_prefix("/api").unwrap_or(path);

    let conn = state.redis.as_ref().clone();
    let mut conn = conn;

    // helper to fetch claims if present
    let claims_opt = req.extensions().get::<crate::middleware::auth::Claims>().cloned();

    // Start with a base risk score; rules below can bump it.
    let mut risk_score: u8 = 0;

    let enforce = std::env::var("POLICY_ENFORCE")
        .unwrap_or_else(|_| "true".into())
        .to_ascii_lowercase()
        != "false";

    let mut would_block = false;
    let mut would_block_status: Option<StatusCode> = None;

    macro_rules! block_or_dry_run {
        ($status:expr, $metric:expr, $msg:expr) => {{
            Metrics::inc(&$metric);
            if enforce {
                return Err($status);
            } else {
                Metrics::inc(&state.metrics.policy_would_block_dry_run);
                would_block = true;
                would_block_status = Some($status);
                tracing::warn!(status = ?$status, "policy(dry-run): {}", $msg);
            }
        }};
    }

    // locked user check (login lockout)
    if path.starts_with("/auth/login") {
        // For login, callers often won't have JWT claims yet. Allow passing a user id hint
        // (or use claims if present) so lockout can be enforced.
        let user_id = claims_opt
            .as_ref()
            .map(|c| c.sub.clone())
            .or_else(|| {
                req.headers()
                    .get("x-user-id")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string())
            });

        if let Some(user_id) = user_id {
            let lock_key = format!("login:locked:{}", user_id);
            if conn.exists(&lock_key).await.unwrap_or(false) {
                tracing::warn!(user = %user_id, key = %lock_key, "policy: user account locked due to failed logins");
                risk_score = risk_score.max(80);
                block_or_dry_run!(
                    StatusCode::LOCKED,
                    state.metrics.policy_blocked_locked,
                    "user locked"
                );
            }
        }
    }

    // deny by default for anything under /private unless user is authenticated
    if path.starts_with("/private") {
        if claims_opt.is_none() {
            tracing::warn!("policy: unauthenticated access to /private blocked");
            risk_score = risk_score.max(60);
            block_or_dry_run!(
                StatusCode::UNAUTHORIZED,
                state.metrics.policy_blocked_private_unauth,
                "private path requires auth"
            );
        }
    }

    // require active device for revocation endpoints
    if path.contains("/devices/revoke") {
        let status = req
            .headers()
            .get("x-device-status")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if status != "active" {
            tracing::warn!(status = %status, "policy: device status not active, blocking revoke");
            risk_score = risk_score.max(70);
            block_or_dry_run!(
                StatusCode::FORBIDDEN,
                state.metrics.policy_blocked_device_inactive,
                "device not active"
            );
        }
        // if the device was added recently require step‑up header
        if let Some(device) = req.headers().get("x-device-id").and_then(|h| h.to_str().ok()) {
            let key = format!("device:added:{}", device);
            if conn.exists(&key).await.unwrap_or(false) {
                let step = req
                    .headers()
                    .get("x-step-up")
                    .and_then(|h| h.to_str().ok());
                if step != Some("true") {
                    tracing::info!(device = %device, "step-up required for revoke");
                    risk_score = risk_score.max(75);
                    block_or_dry_run!(
                        StatusCode::UNAUTHORIZED,
                        state.metrics.policy_blocked_stepup_required,
                        "step-up required"
                    );
                }
            }
        }
    }

    // block new device additions if a recovery flow is pending for the user
    if path.contains("/devices/add") {
        if let Some(claims) = &claims_opt {
            let key = format!("recovery:pending:{}", claims.sub);
            if conn.exists(&key).await.unwrap_or(false) {
                tracing::warn!(user = %claims.sub, key = %key, "policy: recovery pending, blocking device add");
                risk_score = risk_score.max(70);
                block_or_dry_run!(
                    StatusCode::FORBIDDEN,
                    state.metrics.policy_blocked_recovery_pending,
                    "recovery pending"
                );
            }
        }
    }

    // Attach policy context for downstream handlers/observability.
    req.extensions_mut().insert(PolicyContext { risk_score });
    tracing::debug!(
        %path,
        %risk_score,
        enforce,
        would_block,
        would_block_status = ?would_block_status,
        "policy: request evaluated"
    );

    // everything else passes
    Ok(next.run(req).await)
}