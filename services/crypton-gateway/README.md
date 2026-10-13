# Crypton Gateway

Rust/Axum gateway in front of `crypton-id`. On `dev` it is a reverse proxy with a
security middleware stack. It verifies the tokens `crypton-id` issues, applies replay,
rate-limit and risk-policy controls, and records every allow/deny decision.
[Verification status](#verification-status) states exactly what is and is not proven.

## Token contract (trust boundary)

`crypton-id` is the only issuer. Tokens are **EdDSA (Ed25519)**, header `kid = "v1"`,
claims `{sub: user UUID, username, cred_id: credential UUID, iat, exp}`
(`services/crypton-id/src/jwt.rs`). The gateway holds only the **public** key
(`JWT_PUBLIC_KEY_PEM` or `JWT_PUBLIC_KEY_PEM_FILE`, the same key identity is configured
with) and can verify but never mint.

- Rejected: any algorithm other than EdDSA (HS256, `none`, RS*), unknown/missing `kid`,
  bad signature, expired, missing claims, non-UUID `sub`/`cred_id`. There is no HS256 or
  shared-secret fallback.
- No `iss`/`aud` claim exists in the contract, so none is checked; trust rests on the
  signature under the dedicated identity key.
- Revocation mirrors identity, but fails **closed** (503 if Redis/Postgres cannot
  answer): Redis logout denylist `jwt:deny:{sub}:{iat}`, and the credential `cred_id`
  must still be `active` for `sub` in Postgres.
- Tokens carry no role: the admin check for internal diagnostics reads `users.role`.
- Key rotation is not implemented: only `kid = v1` is accepted.

## Request path

- External `0.0.0.0:$PORT` (falls back to `$APP_PORT`, default `8090`): `/health` (open)
  and everything else proxied to `IDENTITY_URL` through the stack below.
- Internal `127.0.0.1:$INTERNAL_PORT` (default `9090`): `/internal/metrics`,
  `/internal/diagnostics/*` (admin), `/internal/audit/ingest` (`x-internal-token`).
  Never expose this listener publicly.

External stack (outermost first): `request-id -> trace -> CORS -> deny-recorder ->
validate_jwt -> nonce -> rate_limit -> device_context -> step_up_context -> policy ->
allow-recorder -> proxy`.

| Layer | What it does |
|---|---|
| `validate_jwt` | Token contract above. Only the explicit public allowlist may omit a token. |
| `nonce` | Requires `x-timestamp` (±60 s) and a well-formed `x-nonce` (≤128 chars of `[A-Za-z0-9._:-]`). Single use for 300 s via atomic `SET NX EX`. Scope below. Redis down -> 503. |
| `rate_limit` | Sensitive paths (`/auth/`, `/recovery/`, `/devices/revoke`): per IP 100/min, per user 60/min, per `x-device-id` 50/min -> 429. |
| `device_context` / `step_up_context` | Credential status (from `x-device-id`, bound to the user) and `stepup:completed:{sub}` for the policy engine. |
| `policy` | `risk_engine` verdict: Deny 403, Challenge 401, Hold 202; `POLICY_ENFORCE=false` is dry run. |
| decision recorders | One durable row per request (see Audit). |
| proxy | Strips hop-by-hop and client-supplied IP headers, sets `x-forwarded-for` from the TCP peer. |

### Public authentication routes

Exact method + exact raw path, nothing else (no prefixes, normalisation, trailing slash
or percent-decoding): `POST /auth/register/start|finish`, `POST /auth/login/start|finish`,
`POST /recovery/public/start`, `GET /recovery/public/status`, `POST /recovery/public/complete`.
They still require nonce/timestamp, are rate limited, and identity enforces WebAuthn on
them. A bearer token presented on one must still be valid. Everything else without a
token is 401.

### Nonce scope

Authenticated: unique per user (`nonce:u:{sub}:{nonce}`). Another user may use the same
value (a different request by a different principal), but replaying a captured request
always lands in the victim's scope and is rejected, so switching identity does not enable
replay. Anonymous public routes: unique per `(client IP, method, path)`. A client must
send a fresh nonce on every request; `@crypton/sdk` does.

### Audit

Everything is written to `audit_logs`, the table `crypton-id`'s migrations create (no
second audit table).

- **Decision trail** (`actor='gateway'`, `action='gateway.decision'`, `outcome='allow'|'deny'`):
  reason, method, path (no query), request id, client IP, user/credential. Never tokens,
  headers, bodies or nonces. Denials are recorded by the outermost layer (a failed write is
  logged and counted, the denial still holds). An allow is recorded **before** forwarding;
  if it cannot be stored the request is refused (503), so nothing executes unaudited.
- **`/internal/audit/ingest`**: row insert + Redis signal change are consistent. The row is
  written in a transaction, the Redis effect is one atomic operation, and the transaction
  commits only if it succeeded (Redis failure -> 503, row rolled back; Postgres failure ->
  500, no Redis change). Residual window: a commit failure after a successful Redis write.

