use std::sync::Arc;

use axum::Router;
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer,
};


use crate::state::AppState;

mod auth_webauthn;
mod devices;
mod health;
mod secure;
mod stubs;

pub fn router() -> Router<AppState> {
    // Rate-limit auth endpoints: 5 requests/second steady-state, burst of 20.
    // Keyed by client IP (respects X-Forwarded-For via SmartIpKeyExtractor).
    // Other routes (health, devices, secure) are not rate-limited here.
    let auth_governor = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(5)
            .burst_size(20)
            .use_headers()
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .unwrap(),
    );

    let auth_router = auth_webauthn::router()
        .layer(GovernorLayer { config: auth_governor });

    Router::<AppState>::new()
        .merge(health::router())
        .merge(auth_router)
        .merge(devices::router())
        .merge(secure::router())
        .merge(stubs::router())
}
