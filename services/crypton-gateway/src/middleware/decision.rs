//! Durable allow/deny decision trail.
//!
//! Every request that enters the security stack ends in exactly one recorded
//! decision, written to `audit_logs` (`actor = 'gateway'`,
//! `action = 'gateway.decision'`, `outcome = 'allow' | 'deny'`):
//!
//! * **deny**: recorded by [`record_deny`], the outermost layer of the stack. A
//!   denial is any response carrying a [`DecisionReason`] (set by [`Reject`]).
//!   The request never reached the upstream, so if the row cannot be written
//!   the denial still stands; the failure is logged and counted
//!   (`gateway_audit_write_failed_total`).
//! * **allow**: recorded by [`record_allow`], the innermost layer, immediately
//!   BEFORE the request is forwarded. If the row cannot be persisted the
//!   request is refused (503) rather than executed unaudited, so an allowed
//!   action never exists without its record.
//!
//! Rows hold the reason, method, path (never the query string), request id,
//! client IP and principal ids. They never hold tokens, headers, bodies or
//! nonces.
//!
//! Not recorded: `/health` and CORS preflights (answered outside the stack).

use std::sync::{Arc, Mutex};

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::middleware::auth::Claims;
use crate::middleware::reject::{DecisionReason, Reject};
use crate::state::{AppState, Metrics};

/// Filled by `validate_jwt` so the outer deny recorder can attribute a denial
/// that happens after authentication (rate limit, nonce, policy, ...).
#[derive(Clone, Default)]
pub struct PrincipalSlot(pub Arc<Mutex<Option<(String, String)>>>);

impl PrincipalSlot {
    pub fn set(&self, claims: &Claims) {
        if let Ok(mut g) = self.0.lock() {
            *g = Some((claims.sub.clone(), claims.cred_id.clone()));
        }
    }
    fn get(&self) -> Option<(String, String)> {
        self.0.lock().ok().and_then(|g| g.clone())
    }
}

struct RequestFacts {
    method: String,
    path: String,
    request_id: String,
    client_ip: Option<String>,
}

fn facts(req: &Request) -> RequestFacts {
    let mut path = req.uri().path().to_string();
    path.truncate(512);
    RequestFacts {
        method: req.method().to_string(),
        path,
        request_id: req
            .headers()
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("-")
            .chars()
            .take(128)
            .collect(),
        client_ip: crate::middleware::client_ip(req).map(|i| i.to_string()),
    }
}

async fn write_decision(
    state: &AppState,
    principal: Option<(String, String)>,
    outcome: &'static str,
    reason: &str,
    status: Option<u16>,
    f: &RequestFacts,
) -> Result<(), sqlx::Error> {
    let (sub, cred) = match &principal {
        Some((s, c)) => (Uuid::parse_str(s).ok(), Uuid::parse_str(c).ok()),
        None => (None, None),
    };
    let detail = serde_json::json!({
        "reason": reason,
        "method": f.method,
        "path": f.path,
        "status": status,
        "request_id": f.request_id,
        "client_ip": f.client_ip,
        "claimed_user_id": principal.as_ref().map(|p| p.0.clone()),
    });
    // user_id is a foreign key to users: resolve through the table so an
    // unknown subject records NULL (kept in `detail`) instead of failing.
    sqlx::query(
        "INSERT INTO audit_logs (user_id, actor, credential_id, action, detail, outcome) \
         VALUES ((SELECT id FROM users WHERE id = $1), 'gateway', $2, 'gateway.decision', $3, $4)",
    )
    .bind(sub)
    .bind(cred)
    .bind(&detail)
    .bind(outcome)
    .execute(&state.pg)
    .await
    .map(|_| ())
}

/// Outermost security layer: attaches the principal slot and records denials.
pub async fn record_deny(State(state): State<AppState>, mut req: Request, next: Next) -> Response {
    let slot = PrincipalSlot::default();
    req.extensions_mut().insert(slot.clone());
    let f = facts(&req);

    let resp = next.run(req).await;

    if let Some(DecisionReason(reason)) = resp.extensions().get::<DecisionReason>().copied() {
        Metrics::inc(&state.metrics.audit_decisions_denied);
        let status = resp.status().as_u16();
        if let Err(e) = write_decision(&state, slot.get(), "deny", reason, Some(status), &f).await {
            Metrics::inc(&state.metrics.audit_write_failed);
            tracing::error!(error = %e, reason, "audit: failed to record deny decision");
        }
    }
    resp
}

/// Innermost security layer: persist the allow decision, then forward.
pub async fn record_allow(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, Reject> {
    let f = facts(&req);
    let principal = req
        .extensions()
        .get::<Claims>()
        .map(|c| (c.sub.clone(), c.cred_id.clone()));
    let reason = if principal.is_some() {
        "allowed"
    } else {
        "allowed_public_route"
    };

    match write_decision(&state, principal, "allow", reason, None, &f).await {
        Ok(()) => {
            Metrics::inc(&state.metrics.audit_decisions_allowed);
            Ok(next.run(req).await)
        }
        Err(e) => {
            Metrics::inc(&state.metrics.audit_write_failed);
            tracing::error!(error = %e, "audit: cannot record allow decision, refusing request");
            Err(Reject::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "audit_unavailable",
            ))
        }
    }
}
