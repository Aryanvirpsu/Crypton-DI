//! Engine error surface.
//!
//! Every variant is a terminal outcome — these are what `?`-propagated
//! failures flow into at adapter boundaries.
//!
//! ### Source-preserving variants (F3)
//!
//! Several variants carry a `#[source]` so the originating library error
//! (SQLx row-not-found, Fred wire-protocol error, serde parse failure) is
//! walkable from a `tracing::error!(error = ?e)` without the adapter having
//! to stringify it pre-wrap. The existing tuple variants
//! (`StoreError(String)`, `InvalidContext(String)`, `ConfigError(String)`)
//! are retained so callers that don't have a typed cause can keep using
//! them — this keeps the F3 migration backwards-compatible.
//!
//! The convention is: *Source variants carry `msg: String, source: BoxedError`
//! and Display as `"{msg}: {source}"`; bare variants carry `String` and
//! Display as `"{0}"`.*

use thiserror::Error;

/// Boxed dynamic error type used for source chains. `Send + Sync` so the
/// error can cross async boundaries; `'static` because source-chain walking
/// in `tracing` etc. assumes owned data.
pub type BoxedError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Debug, Error)]
pub enum RiskEngineError {
    /// Adapter-side failure talking to the signal store, no typed cause
    /// available. Prefer [`StoreErrorWithSource`](Self::StoreErrorWithSource)
    /// when wrapping a concrete library error.
    #[error("signal store error: {0}")]
    StoreError(String),

    /// Adapter-side failure whose underlying cause is preserved via
    /// `#[source]`. The chain is visible to `tracing`/`anyhow`/`eyre`
    /// consumers without forcing the adapter to `to_string()` the cause.
    #[error("signal store error: {msg}")]
    StoreErrorWithSource {
        msg: String,
        #[source]
        source: BoxedError,
    },

    /// A Redis/Postgres operation exceeded its configured budget. The engine
    /// treats this as a "degraded signals" event and applies the
    /// conservative-on-miss policy rather than denying outright.
    #[error("signal collection timed out after {ms}ms")]
    Timeout { ms: u64 },

    /// `RiskContext` failed a pre-scoring invariant.
    #[error("invalid context: {0}")]
    InvalidContext(String),

    /// Invalid context with a typed cause (e.g. a serde_json parse failure).
    #[error("invalid context: {msg}")]
    InvalidContextWithSource {
        msg: String,
        #[source]
        source: BoxedError,
    },

    /// Host-supplied [`crate::PolicyConfig`] does not parse or violates a
    /// structural constraint.
    #[error("invalid policy config: {0}")]
    ConfigError(String),

    /// Config error with a typed cause (serde_yaml, toml, …).
    #[error("invalid policy config: {msg}")]
    ConfigErrorWithSource {
        msg: String,
        #[source]
        source: BoxedError,
    },

    /// Pre-filter (see [`crate::rate_limit`]) rejected the request before
    /// scoring. Callers should map this to HTTP 429.
    #[error("rate limit exceeded: {0}")]
    RateLimited(String),
}

impl RiskEngineError {
    /// Build a [`StoreErrorWithSource`](Self::StoreErrorWithSource) from any
    /// typed error. Prefer this over `StoreError(format!("…: {e}"))` — the
    /// typed source is visible to log-chain walkers.
    pub fn store_with_source<E>(msg: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::StoreErrorWithSource {
            msg: msg.into(),
            source: Box::new(source),
        }
    }

    /// Build an [`InvalidContextWithSource`](Self::InvalidContextWithSource).
    pub fn invalid_context_with_source<E>(msg: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::InvalidContextWithSource {
            msg: msg.into(),
            source: Box::new(source),
        }
    }

    /// Build a [`ConfigErrorWithSource`](Self::ConfigErrorWithSource).
    pub fn config_error_with_source<E>(msg: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::ConfigErrorWithSource {
            msg: msg.into(),
            source: Box::new(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_with_source_preserves_chain() {
        let inner = std::io::Error::new(std::io::ErrorKind::ConnectionReset, "kaboom");
        let e = RiskEngineError::store_with_source("redis lookup failed", inner);
        assert_eq!(e.to_string(), "signal store error: redis lookup failed");
        let source = std::error::Error::source(&e).expect("source set");
        assert!(source.to_string().contains("kaboom"));
    }

    #[test]
    fn legacy_store_error_has_no_source() {
        let e = RiskEngineError::StoreError("basic failure".into());
        assert!(std::error::Error::source(&e).is_none());
    }

    #[test]
    fn invalid_context_with_source_chains() {
        #[derive(Debug)]
        struct Dummy;
        impl std::fmt::Display for Dummy {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "inner cause")
            }
        }
        impl std::error::Error for Dummy {}

        let e = RiskEngineError::invalid_context_with_source("bad nonce", Dummy);
        assert_eq!(e.to_string(), "invalid context: bad nonce");
        assert_eq!(
            std::error::Error::source(&e).unwrap().to_string(),
            "inner cause"
        );
    }

    #[test]
    fn timeout_has_no_source() {
        let e = RiskEngineError::Timeout { ms: 20 };
        assert!(std::error::Error::source(&e).is_none());
    }
}
