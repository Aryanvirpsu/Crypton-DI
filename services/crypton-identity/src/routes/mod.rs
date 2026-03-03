use axum::Router;
use crate::state::AppState;

mod auth_webauthn;
mod devices;
mod health;
mod secure;

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .merge(health::router())
        .merge(auth_webauthn::router())
        .merge(devices::router())
        .merge(secure::router())
}
