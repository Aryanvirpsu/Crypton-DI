# Deployment topology, trust boundary and security signals

Everything here is either demonstrated by `tests/runtime/e2e.mjs` against the real
stack (`infra/docker-compose.stack.yml`), or explicitly marked as not demonstrated.

## Exposure and trust boundary

```
                         untrusted
   client ────────────────────────────────────────────────┐
                                                           ▼
 ┌────────────────────────────── edge network ───────────────────────────┐
 │                         gateway :8090   (the ONE published port)      │
 │                         gateway internal :9090 (127.0.0.1 in-container,│
 │                         never published)                              │
 └───────────────────────────────┬───────────────────────────────────────┘
                                 │  also attached to ▼
 ┌──────────────── internal network (internal: true) ────────────────────┐
 │  identity :8080   postgres :5432   redis :6379                        │
 │  no published ports, no route to host or internet                     │
 └───────────────────────────────────────────────────────────────────────┘
```

| Component | Reachable from clients? | How that is enforced |
|---|---|---|
| gateway `:8090` | Yes, the only entry point | Published on 127.0.0.1 only in the stack file; the only service on the `edge` network |
| gateway internal `:9090` | No | Binds 127.0.0.1 inside its container; not `EXPOSE`d, not published; verified refused from the edge network |
| `crypton-id` `:8080` | No | Only on the `internal: true` network; not resolvable from `edge`; **plus** defence in depth: with `GATEWAY_SHARED_SECRET` set it refuses any request (except `/health`) lacking the gateway's `x-gateway-secret` (403, verified from a hostile neighbour on the internal network, even with a valid token) |
| Postgres, Redis | No | Only on the `internal` network; no published port |
| `risk-engine`, `policy-engine` | n/a | Rust libraries, not network services. `risk-engine` is linked into the gateway; `policy-engine` is not linked into anything deployed |

Not relied on for isolation: CORS, documentation, or token authentication alone. The
shared secret is a second layer, not a substitute for the network.

### What each platform must provide (not verified here; I cannot see production)

- Only the gateway has a public domain/port. Identity, Postgres and Redis use private
  networking only (e.g. Railway: no generated public domain, `*.railway.internal`
  addresses; `IDENTITY_URL` points at the private address).
- `GATEWAY_SHARED_SECRET` is set to the same random value on gateway and identity.
- The gateway service builds from the repo root (`services/crypton-gateway/railway.toml`).
- Dev: `infra/docker-compose.yml` now binds Postgres/Redis to 127.0.0.1; set
  `BIND_ADDR=127.0.0.1` for a bare-metal identity (`dev.env.example`). `start-all.ps1`
  predates the hardened gateway and does not provision its required environment
  (`JWT_PUBLIC_KEY_PEM_FILE`, `INTERNAL_TOKEN`, `DATABASE_URL`); use the containerized
  stack for anything that must behave like the real topology.

## Security signals: producers and consumers

`crypton-id` is the authority for authentication outcomes, so it produces every signal
below, written to the Redis it already shares with the gateway (the same mechanism as the
logout denylist). The gateway consumes them. Identity enforces the account lock itself
because the public login routes carry no token for the gateway's policy to evaluate.

| Signal (Redis key) | Producer | Consumer / enforcement | Demonstrated at runtime |
|---|---|---|---|
| failed login / step-up assertion: `login:failed:{u}`, `velocity:failed:{u}:1h` | identity (`signals.rs`) on failed WebAuthn verification | risk engine (velocity) | yes |
| account lock: `login:locked:{u}` (>5 failures in 5 min, 10 min) | identity | identity `login/start` and `login/finish` refuse (429 `account_locked`); gateway policy denies already-issued tokens | yes |
| successful login clears failure counters | identity | n/a | yes |
| login attempts: `velocity:login:{u}:{1m,5m,1h,24h}` | identity at `login/start` | risk engine | yes (written) |
| credential revocation: `velocity:revocations:{u}:1h` | identity on revoke / mark-lost | risk engine; **the revoked credential itself** is enforced by the gateway reading `credentials.status` per request | yes |
| recovery: `recovery:pending:{u}`, `velocity:recovery:{u}:24h` | identity | risk engine | no (flow needs two actors; not exercised) |
| additional device: `device:added:{cred}` | identity on enrolling a device for an existing user | risk engine (recently added) | not run (needs a second-device flow) |
| logout: `jwt:deny:{u}:{iat}` | identity | gateway `validate_jwt`, identity `AuthUser` | yes |
| step-up completion: `stepup:completed:{u}:{iat}` (300 s) | identity, **only after** `finish_passkey_authentication` succeeds | gateway `step_up_context` -> policy downgrades a Challenge to Allow for that token only; Hold/Deny are never downgraded | yes |
| nonce, rate-limit counters | gateway | gateway | yes |
| allow/deny decision trail | gateway -> `audit_logs` | operators | yes |
| identity audit rows (register, login, login_failed, logout, device_*, step_up_verified, recovery_*) | identity -> `audit_logs` | operators | yes |
| `actions_executed_5m`, `registrations_from_ip_10m`, `active_session_count` | **nobody** | risk engine reads them | no producer (P2) |

`/internal/audit/ingest` on the gateway stays for non-identity producers; identity does
not use it (it cannot reach the gateway's loopback-only listener, by design).

### Step-up: what is and is not claimed

Completion is a server-side fact: identity verifies a real WebAuthn assertion and then
writes a short-lived key bound to that user and that session token (`iat`). A
client-supplied flag cannot create it (tested). **Limits:** the completion is not bound
to the specific action the client names (`action` is an audit label), so any completed
step-up satisfies any Challenge on that token for 5 minutes; and which actions trigger a
Challenge is decided by the risk engine's scores, not by an explicit per-action policy.

## Non-atomic behaviour (documented and tested, not hidden)

- Gateway `/internal/audit/ingest`: audit row insert, Redis effect, then commit. A
  failure at COMMIT leaves the Redis signal applied and no audit row, and the caller sees
  500 (a retry would apply the signal twice). The order is deliberate: a signal without
  its row is safer than a row without its signal. Tested by
  `known_gap_ingest_commit_failure_leaves_the_signal_without_an_audit_row`.
- Identity signals and identity's audit rows are separate writes (Redis, then a
  fail-open `audit_logs` insert). A failed signal write is logged and does not fail the
  login; a failed audit write is logged and does not fail the request.

## How to reproduce

```sh
tests/runtime/run.sh              # builds the real images, starts the stack, runs 88 checks, tears down
KEEP_STACK=1 tests/runtime/run.sh # leave it running to inspect
```

Needs Docker (compose v2) and Node >= 18. Uses throwaway secrets generated into
`.stack-secrets/` (gitignored). The first run builds `crypton-id` and the gateway from
source in Linux containers.
