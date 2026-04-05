use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub app_port: u16,
    pub database_url: Option<String>,
    pub redis_url: Option<String>,

    // WebAuthn
    pub webauthn_rp_id: String,
    pub webauthn_origin: String,
    pub webauthn_rp_name: String,

    // Frontend / CORS
    pub cors_origin: String,

    // JWT
    pub jwt_secret: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let app_port = env::var("APP_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()?;

        let database_url = env::var("DATABASE_URL").ok();
        let redis_url = env::var("REDIS_URL").ok();

        let webauthn_rp_id = env::var("WEBAUTHN_RP_ID")
            .map_err(|_| anyhow::anyhow!("WEBAUTHN_RP_ID is required"))?;

        let webauthn_origin = env::var("WEBAUTHN_ORIGIN")
            .map_err(|_| anyhow::anyhow!("WEBAUTHN_ORIGIN is required"))?;

        let webauthn_rp_name = env::var("WEBAUTHN_RP_NAME")
            .map_err(|_| anyhow::anyhow!("WEBAUTHN_RP_NAME is required"))?;

        let cors_origin = env::var("CORS_ORIGIN")
            .map_err(|_| anyhow::anyhow!("CORS_ORIGIN is required"))?;

        let jwt_secret = env::var("JWT_SECRET")
            .map_err(|_| anyhow::anyhow!("JWT_SECRET is required"))?;

        Ok(Self {
            app_port,
            database_url,
            redis_url,
            webauthn_rp_id,
            webauthn_origin,
            webauthn_rp_name,
            cors_origin,
            jwt_secret,
        })
    }
}
