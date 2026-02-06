mod app;
mod config;
mod error;
mod routes;
mod state;
mod db;
mod redis;

use crate::{app::app, config::Config, state::AppState};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cfg = Config::from_env()?;
    let state = AppState::new(&cfg).await?;
    let app = app::app(state);


    let addr = format!("0.0.0.0:{}", cfg.app_port);
    println!("crypton-identity running on {}", addr);

    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
