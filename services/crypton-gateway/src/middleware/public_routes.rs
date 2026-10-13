//! Explicit allowlist of authentication-bootstrap routes.
//!
//! These are the only routes reachable without a bearer token. Everything else
//! fails closed in `validate_jwt`. Matching is deliberately dumb: exact method
//! and exact, case-sensitive raw path. No prefixes, no normalisation, no
//! trailing-slash tolerance, no percent-decoding. A request that is "almost"
//! one of these (encoded, dotted, double-slashed, different method) is not
//! public and therefore needs a token.
//!
//! These routes keep their own protections: identity enforces WebAuthn
//! challenge/response and a stricter per-IP governor, and the gateway still
//! applies the timestamp/nonce check and per-IP / per-device rate limits.

use axum::http::Method;

/// Request extension marking a request that matched the public allowlist.
#[derive(Clone, Copy, Debug)]
pub struct PublicRoute;

pub fn is_public_route(method: &Method, path: &str) -> bool {
    matches!(
        (method.as_str(), path),
        ("POST", "/auth/register/start")
            | ("POST", "/auth/register/finish")
            | ("POST", "/auth/login/start")
            | ("POST", "/auth/login/finish")
            | ("POST", "/recovery/public/start")
            | ("GET", "/recovery/public/status")
            | ("POST", "/recovery/public/complete")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_routes_are_public() {
        assert!(is_public_route(&Method::POST, "/auth/login/start"));
        assert!(is_public_route(&Method::POST, "/auth/login/finish"));
        assert!(is_public_route(&Method::POST, "/auth/register/start"));
        assert!(is_public_route(&Method::POST, "/auth/register/finish"));
        assert!(is_public_route(&Method::POST, "/recovery/public/start"));
        assert!(is_public_route(&Method::GET, "/recovery/public/status"));
        assert!(is_public_route(&Method::POST, "/recovery/public/complete"));
    }

    #[test]
    fn near_misses_are_not_public() {
        for (m, p) in [
            (Method::GET, "/auth/login/start"),
            (Method::PUT, "/auth/login/start"),
            (Method::DELETE, "/auth/login/start"),
            (Method::POST, "/auth/login/start/"),
            (Method::POST, "/auth/login/start/extra"),
            (Method::POST, "/auth/login"),
            (Method::POST, "/AUTH/login/start"),
            (Method::POST, "/auth//login/start"),
            (Method::POST, "/auth/login/start/../../me"),
            (Method::POST, "/auth/login/%73tart"),
            (Method::POST, "/%61uth/login/start"),
            (Method::POST, "/api/auth/login/start"),
            (Method::POST, "/auth/logout"),
            (Method::POST, "/recovery/start"),
            (Method::POST, "/recovery/public/status"),
            (Method::GET, "/recovery/public/start"),
            (Method::GET, "/me"),
            (Method::POST, ""),
        ] {
            assert!(!is_public_route(&m, p), "{m} {p} must not be public");
        }
    }
}
