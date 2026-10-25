use std::sync::Arc;

use axum::Router;
use tower_governor::{
    GovernorLayer, governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor,
};

use crate::state::AppState;

mod actions;
mod admin;
mod audit_logs;
pub mod auth_webauthn;
mod devices;
mod health;
mod me;
mod oauth;
mod recovery;
mod secure;

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

    let auth_router = auth_webauthn::router().layer(GovernorLayer {
        config: auth_governor.clone(),
    });

    let recovery_router = recovery::router().layer(GovernorLayer {
        config: recovery_governor,
    });

    let mut router = Router::<AppState>::new()
        .merge(health::router())
        .merge(auth_router)
        .merge(audit_logs::router())
        .merge(devices::router())
        .merge(recovery_router)
        .merge(actions::router())
        .merge(secure::router())
        .merge(admin::router())
        .merge(me::router());

    // OAuth is an experimental, incomplete surface (no PKCE, single hardcoded
    // client — see routes/oauth.rs) that nothing in this repo currently
    // consumes. Rather than expose a half-finished auth flow as if it were
    // production-ready, it stays unmounted unless explicitly opted into.
    if std::env::var("ENABLE_OAUTH").as_deref() == Ok("true") {
        tracing::warn!("ENABLE_OAUTH=true — mounting experimental OAuth routes");
        let oauth_router = oauth::router().layer(GovernorLayer {
            config: auth_governor.clone(),
        });
        router = router.merge(oauth_router);
    }

    router
}
