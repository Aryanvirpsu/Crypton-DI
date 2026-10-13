# Crypton Gateway

Rust/Axum gateway in front of `crypton-id`. On `dev` it is a reverse proxy with a
middleware stack wired in front of the proxy. **"Wired" is not "proven":** see
[Verification status](#verification-status) for what the tests actually demonstrate
and the known defects that stop it being production-ready.

## Request path

Two listeners:

- External `0.0.0.0:$PORT` (falls back to `$APP_PORT`, default `8090`): `/health`
  (open) and everything else proxied to `IDENTITY_URL` through the stack below.
- Internal `127.0.0.1:$INTERNAL_PORT` (default `9090`): `/internal/metrics`,
  `/internal/diagnostics/*` (admin JWT), `/internal/audit/ingest` (`x-internal-token`).
  Never expose this listener publicly.

External middleware order (outermost first):

`request-id -> trace -> CORS -> validate_jwt -> enforce_nonce -> rate_limit -> device_context -> step_up_context -> enforce_policy -> proxy`

| Layer | What it does |
|---|---|
| request-id | Assigns a UUID `x-request-id` (keeps a caller-supplied one), logs it on the request span, forwards it upstream, echoes it on the response. Applied outermost so rejected requests carry one too. |
| `validate_jwt` | Requires `Authorization: Bearer`. Decodes **HS256** with `JWT_SECRET`; on failure falls back to `POST {IDENTITY_URL}/verify`. Rejects tokens on the Redis logout denylist / role-invalidation cutoff. |
| `enforce_nonce` | Requires `x-timestamp` (±60 s) and `x-nonce`; a nonce is single-use for 300 s (`SET NX`); replay -> 409; Redis down -> 503. |
| `rate_limit` | Sensitive paths only (`/auth/`, `/recovery/`, `/devices/revoke`): per IP 100/min, per user 60/min, per `x-device-id` 50/min -> 429. |
| `device_context` | Looks up the `x-device-id` credential status in Postgres, bound to the authenticated user. |
| `step_up_context` | Reads `stepup:completed:{jti or sub}` from Redis. |
| `enforce_policy` | Builds a `risk_engine::RiskContext` from signals the gateway can observe (velocity counters, lock, device status, nonce) and enforces the verdict: Deny 403, Challenge 401, Hold 202. `POLICY_ENFORCE=false` is dry-run. |
| proxy | Forwards to identity; strips hop-by-hop and client-supplied IP headers (`x-forwarded-for`, `x-real-ip`, ...) and sets `x-forwarded-for` from the TCP peer. |

The gateway is a coarse pre-filter. It does no GeoIP/breach enrichment and has **no
agent-delegation checks**; `crypton-id` stays authoritative.

## Environment

See `.env.example`. Required at startup: `DATABASE_URL`, `JWT_SECRET`, `INTERNAL_TOKEN`.

## Running

```sh
cd services/crypton-gateway
cargo run
```

## Verification status

`cargo test` runs 28 tests with no infrastructure. The 18 router-level tests in
`src/gateway_tests.rs` drive the real routers against a throwaway Redis + Postgres and a
stub upstream. They are `#[ignore]`d unless you provide infrastructure:

```sh
docker run -d --rm --name gw-test-redis -p 127.0.0.1:16379:6379 redis:7-alpine
docker run -d --rm --name gw-test-pg -e POSTGRES_PASSWORD=<throwaway> -e POSTGRES_DB=gwtest \
  -p 127.0.0.1:15432:5432 postgres:16-alpine
export GATEWAY_TEST_REDIS_URL=redis://127.0.0.1:16379
export GATEWAY_TEST_DATABASE_URL=postgres://postgres:<throwaway>@127.0.0.1:15432/gwtest
cargo test -- --ignored
```

Never point these at shared or production services; the tests write keys and tables.

**Demonstrated by tests** (the nonce, rate-limit and JWT layers, and policy enforcement,
were each also disabled in turn to confirm the relevant tests fail): unauthenticated /
forged / expired / revoked tokens rejected; admin-only diagnostics; locked accounts and
revoked credentials blocked by policy (another user's revoked credential is not applied);
nonce and timestamp required, replay rejected; per-IP / per-user / per-device rate limits;
request IDs generated, propagated upstream and present on rejections; spoofed client-IP
headers not forwarded; upstream failure does not bypass controls.

**Known defects and gaps** (pinned by `known_*` tests, not fixed here):

- **Identity-issued tokens do not match what the gateway verifies.** `crypton-id` signs
  EdDSA and exposes no `/verify` route; the gateway verifies HS256 with a shared secret.
  Established by source inspection, not a runtime test against a live identity service.
- **Public auth routes are unreachable**: `validate_jwt` wraps the whole proxy, so
  `/auth/login` and `/auth/register` return 401 without a bearer token.
- **Audit ingest cannot persist**: it inserts into `audit_events`, which `crypton-id`
  never creates (it creates `audit_logs`), so the endpoint returns 500 after applying
  its Redis side effects.
- **The gateway keeps no audit record of its own allow/deny decisions**; only
  in-memory counters at `/internal/metrics`. Every policy block increments
  `gateway_policy_blocked_locked_total` regardless of reason.
- Nonces are global, not bound to the caller.
- Not tested: Redis outage behaviour, step-up completion, org/tenant policy overrides.
