mod app;
mod config;
mod error;
mod jwt;
mod routes;
mod state;
mod db;
mod redis;

use crate::{config::Config, state::AppState};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use axum_server::tls_rustls::RustlsConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cfg = Config::from_env()?;
    let state = AppState::new(&cfg).await?;

    if let Some(db) = &state.db {
        sqlx::migrate!("./migrations").run(db).await?;
        tracing::info!("database migrations applied");
    }

    let app = app::app(state);

    let addr = format!("0.0.0.0:{}", cfg.app_port);
    tracing::info!("crypton-identity starting on {}", addr);

    let socket_addr = addr.parse()?;

    match RustlsConfig::from_pem_file("crypton.local.pem", "crypton.local-key.pem").await {
        Ok(tls_config) => {
            tracing::info!("TLS enabled (crypton.local.pem found) — https://crypton.local:{}", cfg.app_port);
            axum_server::bind_rustls(socket_addr, tls_config)
                .serve(app.into_make_service())
                .await?;
        }
        Err(e) => {
            tracing::warn!("TLS certs not found ({}). Running plain HTTP — for cloudflared tunnel use HTTP.", e);
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            axum::serve(listener, app.into_make_service()).await?;
        }
    }

    Ok(())
}
