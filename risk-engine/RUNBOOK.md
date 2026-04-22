# risk-engine — Operations Runbook

Operator-facing docs for the `risk-engine` crate when deployed as a
standalone HTTP service (via the `server` feature) or embedded in a host
binary.

Assumes you've read `src/lib.rs` for the architectural contract.

---

## 1. Deploy topology

```
       ┌───────────────┐         ┌──────────────┐
       │ Host service  │◀──HTTP──│  risk-engine │
       │  (auth app)   │  JSON   │   replicas   │
       └───────────────┘         └──────┬───────┘
                                        │
                  ┌─────────────────────┼──────────────────────┐
                  │                     │                      │
                  ▼                     ▼                      ▼
            Redis / Valkey        PostgreSQL           org-graph worker
            (SignalStore)         (credential        (one per cluster —
                                   metadata)          NOT per replica)
```

- **Replica count**: stateless — scale horizontally. Typical 2–4 replicas
  behind a load balancer.
- **Worker**: exactly ONE replica runs the org-graph recompute loop
  (`org_graph::worker::run_recompute_loop_with_shutdown`). Don't run it
  on every replica — you'll duplicate every recompute.
- **Redis**: shared between all replicas. The Lua scripts assume a single
  logical Redis; a clustered deployment needs keys in the same slot.

---

## 2. Required environment / config

| Knob                       | Where                               | Default | Comment                                     |
|----------------------------|-------------------------------------|---------|---------------------------------------------|
| Redis URL                  | host-supplied to `FredSignalStore`  | —       | Prefer a Valkey >= 7.2 endpoint.            |
| PostgreSQL URL             | host-supplied to `SqlxLoader`       | —       | Read replica is fine for the hot path.      |
| HTTP body limit            | `DEFAULT_BODY_LIMIT_BYTES`          | 64 KiB  | Override via `router_with_limits`.          |
| HTTP request timeout       | `DEFAULT_REQUEST_TIMEOUT`           | 5 s     | Override via `router_with_limits`.          |
| Rate limit capacity / rate | `RedisRateLimiter::new(c, r)`       | —       | Start at `cap=20, rate=10` per-IP.          |
| Worker recompute interval  | `WorkerConfig::recompute_interval`  | 5 min   | Long enough that org-graph load is amortised. |
| Worker prune interval      | `WorkerConfig::prune_interval`      | 1 hr    | Decayed edges aged out.                     |

The crate itself reads nothing from the environment — the host binary
owns all config.

---

## 3. Endpoints (axum adapter)

| Path        | Method | Purpose                                    | Probe type         |
|-------------|--------|--------------------------------------------|--------------------|
| `/evaluate` | POST   | Score a `RiskContext`, return `RiskDecision` | hot path            |
| `/health`   | GET    | Always 200 if process is reachable         | `livenessProbe`    |
| `/ready`    | GET    | 200 if host-supplied `ReadinessProbe` ok   | `readinessProbe`   |

`/ready` runs the probe with a hard 1 s budget. It returns 503 on
probe failure AND on probe stall — so a hung Redis is visible to the
load balancer within one probe interval.

Recommended k8s probe config:

```yaml
livenessProbe:
  httpGet: { path: /health, port: http }
  periodSeconds: 10
  failureThreshold: 3

readinessProbe:
  httpGet: { path: /ready, port: http }
  periodSeconds: 5
  failureThreshold: 2
```

---

## 4. Metrics (Prometheus)

All series are registered only when the `metrics` feature is on.

See `src/adapters/metrics.rs` for the full taxonomy. The ones you want
on a dashboard from day 1:

| PromQL                                                                                       | What it tells you                               |
|----------------------------------------------------------------------------------------------|-------------------------------------------------|
| `sum by (decision) (rate(risk_engine_decision_total[5m]))`                                   | Allow/Challenge/Deny mix                        |
| `histogram_quantile(0.99, sum by (le) (rate(risk_engine_evaluate_duration_seconds_bucket[5m])))` | p99 engine latency                            |
| `sum by (gate) (rate(risk_engine_gate_fired_total[5m]))`                                     | Which hard gates are firing                     |
| `rate(risk_engine_signals_degraded_total[5m])`                                               | Redis/PG health leaking into the engine         |
| `sum by (outcome) (rate(risk_engine_org_cycle_total[1h]))`                                   | Worker success/error ratio                      |
| `histogram_quantile(0.95, sum by (le) (rate(risk_engine_http_evaluate_duration_seconds_bucket[5m])))` | p95 end-to-end HTTP                   |

**Label hygiene:** no metric carries `tenant_id` or `user_id`. If you
need per-tenant attribution, use a sampled tracing span — spans carry
both.

---

## 5. Alerts — minimum viable set

