# Changelog

All notable changes to `risk-engine` are recorded here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
The crate tracks **two independent versions**:

- **Crate version** (`Cargo.toml`) — Rust API stability. Pre-1.0 while the
  builder / trait surface is still in motion.
- **Schema version** (`decision::SCHEMA_VERSION`) — JSON wire-format of
  `RiskDecision`. Consumers should assert a minimum `schema_version` on
  inbound payloads.

## [Unreleased]

### Added

- **Phase F — production hardening.**
  - `adapters::fred_rate_limit::RedisRateLimiter` — shared token-bucket
    rate limiter backed by a Redis Lua script. Drop-in replacement for
    `rate_limit::InMemoryRateLimiter` that actually enforces limits across
    replicas.
  - `adapters::axum_server::router` now layers on a 64 KiB body-size cap
    (`DEFAULT_BODY_LIMIT_BYTES`) and a 5 s request timeout
    (`DEFAULT_REQUEST_TIMEOUT`). A new `router_with_limits` variant takes
    both as arguments.
  - `/ready` endpoint on the Axum router, backed by a host-supplied
    `ReadinessProbe` trait. 200 when the probe succeeds within 1 s, 503
    otherwise. Kubernetes should wire this to `readinessProbe` and keep
    `/health` on `livenessProbe`.
  - `AppState::new` + `AppState::with_readiness` builder — replaces the
    public tuple-struct literal.
  - `org_graph::worker::run_recompute_loop_with_shutdown` — graceful
    shutdown via a `tokio::sync::watch::Receiver<bool>`. The loop exits at
    the next cycle boundary OR during its idle sleep.
  - `RiskEngineError::StoreErrorWithSource`,
    `InvalidContextWithSource`, `ConfigErrorWithSource` — source-preserving
    siblings of the existing string variants. Constructors
    (`RiskEngineError::store_with_source`, etc.) box any
    `std::error::Error + Send + Sync + 'static` cause.
  - `context::RedactedContext` — log-safe wrapper over `&RiskContext`
    that elides PII (username, IP, UA, JWT fingerprint, accept-language)
    from the `Debug` output.
  - `decision::SCHEMA_VERSION` (= `1`) + `schema_version` field on
    `RiskDecision`. Bumps are reserved for breaking JSON changes;
    additive changes stay at v1.

### Changed

- `adapters::axum_server::evaluate_handler` routes through
  `adapters::metrics::evaluate_with_metrics` when the `metrics` feature is
  enabled. Previously the HTTP adapter bypassed the metrics wrapper,
  suppressing engine-level counters for HTTP-served traffic.
- `adapters::fred_store` + `adapters::fred_rate_limit` now wrap every Fred
  error with `RiskEngineError::store_with_source`, preserving the
  originating wire-protocol error in the `std::error::Error::source()`
  chain.
- `tower-http` feature list expanded to `trace, limit, timeout`.

## [0.1.0] — Phase E baseline

### Added

- Core engine: `engine::evaluate`, hard-gate policy, scoring formula,
  escalation memory, org-graph bias.
- Adapters: `FredSignalStore` (Redis), `SqlxLoader` (Postgres), Axum
  HTTP wrapper.
- Phase C enrichment modules under `signals::*` (GeoIP, sanctions,
  disposable email, HIBP breach, IP reputation, client-SDK headers).
- Phase D observability: tracing spans on the engine, HTTP handler, and
  background worker; Prometheus metrics (opt-in via `metrics` feature).
- Phase E enrichment pipeline (`enrichment::Pipeline` + `Builder`).
- In-memory token-bucket rate limiter.

[Unreleased]: https://example.invalid/risk-engine/compare/v0.1.0...HEAD
[0.1.0]: https://example.invalid/risk-engine/releases/tag/v0.1.0
