# Crypton Gateway

A small reverse proxy in front of `crypton-id`. Written in Rust (Axum).

## What it actually does

- `GET /health` — the gateway's own liveness check (does not proxy)
- Everything else — forwarded verbatim to `IDENTITY_URL`, including the
  `Authorization` header and cookies
- Assigns an `x-request-id` to every request that doesn't already have one,
  forwards it to identity, and echoes it back on the response, so one
  request can be traced across both services' logs
- CORS, from a configurable origin allowlist

That's it. See `src/main.rs` for the whole implementation — it's under 300
lines on purpose.

## What it does not do (yet)

An earlier version of this service had ~800 lines of JWT verification,
Redis-backed rate limiting, nonce/replay protection, a policy engine, and
audit ingestion under `src/middleware/`, `src/routes/`, and `src/audit/`.
None of it was ever wired into the router (`main.rs` had no `mod`
declarations for any of it), and it depended on crates that weren't even in
`Cargo.toml` — so it could not have compiled if it had been wired in. It was
removed rather than resurrected without review; see git history before this
cleanup if you need it as a reference.

Today, identity does its own JWT verification and rate limiting
(`crypton-id/src/jwt.rs`, `crypton-id/src/routes/mod.rs`). The gateway is
not yet a policy enforcement point.

**Extension point:** the router in `main.rs` has a comment marking where
agent-identity authorization middleware belongs — a place to check "is this
agent allowed to call this route, on this human's behalf?" before the
request reaches identity. Not implemented yet.

## Environment

See `.env.example`.

- `APP_PORT` / `PORT` — port to listen on (default `8090`)
- `IDENTITY_URL` — base URL of `crypton-id` (default `http://127.0.0.1:8080`)
- `ALLOWED_ORIGINS` — comma-separated CORS allowlist (default: local dev origins)
- `RUST_LOG` — log level (default `info`)
- `LOG_FORMAT` — set to `json` for structured logs; default is human-readable text

## Running

Requires the Rust toolchain (`cargo`).

```sh
cd services/crypton-gateway
cargo run
```
