//! Entry-point rate limiting (Phase 7).
//!
//! The engine itself is CPU-bound and cheap, but a flood of `evaluate()` calls
//! keyed to one user or IP is almost always abusive (credential stuffing, scraping,
//! a stuck retry loop). This module provides a pluggable pre-filter that callers
//! apply **before** calling [`crate::evaluate`] so the engine budget stays
//! bounded under spike conditions.
//!
//! ## Design
//!
//! - [`RateLimiter`] is a single-method trait: `check(key) -> Result<(), ...>`.
//!   One method keeps both the user-keyed and IP-keyed paths homogeneous and
//!   keeps callers honest about what they're rate-limiting.
//! - [`InMemoryRateLimiter`] is a per-key token bucket kept in a `Mutex<HashMap>`.
//!   Suitable for single-process deployments; swap for a Redis-backed
//!   implementation at scale.
//! - [`evaluate_with_rate_limit`] is a drop-in wrapper around
//!   [`crate::evaluate`] that checks user-id and request-IP before letting the
//!   scoring engine run. A hit on either key returns
//!   [`RiskEngineError::RateLimited`] without consuming engine/Redis budget.
//!
//! ## What the limiter is *not* for
//!
//! This is an abuse-mitigation throttle, not an authorization primitive. A
//! denied request is a `429`-shaped outcome; actual risk verdicts still flow
//! through [`crate::evaluate`].

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::Instant;

use async_trait::async_trait;
use uuid::Uuid;

use crate::context::RiskContext;
use crate::decision::RiskDecision;
use crate::engine::evaluate;
use crate::error::RiskEngineError;
use crate::store::SignalStore;

// ─────────────────────────────────────────────────────────────────────────────
// Trait
// ─────────────────────────────────────────────────────────────────────────────

/// Opaque rate-limit decision surface. Implementations MUST be cheap — the
/// check runs synchronously on every engine call.
#[async_trait]
pub trait RateLimiter: Send + Sync {
    /// Record one request against `key` and return `Ok(())` if permitted.
    /// On exhaustion return [`RiskEngineError::RateLimited`] with a caller-
    /// useful reason string.
    async fn check(&self, key: &str) -> Result<(), RiskEngineError>;
}

// ─────────────────────────────────────────────────────────────────────────────
// InMemoryRateLimiter — token bucket
// ─────────────────────────────────────────────────────────────────────────────

/// Simple per-key token bucket. Refills `rate_per_sec` tokens per second up to
/// `capacity` tokens. One request consumes one token.
///
/// This is an in-process limiter — state is NOT shared between replicas. For
/// multi-replica deployments, implement [`RateLimiter`] on top of a shared
/// backend (e.g. Redis `INCR` with TTL) and swap it in at the call site.
pub struct InMemoryRateLimiter {
    capacity: f64,
    rate_per_sec: f64,
    buckets: Mutex<HashMap<String, Bucket>>,
}

#[derive(Debug, Clone, Copy)]
struct Bucket {
    tokens: f64,
    last_refill: Instant,
}

impl InMemoryRateLimiter {
    /// `capacity`: max burst size. `rate_per_sec`: sustained refill rate.
    /// Both must be positive.
    pub fn new(capacity: u32, rate_per_sec: u32) -> Self {
        assert!(capacity > 0, "rate limiter capacity must be > 0");
        assert!(rate_per_sec > 0, "rate limiter refill rate must be > 0");
        Self {
            capacity: capacity as f64,
            rate_per_sec: rate_per_sec as f64,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Synchronous check that the async trait impl delegates to. Exposed so
    /// callers that prefer not to pay for an async hop can use it directly.
    pub fn check_sync(&self, key: &str) -> Result<(), RiskEngineError> {
        let now = Instant::now();
        let mut buckets = self.buckets.lock().unwrap();
        let bucket = buckets.entry(key.to_string()).or_insert(Bucket {
            tokens: self.capacity,
            last_refill: now,
        });

        // Refill proportional to elapsed wall-clock time, clamped at capacity.
        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * self.rate_per_sec).min(self.capacity);
        bucket.last_refill = now;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            Ok(())
        } else {
            Err(RiskEngineError::RateLimited(format!(
                "key={key} cap={} rate_per_sec={}",
                self.capacity as u32, self.rate_per_sec as u32
            )))
        }
    }

