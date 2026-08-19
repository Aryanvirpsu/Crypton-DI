# Architecture

## Components

```
                     browser
                        │
        ┌───────────────┼───────────────┐
        │                               │
  crypton-main                  crypton-console
  (marketing, :3000)         (product app, :3001)
  no backend calls                       │
                                          │ CRA dev proxy (same-origin)
                                          ▼
                                crypton-gateway (:8090)
                                proxy + CORS + health +
                                x-request-id + logging
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
3. Requests go through the CRA dev proxy to `crypton-gateway`, which
   assigns/forwards an `x-request-id` and proxies everything (headers,
   cookies, body) to `crypton-id` unchanged.
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

## Extension point for agent identity

`crypton-gateway/src/main.rs` has a marked spot, between routing and the
transparent proxy fallback, for agent-authorization middleware — the place
that will eventually answer "is this agent allowed to call this route, on
this human's behalf, with this delegated scope?" Nothing agent-specific is
implemented yet; see `labs/README.md` for the planned next step.

## What's real vs. experimental

| Area | Status |
|---|---|
| WebAuthn register/login, devices, audit log (write+read), admin bootstrap/roles | Real, working |
| Recovery (lost-device, trusted-device-approves, no email/SMS) | Real, working |
| Step-up verification (`/actions/*`) | Real cryptographic proof-of-presence; carries no business logic itself |
| OAuth authorization-code flow | Real state machine, but no PKCE, single hardcoded client — unmounted unless `ENABLE_OAUTH=true` |
| Rate limiting / replay protection / policy engine at the gateway | Not implemented (see `DECISIONS.md`) |
| Agent identity / delegation / agent-security testing | Not implemented (see `labs/README.md`) |
