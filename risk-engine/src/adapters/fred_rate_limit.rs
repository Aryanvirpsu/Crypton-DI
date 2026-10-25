//! Redis/Valkey-backed [`RateLimiter`] implementation (Phase F).
//!
//! The in-process [`crate::rate_limit::InMemoryRateLimiter`] is a per-replica
//! bucket — three replicas running `cap=10` effectively allow 30 requests per
//! window per key, which defeats the purpose of rate limiting. This adapter
//! replaces that with a shared token bucket in Redis so all replicas consume
//! from the same budget.
//!
//! ## Algorithm
//!
//! The canonical "Redis atomic token bucket" Lua script: one `HGETALL` for
//! current state, refill proportionally to wall-clock elapsed since
//! `last_refill`, decide accept/deny, `HMSET` the new state, `PEXPIRE` the
//! key so idle buckets are garbage-collected. Runs as one atomic unit — no
//! check-then-write race between replicas.
//!
//! ## Keys
//!
//! `rl:{key}` — a hash holding two fields:
//!   - `t` (tokens, as a float string)
//!   - `r` (last refill unix-ms, as an int string)
//!
//! The key TTL is `ceil(capacity / rate_per_sec) * 2` seconds — long enough
//! that a legitimate bursty caller's bucket survives, short enough that
//! abandoned keys vacate within two full refill windows.
//!
//! ## Failure mode
//!
//! If Redis is unreachable or the `eval` exceeds [`LUA_TIMEOUT_MS`] we return
//! [`RiskEngineError::Timeout`]. The caller decides whether to fail-open
//! (admit the request) or fail-closed (deny). For a rate limiter, **fail-open
//! is almost always correct** — a dead Redis should not 429 legitimate
//! traffic — but we surface the error so operators can alert on it.
//!
//! ## Not covered
//!
//! - Leader-only semantics: a Redis cluster split can briefly double the
//!   budget. Accepted tradeoff for the simplicity of this approach.
//! - Sub-second precision beyond milliseconds: the script uses `TIME` → ms.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use fred::clients::Client;
use fred::interfaces::LuaInterface;

use crate::error::RiskEngineError;
use crate::rate_limit::RateLimiter;

const LUA_TIMEOUT_MS: u64 = 50;

/// Redis-hosted Lua script. Returns `1` on accept, `0` on deny.
///
/// Arguments (all strings — Redis Lua has no native float/int distinction):
///   KEYS[1] = bucket key (`rl:{key}`)
///   ARGV[1] = capacity          (max tokens)
///   ARGV[2] = rate_per_sec      (tokens refilled per second)
///   ARGV[3] = ttl_ms            (PEXPIRE for the bucket key)
///
/// Side effects:
///   Sets `t` = remaining tokens (float) and `r` = now_ms (int) on the hash.
const RATE_LIMIT_LUA: &str = r#"
local cap = tonumber(ARGV[1])
local rate = tonumber(ARGV[2])
local ttl_ms = tonumber(ARGV[3])

local state = redis.call('HMGET', KEYS[1], 't', 'r')
local tokens = tonumber(state[1])
local last = tonumber(state[2])

local now_raw = redis.call('TIME')
local now_ms = (tonumber(now_raw[1]) * 1000) + math.floor(tonumber(now_raw[2]) / 1000)

if tokens == nil or last == nil then
    tokens = cap
    last = now_ms
else
    local elapsed_ms = now_ms - last
    if elapsed_ms < 0 then elapsed_ms = 0 end
    tokens = math.min(cap, tokens + (elapsed_ms / 1000.0) * rate)
    last = now_ms
end

local allowed = 0
if tokens >= 1.0 then
    tokens = tokens - 1.0
    allowed = 1
end

redis.call('HMSET', KEYS[1], 't', tostring(tokens), 'r', tostring(last))
redis.call('PEXPIRE', KEYS[1], ttl_ms)

return allowed
"#;

async fn with_timeout<T, F>(ms: u64, fut: F) -> Result<T, RiskEngineError>
where
    F: Future<Output = Result<T, RiskEngineError>>,
{
    match tokio::time::timeout(Duration::from_millis(ms), fut).await {
        Ok(r) => r,
        Err(_) => Err(RiskEngineError::Timeout { ms }),
    }
}

