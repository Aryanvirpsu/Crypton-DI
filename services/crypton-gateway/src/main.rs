use axum::{routing::get, Router};
use std::net::SocketAddr;

mod middleware;
mod routes;
mod state;
mod audit;

use crate::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_ansi(true)
        .init();

    tracing::info!("gateway starting");

    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".into());
    let pg_conn = std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://user:pass@localhost/crypton".into());
    let state = AppState::new(&redis_url, &pg_conn).await?;

    // ensure audit table exists
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS audit_events (
            id UUID PRIMARY KEY,
            event_type TEXT NOT NULL,
            user_id TEXT,
            details JSONB,
            created_at TIMESTAMPTZ DEFAULT now()
        )",
    )
    .execute(&state.pg)
    .await?;

    let api = Router::new()
        .route("/*path", axum::routing::any(routes::proxy::handler))
        // middleware order: auth -> rate limit -> policy -> nonce
        .layer(axum::middleware::from_fn(middleware::auth::validate_jwt))
        .layer(axum::middleware::from_fn(middleware::rate_limit::limit))
        .layer(axum::middleware::from_fn(middleware::policy::enforce_policy))
        .layer(axum::middleware::from_fn(middleware::nonce::enforce_nonce));

    let app = Router::new()
        .route(
            "/health",
            get(|State(state): axum::extract::State<AppState>| async move {
                // Basic operational check: can we talk to Redis and Postgres?
                let mut ok_redis = true;
                let mut ok_pg = true;

                // ping Redis
                if let Err(e) = redis::cmd("PING")
                    .query_async::<_, String>(&mut state.redis.as_ref().clone())
                    .await
                {
                    ok_redis = false;
                    tracing::error!(error = ?e, "health: redis ping failed");
                }

                // simple Postgres query
                if let Err(e) = sqlx::query("SELECT 1").execute(&state.pg).await {
                    ok_pg = false;
                    tracing::error!(error = ?e, "health: postgres check failed");
                }

                if ok_redis && ok_pg {
                    axum::Json(serde_json::json!({
                        "status": "ok",
                        "redis": "ok",
                        "postgres": "ok",
                    }))
                } else {
                    axum::Json(serde_json::json!({
                        "status": "degraded",
                        "redis": if ok_redis { "ok" } else { "error" },
                        "postgres": if ok_pg { "ok" } else { "error" },
                    }))
                }
            }),
        )
        .route("/metrics", axum::routing::get(routes::metrics::handler))
        .route(
            "/admin/diagnostics/user/:user_id",
            axum::routing::get(routes::diagnostics::user),
        )
        .route(
            "/admin/diagnostics/device/:device_id",
            axum::routing::get(routes::diagnostics::device),
        )
        .route(
            "/admin/diagnostics/ip/:ip",
            axum::routing::get(routes::diagnostics::ip),
        )
        .route("/audit", axum::routing::post(audit::ingest))
        .nest("/api", api)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!(%addr, "gateway listening");
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await?;

    Ok(())
}