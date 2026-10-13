//! Security signals identity produces for the gateway's risk policy.
//!
//! Ownership: `crypton-id` is the authority for authentication outcomes, so it is
//! the PRODUCER of every signal below, written directly to the Redis it already
//! shares with the gateway (the same way it already writes the logout
//! denylist `jwt:deny:*`). The gateway CONSUMES them (risk engine counters,
//! lock, recovery flags) and enforces on authenticated traffic; identity itself
//! enforces the account lock at `login/start`, because the public login routes
//! carry no token for the gateway's policy to evaluate.
//!
//! Key contract (must match `crypton-gateway/src/middleware/risk_store.rs` and
//! `crypton-gateway/src/audit/ingest.rs`):
//!
//! | key                                  | written on                          | TTL    |
//! |--------------------------------------|-------------------------------------|--------|
//! | `login:failed:{user}`                | failed login/step-up assertion      | 300 s  |
//! | `login:locked:{user}`                | > 5 failures inside that window     | 600 s  |
//! | `velocity:failed:{user}:1h`          | failed assertion                    | 3600 s |
//! | `velocity:login:{user}:{1m,5m,1h,24h}` | `login/start` for a known user    | window |
//! | `velocity:revocations:{user}:1h`     | device revoke / mark-lost           | 3600 s |
//! | `velocity:recovery:{user}:24h`       | recovery started                    | 86400 s|
//! | `recovery:pending:{user}`            | recovery started (del on completed) | 86400 s|
//! | `device:added:{credential uuid}`     | additional device registered        | 600 s  |
//! | `stepup:completed:{user}:{iat}`      | server-verified WebAuthn step-up    | 300 s  |
//!
//! Writes are single atomic Lua scripts. Failures to RECORD a signal are logged
//! and never fail the user's request (the audit row in `audit_logs` is the
//! durable record); a failure to READ the lock at `login/start` does fail the
//! request, since login already needs Redis for its challenge.

use redis::aio::MultiplexedConnection;
use uuid::Uuid;

use crate::{error::AppError, state::AppState};

/// Failures within `FAILED_WINDOW_SECS` that lock the account for `LOCK_SECS`.
pub const LOCK_AFTER_FAILURES: i64 = 5;
const FAILED_WINDOW_SECS: u32 = 300;
const LOCK_SECS: u32 = 600;
pub const STEP_UP_TTL_SECS: u64 = 300;

// KEYS: failed, locked, velocity-failed-1h
const FAILED_LUA: &str = "
local c = redis.call('INCR', KEYS[1])
if c == 1 then redis.call('EXPIRE', KEYS[1], tonumber(ARGV[1])) end
if c > tonumber(ARGV[3]) then redis.call('SET', KEYS[2], 1, 'EX', tonumber(ARGV[2])) end
local v = redis.call('INCR', KEYS[3])
if v == 1 then redis.call('EXPIRE', KEYS[3], 3600) end
return c
";

// KEYS: one counter per window; ARGV: ttl per key. Returns nothing useful.
const COUNTERS_LUA: &str = "
for i = 1, #KEYS do
  local v = redis.call('INCR', KEYS[i])
  if v == 1 then redis.call('EXPIRE', KEYS[i], tonumber(ARGV[i])) end
end
return 1
";

// KEYS: pending flag, 24h velocity counter.
const RECOVERY_LUA: &str = "
redis.call('SET', KEYS[1], 1, 'EX', 86400)
local v = redis.call('INCR', KEYS[2])
if v == 1 then redis.call('EXPIRE', KEYS[2], 86400) end
return v
";

pub fn locked_key(user: Uuid) -> String {
    format!("login:locked:{user}")
}

pub fn step_up_key(user: &str, iat: u64) -> String {
    format!("stepup:completed:{user}:{iat}")
}

async fn conn(state: &AppState) -> Option<MultiplexedConnection> {
    state.redis_conn().await.ok()
}

fn log_failure(what: &str, e: redis::RedisError) {
    tracing::error!(signal = what, error = %e, "failed to record security signal");
}

/// Refuse login while the account is locked. Fails closed on Redis errors.
pub async fn ensure_not_locked(state: &AppState, user: Uuid) -> Result<(), AppError> {
    let mut c = state
        .redis_conn()
        .await
        .map_err(|e| AppError::internal(format!("redis_unavailable: {e}")))?;
    let locked: bool = redis::cmd("EXISTS")
        .arg(locked_key(user))
        .query_async(&mut c)
        .await
        .map_err(|e| AppError::internal(format!("redis_unavailable: {e}")))?;
    if locked {
        tracing::warn!(user_id = %user, "login refused: account_locked");
        return Err(AppError::too_many_requests("account_locked"));
    }
    Ok(())
}