    /// Drop the bucket for `key`. Intended for tests and operator tooling.
    pub fn reset(&self, key: &str) {
        self.buckets.lock().unwrap().remove(key);
    }
}

#[async_trait]
impl RateLimiter for InMemoryRateLimiter {
    async fn check(&self, key: &str) -> Result<(), RiskEngineError> {
        self.check_sync(key)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Keys
// ─────────────────────────────────────────────────────────────────────────────

/// Canonical key helpers. Using these keeps metric labels and bucket lookups
/// aligned across adapters.
pub fn user_key(user_id: Uuid) -> String {
    format!("user:{user_id}")
}

pub fn ip_key(ip: IpAddr) -> String {
    format!("ip:{ip}")
}

// ─────────────────────────────────────────────────────────────────────────────
// Wrapper
// ─────────────────────────────────────────────────────────────────────────────

/// Rate-limit-guarded wrapper over [`crate::evaluate`]. Checks the user-id
/// and request-IP buckets before evaluating; on exhaustion returns
/// [`RiskEngineError::RateLimited`] without touching the signal store.
///
/// The user check runs first because a user-keyed flood is strictly more
/// interesting than an IP-keyed one (shared NAT is common, shared accounts
/// are not). Callers that want a different ordering should compose the
/// limiter manually.
///
/// ## Fail mode — STRICT (default)
///
/// If the limiter itself returns a non-[`RiskEngineError::RateLimited`]
/// error (e.g. [`RiskEngineError::Timeout`] from an unreachable Redis),
/// this function propagates it. That is **fail-closed**: a dead Redis
/// will cause the wrapper to return an error to the caller, who presumably
/// maps it to 5xx. This is the right choice only when you actively prefer
/// "reject traffic until Redis recovers" over "admit traffic unthrottled".
///
/// For almost every web-facing deployment, the correct choice is
/// [`evaluate_with_rate_limit_fail_open`] below. Keep this variant if your
/// security model treats "can't enforce rate limit" as a hard stop.
pub async fn evaluate_with_rate_limit<L: RateLimiter, S: SignalStore>(
    ctx: RiskContext,
    store: &S,
    limiter: &L,
) -> Result<RiskDecision, RiskEngineError> {
    limiter.check(&user_key(ctx.user_id)).await?;
    if let Some(ip) = ctx.request_ip {
        limiter.check(&ip_key(ip)).await?;
    }
    Ok(evaluate(ctx, store).await)
}

/// Fail-open variant of [`evaluate_with_rate_limit`].
///
/// Only an explicit [`RiskEngineError::RateLimited`] blocks the request —
/// any other error from the limiter (Redis timeout, store error) is
/// treated as "limiter is broken, admit the request" and the engine runs
/// normally. The failure is logged at warn level so operators can alert.
///
/// This is the correct default for user-facing HTTP traffic: a dead Redis
/// should not cause legitimate users to see 5xx on their login. The
/// tradeoff is that a sustained Redis outage means no rate limiting; you
/// must detect that condition through metrics (`risk_engine_rate_limit_degraded_total`)
/// rather than relying on traffic back-pressure.
pub async fn evaluate_with_rate_limit_fail_open<L: RateLimiter, S: SignalStore>(
    ctx: RiskContext,
    store: &S,
    limiter: &L,
) -> Result<RiskDecision, RiskEngineError> {
    if let Err(e) = limiter.check(&user_key(ctx.user_id)).await {
        if matches!(e, RiskEngineError::RateLimited(_)) {
            return Err(e);
        }
        record_rate_limit_degraded();
        tracing::warn!(error = %e, "rate limiter failed on user key, admitting request");
    }
    if let Some(ip) = ctx.request_ip {
        if let Err(e) = limiter.check(&ip_key(ip)).await {
            if matches!(e, RiskEngineError::RateLimited(_)) {
                return Err(e);
            }
            record_rate_limit_degraded();
            tracing::warn!(error = %e, ip = %ip, "rate limiter failed on ip key, admitting request");
        }
    }
    Ok(evaluate(ctx, store).await)
}

#[inline]
fn record_rate_limit_degraded() {
    #[cfg(feature = "metrics")]
    metrics::counter!("risk_engine_rate_limit_degraded_total").increment(1);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn burst_then_deny() {
        let limiter = InMemoryRateLimiter::new(3, 1);
        for _ in 0..3 {
            limiter.check_sync("k").unwrap();
        }
        assert!(matches!(
            limiter.check_sync("k"),
            Err(RiskEngineError::RateLimited(_))
        ));
    }

    #[test]
    fn refill_after_wait() {
        let limiter = InMemoryRateLimiter::new(1, 10); // 10 tokens/sec
        limiter.check_sync("k").unwrap();
        assert!(limiter.check_sync("k").is_err());
        sleep(Duration::from_millis(200)); // >= 1 token refilled
        limiter.check_sync("k").unwrap();
    }

    #[test]
    fn keys_are_isolated() {
        let limiter = InMemoryRateLimiter::new(1, 1);
        limiter.check_sync("a").unwrap();
        limiter.check_sync("b").unwrap();
        assert!(limiter.check_sync("a").is_err());
        assert!(limiter.check_sync("b").is_err());
    }

    #[test]
    fn reset_restores_bucket() {
        let limiter = InMemoryRateLimiter::new(1, 1);
        limiter.check_sync("k").unwrap();
        assert!(limiter.check_sync("k").is_err());
        limiter.reset("k");
        limiter.check_sync("k").unwrap();
    }

    // ── Fail-mode tests ──────────────────────────────────────────────────────

    /// Limiter that always surfaces `Timeout` — simulates a dead Redis.
    struct BrokenLimiter;
    #[async_trait]
    impl RateLimiter for BrokenLimiter {
        async fn check(&self, _key: &str) -> Result<(), RiskEngineError> {
            Err(RiskEngineError::Timeout { ms: 50 })
        }
    }

    /// Limiter that always rejects with RateLimited — real exhaustion.
    struct AlwaysExhausted;
    #[async_trait]
    impl RateLimiter for AlwaysExhausted {
        async fn check(&self, _key: &str) -> Result<(), RiskEngineError> {
            Err(RiskEngineError::RateLimited("test".into()))
        }
    }

    /// Minimal `SignalStore` noop for the wrapper tests — returns blank state.
    struct NoopStore;
    #[async_trait]
    impl crate::store::SignalStore for NoopStore {
        async fn get_velocity_counters(
            &self,
            _u: uuid::Uuid,
            _ip: Option<IpAddr>,
        ) -> Result<crate::store::VelocityCounters, RiskEngineError> {
            Ok(Default::default())
        }
        async fn get_recent_decisions(
            &self,
            _u: uuid::Uuid,
        ) -> Result<Vec<crate::store::EscalationEntry>, RiskEngineError> {
            Ok(vec![])
        }
        async fn record_decision(
            &self,
            _u: uuid::Uuid,
            _e: crate::store::EscalationEntry,
        ) -> Result<(), RiskEngineError> {
            Ok(())
        }
        async fn check_and_record_escalation(
            &self,
            _u: uuid::Uuid,
            d: &crate::decision::Decision,
            _s: u8,
            _t: i64,
        ) -> Result<(crate::decision::Decision, bool), RiskEngineError> {
            Ok((d.clone(), false))
        }
        async fn lock_account(&self, _u: uuid::Uuid, _t: u64) -> Result<(), RiskEngineError> {
            Ok(())
        }
        async fn block_ip(&self, _i: IpAddr, _t: u64) -> Result<(), RiskEngineError> {
            Ok(())
        }
        async fn get_jwt_fingerprint(
            &self,
            _k: &str,
        ) -> Result<Option<String>, RiskEngineError> {
            Ok(None)
        }
        async fn set_jwt_fingerprint(
            &self,
            _k: &str,
            _v: &str,
            _t: u64,
        ) -> Result<(), RiskEngineError> {
            Ok(())
        }
        async fn is_nonce_used(&self, _n: &str) -> Result<bool, RiskEngineError> {
            Ok(false)
        }
        async fn consume_nonce(&self, _n: &str) -> Result<(), RiskEngineError> {
            Ok(())
        }
    }

    fn fail_mode_ctx() -> RiskContext {
        use crate::context::{CredentialStatus, OrgRiskLevel, RiskAction};
        use chrono::Utc;
        use std::sync::Arc;
        RiskContext {
            request_id: Uuid::new_v4(),
            evaluated_at: Utc::now(),
            user_id: Uuid::new_v4(),
            username: "x".into(),
            credential_id: Some(Uuid::new_v4()),
            action: RiskAction::Login,
            resource_id: None,
            credential_status: CredentialStatus::Active,
            credential_created_at: Utc::now(),
            credential_last_used_at: None,
            credential_sign_count_prev: 0,
            credential_sign_count_new: 0,
            credential_registered_ua: None,
            credential_count_for_user: 1,
            prior_audit_event_count: 0,
            last_login_ip: None,
            last_login_at: None,
            last_login_geo: None,
            jwt_issued_at: None,
            jwt_expires_at: None,
            jwt_fingerprint_stored: None,
            jwt_fingerprint_current: None,
            request_ip: Some("203.0.113.1".parse().unwrap()),
            request_ua: None,
            request_geo: None,
            ip_asn_type: None,
            login_attempts_5m: None,
            failed_login_attempts_1h: None,
            actions_executed_5m: None,
            recovery_requests_24h: None,
            device_revocations_1h: None,
            registrations_from_ip_10m: None,
            active_session_count: None,
            account_locked: false,
            recovery_pending: false,
            oauth_authorize_ip: None,
            nonce_present: false,
            nonce_already_used: false,
            org_risk_level: OrgRiskLevel::Normal,
            redis_signals_degraded: false,
            db_signals_degraded: false,
            tenant_id: "t".into(),
            org_risk_score: None,
            cluster_membership: None,
            threshold_shift: None,
            org_active_cluster_count: 0,
            login_attempts_1m: None,
            login_attempts_1h: None,
            login_attempts_24h: None,
            client_timestamp: None,
            device_fingerprint_hash: None,
            ja3_fingerprint: None,
            known_ip_for_user: None,
            ip_is_vpn: None,
            ip_is_proxy: None,
            ip_is_relay: None,
            ip_abuse_confidence: None,
            geo_allowed_countries: None,
            is_sanctioned_country: None,
            webdriver_detected: None,
            captcha_score: None,
            screen_resolution: None,
            touch_capable: None,
            account_age_days: None,
            email_verified: None,
            email_domain_disposable: None,
            breached_credential: None,
            user_typical_hours: None,
            accept_language: None,
            previous_accept_language: None,
            device_trust_level: None,
            policy: Arc::new(crate::PolicyConfig::default()),
        }
    }

    #[tokio::test]
    async fn strict_wrapper_propagates_redis_timeout() {
        let ctx = fail_mode_ctx();
        let res = evaluate_with_rate_limit(ctx, &NoopStore, &BrokenLimiter).await;
        assert!(matches!(res, Err(RiskEngineError::Timeout { .. })));
    }

    #[tokio::test]
    async fn fail_open_wrapper_admits_on_redis_timeout() {
        let ctx = fail_mode_ctx();
        let res = evaluate_with_rate_limit_fail_open(ctx, &NoopStore, &BrokenLimiter).await;
        // A dead limiter must NOT block the request — the engine should run.
        assert!(res.is_ok(), "fail-open must admit when limiter is broken");
    }

    #[tokio::test]
    async fn fail_open_wrapper_still_blocks_on_explicit_ratelimit() {
        let ctx = fail_mode_ctx();
        let res = evaluate_with_rate_limit_fail_open(ctx, &NoopStore, &AlwaysExhausted).await;
        // An actual RateLimited response must still block, even in fail-open.
        assert!(matches!(res, Err(RiskEngineError::RateLimited(_))));
    }
}
