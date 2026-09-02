use std::env;

/// Reads a PEM-valued env var one of two ways: `{name}_FILE`, if set, is a
/// path read from disk (real newlines, no escaping needed — the natural
/// shape for a key generated straight from `openssl genpkey`/`pkey`, and
/// the standard convention for mounting a secret as a file, e.g. a Docker
/// secret); otherwise falls back to `{name}` containing the PEM text
/// directly (dotenvy unescapes `\n` inside a double-quoted `.env` value —
/// see .env.example). Multi-line PEM content doesn't fit cleanly into a
/// single `KEY=VALUE` line without one of these two mechanisms.
fn read_pem_env(name: &str) -> anyhow::Result<String> {
    if let Ok(path) = env::var(format!("{name}_FILE")) {
        return std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("failed to read {name}_FILE ({path}): {e}"));
    }
    env::var(name).map_err(|_| anyhow::anyhow!("{name} (or {name}_FILE) is required"))
}

#[derive(Clone, Debug)]
pub struct Config {
    pub app_port: u16,
    pub database_url: Option<String>,
    pub redis_url: Option<String>,

    // WebAuthn – required for Week 3+
    pub webauthn_rp_id: String,
    pub webauthn_origin: String,
    pub webauthn_rp_name: String,

    // JWT – asymmetric (Ed25519/EdDSA) since v0.5.1, see DECISIONS.md
    // "JWT: asymmetric signing (Ed25519) replaces the shared HS256 secret".
    // Only crypton-id ever needs the private key; every verifier (the
    // gateway, Agent Lab, any future service) needs only jwt_public_key_pem.
    pub jwt_signing_key_pem: String,
    pub jwt_public_key_pem: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let app_port = env::var("APP_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()?;

        let database_url = Some(
            env::var("DATABASE_URL").map_err(|_| anyhow::anyhow!("DATABASE_URL is required"))?,
        );
        let redis_url =
            Some(env::var("REDIS_URL").map_err(|_| anyhow::anyhow!("REDIS_URL is required"))?);

        let webauthn_rp_id = env::var("WEBAUTHN_RP_ID")
            .map_err(|_| anyhow::anyhow!("WEBAUTHN_RP_ID is required"))?;

        let webauthn_origin = env::var("WEBAUTHN_ORIGIN")
            .map_err(|_| anyhow::anyhow!("WEBAUTHN_ORIGIN is required"))?;

        let webauthn_rp_name = env::var("WEBAUTHN_RP_NAME")
            .map_err(|_| anyhow::anyhow!("WEBAUTHN_RP_NAME is required"))?;

        // Generate a pair with:
        //   openssl genpkey -algorithm ed25519 -out jwt_signing_key.pem
        //   openssl pkey -in jwt_signing_key.pem -pubout -out jwt_public_key.pem
        let jwt_signing_key_pem = read_pem_env("JWT_SIGNING_KEY_PEM")?;
        let jwt_public_key_pem = read_pem_env("JWT_PUBLIC_KEY_PEM")?;

        Ok(Self {
            app_port,
            database_url,
            redis_url,
            webauthn_rp_id,
            webauthn_origin,
            webauthn_rp_name,
            jwt_signing_key_pem,
            jwt_public_key_pem,
        })
    }
}