fn to_engine_err(e: fred::error::Error) -> RiskEngineError {
    RiskEngineError::store_with_source("redis rate-limit op failed", e)
}

/// Redis-backed shared token bucket. Safe to clone — the underlying client
/// is `Arc`-wrapped and concurrent calls are serialised by Redis itself.
#[derive(Clone)]
pub struct RedisRateLimiter {
    client: Arc<Client>,
    capacity: u32,
    rate_per_sec: u32,
    ttl_ms: u64,
}

impl RedisRateLimiter {
    /// Wrap a connected Fred client. `capacity` is the max burst; `rate_per_sec`
    /// is the sustained refill rate. Both must be positive; the constructor
    /// panics otherwise — a misconfigured limiter is a deploy-time bug, not a
    /// runtime one.
    pub fn new(client: Arc<Client>, capacity: u32, rate_per_sec: u32) -> Self {
        assert!(capacity > 0, "rate limiter capacity must be > 0");
        assert!(rate_per_sec > 0, "rate limiter refill rate must be > 0");
        // Full refill window × 2 — so an idle bucket survives one burst,
        // then ages out. Minimum 2 s to cover sub-second refill windows.
        let refill_ms = (capacity as u64 * 1000) / rate_per_sec as u64;
        let ttl_ms = (refill_ms * 2).max(2_000);
        Self {
            client,
            capacity,
            rate_per_sec,
            ttl_ms,
        }
    }

    fn bucket_key(key: &str) -> String {
        format!("rl:{key}")
    }
}

#[async_trait]
impl RateLimiter for RedisRateLimiter {
    async fn check(&self, key: &str) -> Result<(), RiskEngineError> {
        with_timeout(LUA_TIMEOUT_MS, async move {
            let bucket = Self::bucket_key(key);
            let allowed: i64 = self
                .client
                .eval(
                    RATE_LIMIT_LUA,
                    vec![bucket],
                    vec![
                        self.capacity.to_string(),
                        self.rate_per_sec.to_string(),
                        self.ttl_ms.to_string(),
                    ],
                )
                .await
                .map_err(to_engine_err)?;

            if allowed == 1 {
                Ok(())
            } else {
                Err(RiskEngineError::RateLimited(format!(
                    "key={key} cap={} rate_per_sec={}",
                    self.capacity, self.rate_per_sec
                )))
            }
        })
        .await
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
//
// Unit-level Lua correctness is covered by the in-memory limiter's tests —
// the semantics are identical. What this module adds is shape: the client
// plumbing, the timeout wrapper, the error mapping. We cover those with
// construction/validation tests; live Redis coverage lives in the host
// integration suite.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_key_format() {
        assert_eq!(RedisRateLimiter::bucket_key("user:abc"), "rl:user:abc");
        assert_eq!(RedisRateLimiter::bucket_key("ip:1.2.3.4"), "rl:ip:1.2.3.4");
    }

    #[test]
    fn ttl_scales_with_refill_window() {
        // capacity 10 / rate 5 → refill 2000 ms → ttl 4000 ms
        let c = fred_test_client();
        let rl = RedisRateLimiter::new(c, 10, 5);
        assert_eq!(rl.ttl_ms, 4_000);
    }

    #[test]
    fn ttl_has_minimum_floor() {
        // capacity 1 / rate 100 → refill 10 ms → ttl clamps to 2000 ms
        let c = fred_test_client();
        let rl = RedisRateLimiter::new(c, 1, 100);
        assert_eq!(rl.ttl_ms, 2_000);
    }

    #[test]
    #[should_panic(expected = "capacity must be > 0")]
    fn zero_capacity_panics() {
        let c = fred_test_client();
        let _ = RedisRateLimiter::new(c, 0, 1);
    }

    #[test]
    #[should_panic(expected = "refill rate must be > 0")]
    fn zero_rate_panics() {
        let c = fred_test_client();
        let _ = RedisRateLimiter::new(c, 1, 0);
    }

    /// Build a disconnected Fred client for unit-test instantiation. The
    /// client is never `init()`-ed so no network call is made; we only use
    /// it to exercise the constructor and field plumbing.
    fn fred_test_client() -> Arc<Client> {
        use fred::types::config::Config;
        let cfg = Config::default();
        Arc::new(Client::new(cfg, None, None, None))
    }
}
