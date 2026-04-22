//! # Concrete adapter implementations (Phase 3)
//!
//! The existing [`crate::adapter`] module is documentation-only — a reference
//! contract for adapter authors. This module provides working, compile-tested
//! implementations behind optional feature flags:
//!
//! | Feature         | Module              | Role                                           |
//! |-----------------|---------------------|------------------------------------------------|
//! | `fred-store`    | [`fred_store`]      | `SignalStore` backed by the Fred Redis client  |
//! | `sqlx-loader`   | [`sqlx_loader`]     | Load credential / login-history from Postgres  |
//! | `axum-server`   | [`axum_server`]     | HTTP POST `/evaluate` exposing `evaluate()`    |
//!
//! None of these change the core library's public surface or default build.
//! Callers who want to run the engine as a standalone service can enable
//! `--features server` to pull all three in.

#[cfg(feature = "fred-store")]
pub mod fred_store;

#[cfg(feature = "fred-store")]
pub mod fred_rate_limit;

#[cfg(feature = "sqlx-loader")]
pub mod sqlx_loader;

#[cfg(feature = "axum-server")]
pub mod axum_server;

#[cfg(feature = "metrics")]
pub mod metrics;
