# risk-engine

Standalone, deterministic risk scoring and policy engine for Crypton-DI.

```text
evaluate()
  ├── evaluate_hard_gates()   → immediate DENY on known-bad conditions
  ├── compute_score()         → R = (D + S + N + B) * M_action * M_org + A
  ├── apply_escalation()      → promote decision based on recent history
  └── decide()                → map score to Decision + directives
```

## Design contract

- Zero dependencies on Crypton service code.
- All signal I/O abstracted behind traits (see `store` module).
- Scoring logic fully synchronous — async only at signal-fetch boundaries.
- No ML, no probabilistic models. Every outcome is traceable.

## Features

| Feature         | Pulls in                                   | Purpose                                |
|-----------------|--------------------------------------------|----------------------------------------|
| *(none)*        | `serde`, `tokio`, `tracing`, `thiserror`   | Core engine, no adapters.              |
| `fred-store`    | `fred` (Redis/Valkey)                      | `SignalStore` + `RedisRateLimiter`.    |
| `sqlx-loader`   | `sqlx` (Postgres)                          | Credential / login-history loader.     |
| `axum-server`   | `axum`, `tower`, `tower-http`              | HTTP `/evaluate` adapter.              |
| `metrics`       | `metrics` facade                           | Prometheus-compatible counters/hist.   |
| `geoip`         | `maxminddb`                                | Offline GeoIP enrichment.              |
| `server`        | all of the above except `geoip`            | All-in-one binary.                     |

Core crate (no features) has a fixed, minimal dep set — safe to embed.

## Quick start

```rust
use std::sync::Arc;
use risk_engine::{evaluate, RiskContext};
use risk_engine::adapters::fred_store::FredSignalStore;

# async fn run(store: FredSignalStore, ctx: RiskContext) {
let decision = evaluate(ctx, &store).await;
match decision.decision {
    risk_engine::Decision::Allow => { /* proceed */ }
    _ => { /* return 403 / step-up challenge / etc. */ }
}
# }
```

HTTP service (all features):

```rust,ignore
use std::sync::Arc;
use risk_engine::adapters::axum_server::{router, AppState};
use risk_engine::adapters::fred_store::FredSignalStore;

# async fn serve(store: FredSignalStore) -> std::io::Result<()> {
let state = AppState::new(Arc::new(store));
let app = router(state);
let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
axum::serve(listener, app).await
# }
```

## Running

```bash
# Unit tests
cargo test --all-features

# Benchmarks
cargo bench --bench engine_evaluate
```

## Docs

- Architecture: `src/lib.rs`
- Ops playbook: `RUNBOOK.md`
- Version history: `CHANGELOG.md`
- Scoring formula: see `scoring::` module docs
- Hard-gate catalogue: see `policy::hard_gates`

## License

Proprietary — Crypton internal.
