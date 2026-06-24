pub mod auth;
pub mod device_context;
pub mod nonce;
pub mod policy;
pub mod rate_limit;
pub mod risk_store;
pub mod step_up_context;

use std::net::IpAddr;

use axum::extract::{ConnectInfo, Request};

/// Resolve the client IP for security decisions (rate limiting, risk scoring).
///
/// Trust model: the real peer address comes from `ConnectInfo<SocketAddr>`,
/// which the external listener installs via `into_make_service_with_connect_info`.
/// `X-Forwarded-For` is honoured ONLY when the direct peer is a loopback or
/// RFC-1918 private address — i.e. we are behind a reverse proxy / load
/// balancer that we control. If the peer is a public IP, XFF is fully
/// client-controlled and MUST NOT be trusted: an attacker could otherwise spoof
/// `X-Forwarded-For: 10.0.0.1` to claim an internal address and manipulate
/// IP-based risk signals (RFC-1918 trust bonus, known-IP history, geo).
pub fn client_ip(req: &Request) -> Option<IpAddr> {
    let peer_ip: Option<IpAddr> = req
        .extensions()
        .get::<ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip());

    let peer = peer_ip?;

    let from_trusted_proxy = match peer {
        IpAddr::V4(ip) => ip.is_loopback() || ip.is_private(),
        IpAddr::V6(ip) => ip.is_loopback(),
    };

    if from_trusted_proxy {
        if let Some(xff) = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split(',').next())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse::<IpAddr>().ok())
        {
            return Some(xff);
        }
    }

    Some(peer)
}
