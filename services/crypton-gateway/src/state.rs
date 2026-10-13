use jsonwebtoken::DecodingKey;
use redis::aio::ConnectionManager;
use redis::Client as RedisClient;
use sqlx::PgPool;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Default)]
pub struct Metrics {
    pub auth_jwt_accepted: AtomicU64,
    pub auth_rejected: AtomicU64,

    pub rate_limit_block_ip: AtomicU64,
    pub rate_limit_block_user: AtomicU64,
    pub rate_limit_block_device: AtomicU64,

    pub nonce_replay_blocked: AtomicU64,
    pub nonce_timestamp_missing: AtomicU64,
    pub nonce_timestamp_invalid: AtomicU64,

    /// Requests the risk-engine policy actually blocked, by verdict.
    pub policy_blocked_deny: AtomicU64,
    pub policy_blocked_challenge: AtomicU64,
    pub policy_blocked_hold: AtomicU64,
    /// Verdicts that WOULD have blocked but POLICY_ENFORCE=false let through.
    pub policy_would_block_dry_run: AtomicU64,

    pub proxy_upstream_2xx: AtomicU64,
    pub proxy_upstream_4xx: AtomicU64,
    pub proxy_upstream_5xx: AtomicU64,
    pub proxy_upstream_error: AtomicU64,

    /// Decision-trail rows that could not be written. Allow decisions fail
    /// closed (503) when this happens; deny decisions are still enforced.
    pub audit_write_failed: AtomicU64,
    pub audit_decisions_allowed: AtomicU64,
    pub audit_decisions_denied: AtomicU64,
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
    /// Shared HTTP client for the upstream proxy.
    pub http_client: reqwest::Client,
    pub identity_base: String,
    pub metrics: Arc<Metrics>,
    /// When true (default), risk-engine DENY/CHALLENGE/HOLD actually block the
    /// request. Set POLICY_ENFORCE=false to run in dry-run mode.
    pub policy_enforce: bool,
    /// Public half of the `crypton-id` Ed25519 signing key. The gateway can
    /// verify identity tokens but never mint them.
    pub jwt_decoding_key: DecodingKey,
    // Security: INTERNAL_TOKEN is required at startup. Process exits if absent.
    pub internal_token: String,
    /// When set, added as `x-gateway-secret` to every request forwarded to
    /// identity (which refuses requests without it when it has the same secret).
    pub gateway_secret: Option<String>,
}

impl AppState {
    pub async fn new(redis_url: &str, pg_conn: &str) -> anyhow::Result<Self> {
        let jwt_decoding_key = load_decoding_key()?;

        let internal_token = std::env::var("INTERNAL_TOKEN").map_err(|_| {
            anyhow::anyhow!("INTERNAL_TOKEN is required — authenticates service-to-service calls")
        })?;

        let client = RedisClient::open(redis_url)?;
        // Bounded retries and timeouts: with Redis unreachable every Redis-backed
        // check (revocation, nonce, rate limit, policy) must fail fast and closed,
        // not queue behind default exponential backoff.
        let conn = redis::aio::ConnectionManager::new_with_config(
            client,
            redis::aio::ConnectionManagerConfig::new()
                .set_number_of_retries(1)
                .set_connection_timeout(std::time::Duration::from_secs(2))
                .set_response_timeout(std::time::Duration::from_secs(2)),
        )
        .await?;

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
            jwt_decoding_key,
            internal_token,
            gateway_secret: std::env::var("GATEWAY_SHARED_SECRET")
                .ok()
                .filter(|s| !s.is_empty()),
        })
    }
}

/// Read the identity service's Ed25519 PUBLIC key (SPKI PEM), same convention
/// as `crypton-id`: `JWT_PUBLIC_KEY_PEM_FILE` (a path) wins over
/// `JWT_PUBLIC_KEY_PEM` (inline, newline-escaped in `.env`). Startup aborts if it
/// is missing or not a valid Ed25519 public key.
fn load_decoding_key() -> anyhow::Result<DecodingKey> {
    let pem = match std::env::var("JWT_PUBLIC_KEY_PEM_FILE") {
        Ok(path) => std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("failed to read JWT_PUBLIC_KEY_PEM_FILE ({path}): {e}"))?,
        Err(_) => std::env::var("JWT_PUBLIC_KEY_PEM")
            .map_err(|_| anyhow::anyhow!("JWT_PUBLIC_KEY_PEM (or JWT_PUBLIC_KEY_PEM_FILE) is required"))?,
    };
    DecodingKey::from_ed_pem(pem.as_bytes())
        .map_err(|e| anyhow::anyhow!("JWT public key is not a valid Ed25519 SPKI PEM key: {e}"))
}
