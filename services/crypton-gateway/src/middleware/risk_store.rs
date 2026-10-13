use std::net::IpAddr;
use std::sync::Arc;

use async_trait::async_trait;
use redis::{aio::ConnectionManager, AsyncCommands};
use uuid::Uuid;

use risk_engine::{
    engine::escalation::{check_escalation, EscalationOutcome},
    store::{EscalationEntry, SignalStore, VelocityCounters, NONCE_TTL_SECONDS},
    Decision, RiskEngineError,
};

pub struct GatewaySignalStore {
    pub redis: Arc<ConnectionManager>,
}

impl GatewaySignalStore {
    pub fn new(redis: Arc<ConnectionManager>) -> Self {
        Self { redis }
    }
}

#[async_trait]
impl SignalStore for GatewaySignalStore {
    async fn get_velocity_counters(
        &self,
        user_id: Uuid,
        request_ip: Option<IpAddr>,
    ) -> Result<VelocityCounters, RiskEngineError> {
        let mut conn = self.redis.as_ref().clone();

        let (
            login_5m,
            login_1m,
            login_1h,
            login_24h,
            failed_1h,
            actions_5m,
            recovery_24h,
            revocations_1h,
            active_sessions,
            locked,
            recovery_pending,
        ): (
            Option<u32>,
            Option<u32>,
            Option<u32>,
            Option<u32>,
            Option<u32>,
            Option<u32>,
            Option<u32>,
            Option<u32>,
            Option<u32>,
            bool,
            bool,
        ) = redis::pipe()
            .get(format!("velocity:login:{user_id}:5m"))
            .get(format!("velocity:login:{user_id}:1m"))
            .get(format!("velocity:login:{user_id}:1h"))
            .get(format!("velocity:login:{user_id}:24h"))
            .get(format!("velocity:failed:{user_id}:1h"))
            .get(format!("velocity:actions:{user_id}:5m"))
            .get(format!("velocity:recovery:{user_id}:24h"))
            .get(format!("velocity:revocations:{user_id}:1h"))
            .get(format!("sessions:active:{user_id}"))
            .exists(format!("login:locked:{user_id}"))
            .exists(format!("recovery:pending:{user_id}"))
            .query_async(&mut conn)
            .await
            .map_err(|e| RiskEngineError::StoreError(e.to_string()))?;

        let registrations_from_ip_10m = if let Some(ip) = request_ip {
            let mut c2 = self.redis.as_ref().clone();
            c2.get::<_, Option<u32>>(format!("velocity:registrations:{ip}:10m"))
                .await
                .unwrap_or(None)
        } else {
            None
        };

        Ok(VelocityCounters {
            login_attempts_5m: login_5m,
            login_attempts_1m: login_1m,
            login_attempts_1h: login_1h,
            login_attempts_24h: login_24h,
            failed_login_attempts_1h: failed_1h,
            actions_executed_5m: actions_5m,
            recovery_requests_24h: recovery_24h,
            device_revocations_1h: revocations_1h,
            active_session_count: active_sessions,
            account_locked: locked,
            recovery_pending,
            registrations_from_ip_10m,
        })
    }

    async fn get_recent_decisions(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<EscalationEntry>, RiskEngineError> {
        let mut conn = self.redis.as_ref().clone();
        let raw: Vec<String> = conn
            .lrange(format!("risk:recent:{user_id}"), 0, 9)
            .await
            .map_err(|e| RiskEngineError::StoreError(e.to_string()))?;
        Ok(raw
            .into_iter()
            .filter_map(|s| serde_json::from_str(&s).ok())
            .collect())
    }

    async fn record_decision(
        &self,
        user_id: Uuid,
        entry: EscalationEntry,
    ) -> Result<(), RiskEngineError> {
        let mut conn = self.redis.as_ref().clone();
        let json = serde_json::to_string(&entry)
            .map_err(|e| RiskEngineError::StoreError(e.to_string()))?;
        let key = format!("risk:recent:{user_id}");
        redis::pipe()
            .lpush(&key, &json)
            .ltrim(&key, 0, 9)
            .expire(&key, 300)
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| RiskEngineError::StoreError(e.to_string()))
    }

