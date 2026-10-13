use axum::response::{IntoResponse, Response};
use axum::{extract::State, http::header::CONTENT_TYPE};

use crate::state::{AppState, Metrics};

fn format_counter(name: &str, value: u64) -> String {
    format!("# TYPE {name} counter\n{name} {value}\n")
}

pub async fn handler(State(state): State<AppState>) -> impl IntoResponse {
    let m: &Metrics = state.metrics.as_ref();

    let mut body = String::new();

    body.push_str(&format_counter(
        "gateway_auth_jwt_accepted_total",
        m.auth_jwt_accepted
            .load(std::sync::atomic::Ordering::Relaxed),
    ));
    body.push_str(&format_counter(
        "gateway_auth_rejected_total",
        m.auth_rejected.load(std::sync::atomic::Ordering::Relaxed),
    ));

    body.push_str(&format_counter(
        "gateway_rate_limit_block_ip_total",
        m.rate_limit_block_ip
            .load(std::sync::atomic::Ordering::Relaxed),
    ));
    body.push_str(&format_counter(
        "gateway_rate_limit_block_user_total",
        m.rate_limit_block_user
            .load(std::sync::atomic::Ordering::Relaxed),
    ));
    body.push_str(&format_counter(
        "gateway_rate_limit_block_device_total",
        m.rate_limit_block_device
            .load(std::sync::atomic::Ordering::Relaxed),
    ));

    body.push_str(&format_counter(
        "gateway_nonce_replay_blocked_total",
        m.nonce_replay_blocked
            .load(std::sync::atomic::Ordering::Relaxed),
    ));
    body.push_str(&format_counter(
        "gateway_nonce_timestamp_missing_total",
        m.nonce_timestamp_missing
            .load(std::sync::atomic::Ordering::Relaxed),
    ));
    body.push_str(&format_counter(
        "gateway_nonce_timestamp_invalid_total",
        m.nonce_timestamp_invalid
            .load(std::sync::atomic::Ordering::Relaxed),
    ));

    for (name, c) in [
        ("gateway_policy_blocked_deny_total", &m.policy_blocked_deny),
        (
            "gateway_policy_blocked_challenge_total",
            &m.policy_blocked_challenge,
        ),
        ("gateway_policy_blocked_hold_total", &m.policy_blocked_hold),
    ] {
        body.push_str(&format_counter(
            name,
            c.load(std::sync::atomic::Ordering::Relaxed),
        ));
    }
    body.push_str(&format_counter(
        "gateway_policy_would_block_dry_run_total",
        m.policy_would_block_dry_run
            .load(std::sync::atomic::Ordering::Relaxed),
    ));

    body.push_str(&format_counter(
        "gateway_proxy_upstream_2xx_total",
        m.proxy_upstream_2xx
            .load(std::sync::atomic::Ordering::Relaxed),
    ));
    body.push_str(&format_counter(
        "gateway_proxy_upstream_4xx_total",
        m.proxy_upstream_4xx
            .load(std::sync::atomic::Ordering::Relaxed),
    ));
    body.push_str(&format_counter(
        "gateway_proxy_upstream_5xx_total",
        m.proxy_upstream_5xx
            .load(std::sync::atomic::Ordering::Relaxed),
    ));
    body.push_str(&format_counter(
        "gateway_proxy_upstream_error_total",
        m.proxy_upstream_error
            .load(std::sync::atomic::Ordering::Relaxed),
    ));

    for (name, c) in [
        ("gateway_audit_write_failed_total", &m.audit_write_failed),
        (
            "gateway_audit_decisions_allowed_total",
            &m.audit_decisions_allowed,
        ),
        (
            "gateway_audit_decisions_denied_total",
            &m.audit_decisions_denied,
        ),
    ] {
        body.push_str(&format_counter(
            name,
            c.load(std::sync::atomic::Ordering::Relaxed),
        ));
    }

    let mut resp = Response::new(body);
    resp.headers_mut()
        .insert(CONTENT_TYPE, "text/plain; version=0.0.4".parse().unwrap());
    resp
}
