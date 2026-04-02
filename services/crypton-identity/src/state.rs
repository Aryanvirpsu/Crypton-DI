use crate::config::Config;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use url::Url;
use uuid::Uuid;
use webauthn_rs::prelude::{PasskeyAuthentication, PasskeyRegistration, Webauthn, WebauthnBuilder};

// ── In-memory challenge state types ──────────────────────────────────────────
// These are stored directly (no serialisation needed — no danger-allow-state-serialisation).

pub struct RegChallenge {
    pub user_id: Uuid,
    pub username: String,
    pub reg_state: PasskeyRegistration,
}

pub struct AuthChallenge {
    pub user_id: Uuid,
    pub username: String,
    pub auth_state: PasskeyAuthentication,
}

/// TTL for pending challenges (seconds). Must match the value used when inserting.
pub const CHALLENGE_TTL_SECS: u64 = 300;

pub type RegChallengeStore = Arc<Mutex<HashMap<String, (RegChallenge, Instant)>>>;
pub type AuthChallengeStore = Arc<Mutex<HashMap<String, (AuthChallenge, Instant)>>>;

#[derive(Clone)]
pub struct AppState {
    pub db: Option<PgPool>,
    pub redis: Option<::redis::Client>,
    pub webauthn: Webauthn,
    pub jwt_secret: String,
    pub cors_origin: String,
    /// In-memory store for pending registration challenges.
    pub reg_challenges: RegChallengeStore,
    /// In-memory store for pending authentication challenges.
    pub auth_challenges: AuthChallengeStore,
}

impl AppState {
    pub async fn new(cfg: &Config) -> anyhow::Result<Self> {
        let db = if let Some(url) = cfg.database_url.as_deref() {
            Some(PgPool::connect(url).await?)
        } else {
            tracing::warn!("DATABASE_URL not set; running without Postgres");
            None
        };

        let redis = if let Some(url) = cfg.redis_url.as_deref() {
            Some(::redis::Client::open(url)?)
        } else {
            tracing::warn!("REDIS_URL not set; running without Redis");
            None
        };

        let rp_origin = Url::parse(&cfg.webauthn_origin)
            .map_err(|e| anyhow::anyhow!("Invalid WEBAUTHN_ORIGIN: {}", e))?;

        let webauthn = WebauthnBuilder::new(&cfg.webauthn_rp_id, &rp_origin)?
            .rp_name(&cfg.webauthn_rp_name)
            .build()?;

        Ok(Self {
            db,
            redis,
            webauthn,
            jwt_secret: cfg.jwt_secret.clone(),
            cors_origin: cfg.webauthn_origin.clone(),
            reg_challenges: Arc::new(Mutex::new(HashMap::new())),
            auth_challenges: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn redis_conn(&self) -> anyhow::Result<::redis::aio::MultiplexedConnection> {
        let client = self
            .redis
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("REDIS_URL not configured"))?;
        Ok(client.get_multiplexed_tokio_connection().await?)
    }
}
