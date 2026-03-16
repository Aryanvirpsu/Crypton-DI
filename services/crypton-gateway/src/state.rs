use redis::aio::ConnectionManager;
use redis::Client as RedisClient;
use sqlx::PgPool;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Default)]
pub struct Metrics {
    pub auth_jwt_accepted: AtomicU64,
    pub auth_opaque_accepted: AtomicU64,
    pub auth_rejected: AtomicU64,

    pub rate_limit_block_ip: AtomicU64,
    pub rate_limit_block_user: AtomicU64,
    pub rate_limit_block_device: AtomicU64,

    pub nonce_replay_blocked: AtomicU64,
    pub nonce_timestamp_missing: AtomicU64,
    pub nonce_timestamp_invalid: AtomicU64,

    pub policy_blocked_locked: AtomicU64,
    pub policy_blocked_private_unauth: AtomicU64,
    pub policy_blocked_device_inactive: AtomicU64,
    pub policy_blocked_stepup_required: AtomicU64,
    pub policy_blocked_recovery_pending: AtomicU64,
    pub policy_would_block_dry_run: AtomicU64,

    pub proxy_upstream_2xx: AtomicU64,
    pub proxy_upstream_4xx: AtomicU64,
    pub proxy_upstream_5xx: AtomicU64,
    pub proxy_upstream_error: AtomicU64,
}

impl Metrics {
    #[inline]
    pub fn inc(c: &AtomicU64) {
        c.fetch_add(1, Ordering::Relaxed);
    }
}

// shared application state carried through requests
#[derive(Clone)]
pub struct AppState {
    pub redis: Arc<ConnectionManager>,
    pub pg: PgPool,
    pub metrics: Arc<Metrics>,
}

impl AppState {
    pub async fn new(redis_url: &str, pg_conn: &str) -> anyhow::Result<Self> {
        let client = RedisClient::open(redis_url)?;
        let conn = client.get_tokio_connection_manager().await?;

        let pg = PgPool::connect(pg_conn).await?;

        Ok(AppState {
            redis: Arc::new(conn),
            pg,
            metrics: Arc::new(Metrics::default()),
        })
    }
}