| Alert                                 | Condition (5m window)                                      | Severity | Action                                              |
|---------------------------------------|------------------------------------------------------------|----------|-----------------------------------------------------|
| `RiskEngineDegradedSignals`           | `rate(signals_degraded_total) > 0.05 * rate(decision_total)` | warn     | Check Redis/PG. Challenge rate will be elevated.    |
| `RiskEngineP99Latency`                | p99 `evaluate_duration_seconds` > 50 ms                    | warn     | Inspect Redis latency + pipeline depth.             |
| `RiskEngineOrgWorkerErrors`           | any `org_cycle_total{outcome="error"}` over 10 min         | warn     | Tail worker logs. Tenant directory or Redis issue.  |
| `RiskEngineHttpErrors`                | `rate(http_evaluate_total{outcome="error"}) > 1/s`         | page     | Broken input shape, auth drift, or dep outage.      |
| `RiskEngineBreachedRate`              | `rate(factor_fired_total{factor="breached_credential"}) > baseline × 5` | page     | Credential-stuffing campaign in progress.           |

Baseline alert thresholds assume steady-state traffic — retune after
the first week of prod metrics.

---

## 6. Common incidents

### 6.1 Challenge rate spikes to ~100%

**Cause:** Redis or Postgres is degraded. `signals_degraded_total`
should confirm. The engine's policy on degraded signals is to force
Challenge on sensitive actions rather than risk an allow.

**Fix:** restore the degraded dep. No engine-side intervention needed.

### 6.2 `/ready` returning 503 with `exceeded 1000ms`

**Cause:** host-supplied readiness probe is blocking on a slow
dependency. The handler enforces a 1 s budget so k8s can drop the pod
from rotation cleanly.

**Fix:** profile the probe. Often a missing index, a full Postgres
connection pool, or a Redis cluster resync.

### 6.3 Rate-limit-driven 429s rising

**Cause:** either a real burst or `RedisRateLimiter` is double-counting
because two Redis clusters (replica split) are handing out different
buckets.

**Debug:**
```bash
# Inspect a bucket directly
redis-cli HGETALL 'rl:user:<uuid>'
# t = remaining tokens, r = last refill unix-ms
```

**Fix:** if clusters are split, repair the cluster. Cap+rate are
tuned via `RedisRateLimiter::new` at service startup — a config bump
needs a redeploy.

### 6.4 Worker cycle latency climbing

**Cause:** a tenant has outgrown the single-worker design.
`risk_engine_org_cycle_duration_seconds` crosses the
`RECOMPUTE_TIMEOUT_MS = 10_000` ms budget.

**Fix:** shard tenants across multiple worker processes. Each one gets
a `TenantDirectory` that returns a different partition. See worker
module docstring.

### 6.5 Graceful shutdown not draining

**Symptom:** SIGTERM kills the pod before the in-flight cycle
finishes; `org_cycle_total{outcome="error"}` ticks up on shutdown.

**Fix:** the host must construct the worker with
`run_recompute_loop_with_shutdown` AND wire its `tx.send(true)` to the
SIGTERM handler. Confirm by logging "shutdown observed" lines in the
worker tracing.

---

## 7. Rollbacks

- **Engine deploy**: stateless, rollbacks are safe at any time. No
  schema migrations. `RiskDecision::schema_version = 1`; bumping this
  requires a coordinated host-service update — see CHANGELOG.md.
- **PolicyConfig changes**: if a new policy makes the challenge rate
  explode, host services should pin the prior `PolicyConfig` version
  (via `CachedPolicyStore`) and redeploy. This is a data-plane
  rollback — no engine redeploy needed.
- **Redis schema**: the adapter writes to well-known keys with TTLs.
  Wiping a prefix is the nuclear option: `redis-cli --scan --pattern 'velocity:*' | xargs redis-cli DEL`.

---

## 8. Capacity model

Order-of-magnitude numbers from the `engine_evaluate` criterion bench
on a 2024 dev laptop:

| Scenario                | median latency  |
|-------------------------|-----------------|
| `allow_hot_path`        | ~3 µs           |
| `deny_hard_gate`        | ~1 µs           |
| `challenge_degraded`    | ~2 µs           |

These are CPU-only — Redis/PG dominate real end-to-end latency. Budget
~5 ms for the full request path with warm caches.

Re-run benches on new hardware:

```bash
cargo bench --bench engine_evaluate
```

---

## 9. Security posture (what the engine does NOT do)

- **Auth on `/evaluate`**: the axum adapter has no auth layer —
  protect it at the host boundary (mTLS, bearer token via a
  `tower::Layer`, or a reverse proxy).
- **PII in logs**: the default `Debug` on `RiskContext` prints every
  field. Use `RedactedContext(&ctx)` at log sites you can't otherwise
  strip.
- **Rate limit as authz**: the `RateLimiter` is abuse-mitigation, not
  an authorization primitive. A 429 is a 429, not a permission denial.

---

## 10. Versioning rules

- **Crate version** (`Cargo.toml`): semver for the Rust API.
- **Schema version** (`decision::SCHEMA_VERSION`): semver for the
  `RiskDecision` JSON wire format. Consumers should assert a minimum
  expected version on inbound payloads.

Additive fields (new `Option<T>`, new enum variant on a `#[serde(other)]`
trail) do NOT bump either version. Breaking changes to JSON or public
Rust types DO — and show up in CHANGELOG.md.
