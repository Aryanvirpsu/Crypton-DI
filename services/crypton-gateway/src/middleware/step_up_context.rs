use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::middleware::policy::StepUpContext;
use crate::state::AppState;

/// Populate [`StepUpContext`] for the current request.
///
/// Uses the authenticated user's `sub` claim as the session key and checks
/// whether `stepup:completed:{session}` exists in Redis.  Fails closed: if
/// Redis is unavailable or the key is absent, `verified` is false.
///
/// Must run AFTER auth middleware so that Claims are available in extensions.
pub async fn inject_step_up_context(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    // Identity tokens carry no jti: bind to this token (user + iat), the key
    // crypton-id writes after a server-verified WebAuthn step-up.
    let session_key = req
        .extensions()
        .get::<crate::middleware::auth::Claims>()
        .map(|c| format!("{}:{}", c.sub, c.iat));

    let verified = match session_key {
        Some(session) => {
            let mut conn = state.redis.as_ref().clone();
            let key = format!("stepup:completed:{}", session);
            redis::cmd("EXISTS")
                .arg(&key)
                .query_async::<bool>(&mut conn)
                .await
                .unwrap_or(false)
        }
        None => false,
    };

    req.extensions_mut().insert(StepUpContext { verified });
    next.run(req).await
}
