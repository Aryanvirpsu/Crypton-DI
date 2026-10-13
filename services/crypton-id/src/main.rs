mod app;
mod audit;
mod config;
mod error;
mod jwt;
mod redis;
mod routes;
mod signals;
mod state;

use crate::{config::Config, state::AppState};
use axum_server::tls_rustls::RustlsConfig;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    // Set LOG_FORMAT=json for structured logs (matches crypton-gateway).
    if std::env::var("LOG_FORMAT").as_deref() == Ok("json") {
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer())
            .init();
    }

    let cfg = Config::from_env()?;
    let state = AppState::new(&cfg).await?;

    if let Some(db) = &state.db {
        sqlx::migrate!("./migrations").run(db).await?;
        tracing::info!("database migrations applied");
    }

    let app = app::app(state);

    // Containers need 0.0.0.0 (the default); a bare-metal/dev run should set
    // BIND_ADDR=127.0.0.1 so identity is not reachable around the gateway.
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0".to_string());
    let addr = format!("{}:{}", bind_addr, cfg.app_port);
    tracing::info!("crypton-identity starting on {}", addr);

    let socket_addr = addr.parse()?;

    match RustlsConfig::from_pem_file("crypton.local.pem", "crypton.local-key.pem").await {
        Ok(tls_config) => {
            tracing::info!(
                "TLS enabled (crypton.local.pem found) — https://crypton.local:{}",
                cfg.app_port
            );
            axum_server::bind_rustls(socket_addr, tls_config)
                .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                .await?;
        }
        Err(e) => {
            tracing::warn!(
                "TLS certs not found ({}). Running plain HTTP — for cloudflared tunnel use HTTP.",
                e
            );
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await?;
        }
    }

    Ok(())
}
