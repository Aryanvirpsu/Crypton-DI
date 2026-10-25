//! Minimal stand-in for crypton-id's `AppState`: only the fields the included
//! token code reads (`db`, `redis_conn()`, `jwt_decoding_key`). The real one
//! also holds WebAuthn state, which is not needed here and pulls in OpenSSL.

use jsonwebtoken::DecodingKey;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub db: Option<PgPool>,
    pub redis: Option<::redis::Client>,
    pub jwt_decoding_key: DecodingKey,
}

impl AppState {
    pub fn new(db: PgPool, redis_url: &str, public_key_pem: &str) -> anyhow::Result<Self> {
        Ok(Self {
            db: Some(db),
            redis: Some(::redis::Client::open(redis_url)?),
            jwt_decoding_key: DecodingKey::from_ed_pem(public_key_pem.as_bytes())?,
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