/// A login attempt began for a known user (feeds the risk engine's velocity).
pub async fn record_login_attempt(state: &AppState, user: Uuid) {
    let Some(mut c) = conn(state).await else {
        return;
    };
    let keys = [
        (format!("velocity:login:{user}:1m"), 60u32),
        (format!("velocity:login:{user}:5m"), 300),
        (format!("velocity:login:{user}:1h"), 3600),
        (format!("velocity:login:{user}:24h"), 86400),
    ];
    let script = redis::Script::new(COUNTERS_LUA);
    let mut inv = script.prepare_invoke();
    for (k, ttl) in &keys {
        inv.key(k.as_str());
        inv.arg(*ttl);
    }
    if let Err(e) = inv.invoke_async::<i64>(&mut c).await {
        log_failure("login_attempt", e);
    }
}

/// A WebAuthn assertion for `user` failed verification (login or step-up).
pub async fn record_failed_assertion(state: &AppState, user: Uuid) {
    let Some(mut c) = conn(state).await else {
        return;
    };
    let r: Result<i64, _> = redis::Script::new(FAILED_LUA)
        .key(format!("login:failed:{user}"))
        .key(locked_key(user))
        .key(format!("velocity:failed:{user}:1h"))
        .arg(FAILED_WINDOW_SECS)
        .arg(LOCK_SECS)
        .arg(LOCK_AFTER_FAILURES)
        .invoke_async(&mut c)
        .await;
    if let Err(e) = r {
        log_failure("failed_assertion", e);
    }
}

/// Successful login: clear the failure counters (never a held lock's expiry
/// shortcut: a locked account cannot reach this point, `login/start` refuses).
pub async fn record_login_success(state: &AppState, user: Uuid) {
    let Some(mut c) = conn(state).await else {
        return;
    };
    let r: Result<(), _> = redis::pipe()
        .atomic()
        .del(format!("login:failed:{user}"))
        .del(locked_key(user))
        .query_async(&mut c)
        .await;
    if let Err(e) = r {
        log_failure("login_success", e);
    }
}

/// A credential was revoked or marked lost.
pub async fn record_revocation(state: &AppState, user: Uuid) {
    let Some(mut c) = conn(state).await else {
        return;
    };
    let r: Result<i64, _> = redis::Script::new(COUNTERS_LUA)
        .key(format!("velocity:revocations:{user}:1h"))
        .arg(3600u32)
        .invoke_async(&mut c)
        .await;
    if let Err(e) = r {
        log_failure("revocation", e);
    }
}

pub async fn record_recovery_started(state: &AppState, user: Uuid) {
    let Some(mut c) = conn(state).await else {
        return;
    };
    let r: Result<i64, _> = redis::Script::new(RECOVERY_LUA)
        .key(format!("recovery:pending:{user}"))
        .key(format!("velocity:recovery:{user}:24h"))
        .invoke_async(&mut c)
        .await;
    if let Err(e) = r {
        log_failure("recovery_started", e);
    }
}

pub async fn record_recovery_completed(state: &AppState, user: Uuid) {
    let Some(mut c) = conn(state).await else {
        return;
    };
    let r: Result<(), _> = redis::cmd("DEL")
        .arg(format!("recovery:pending:{user}"))
        .query_async(&mut c)
        .await;
    if let Err(e) = r {
        log_failure("recovery_completed", e);
    }
}

/// An additional device was enrolled for an existing user.
pub async fn record_device_added(state: &AppState, credential: Uuid) {
    let Some(mut c) = conn(state).await else {
        return;
    };
    let r: Result<(), _> = redis::cmd("SET")
        .arg(format!("device:added:{credential}"))
        .arg(1)
        .arg("EX")
        .arg(600u32)
        .query_async(&mut c)
        .await;
    if let Err(e) = r {
        log_failure("device_added", e);
    }
}

/// Server-side proof that `user` just completed a WebAuthn step-up for the
/// session token issued at `iat`. Only called after
/// `finish_passkey_authentication` succeeded: never from a client-supplied flag.
pub async fn record_step_up_completed(
    state: &AppState,
    user: Uuid,
    iat: u64,
) -> Result<(), AppError> {
    let mut c = state
        .redis_conn()
        .await
        .map_err(|e| AppError::internal(format!("redis_unavailable: {e}")))?;
    redis::cmd("SET")
        .arg(step_up_key(&user.to_string(), iat))
        .arg(1)
        .arg("EX")
        .arg(STEP_UP_TTL_SECS)
        .query_async::<()>(&mut c)
        .await
        .map_err(|e| AppError::internal(format!("redis_unavailable: {e}")))
}
