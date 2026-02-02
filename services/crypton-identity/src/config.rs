use std::env;

#[derive(Clone)]
pub struct Config {
    pub app_port: u16,
    pub database_url: String,
    pub redis_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let app_port = env::var("APP_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()?;

        let database_url = env::var("DATABASE_URL")?;
        let redis_url = env::var("REDIS_URL")?;

        Ok(Self {
            app_port,
            database_url,
            redis_url,
        })
    }
}
