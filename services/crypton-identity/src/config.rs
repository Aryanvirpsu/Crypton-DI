use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub app_port: u16,
    pub database_url: Option<String>,
    pub redis_url: Option<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let app_port = env::var("APP_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()?;

        let database_url = env::var("DATABASE_URL").ok();
        let redis_url = env::var("REDIS_URL").ok();

        Ok(Self {
            app_port,
            database_url,
            redis_url,
        })
    }
}
