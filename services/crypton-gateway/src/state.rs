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

#[derive(Clone)]
pub struct AppState {
    pub redis: Arc<ConnectionManager>,
    pub pg: PgPool,
    /// Shared HTTP client for upstream proxy and opaque-token verification.
    pub http_client: reqwest::Client,
    pub identity_base: String,
    pub metrics: Arc<Metrics>,
    /// When true (default), risk-engine DENY/CHALLENGE/HOLD actually block the
    /// request. Set POLICY_ENFORCE=false to run in dry-run mode.
    pub policy_enforce: bool,
    // Security: both env vars are required at startup — process exits if absent.
    pub jwt_secret: String,
    pub internal_token: String,
}

impl AppState {
    pub async fn new(redis_url: &str, pg_conn: &str) -> anyhow::Result<Self> {
        let jwt_secret = std::env::var("JWT_SECRET")
            .map_err(|_| anyhow::anyhow!("JWT_SECRET is required but not set"))?;

        let internal_token = std::env::var("INTERNAL_TOKEN").map_err(|_| {
            anyhow::anyhow!("INTERNAL_TOKEN is required — authenticates service-to-service calls")
        })?;

        let client = RedisClient::open(redis_url)?;
        let conn = client.get_connection_manager().await?;

        let pg = PgPool::connect(pg_conn).await?;

        let http_client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        let identity_base = std::env::var("IDENTITY_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());

        let policy_enforce = std::env::var("POLICY_ENFORCE")
            .unwrap_or_else(|_| "true".into())
            .to_ascii_lowercase()
            != "false";

        Ok(AppState {
            redis: Arc::new(conn),
            pg,
            http_client,
            identity_base,
            metrics: Arc::new(Metrics::default()),
            policy_enforce,
            jwt_secret,
            internal_token,
        })
    }
}
