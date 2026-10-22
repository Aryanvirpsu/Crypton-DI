# Crypton

Crypton is an experimental security prototype for exploring identity,
authorization, delegation, approval, and accountability in action-taking
AI agent workflows. Its core is a real Rust/Postgres identity service
(`crypton-id`) that does WebAuthn passkey authentication for humans,
fronted by a gateway (`crypton-gateway`) that verifies its tokens and applies
replay, rate-limit and risk-policy controls, and a Python prototype
(`agent-lab`) built on that identity that gives *agents* their own identity,
delegated authority, and pre-execution authorization checks. Around them sit
assessment tooling and evidence (`assessment-kit`, `labs/`) and a public site
(`crypton-main`) with an assessment intake. **New to the repository? Start with
[`docs/handoff/CTO-HANDOFF.md`](docs/handoff/CTO-HANDOFF.md).**

This README describes what's actually implemented, verified against the
code — not a roadmap. See [`DECISIONS.md`](DECISIONS.md) for why things
are shaped this way.

## Current work

The current prototype focuses on how identity and authority move through
action-taking AI agent workflows, on top of a working human-identity
service. Recent work:

- Migrated JWT signing from a shared HMAC secret to asymmetric Ed25519
  (`v0.5.1`) — see [`services/crypton-id/src/jwt.rs`](services/crypton-id/src/jwt.rs)
- Added human-approval binding to exact tool calls via WebAuthn step-up
  (`v0.5`) — [`labs/agent-lab/src/agentlab/approval.py`](labs/agent-lab/src/agentlab/approval.py)
- Built provable multi-hop agent delegation with scope attenuation
  (`v0.6`) — a delegated agent's authority can only ever narrow, never
  widen, at each hop — [`labs/agent-lab/src/agentlab/chain.py`](labs/agent-lab/src/agentlab/chain.py)
- Added a deterministic pre-execution authorization pipeline that the
  model cannot override — [`labs/agent-lab/src/agentlab/authorization.py`](labs/agent-lab/src/agentlab/authorization.py)
- Added a real, persisted Agent Registry (agent identity + ownership),
  replacing hardcoded agent-name strings — [`labs/agent-lab/src/agentlab/agents.py`](labs/agent-lab/src/agentlab/agents.py)

## What it currently does

**Human identity (`crypton-id`, Rust/Axum, production-shaped):**

- WebAuthn passkey registration and login, real cryptographic
  verification via `webauthn-rs`
- Ed25519-signed JWT issuance/verification, with a Redis-backed logout
  denylist and a live device-active check on every request
- Device management (list / revoke / mark-lost), audit logging, admin
  roles, and lost-device recovery — all backed by real Postgres state

**Agent identity & authorization (`agent-lab`, Python prototype, not yet
wired behind the gateway):**

- Agent identity and ownership, separate from the human who authorized it
  and the specific run executing it
- Authorization checks that run deterministically immediately before
  tool execution — never decided by the model
- Human approval bound to one exact tool call, via the same WebAuthn
  step-up primitive `crypton-id` exposes
- Multi-hop agent-to-agent delegation, cryptographically signed at each
  hop, with authority guaranteed to only narrow along the chain
- A full, persisted trace of every authorization decision and tool call,
  usable as evidence

See [`docs/architecture.md`](docs/architecture.md) for the full real-vs-experimental
breakdown, and [`labs/agent-lab/LEARNING.md`](labs/agent-lab/LEARNING.md) for how the
agent side actually works.

### Not implemented (see `DECISIONS.md`)

Agent identity is behind the gateway's extension point in name only —
`agent-lab` runs as a standalone CLI today, not wired into
`crypton-gateway`'s request path. Also not implemented: agent-security /
adversarial testing (prompt or tool injection, delegation-attack
simulation), MCP integration, and any agent-delegation check in the
gateway. (The gateway's token verification, replay protection, rate limiting and
risk-engine policy are implemented and demonstrated against the real services in
the container stack; production-platform network isolation is not verified from
this repository. See the gateway README, "Verification status".)

## Architecture

