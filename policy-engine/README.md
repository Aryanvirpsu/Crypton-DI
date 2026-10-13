# policy-engine

A standalone, composable **adaptive policy engine** for authentication flows.

Each policy is an independent *evaluator* that inspects a `PolicyContext`
(user, device, IP/geo, login time, failure counts, risk score) and returns an
`Allow`, `StepUp(reason)`, or `Deny(reason)`. `evaluators::evaluate_all` runs
them in priority order:

1. **Device approval gate** (`new_device`) — blocks credentials awaiting admin approval
2. **Hard denies** — `failed_attempts`, `tor_block`, `geo_block`, `geo_velocity`, `device_trust`
3. **Step-up triggers** — `stepup_risk`, `off_hours`

First `Deny` wins and short-circuits; every `StepUp` reason is accumulated so
operators see all triggered policies. Policy config is held in an in-memory
`PolicyCache` (thread-safe, hot-reloadable via `upsert`).

## ⚠️ Status: scaffold (not yet wired)

This crate is an **isolated preview** so the engine can be reviewed on its own.
It is **not** imported by any service and is **non-functional** end-to-end:

- **No DB persistence layer.** Production hydrates/refreshes `PolicyCache` from
  Postgres; that loader is service-coupled (connection pool + error type) and is
  added when the engine is wired in. Here the cache is in-memory only.
- **No service wiring.** Hooking `evaluate_all` into the login/OAuth flow and
  exposing the policy-management API are follow-up changes.

The `migrations/` folder carries the policy schema (`policies`,
`policy_evaluations`) for that future wiring.

## Build & test

```bash
cargo test
```

The crate compiles standalone and the evaluator logic is covered by unit tests
(each evaluator + the `evaluate_all` dispatcher).
