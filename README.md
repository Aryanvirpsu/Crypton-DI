# Crypton

Passkey (WebAuthn) device identity and authentication. A Rust backend does
real FIDO2 registration/authentication, JWT issuance, device management,
audit logging, and lost-device recovery; a small gateway proxies to it; one
React app is the product surface.

This README describes what's actually implemented, verified against the
code — not a roadmap. See `DECISIONS.md` for why things are shaped this
way and `labs/README.md` for what's planned next.

## Layout

```
crypton-id        Rust/Axum identity service — WebAuthn, JWT, devices,
                   recovery, audit log, admin, OAuth (experimental)
crypton-gateway    Rust/Axum reverse proxy — routing, CORS, correlation IDs
crypton-sdk        TypeScript client SDK (libs/crypton-sdk) — the only
                   thing that talks to crypton-id over HTTP
crypton-console    The product frontend — register, login, devices,
                   recovery, admin (services/crypton-console)
crypton-main       Public marketing site, no backend calls (services/crypton-main)
labs/              Placeholder for the next planned component (empty)
docs/              Architecture, recovery flow, testing notes
```

## Implemented (real, working)

- **WebAuthn passkey registration + login**, real cryptographic
  verification via `webauthn-rs`, credentials persisted to Postgres
  (`crypton-id/src/routes/auth_webauthn.rs`)
- **JWT issuance/verification** (HS256, 1h expiry) with a Redis-backed
  logout denylist and live device-active check on every request
  (`crypton-id/src/jwt.rs`)
- **Device management** — list / revoke / mark-lost, all real Postgres
  state (`crypton-id/src/routes/devices.rs`)
- **Audit log** — write and (admin-gated) read, real Postgres
  (`crypton-id/src/routes/audit_logs.rs`, `admin.rs`)
- **Lost-device recovery**, no email/SMS by design — see
  `docs/recovery-flow.md` for the full sequence
- **Admin roles** — real per-user role in Postgres, a one-time bootstrap
  path for a fresh deployment's first admin, and a safeguard against
  demoting the last admin (`crypton-id/src/routes/admin.rs`)
- **Step-up (re-authentication) verification** — proves a signed-in user
  still holds a passkey right now, audit-logged (`crypton-id/src/routes/actions.rs`)
- **Request correlation IDs** — `x-request-id` assigned at the gateway,
  forwarded to identity, present in both services' logs, so one request
  can be traced end to end
- **crypton-console** — one login, one JWT, real data on every page (no
  mock-data fallbacks anywhere)

## Experimental (real code, not production-ready, off by default)

- **OAuth 2.0 authorization-code flow** (`crypton-id/src/routes/oauth.rs`)
  — a real state machine, but no PKCE and a single hardcoded client. Not
  mounted unless `ENABLE_OAUTH=true`. See `DECISIONS.md`.

## Planned (not implemented — see `labs/README.md`)

- Agent identity, delegation, and authorization
- Agent/AI security testing (adversarial testing, prompt/tool injection
  testing, permission/delegation attack simulation)
- MCP integration
- Rate limiting / replay protection / a policy engine at the gateway

None of the above exists anywhere in this repo yet. If you're looking for
it because of something in git history or an old doc, it was either never
built or was removed — see `DECISIONS.md`'s "Removed outright" section.

## Running it locally

Prerequisites: [Docker Desktop](https://www.docker.com/products/docker-desktop/),
[Node.js ≥ 18](https://nodejs.org/), [Rust + Cargo](https://rustup.rs/).

```powershell
.\start-all.ps1              # first run auto-generates dev.env with a random JWT secret
.\status.ps1                 # health check every service
.\stop-all.ps1                # stop (Postgres/Redis stay up — pass -StopInfra to stop those too)
```

This brings up Postgres + Redis (Docker), `crypton-id` (:8080),
`crypton-gateway` (:8090), `crypton-main` (:3000), and `crypton-console`
(:3001). Open http://localhost:3001 and register — the browser will prompt
for Touch ID / Face ID / a security key.

### Manual / per-service

```powershell
docker compose -f infra\docker-compose.yml up -d
cd services\crypton-id      && cargo run --release   # :8080
cd services\crypton-gateway && cargo run --release   # :8090
cd services\crypton-main    && npm install && npm start    # :3000
cd services\crypton-console && npm install && npm start    # :3001
```

### WebAuthn notes

Passkeys are bound to the exact `rpId`/origin they were registered on. A
passkey registered on `localhost:3001` will not work through a different
hostname (e.g. a tunnel) — register a fresh one if you switch. See each
service's `.env.example` for `WEBAUTHN_RP_ID`/`WEBAUTHN_ORIGIN` options.

## Testing

See `docs/testing.md`. Short version: `cargo test` in `services/crypton-id`
(needs Postgres + Redis), `npm test` in `libs/crypton-sdk` and
`services/crypton-console`. CI (`.github/workflows/ci.yml`) runs all of it
against real service containers on every push/PR.

## Secrets

Never commit a real `JWT_SECRET`. Local dev: `services/crypton-id/start.sh`
and `run.bat` generate a random one into a gitignored `.env.local` on first
run. `start-all.ps1` does the same for the root `dev.env`. Copy
`*.env.example` files for every other value.