```mermaid
flowchart LR
    subgraph Human identity path
        Browser --WebAuthn passkey--> ID[crypton-id]
        Console[crypton-console] --> Gateway[crypton-gateway]
        Gateway --> ID
        ID --> PG[(Postgres)]
        ID --> Redis[(Redis)]
    end

    subgraph Agent identity path — agent-lab, standalone
        HumanJWT[Human identity / JWT] --> Agent
        Agent --delegates, narrowed scope--> DelegatedAgent[Delegated agent]
        DelegatedAgent --> AuthZ[Authorization + Approval check]
        AuthZ --allow--> Tool
        AuthZ --deny--> Trace[Audit trace]
    end
```

The two paths share one identity source (the JWT `crypton-id` issues) but
run separately today: `agent-lab` verifies the same JWTs, it doesn't call
`crypton-id` over the network, and it isn't routed through the gateway, which
has no agent-delegation checks. See
[`docs/architecture.md`](docs/architecture.md#agent-identity-and-the-gateway).

## Repository structure

```text
services/crypton-id        Rust/Axum identity service — WebAuthn, JWT, devices, recovery, audit log, admin
services/crypton-gateway   Rust/Axum gateway — verifies crypton-id EdDSA tokens, nonce/rate-limit/risk-policy, durable decision trail (see its README for what is and is not proven)
services/crypton-console   Product frontend — register, login, devices, recovery, admin
services/crypton-main      Public site (Agent Floor) + serverless assessment-intake API in api/ (Vercel Functions + Supabase Postgres); it does not call crypton-id
services/crypton-inquiries Local / single-host assessment inbox (SQLite), same public contract as crypton-main/api; not the production path
supabase/                  Migration for the assessment_inquiries schema (least-privilege roles)
libs/crypton-sdk           TypeScript client SDK — the only thing that talks to crypton-id over HTTP
risk-engine                Rust library — risk scoring, linked into the gateway
policy-engine              Rust library — standalone policy evaluators (not linked into anything deployed)
labs/agent-lab             Agent identity, delegation, and authorization prototype (Python)
labs/exp-*, labs/validation-benchmark   Reproducible experiments and benchmark cases
assessment-kit, assessment-engagement, pilot-pack, crypton-bigidea-2026   Assessment manual, reference engagement, pilot and verification evidence
tests/runtime              Real-stack verification suite (tests/runtime/run.sh)
infra/                     docker-compose: dev Postgres+Redis, and the full single-entry-point stack
scripts/                   rust-audit.sh (RustSec gate) and its justified-exceptions list
docs/                      Documentation — start at docs/README.md (one authoritative place per topic)
```

Both frontends are kept deliberately: `crypton-console` is the product app, `crypton-main` is
the public site and assessment intake (see [`docs/repository.md`](docs/repository.md)).

## Running it locally

Prerequisites: [Docker Desktop](https://www.docker.com/products/docker-desktop/),
[Node.js ≥ 18](https://nodejs.org/), [Rust + Cargo](https://rustup.rs/).

```powershell
.\start-all.ps1              # first run creates dev.env from dev.env.example and an Ed25519 JWT keypair (needs openssl on PATH)
.\status.ps1                 # health check every service
.\stop-all.ps1                # stop (Postgres/Redis stay up — pass -StopInfra to stop those too)
```

This brings up Postgres + Redis (Docker), `crypton-id` (:8080),
`crypton-gateway` (:8090), `crypton-main` (:3000), and `crypton-console`
(:3001). Open http://localhost:3001 and register — the browser will prompt
for Touch ID / Face ID / a security key.

> **Caveat (from `docs/deployment-topology.md`, not re-run in the 2026-10-21 cleanup):**
> `start-all.ps1` predates the hardened gateway and does not provision all of the gateway's
> required environment (`JWT_PUBLIC_KEY_PEM_FILE`, `INTERNAL_TOKEN`, `DATABASE_URL`). For
> anything that must behave like the real topology, use the containerized stack:
> `tests/runtime/run.sh` (add `KEEP_STACK=1` to leave it running) or
> `infra/docker-compose.stack.yml`.

### Manual / per-service

```powershell
docker compose -f infra\docker-compose.yml up -d
cd services\crypton-id      && cargo run --release   # :8080
cd services\crypton-gateway && cargo run --release   # :8090
cd services\crypton-main    && npm install && npm start    # :3000
cd services\crypton-console && npm install && npm start    # :3001
```

### Agent Lab (no Docker/Rust needed)

```bash
cd labs/agent-lab
python -m venv .venv
./.venv/Scripts/pip install -r requirements.txt   # Windows; macOS/Linux: source .venv/bin/activate && pip install -r requirements.txt
PYTHONPATH=src python -m agentlab.cli \
  --bootstrap-support-agent --bootstrap-delegation --dev-user user-alice --task "who is cust_1?" --fake
```

No LLM API key needed for the command above (`--fake` uses a canned model
response). See [`labs/agent-lab/README.md`](labs/agent-lab/README.md) for
registering agents, granting delegations, and verifying a multi-hop chain.

### WebAuthn notes

Passkeys are bound to the exact `rpId`/origin they were registered on. A
passkey registered on `localhost:3001` will not work through a different
hostname (e.g. a tunnel) — register a fresh one if you switch. See each
service's `.env.example` for `WEBAUTHN_RP_ID`/`WEBAUTHN_ORIGIN` options.

## Where to look

```text
Authentication (WebAuthn, JWT):  services/crypton-id/src/routes/auth_webauthn.rs, services/crypton-id/src/jwt.rs
Agent identity / registry:       labs/agent-lab/src/agentlab/agents.py, identity.py
Delegation / scope attenuation:  labs/agent-lab/src/agentlab/delegation.py, chain.py
Authorization pipeline:          labs/agent-lab/src/agentlab/authorization.py
Approval (human-in-the-loop):    labs/agent-lab/src/agentlab/approval.py
The agent loop itself:           labs/agent-lab/src/agentlab/agent.py
Gateway (token, nonce, policy):  services/crypton-gateway/src/middleware/, services/crypton-gateway/README.md
Real-stack verification:         tests/runtime/ (run.sh, e2e.mjs), infra/docker-compose.stack.yml
Tests:                           services/crypton-id/src (inline), services/crypton-gateway/src, libs/crypton-sdk/src/__tests__, labs/agent-lab/tests
Handoff / how everything fits:   docs/handoff/CTO-HANDOFF.md
```

## Testing

See [`docs/testing.md`](docs/testing.md) for the full breakdown. Short version:

```sh
cd services/crypton-id && cargo test          # needs Postgres + Redis (infra/docker-compose.yml)
cd libs/crypton-sdk && npm test
cd services/crypton-console && npm test -- --watchAll=false
cd labs/agent-lab && ./.venv/Scripts/python -m pytest -q   # Windows; .venv/bin/python on macOS/Linux
tests/runtime/run.sh                          # the real services in containers, 88 checks, needs Docker + Node
```

CI ([`.github/workflows/ci.yml`](.github/workflows/ci.yml)) runs the Rust, gateway-integration,
real-stack runtime, frontend-build and `rust-audit` jobs on every push to `main`/`dev` and every
PR, plus a weekly `rust-audit`. It runs on GitHub-hosted runners. As of 2026-10-21 the `rust`,
`runtime` and `frontend` jobs pass on `main`, but `rust-audit` fails there; it needs the
rustls bump in PR #13 plus a small lockfile follow-up (see
[`docs/dependency-audit.md`](docs/dependency-audit.md)).

## Current status

Experimental, pre-customer-discovery, under active development. The
identity service (`crypton-id`) is the most mature and production-shaped
piece; `agent-lab` is a working prototype that reuses that identity but
runs standalone, not yet wired behind the gateway. Nothing in "Not
implemented" above should be assumed to exist.

## Secrets

Never commit a real JWT signing key. JWTs are asymmetric (Ed25519/EdDSA)
since v0.5.1 — see `DECISIONS.md`. Local dev: `services/crypton-id/start.sh`
and `run.bat` generate a fresh Ed25519 keypair into gitignored
`jwt_signing_key.local.pem` / `jwt_public_key.local.pem` files on first
run. `start-all.ps1` does the same for the root dev workflow. Copy
`*.env.example` files for every other value.
