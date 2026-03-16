# Crypton Gateway Service

This service implements the API gateway / policy layer and is written in Rust using Axum.

## Environment

- `REDIS_URL` – connection string for Redis (defaults to `redis://127.0.0.1/`).
- `DATABASE_URL` – Postgres connection string for audit logger (e.g. `postgres://user:pass@localhost/crypton`).
- `JWT_SECRET` – secret used to decode JWTs for bearer tokens.
- `IDENTITY_URL` – base URL of the identity service (defaults to `http://localhost:8081`).

## Replay protection

All requests must include an `x-timestamp` header with a Unix timestamp (seconds since epoch).
The timestamp must be within ±60 seconds of the server's current time, or the request is rejected with `400 Bad Request`.

Optionally, clients can include an `x-nonce` header with a unique value; the gateway will reject any request with a previously-seen nonce (HTTP 409) to prevent replay attacks.
Nonces are stored in Redis with a 5-minute TTL.

## Policy engine

The gateway enforces several simple policies tracked in Redis by audit events:

* `login_failed` and `login_success` events update counters and lock accounts after five failures (login paths will return 423 Locked).
* `device_added` events set Redis flags causing `/devices/revoke` requests for that device to require a `x-step-up: true` header for a short window.
* `recovery_started` / `recovery_completed` events toggle a `recovery:pending:{user}` key; while pending, `/devices/add` is forbidden.

See `src/middleware/policy.rs` for the implementation details.

Audit ingestion now populates these keys automatically when events are received.

The gateway will automatically create the `audit_events` table on startup if it doesn't exist.

If you prefer managing migrations with `sqlx`, you can use:

```sh
cargo install sqlx-cli --no-default-features --features native-tls,postgres
export DATABASE_URL="postgres://user:pass@localhost/crypton"
sqlx migrate add create_audit_events
# edit the migration file, then:
sqlx migrate run
```

## Running

Requires Rust toolchain (`cargo`).

```sh
cd services/crypton-gateway
cargo run
```

The gateway listens on `0.0.0.0:8080` by default.

## Features implemented so far

- `/health` endpoint
- reverse proxy for `/api/*`
- JWT/opaque token validation middleware (claims attached to request)
- Redis-backed rate limiting (IP/user/device)
- Nonce replay prevention (`x-nonce` header, optional; `x-timestamp` header required)
- Policy engine (account lockout, recovery blocking, device step-up)
- Simple policy engine (deny-by-default, device status enforcement)
- `/audit` ingestion stub

Next steps: add Postgres auditing, more policies, step-up auth, etc.
