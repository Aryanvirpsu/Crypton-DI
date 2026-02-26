use crate::config::Config;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub db: Option<PgPool>,
    pub redis: Option<::redis::Client>,
}

impl AppState {
    pub async fn new(cfg: &Config) -> anyhow::Result<Self> {
        let db = if let Some(url) = cfg.database_url.as_deref() {
            Some(PgPool::connect(url).await?)
        } else {
            tracing::warn!("DATABASE_URL not set; running without Postgres (Week 1)");
            None
        };

        let redis = if let Some(url) = cfg.redis_url.as_deref() {
            Some(::redis::Client::open(url)?)
        } else {
            tracing::warn!("REDIS_URL not set; running without Redis (Week 1)");
            None
        };

        Ok(Self { db, redis })
    }

    pub async fn redis_conn(&self) -> anyhow::Result<::redis::aio::MultiplexedConnection> {
        let client = self
            .redis
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("REDIS_URL not configured"))?;
        Ok(client.get_multiplexed_tokio_connection().await?)
    }
}
