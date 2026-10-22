# Architecture

This file describes how the pieces fit together and what is real versus experimental.
Network exposure, the trust boundary and the security-signal producer/consumer table are in
[`deployment-topology.md`](deployment-topology.md) (the single authoritative place for them);
test layers are in [`testing.md`](testing.md); the repository map and branch workflow are in
[`repository.md`](repository.md).

## Components (development view)

```
                     browser
                        │
        ┌───────────────┼───────────────┐
        │                               │
  crypton-main                  crypton-console
  (public site + Agent Floor,   (product app, :3001)
   :3000; own /api/assessment,           │
   not wired to crypton-id)              │ CRA dev proxy (same-origin)
                                          ▼
                                crypton-gateway (:8090)
                          verifies crypton-id tokens · nonce ·
                          rate limit · risk policy · decision trail
                                          │
                                          ▼
                                  crypton-id (:8080)
                          WebAuthn · JWT · devices · recovery
                          · audit log · admin · OAuth (opt-in)
                                    │           │
                                    ▼           ▼
                               Postgres       Redis
                          (durable state)  (challenges,
                                            JWT denylist)
```

`crypton-sdk` (`libs/crypton-sdk`) is the only thing that talks to
`crypton-id` over HTTP from the frontends — both `crypton-console` and any
future integrator use it, never raw `fetch`.

## Request flow: register + login (the two fully working end-to-end paths)

1. `crypton-console` calls `client.auth.register(email)` /
   `client.auth.login(email)` (`libs/crypton-sdk/src/modules/auth.ts`).
2. The SDK does a real WebAuthn ceremony (`navigator.credentials.create` /
   `.get`) against challenges the backend issued.
3. Requests go through the CRA dev proxy to `crypton-gateway`. The SDK adds a fresh
   `x-nonce` and `x-timestamp` to every request. The public auth-bootstrap routes (exact
   allowlist) pass without a token; everything else needs a valid `crypton-id` token. The
   gateway enforces nonce/rate-limit/risk policy, records the decision, assigns
   `x-request-id`, and proxies to `crypton-id` (adding `x-gateway-secret`).
4. `crypton-id` verifies the WebAuthn response
   (`src/routes/auth_webauthn.rs`), persists the credential in Postgres,
   issues a JWT (`src/jwt.rs`), and writes an audit log row.
5. The JWT is stored client-side (`localStorage`, key `crypton_token`) and
   sent as `Authorization: Bearer <jwt>` on every subsequent request.

## Authorization model

One JWT, one claim set (`sub`, `username`, `cred_id`, `iat`, `exp`) — no
separate "admin token". `AuthUser` (`crypton-id/src/jwt.rs`) verifies the
signature, checks the Redis logout-denylist, and confirms the credential is
still `active`. `AdminUser` wraps `AuthUser` and additionally requires
`users.role IN ('admin', 'super_admin')` from Postgres — that's the only
place adminness is decided. `crypton-console`'s sidebar hides/shows admin
links based on `GET /me`, but that's a UI convenience, not the boundary.

A fresh deployment starts with zero admins. `POST /admin/bootstrap`
promotes the calling user to `super_admin` — but only while no admin
exists yet; every subsequent call 403s. See `crypton-id/src/routes/admin.rs`.

## Agent identity and the gateway

The gateway has **no agent-delegation checks**. Agent authorization is enforced in-process in
`labs/agent-lab` (`authorization.py`, `chain.py`, `approval.py`), not at the gateway; see
`labs/README.md`. The gateway enforces human-session controls only (token, revocation, nonce,
rate limit, risk policy).

## Enforcement coverage

An enforcement point only sees traffic that reaches it. The container stack
(`infra/docker-compose.stack.yml`) makes the gateway the single published port and keeps
identity, Postgres and Redis on an internal network; `GATEWAY_SHARED_SECRET` adds a second
layer on identity. That is demonstrated for the stack and **not** verifiable for any production
platform from this repository. See `deployment-topology.md`.

## What's real vs. experimental

| Area | Status |
|---|---|
| WebAuthn register/login, devices, audit log (write+read), admin bootstrap/roles | Real, working |
| Recovery (lost-device, trusted-device-approves, no email/SMS) | Real, working |
| Step-up verification (`/actions/*`) | Real cryptographic proof-of-presence; carries no business logic itself |
| OAuth authorization-code flow | Real state machine, but no PKCE, single hardcoded client — unmounted unless `ENABLE_OAUTH=true` |
| Rate limiting / replay protection / risk-engine policy at the gateway | On `main` (and `dev`): verifies `crypton-id` EdDSA tokens; demonstrated against the real services in containers with a single external entry point and identity-produced security signals (`docs/deployment-topology.md`). Production-platform isolation is unverified. The standalone `policy-engine` crate is **not** part of this path (it is not linked into anything deployed). |
| Public assessment intake (`crypton-main/api`, Supabase Postgres) | Serverless endpoints with encrypted-at-rest payloads, signed challenges and fail-closed configuration; separate from the identity path. `docs/agent-floor-readiness-2026-10-14.md` records the pre-cutover local verification; the later production cutover is recorded in private operations notes outside this repository. `services/crypton-inquiries` is its local/single-host SQLite twin. |
| Agent identity / delegation (`labs/agent-lab`) | Real, working, but a separate Python lab, not wired behind the gateway — see `labs/README.md`. As of v0.6: multi-hop delegation chains (`Human -> Agent -> Agent -> ...`), Ed25519-signed, scope-attenuated (child authority can only narrow), verified by one authoritative walker (`chain.py`) — see `docs/CRYPTON_TECHNICAL_SOT_v1.1.md` §11 and `DECISIONS.md`. Root-only (single-hop) delegation remains fully supported as the degenerate case. Human-issued Approval sits on top, unchanged. |
| Agent-security testing (adversarial/attack simulation) | Not implemented |
