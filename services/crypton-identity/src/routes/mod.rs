use std::sync::Arc;

use axum::Router;
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer,
};


use crate::state::AppState;

pub mod auth_webauthn;
mod actions;
mod devices;
mod health;
mod oauth;
mod secure;
mod stubs;

pub fn router() -> Router<AppState> {
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
        .merge(actions::router())
        .merge(secure::router())
        .merge(stubs::router())
        .merge(oauth::router())
}
