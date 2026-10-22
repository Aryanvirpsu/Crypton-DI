# Testing

This is the single reference for how to run every test layer and what each one does
and does not prove. Numbers are the baselines recorded when this file was last updated
(2026-10-13, branch `dev`); if a number here disagrees with a run, the run wins and this
file is stale.

## Layers

| Layer | Where | Baseline | What it proves | Needs |
|---|---|---|---|---|
| **Real-stack runtime suite** | `tests/runtime/` | 88 checks | The REAL `crypton-id` and gateway images, real Postgres/Redis, real WebAuthn verification (driven by a software authenticator): registration, login, token verification, revocation, logout, account lock and signals, step-up changing a Challenge decision, audit persistence, single-entry-point isolation and bypass attempts, Redis/Postgres outages | Docker (compose v2), Node >= 18 |
| Gateway unit tests | `services/crypton-gateway/src` | 32 | Pure logic: policy action mapping, public-route allowlist, nonce format/scope, claims shape, secret policy | none |
| Gateway router + end-to-end tests | `services/crypton-gateway/src/gateway_tests.rs` | 37 (`#[ignore]`d without infra) | The full middleware stack at the request boundary against a stub upstream and against `crypton-id`'s real token/auth source (`tests-support/identity-src`). Faster, narrower sibling of the runtime suite; **not** a substitute for it | throwaway Redis + Postgres |
| `crypton-id` tests | `services/crypton-id/src` (`#[sqlx::test]`, unit) | 41 | Real SQL and authorization in handlers (devices, admin, recovery state machine), JWT issue/verify, Redis denylist, gateway-secret policy | Postgres + Redis; **Linux/OpenSSL dev libs** |
| `risk-engine` | `risk-engine/` | 309 (+3 ignored) | Scoring, gates, escalation, golden snapshots | none |
| `policy-engine` | `policy-engine/` | 35 | Standalone evaluators (**not wired into anything deployed**) | none |
| agent-lab | `labs/agent-lab` | 453 | Agent identity, delegation chains, authorization, approval, tracing | project `.venv` (see its README) |
| SDK | `libs/crypton-sdk` | 11 | Request shapes, envelope handling, replay headers (fetch and WebAuthn mocked) | Node |
| Frontend | `services/crypton-console` | `App.test.js`, `auth.regression.test.js` | Renders; token-expiry boundary | Node |
| Experiments / assessment | `labs/exp-*`, `labs/validation-benchmark`, `assessment-kit` | per experiment | Reproducible findings; see each directory's README | per experiment |

## Commands

```sh
# Real-stack runtime suite: builds images, starts the stack, runs the checks, tears down.
tests/runtime/run.sh                 # KEEP_STACK=1 to leave it running

# Gateway
cd services/crypton-gateway
cargo test                           # unit tests
# router + end-to-end tests: start THROWAWAY infra, never a shared one
docker run -d --rm --name gw-redis -p 127.0.0.1:16379:6379 redis:7-alpine
docker run -d --rm --name gw-pg -e POSTGRES_PASSWORD=<throwaway> -e POSTGRES_DB=gwtest -p 127.0.0.1:15432:5432 postgres:16-alpine
GATEWAY_TEST_REDIS_URL=redis://127.0.0.1:16379 \
GATEWAY_TEST_DATABASE_URL=postgres://postgres:<throwaway>@127.0.0.1:15432/gwtest \
  cargo test -- --ignored

# crypton-id: Linux, or Docker (see "Windows" below)
cd services/crypton-id && cargo test   # DATABASE_URL, REDIS_URL, JWT_*_PEM_FILE, WEBAUTHN_* set (see ci.yml)

# Libraries and labs
(cd risk-engine && cargo test)
(cd policy-engine && cargo test)
(cd libs/crypton-sdk && npm test)
(cd labs/agent-lab && .venv/Scripts/python -m pytest -q)     # Linux/macOS: .venv/bin/python
(cd services/crypton-console && npm test -- --watchAll=false)
```

Running `risk-engine`'s tests rewrites `risk-engine/risk_playground.log` (generated output,
gitignored).

## What is and is not demonstrated

- The runtime suite is the only layer that exercises the real services together. It uses a
  **software WebAuthn authenticator** (ES256, `none` attestation) that the server verifies for
  real; it does not use a physical passkey or a platform authenticator.
- The gateway's router tests and the `identity-src` harness use a stub upstream or
  `crypton-id`'s token source only; label results from them as such.
- Not covered anywhere: production platform networking; recovery and device-added signals at
  runtime (they need two actors); the frontends against a live gateway; any real browser.
- `public_complete_recovery` (Postgres + Redis) is not covered by the `#[sqlx::test]` recovery
  tests, which manage only a throwaway Postgres.

## Windows: `crypton-id` needs OpenSSL development libraries

