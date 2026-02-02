use axum::Router;

pub mod health;
pub mod secure;

pub fn router() -> Router {
    Router::new()
        .merge(health::router())
        .merge(secure::router())
}
    