## Signals and identity integration

Identity produces the security signals and writes them to the shared Redis; the gateway
consumes them (full producer/consumer matrix: `docs/deployment-topology.md`). The gateway
adds `x-gateway-secret` (`GATEWAY_SHARED_SECRET`) to every forwarded request and strips
any client-supplied one; identity refuses requests without it. Step-up completion
(`stepup:completed:{sub}:{iat}`) is written only by identity after server-side WebAuthn
verification; the gateway downgrades a Challenge to Allow for that token, never a Hold or
Deny. `/internal/audit/ingest` remains for non-identity producers.

Metrics: policy blocks are counted per verdict (`gateway_policy_blocked_{deny,challenge,hold}_total`);
the former catch-all `..._blocked_locked_total` (which counted every verdict) is removed,
as are four counters that were never incremented.

## Environment

See `.env.example`. Required at startup: `DATABASE_URL`, `INTERNAL_TOKEN`, and the identity
public key.

## Running

```sh
cd services/crypton-gateway
cargo run
```

## Verification status

**Real-service runtime verification** (`tests/runtime/run.sh`, 82 checks, no stubs): the real
`crypton-id` and gateway images built from source in Linux containers with real Postgres and
Redis; registration, login, step-up and failed-login flows driven with a software WebAuthn
authenticator that the server verifies for real. It covers token issuance and gateway
verification, protected requests, credential revocation, logout, account lock and signals,
step-up changing a Challenge decision, audit persistence, bypass attempts (published ports,
network reachability, direct calls with a valid token, gateway-internal paths), and Redis /
Postgres outages. See `docs/deployment-topology.md`.

**Router-level and end-to-end tests below** use a stub upstream or crypton-id's real token
source compiled via `tests-support/identity-src`; they are a different, faster layer and are
not the real-service run.

`cargo test` runs 30 tests with no infrastructure. The 37 router-level and end-to-end tests
in `src/gateway_tests.rs` are `#[ignore]`d unless you provide throwaway infrastructure:

```sh
docker run -d --rm --name gw-test-redis -p 127.0.0.1:16379:6379 redis:7-alpine
docker run -d --rm --name gw-test-pg -e POSTGRES_PASSWORD=<throwaway> -e POSTGRES_DB=gwtest \
  -p 127.0.0.1:15432:5432 postgres:16-alpine
export GATEWAY_TEST_REDIS_URL=redis://127.0.0.1:16379
export GATEWAY_TEST_DATABASE_URL=postgres://postgres:<throwaway>@127.0.0.1:15432/gwtest
cargo test -- --ignored
```

Never point these at shared or production services: they write keys, create/drop
databases, and apply the identity migrations.

**Demonstrated**: token contract rejections (11 forged/invalid shapes, each with its
recorded reason); credential revocation/loss/foreign/deleted; logout denylist; public-route
allowlist and 20 near-miss bypass attempts; nonce single use, principal scope, 24-way
concurrent reuse admits one; Redis outage fails closed for auth, replay protection and
ingest; rate limits; decision trail with accurate reasons and no secrets; allow fails
closed when the trail cannot be written; ingest atomicity (both failure directions);
request ids; upstream failure does not bypass controls. Each of: public-route widening,
removing the credential check, global nonces, failing open on audit, ignoring Redis
errors in ingest, and accepting non-EdDSA tokens makes the matching tests fail.

**End to end with crypton-id's real token code** (`e2e_*`): tokens come from crypton-id's
production `issue_jwt`; the upstream authenticates with crypton-id's real `AuthUser`,
`/me`, Redis denylist and credential check. These are compiled verbatim from
`services/crypton-id/src` by `tests-support/identity-src`. **Not** the full crypton-id
binary: it cannot be built on every host (`webauthn-rs` needs OpenSSL dev libraries), so
the passkey ceremony, register/login handlers and identity's own audit writes are not
exercised.

**Remaining gaps**
- Isolation is a deployment property. The container stack enforces and tests it (see above);
  the router-test harness reaches its upstream directly by construction
  (`known_gap_identity_is_reachable_directly...`), and no production platform is verified from
  here.
- `actions_executed_5m`, `registrations_from_ip_10m` and `active_session_count` (read by the
  risk engine) have no producer.
- A completed step-up is not bound to the action it names: any completion satisfies any
  Challenge on that token for 5 minutes.
- Two failed assertions in an hour make the risk engine deny every request on already-issued
  tokens (observed at runtime). That is a policy-tuning question, not changed here.
- `GATEWAY_SHARED_SECRET` is opt-in; unset, identity accepts requests that bypassed the
  gateway if the network allows it.
- No key rotation; no `iss`/`aud`; no gateway-side agent-delegation checks.
- Not tested: step-up completion, org/tenant policy overrides, Postgres outage during auth.
