use sqlx::PgPool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: redis::Client,
}

impl AppState {
    pub async fn new(cfg: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        let db = PgPool::connect(&cfg.database_url).await?;
        let redis = redis::Client::open(cfg.redis_url.clone())?;

        Ok(Self { db, redis })
    }
}
