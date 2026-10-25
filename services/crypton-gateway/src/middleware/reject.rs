//! A rejection that carries a stable, machine-readable reason.
//!
//! Every security middleware returns `Reject` instead of a bare `StatusCode`
//! so the decision recorder (see `decision.rs`) can persist *why* a request
//! was denied. The reason is attached to the response extensions, never
//! serialised to the client.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

/// Why the gateway denied a request. Static strings only: reasons are an enum
/// in spirit, never built from request data (no secrets, no user input).
#[derive(Clone, Copy, Debug)]
pub struct DecisionReason(pub &'static str);

#[derive(Debug)]
pub struct Reject {
    pub status: StatusCode,
    pub reason: &'static str,
}

impl Reject {
    pub fn new(status: StatusCode, reason: &'static str) -> Self {
        Self { status, reason }
    }
}

impl IntoResponse for Reject {
    fn into_response(self) -> Response {
        let mut resp = self.status.into_response();
        resp.extensions_mut().insert(DecisionReason(self.reason));
        resp
    }
}
