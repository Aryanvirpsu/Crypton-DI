use std::sync::Arc;

use axum::Router;
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer,
};


use crate::state::AppState;

pub mod auth_webauthn;
mod actions;
mod audit_logs;
mod devices;
mod health;
mod oauth;
mod recovery;
mod secure;
mod stubs;
mod contact;

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

    // Tighter limit for unauthenticated recovery endpoints (DoS/enumeration surface)
    let recovery_governor = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(2)
            .burst_size(5)
            .use_headers()
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .unwrap(),
    );

    let auth_router = auth_webauthn::router()
        .layer(GovernorLayer { config: auth_governor.clone() });

    let recovery_router = recovery::router()
        .layer(GovernorLayer { config: recovery_governor });

    let oauth_router = oauth::router()
        .layer(GovernorLayer { config: auth_governor.clone() });

    Router::<AppState>::new()
        .merge(health::router())
        .merge(auth_router)
        .merge(audit_logs::router())
        .merge(devices::router())
        .merge(recovery_router)
        .merge(actions::router())
        .merge(secure::router())
        .merge(stubs::router())
        .merge(oauth_router)
        .merge(contact::router())
}
