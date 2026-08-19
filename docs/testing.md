# Testing

## What exists

| Layer | Where | What it proves |
|---|---|---|
| Rust unit tests | `crypton-id/src/jwt.rs` | JWT issue/verify/expiry logic — no DB/Redis needed |
| Rust unit tests | `crypton-id/src/routes/admin.rs` (bottom) | Pure display-formatting helpers (`security_score`, `relative_time_secs`, `map_audit_to_activity`) |
| Rust unit tests | `crypton-id/src/routes/contact.rs` was removed with the route | (n/a — route deleted, zero callers) |
| **Rust integration tests** | `crypton-id/src/routes/devices.rs`, `admin.rs`, `recovery.rs` (bottom of each file, `#[cfg(test)] mod tests`) | Real Postgres via `#[sqlx::test]` — handlers called directly (not mocked), exercising real SQL, real authorization checks (last-admin guard, wrong-user rejection, status-transition guards), and the real recovery state machine |
| **Redis integration test** | `crypton-id/src/redis/sessions.rs` | Real Redis round-trip for the logout/JWT-denylist mechanism |
| SDK unit tests | `libs/crypton-sdk/src/__tests__/sdk.test.ts` | `fetch` and `navigator.credentials` are mocked (real WebAuthn hardware can't run in CI) — but every mock assertion checks the exact URL and request body the SDK sends, so an endpoint or payload-shape regression fails these tests. Verified: `npm test` → 10/10 passing. |
| Frontend unit test | `crypton-console/src/auth.regression.test.js` | `getValidToken()` expiry-boundary logic — a real regression this pinned before |
| Frontend smoke test | `crypton-console/src/App.test.js` | Renders without crashing and shows the login page when logged out |

## What's intentionally NOT covered

Full WebAuthn ceremonies (register/login ending in a real signature
verification) aren't exercised by any automated test — that needs a
virtual FIDO2 authenticator, which this cleanup didn't build. This is the
one explicit "mocks are acceptable" carve-out per the cleanup brief:
hardware interaction can't be automated cheaply, so the SDK's WebAuthn
plumbing is unit-tested against a mock, and the backend's WebAuthn
verification itself relies on the `webauthn-rs` crate (not re-tested here —
it's a well-tested third-party library, not code this repo wrote).

`public_complete_recovery` (the one recovery handler that touches both
Postgres *and* Redis) isn't covered by the recovery.rs integration tests,
because `#[sqlx::test]` only manages a throwaway Postgres database, not
Redis. Every other part of the recovery state machine is covered.

## Running tests

```sh
# Rust — needs Postgres + Redis reachable (see infra/docker-compose.yml)
cd services/crypton-id && cargo test
cd services/crypton-gateway && cargo test   # no tests currently — gateway is proxy-only

# SDK
cd libs/crypton-sdk && npm test

# Frontend
cd services/crypton-console && npm test -- --watchAll=false
```

CI (`.github/workflows/ci.yml`) runs all of the above against real
Postgres/Redis service containers on every push/PR.

**Caveat on this being verified:** the Rust tests above were written and
statically reviewed but could not be run in the environment this cleanup
was done in (no local Rust toolchain was available). They're expected to
compile and pass against the schema in `migrations/`, but the first real
`cargo test` run against them is the actual proof — treat that run, not
this document, as the source of truth.
