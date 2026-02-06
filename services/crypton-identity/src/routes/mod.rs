use axum::Router;
use crate::state::AppState;

mod health;
mod secure; // if you have it
mod auth_webauthn; // if you have it

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .merge(health::router())
        .merge(secure::router())
        .merge(auth_webauthn::router())
}
