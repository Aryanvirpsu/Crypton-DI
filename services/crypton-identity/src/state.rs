use crate::config::Config;
use sqlx::PgPool;
use url::Url;
use webauthn_rs::prelude::*;

#[derive(Clone)]
pub struct AppState {
    pub db: Option<PgPool>,
    pub redis: Option<::redis::Client>,
    pub webauthn: Webauthn,
    pub jwt_secret: String,
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