`services/crypton-id` depends on `webauthn-rs` -> `openssl-sys`, which needs OpenSSL headers
and import libraries. Git for Windows' `openssl.exe` is a runtime binary only, so
`cargo test` for `crypton-id` fails with "Could not find directory of OpenSSL installation."
Either run it in Linux (the Dockerfile's builder image has what is needed; the runtime suite
builds `crypton-id` that way) or install OpenSSL via vcpkg
(`vcpkg install openssl:x64-windows-static-md`, set `VCPKG_ROOT`). Example, from the repo root:

```sh
docker run --rm -v "$PWD/services/crypton-id:/src" -v idtest-target:/src/target -w /src \
  --network <net with postgres+redis> -e DATABASE_URL=... -e REDIS_URL=... \
  rust:1.89-bookworm bash -c "apt-get update -qq && apt-get install -y -qq pkg-config libssl-dev && cargo test"
```

## CI

`.github/workflows/ci.yml` (push to `main`/`dev`, every PR, and a weekly schedule that runs only
`rust-audit`) has four jobs:

- `rust`: `cargo fmt --check`, `check --all-targets` and `test` for `crypton-id` and the gateway
  against Postgres/Redis service containers, then the gateway's `-- --ignored` integration tests
  against a dedicated `gwtest` database.
- `rust-audit`: `bash scripts/rust-audit.sh`, RustSec audit of all four lockfiles; fails on any
  advisory not listed with a justification in `scripts/rust-audit-ignore.txt` (see
  `dependency-audit.md`).
- `runtime`: `bash tests/runtime/run.sh`, the real-stack suite (builds both images, 88 checks).
- `frontend`: `npm ci`, SDK build/typecheck/tests, production builds of `crypton-main` and
  `crypton-console` with `CI=true` (lint warnings are errors), and their tests.

Status (read from the GitHub API, 2026-10-21): the `rust`, `runtime` and `frontend` jobs passed
on GitHub-hosted runners for `3a02135` (run 38462305933 on `dev`, 38487878105 on `main`) and
again for the push of `4344036` (run 38959492440). The `rust-audit` job **has now run on hosted
runners and is failing on `main`** (same push run, and the weekly run 39260629639): new RustSec
advisories appeared after the last remediation. See "Status update, 2026-10-21" in
[`dependency-audit.md`](dependency-audit.md); PR #13 plus a lockfile-only follow-up clears it.

### What CI does not run

These suites exist and pass locally (numbers in the next section) but no CI job executes them,
so a regression in them would not turn CI red:

- `risk-engine` (309 tests) and `policy-engine` (35 tests). The gateway links `risk-engine`,
  so its code is compiled in CI, but its own tests are not run.
- `labs/agent-lab` (453) and the other Python labs (`exp-003-*`, `exp-007`).
- `services/crypton-main/api` tests (`npm run test:api`) and `services/crypton-inquiries` tests.
- `cargo clippy`, and `cargo fmt --check` for `risk-engine` and `policy-engine` (both currently
  report formatting differences: 506 and 50 hunks respectively).

### Fresh run, 2026-10-21 (cleanup worktree on `origin/main` `4344036`)

Windows 11, Docker Desktop, throwaway Postgres 16 and Redis 7 on loopback ports. Raw logs are
kept outside the repository; the full table, commands and exceptions are in
[`handoff/HANDOFF-VERIFICATION-REPORT.md`](handoff/HANDOFF-VERIFICATION-REPORT.md).

| Layer | Result |
|---|---|
| Real-stack runtime suite (`tests/runtime/run.sh`, run through Git Bash) | 88/88 checks |
| `crypton-id` (`cargo test`, with Postgres and Redis) | 41 passed |
| Gateway (`cargo test`; `cargo test -- --ignored` with Postgres and Redis) | 32 passed; 37 passed |
| `risk-engine` / `policy-engine` | 309 passed (+3 ignored) / 35 passed |
| `crypton-sdk` | 11 passed (typecheck and build clean) |
| `crypton-console` / `crypton-main` (`CI=true`) | 7 passed / 123 passed (9 suites); both production builds succeed |
| `crypton-main/api` (with throwaway Postgres) | 64 passed, 0 skipped |
| `crypton-inquiries` | 20 passed |
| `labs/agent-lab`; EXP-007A; EXP-003 OpenAI / Microsoft / LangGraph | 453; 16; 17 / 12 / 12 passed |

Windows notes from this run: `crypton-id` built with the vcpkg OpenSSL described above
(`VCPKG_ROOT`), and `bash` on PATH may resolve to WSL's launcher (no distribution installed
here), so run `tests/runtime/run.sh` and `scripts/rust-audit.sh` through Git Bash
(`"C:\Program Files\Git\bin\bash.exe" tests/runtime/run.sh`) and leave MSYS path conversion on.
Running the Python and experiment suites rewrites tracked `labs/exp-003*/evidence/*.json`
files (new timestamps and ids); restore them with `git restore labs/*/evidence` afterwards.