    // Atomic snapshot → check escalation → push → trim via Lua.
    // Prevents TOCTOU: concurrent requests see each other's entries
    // rather than reading the same pre-burst snapshot independently.
    async fn check_and_record_escalation(
        &self,
        user_id: Uuid,
        current_decision: &Decision,
        score: u8,
        ts_unix: i64,
    ) -> Result<(Decision, bool), RiskEngineError> {
        // Read at most ARGV[2] entries (bounded). Under a concurrent burst two
        // requests can both LPUSH before either LTRIM runs, momentarily growing
        // the list past the cap; an unbounded `LRANGE 0 -1` would then surface a
        // stale entry the previous trim should have evicted, which can skew the
        // escalation window. Bounding the read to the window size avoids that.
        const LUA: &str = "
local snapshot = redis.call('LRANGE', KEYS[1], 0, tonumber(ARGV[2]) - 1)
redis.call('LPUSH', KEYS[1], ARGV[1])
redis.call('LTRIM', KEYS[1], 0, tonumber(ARGV[2]) - 1)
redis.call('EXPIRE', KEYS[1], tonumber(ARGV[3]))
return snapshot
";
        let entry = EscalationEntry {
            score,
            decision: current_decision.clone(),
            ts_unix,
        };
        let json = serde_json::to_string(&entry)
            .map_err(|e| RiskEngineError::StoreError(e.to_string()))?;
        let key = format!("risk:recent:{user_id}");
        let mut conn = self.redis.as_ref().clone();

        let raw: Vec<String> = redis::Script::new(LUA)
            .key(&key)
            .arg(&json)
            .arg(10_i64)
            .arg(300_i64)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| RiskEngineError::StoreError(e.to_string()))?;

        let prior: Vec<EscalationEntry> = raw
            .into_iter()
            .filter_map(|s| serde_json::from_str(&s).ok())
            .collect();

        // Window matches this store's Redis retention (LPUSH/EXPIRE 300s above)
        // and the risk-engine default; mirrors the fred_store adapter signature.
        let outcome = check_escalation(current_decision, &prior, ts_unix, 300);
        let (final_decision, was_escalated) = match outcome {
            EscalationOutcome::NoChange => (current_decision.clone(), false),
            EscalationOutcome::EscalateTo(d) => (d, true),
        };

        Ok((final_decision, was_escalated))
    }

    async fn lock_account(&self, user_id: Uuid, ttl_seconds: u64) -> Result<(), RiskEngineError> {
        let mut conn = self.redis.as_ref().clone();
        redis::cmd("SET")
            .arg(format!("login:locked:{user_id}"))
            .arg(1_i64)
            .arg("EX")
            .arg(ttl_seconds as i64)
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| RiskEngineError::StoreError(e.to_string()))
    }

    async fn block_ip(&self, ip: IpAddr, ttl_seconds: u64) -> Result<(), RiskEngineError> {
        let mut conn = self.redis.as_ref().clone();
        redis::cmd("SET")
            .arg(format!("ip:blocked:{ip}"))
            .arg(1_i64)
            .arg("EX")
            .arg(ttl_seconds as i64)
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| RiskEngineError::StoreError(e.to_string()))
    }

    async fn get_jwt_fingerprint(
        &self,
        fingerprint_key: &str,
    ) -> Result<Option<String>, RiskEngineError> {
        let mut conn = self.redis.as_ref().clone();
        conn.get::<_, Option<String>>(format!("jwt:fingerprint:{fingerprint_key}"))
            .await
            .map_err(|e| RiskEngineError::StoreError(e.to_string()))
    }

    async fn set_jwt_fingerprint(
        &self,
        fingerprint_key: &str,
        fingerprint_value: &str,
        ttl_seconds: u64,
    ) -> Result<(), RiskEngineError> {
        let mut conn = self.redis.as_ref().clone();
        redis::cmd("SET")
            .arg(format!("jwt:fingerprint:{fingerprint_key}"))
            .arg(fingerprint_value)
            .arg("EX")
            .arg(ttl_seconds as i64)
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| RiskEngineError::StoreError(e.to_string()))
    }

    async fn is_nonce_used(&self, nonce: &str) -> Result<bool, RiskEngineError> {
        let mut conn = self.redis.as_ref().clone();
        conn.exists::<_, bool>(format!("nonce:{nonce}"))
            .await
            .map_err(|e| RiskEngineError::StoreError(e.to_string()))
    }

    async fn consume_nonce(&self, nonce: &str) -> Result<(), RiskEngineError> {
        let mut conn = self.redis.as_ref().clone();
        redis::cmd("SET")
            .arg(format!("nonce:{nonce}"))
            .arg(1_i64)
            .arg("EX")
            .arg(NONCE_TTL_SECONDS as i64)
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| RiskEngineError::StoreError(e.to_string()))
    }
}
