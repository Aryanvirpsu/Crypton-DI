use crate::config::Config;
use jsonwebtoken::{DecodingKey, EncodingKey};
use sqlx::PgPool;
use url::Url;
use uuid::Uuid;
use webauthn_rs::prelude::{PasskeyAuthentication, PasskeyRegistration, Webauthn, WebauthnBuilder};

// ── Challenge state types (serialised to Redis) ──────────────────────────────
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct RegChallenge {
    pub user_id: Uuid,
    pub username: String,
    pub reg_state: PasskeyRegistration,
    #[serde(default)]
    pub existing_user: bool,
    #[serde(default)]
    pub enrollment_token: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct AuthChallenge {
    pub user_id: Uuid,
    pub username: String,
    pub auth_state: PasskeyAuthentication,
}

/// TTL for pending challenges (seconds).
pub const CHALLENGE_TTL_SECS: u64 = 300;

#[derive(Clone)]
pub struct AppState {
    pub db: Option<PgPool>,
    pub redis: Option<::redis::Client>,
    pub webauthn: Webauthn,
    // Asymmetric (Ed25519/EdDSA) — see jwt.rs and DECISIONS.md. Parsed once
    // at startup, not per-request. crypton-id is the only service that ever
    // constructs an EncodingKey (it is the signing authority); every
    // verifier elsewhere only ever needs jwt_decoding_key's public half —
    // see labs/agent-lab/src/agentlab/identity.py for the client-side
    // mirror of this same split.
    pub jwt_encoding_key: EncodingKey,
    pub jwt_decoding_key: DecodingKey,
    pub cors_origin: String,
    // Note: Challenges are now strictly stored in Redis.
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

        let jwt_encoding_key = EncodingKey::from_ed_pem(cfg.jwt_signing_key_pem.as_bytes())
            .map_err(|e| {
                anyhow::anyhow!("JWT_SIGNING_KEY_PEM is not a valid Ed25519 PKCS8 PEM key: {e}")
            })?;
        let jwt_decoding_key = DecodingKey::from_ed_pem(cfg.jwt_public_key_pem.as_bytes())
            .map_err(|e| {
                anyhow::anyhow!("JWT_PUBLIC_KEY_PEM is not a valid Ed25519 SPKI PEM key: {e}")
            })?;

        Ok(Self {
            db,
            redis,
            webauthn,
            jwt_encoding_key,
            jwt_decoding_key,
            cors_origin: cfg.webauthn_origin.clone(),
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
