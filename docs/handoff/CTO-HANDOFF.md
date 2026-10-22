# CTO handoff

Written 2026-10-21 against `origin/main` @ `4344036`. Every test number below was produced by a
run made on that commit during the handoff (details, commands and exceptions:
[`HANDOFF-VERIFICATION-REPORT.md`](HANDOFF-VERIFICATION-REPORT.md)). Statements about
production, customers and money are marked as not verifiable from the repository.

Reading order for a first day: this file; [`docs/README.md`](../README.md) (the map of which
document is authoritative for what); [`docs/deployment-topology.md`](../deployment-topology.md);
[`services/crypton-gateway/README.md`](../../services/crypton-gateway/README.md);
[`docs/testing.md`](../testing.md). [`AGENTS.md`](../../AGENTS.md) is the founder's operating
document (strategy, market, decision log); read it for intent, not for current engineering state.

## 1. What Crypton currently is

An early-stage, pre-customer security venture. The product thesis is identity and
authorization for AI agents: who an agent is, what authority it was actually given, whether a
specific action stays inside that authority, and evidence of what happened. The commercial
wedge (decided 2026-09-10, `AGENTS.md` §8c) is an **action-integrity assessment** for teams
that give autonomous security agents consequential permissions. Today the company has working
infrastructure, a research lab, assessment tooling and a public site. It has no customers,
no paid engagement and no production deployment that this repository can attest to.

## 2. What is genuinely implemented (verified by a run on 2026-10-21)

| Component | What it does | Evidence from this pass |
|---|---|---|
| `services/crypton-id` (Rust/Axum, Postgres, Redis) | WebAuthn passkey register/login, EdDSA JWTs, Redis logout denylist, device management, admin roles, lost-device recovery, audit log, step-up | 41 tests (real Postgres, Redis); exercised end to end by the runtime suite |
| `services/crypton-gateway` (Rust/Axum) | The stack's single external entry point: verifies `crypton-id` tokens (public key only), nonce replay protection, rate limits, risk-engine policy, durable allow/deny trail in `audit_logs`, forwards a shared secret so identity can refuse requests that bypassed it | 32 unit tests; 37 router/E2E tests against real Redis/Postgres |
| `risk-engine` (Rust library) | Deterministic risk scoring, linked into the gateway | 309 tests (+3 ignored) |
| Real-stack runtime suite (`tests/runtime`) | Builds the real `crypton-id` and gateway images, runs them with real Postgres/Redis, drives registration, login, step-up, revocation, lock-out, bypass attempts and Redis/Postgres outages with a software WebAuthn authenticator | 88/88 checks |
| `libs/crypton-sdk` (TypeScript) | Client for the gateway/identity API; adds nonce and timestamp headers | 11 tests (HTTP and WebAuthn mocked); typecheck and build clean |
| `services/crypton-console` (React/CRA) | The product UI: register, login, devices, recovery, admin | 7 tests; production build with `CI=true` |
| `services/crypton-main` (React/CRA + Vercel Functions) | Public site (Agent Floor) and the assessment-intake API in `api/` (signed challenge, validation, AES-256-GCM at rest, least-privilege Postgres roles, fail-closed config) | 123 frontend tests; 64 API tests with a real Postgres, 0 skipped |
| `labs/agent-lab` (Python) | Agent registry, scoped delegation with multi-hop chains where authority can only narrow, deterministic pre-execution authorization, human approval bound to an exact tool call, traces, and the assessment engine (`assessment/`) | 453 tests |
| Assessment methodology and kit | `docs/methodology/`, `assessment-kit/v1.0/` (operating manual), `assessment-engagement/` (reference engagement), `pilot-pack/v0.1/` | Documents; the code they cite is covered by the lab tests |

## 3. What is experimental

- **Agent Lab** is a reference implementation: SQLite, one process, private keys returned to
  the caller, approvals verified by `FakeWebAuthnVerifier` (a simulation). It verifies the same
  JWTs as `crypton-id` but never calls it, and nothing routes agent traffic through the gateway.
- **Research and experiments**: `labs/exp-003-*` (three frameworks), `labs/exp-005-openhands-sdk`,
  `labs/exp-007-principal-continuity`, `labs/experiments/001-*`. Frozen evidence; the tests
  re-run the harnesses but the *findings* are research results, not product behaviour.
- **Validation Benchmark** (`labs/validation-benchmark/v0.1`): two retrospective cases (MCP
  Python SDK CVE-2026-52869, `@bytebase/dbhub` CVE-2026-61788) showing the same test script
  separates a known-vulnerable from a known-fixed release. It shows the test discriminates,
  not that Crypton discovers vulnerabilities. It was not re-run in this pass.
- **`policy-engine`**: 35 passing tests, but not linked into anything deployed. Keep, wire or
  archive is an open decision.
- **Agent Floor and the intake path**: functionally tested locally and in
  `docs/agent-floor-readiness-2026-10-14.md`. The public site has since gone live on the
  production domain; that cutover, the production environment's identifiers and its open
  follow-ups are in the founder's private operations notes **outside this repository** (the
  repository is public, so they are deliberately not copied here). Ask the founder to hand
  them over directly; nothing in this repository lets you verify the live environment.
- **OAuth** in `crypton-id`: real state machine but no PKCE and a single hardcoded client;
  unmounted unless `ENABLE_OAUTH=true`.
- **`services/crypton-inquiries`**: a local/single-host SQLite twin of the intake API; not the
  production path.

## 4. What is not implemented

- Any agent-delegation or agent-identity check at the gateway. Agent authorization exists only
  inside Agent Lab's process.
- MCP integration; automated adversarial testing of other people's agents as a product (the
  assessment is a human-run service with tooling).
- Token key rotation (only `kid = v1` is accepted) and `iss`/`aud` claims.
- Producers for three risk-engine inputs: `actions_executed_5m`, `registrations_from_ip_10m`,
  `active_session_count`. The engine reads them; nothing writes them.
- Binding of a completed step-up to the specific action it names (any completion satisfies any
  Challenge on that token for 5 minutes).
- Any verification of production network isolation (see §8). Any test with a physical or
  platform passkey. Any customer, design-partner or external-review evidence.

## 5. Architecture and module boundaries

```text
client ─► gateway :8090 ─► crypton-id :8080 ─► Postgres, Redis
          (only published     (signs EdDSA tokens;
           port; verifies,     only issuer)
           rate-limits,
           applies risk policy,
           records decisions)
crypton-console ─(SDK)─► gateway            crypton-main ─► /api/assessment ─► Supabase Postgres
labs/agent-lab : standalone, verifies crypton-id's JWTs offline
```

| Boundary | Rule |
|---|---|
| Token contract | `crypton-id` is the only issuer: EdDSA/Ed25519, header `kid=v1`, claims `{sub, username, cred_id, iat, exp}`. Verifiers (gateway, Agent Lab) hold only the public key. Anything but EdDSA is rejected. |
| Admin authority | Tokens carry no role. `AdminUser` reads `users.role` from Postgres on every request. |
| Shared state | Gateway and identity share one Redis (denylist, signals, nonces, step-up) and one Postgres (`audit_logs`, `credentials`, `users`). Schema authority is `services/crypton-id/migrations`. |
| Library boundary | `risk-engine` is a path dependency of the gateway; `policy-engine` is unlinked. |
| Frontends | `crypton-console` talks only to the gateway through `@crypton/sdk`. `crypton-main` never calls `crypton-id`. |
| Assessment code | `labs/agent-lab/src/agentlab/assessment/` (models, report, evidence). Five outcomes: ENFORCED, VIOLATED, UNVERIFIABLE, UNREPRESENTABLE, NOT_APPLICABLE; a VIOLATED finding requires all gates satisfied. Do not change this vocabulary without a demonstrated defect and regression tests: customer reports depend on it. |

Details: [`docs/architecture.md`](../architecture.md), [`docs/repository.md`](../repository.md).

## 6. How to build and test

Prerequisites: Rust (the images pin `rust:1.89`; local runs here used 1.97.1), Node (24 used
here; CI uses 20; `crypton-inquiries` needs ≥ 24), Python 3.11+ (3.14.7 used here), Docker,
OpenSSL. Use **throwaway** Postgres/Redis; the gateway and API tests create and drop databases.

```sh
# infra for tests (loopback only, throwaway credentials)
docker run -d --name t-pg --rm -e POSTGRES_PASSWORD=<throwaway> -e POSTGRES_DB=crypton -p 127.0.0.1:55432:5432 postgres:16-alpine
docker run -d --name t-redis --rm -p 127.0.0.1:56379:6379 redis:7-alpine
openssl genpkey -algorithm ed25519 -out /tmp/jwt_signing_key.pem
openssl pkey -in /tmp/jwt_signing_key.pem -pubout -out /tmp/jwt_public_key.pem

# Rust services (env as in .github/workflows/ci.yml)
export DATABASE_URL=postgres://postgres:<throwaway>@127.0.0.1:55432/crypton REDIS_URL=redis://127.0.0.1:56379
export JWT_SIGNING_KEY_PEM_FILE=/tmp/jwt_signing_key.pem JWT_PUBLIC_KEY_PEM_FILE=/tmp/jwt_public_key.pem
export WEBAUTHN_RP_ID=localhost WEBAUTHN_ORIGIN=http://localhost:8080 WEBAUTHN_RP_NAME=test
docker exec t-pg psql -U postgres -c 'create database gwtest'   # base DB for the gateway integration tests
(cd services/crypton-id && cargo test)
(cd services/crypton-gateway && cargo test && GATEWAY_TEST_REDIS_URL=$REDIS_URL \
   GATEWAY_TEST_DATABASE_URL=postgres://postgres:<throwaway>@127.0.0.1:55432/gwtest cargo test -- --ignored)
(cd risk-engine && cargo test); (cd policy-engine && cargo test)
bash scripts/rust-audit.sh                       # RustSec gate

# Real stack: builds images, 88 checks, tears down
bash tests/runtime/run.sh

# JavaScript
npm ci && npm run build:sdk
(cd libs/crypton-sdk && npm run typecheck && npm test)
(cd services/crypton-console && CI=true npm test -- --watchAll=false && CI=true npm run build)
(cd services/crypton-main && CI=true npm test -- --watchAll=false && CI=true npm run build)
(cd services/crypton-main && ASSESSMENT_TEST_DATABASE_URL=<superuser url of a throwaway Postgres> npm run test:api)
(cd services/crypton-inquiries && npm test)

# Python (per lab: python -m venv .venv; pip install -r requirements.txt)
(cd labs/agent-lab && .venv/bin/python -m pytest -q)
```

Windows specifics (all hit during this pass): `crypton-id` needs OpenSSL development
libraries (`vcpkg install openssl:x64-windows-static-md`, `VCPKG_ROOT`); run the `.sh` scripts
through Git Bash because `bash` on PATH may be a WSL stub; running the lab suites rewrites
tracked `labs/exp-003*/evidence/*.json` (restore with `git restore`). See
[`docs/testing.md`](../testing.md).

## 7. External services and environment variables

No secrets are stored in the repository; copy each `.env.example`. Names only:

| Service | Variables (see its `.env.example`) |
|---|---|
| `crypton-id` | `APP_PORT`, `DATABASE_URL`, `REDIS_URL`, `WEBAUTHN_RP_ID`, `WEBAUTHN_ORIGIN`, `WEBAUTHN_RP_NAME`, `JWT_SIGNING_KEY_PEM[_FILE]`, `JWT_PUBLIC_KEY_PEM[_FILE]`, `GATEWAY_SHARED_SECRET`, `REQUIRE_GATEWAY_SECRET`, `BIND_ADDR`, `ENABLE_OAUTH` (+ `OAUTH_*`), `RUST_LOG` |
| `crypton-gateway` | `PORT`, `INTERNAL_PORT`, `IDENTITY_URL`, `DATABASE_URL`, `REDIS_URL`, `JWT_PUBLIC_KEY_PEM[_FILE]`, `INTERNAL_TOKEN`, `GATEWAY_SHARED_SECRET`, `REQUIRE_GATEWAY_SECRET`, `POLICY_ENFORCE`, `LOG_FORMAT` (CORS origins are hard-coded, not configurable: R16) |
| `crypton-main/api` (Vercel Functions) | `ASSESSMENT_MASTER_KEY` (64 hex), `ASSESSMENT_DATABASE_URL` (least-privilege `assessment_writer`), `ASSESSMENT_ALLOWED_ORIGINS`, optional `ASSESSMENT_DATABASE_SSL`/`_CA`; operator tooling `ASSESSMENT_OPERATOR_DATABASE_URL`; tests `ASSESSMENT_TEST_DATABASE_URL` |
| `crypton-console` / `crypton-main` | `REACT_APP_API_BASE`, `REACT_APP_MAIN_URL` / `REACT_APP_DEMO_URL` |
| `crypton-inquiries` | `INQUIRY_DB`, `INQUIRY_ORIGIN`, `HOST`, `PORT` |
| `labs/agent-lab` | `JWT_PUBLIC_KEY_PEM`, optional `ANTHROPIC_API_KEY` and `AGENT_LAB_MODEL` (live model only; tests and `--fake` need no key) |

Third parties: Postgres 16 and Redis 7; Supabase Postgres ≥ 15 for the intake schema
(`supabase/migrations`); Vercel (frontends and intake functions; `vercel.json` in each
frontend); Railway (`railway.toml` for identity and gateway; marked "Planned" in `AGENTS.md` §4);
GitHub Actions; Anthropic API (optional, labs only).

## 8. Security model and invariants

Enforced and tested today (each is a regression target; do not weaken):

1. `crypton-id` is the only token issuer; verifiers cannot mint. Non-EdDSA, unknown `kid`, bad
   signature, expired, missing claims and non-UUID subjects are rejected (gateway tests list 11
   forged shapes).
2. Revocation: Redis logout denylist plus "credential still `active` for this user", checked
   per request. The gateway **fails closed** (503) if Redis/Postgres cannot answer.
3. The public authentication routes are an exact method-and-path allowlist; everything else
   without a valid token is 401. A presented token must be valid even on a public route.
4. Nonces are single-use, scoped to the authenticated user; replay of a captured request lands
   in the victim's scope and is rejected.
5. A request is allowed only after its allow decision is durably recorded; if it cannot be
   recorded the request is refused.
6. Step-up completion is written only after server-side WebAuthn verification and is bound to
   user and session; the policy downgrades only a Challenge, never a Hold or Deny.
7. With `GATEWAY_SHARED_SECRET` set, identity refuses everything except `/health` without the
   gateway's header; with `REQUIRE_GATEWAY_SECRET=true` a service refuses to start without it.
8. Admin is decided from `users.role` on each request; the UI hiding admin links is not a boundary.
9. The intake API answers "received" only after the database commit, never logs payloads or
   connection strings, and returns 503 on any configuration it cannot validate.
10. Agent Lab: authority can only narrow along a delegation chain; the model cannot override
    the authorization pipeline; an approval is bound to one exact tool call.

Known weaknesses (all unfixed on purpose; see §10): `crypton-id`'s own extractor skips the
logout-denylist check when Redis is unreachable (`services/crypton-id/src/jwt.rs:148`); safe
only while identity is reachable solely through the gateway. `POST /admin/bootstrap` promotes
the first authenticated caller to `super_admin` while no admin exists, so a fresh deployment
must be bootstrapped by the intended person before it is exposed. The gateway-secret defence is
opt-in. No production platform's isolation has been verified.

## 9. Assessment methodology and its limits

The service is a bounded assessment of one agent workflow: map the authority path, test the
property families in `pilot-pack/v0.1/03-test-catalog.md` (four scored core families, two
conditional modules, one research preview), and report reproducible findings with expected vs
observed behaviour. Operating procedure: `assessment-kit/v1.0/` (15 documents, with an
independent re-review in `VALIDATION-REPORT.md`). Triage rule `§9a`: a candidate failure is
not a security failure until the mutator, its trust level, whether the framework permits it and
whether a claimed boundary was crossed are established.

Limits to state to anyone relying on it: no external customer engagement has happened; the
sample report is a self-assessment; both benchmark cases are retrospective; the Python corpus
is the only validated target shape; the external methodology review has not been sent
(`pilot-pack/v0.1/EXTERNAL-REVIEW-BRIEF.md`). The kit's own verdict is READY WITH CONDITIONS.
A signed Authorization-to-Test and a legal entity are a hard gate before any paid work
(`docs/business/ASSESSMENT_PACK.md` §C).

## 10. Open bugs, technical debt and unresolved risks

| # | Item | Class | Evidence / where |
|---|---|---|---|
| R1 | Hosted CI `rust-audit` is **red on `main`**; PR #13 alone leaves two advisories (`-0381`, `-0382`) | Dependency, confirmed | `docs/dependency-audit.md` status update; runs 38959492440, 39260629639 |
| R2 | `risk-engine`, `policy-engine`, all Python labs, API and inquiries tests are **not run by CI** | Process gap, confirmed | `.github/workflows/ci.yml` |
| R3 | Identity's `AuthUser` fails open on Redis outage | Security hypothesis→confirmed in code | `jwt.rs:148`; gateway documents fail-closed instead |
| R4 | Admin bootstrap is first-caller-wins | Design risk, documented | `routes/admin.rs`, `docs/architecture.md` |
| R5 | Production isolation, gateway-secret opt-in, no `iss`/`aud`, no key rotation | Unverified / not built | `docs/deployment-topology.md` |
| R6 | Three risk-signal inputs have no producer; step-up not action-bound; two failed assertions in an hour deny all requests on issued tokens | Functional gap, documented | gateway README "Remaining gaps" |
| R7 | `cargo fmt --check` fails for `risk-engine` (506 hunks) and `policy-engine` (50); clippy: 61 and 2 warnings in those, 6 in `crypton-id`, 0 in the gateway | Hygiene, confirmed | run logs; not fixed here (a 500-hunk reformat should be its own reviewed change) |
| R8 | 97 npm findings (4 critical) via `react-scripts` 5 (end of life) and Jest; not in the browser bundle per `dependency-audit.md`, not re-analysed this pass | Dependency | `npm audit` |
| R9 | `start-all.ps1` does not provision the gateway's required environment | Dev-experience defect, documented | `docs/deployment-topology.md` |
| R10 | Running the lab tests rewrites about two dozen tracked `labs/exp-003*/evidence/*.json` files (new timestamps and ids) | Reproducibility defect | restore with `git restore` |
| R11 | Python labs use `>=` ranges and only EXP-003C has a lock file; one interpreter (3.14.7) tested | Reproducibility | `requirements*.txt` |
| R12 | `risk-engine/log.txt`, `failed_scenario.md`, `test_results.md` are tracked April-2026 artifacts (the first is linked from a guide); `failed_scenario.md` describes a test failure that no longer occurs (309 tests pass) | Stale artifacts, kept as history | not deleted |
| R13 | Branch model (`main`/`dev`/`lab`) is documented but not followed; PRs #9 (stale since June) and #10 target other branches | Process | `docs/repository.md` |
| R14 | `.env.production.example` carried dev credentials and no gateway-secret guidance | Config hygiene | fixed in this change (example file only) |
| R15 | The public site is live, but its production configuration and open operational follow-ups are documented only in private notes and were not verified in this pass | Deployment / operations | private operations notes (founder) |
| R17 | (low) `python -m agentlab.assessment.reference_dryrun` overwrites the tracked `assessment-engagement/reference-agentlab-dryrun/` files and does **not** reproduce them: the regenerated copy records the current commit as `target_commit` (committed: `ca2f52f`) and has `authority_map: null` (committed: a populated Authority Model section). Which step produced the committed variant was not investigated | Reproducibility gap, confirmed (low) | run on 2026-10-21; files restored afterwards |
| R16 | The gateway's CORS allowlist is hard-coded (`app.cryptonid.tech`, `demo.cryptonid.tech`, two localhost origins; `services/crypton-gateway/src/main.rs:160`); the `ALLOWED_ORIGINS` variable once shown in `dev.env.example` was read by nothing (removed from the example in this change) | Config defect, confirmed | `main.rs`; any new frontend origin needs a code change |

## 11. Existing PRs and deployment state

| PR | Purpose | State / recommendation |
|---|---|---|
| [#13](https://github.com/Aryanvirpsu/Crypton-DI/pull/13) | rustls 0.23.46 (`-0346/-0347/-0348`; also clears `-0349`) | Green when it ran (2026-10-16). Merge it **with** the lockfile follow-up in `dependency-audit.md` (verified on a throwaway worktree: 0 unlisted vulnerabilities, `cargo check --all-targets` passes for gateway and identity) |
| [#10](https://github.com/Aryanvirpsu/Crypton-DI/pull/10) | Agent Floor RC1 into `dev` | Its branch (`release/agent-floor-rc1`) is fully contained in `origin/main` (0 unique commits); close |
| [#9](https://github.com/Aryanvirpsu/Crypton-DI/pull/9) | gateway hardening into `feat/risk-engine` | Stale since 2026-06-24; `feat/gateway-hardening` is fully contained in `origin/main`; close |
| [#6](https://github.com/Aryanvirpsu/Crypton-DI/pull/6), [#5](https://github.com/Aryanvirpsu/Crypton-DI/pull/5) | Dependabot npm bumps | Obsolete lockfile bumps (#5 touches a service that no longer exists); regenerate rather than merge |

Merged to `main`: #7 risk engine, #8 policy engine, #11 Agent Floor + intake, #12, #14.
Branch audit (2026-10-21): every local and remote branch except `chore/rustls-0.23.46-advisories`
(1 unique commit, PR #13) is already fully contained in `origin/main`, so none holds unique
work. The founder's other local worktrees (`Crypton-DI-floor`, `-leads`, `-release`,
`-rustls`) sit on those same branches. Deployment: nothing in the repository proves a running
production environment; Vercel and Railway are configured as targets only.

## 12. Recommended technical priorities (ranked)

1. **Turn CI green and keep it honest.** Merge #13 plus the lockfile follow-up; decide whether
   `rust-audit` is a required check; add `risk-engine`, `policy-engine`, the Python labs, API
   and inquiries tests to CI (R1, R2).
2. **Settle the branch model** and close the dead PRs (R13). Fast-forward your local `main`
   (it is 46 commits behind `origin/main`).
3. **Make identity fail closed on Redis** with a regression test, as the gateway does (R3);
   make `REQUIRE_GATEWAY_SECRET` the default for production configs.
4. **Take over the live environment**: get the private operations notes from the founder,
   then verify the gateway/identity deployment (private networking, the shared secret) and
   the intake database (backups, retention, credential rotation) (R5, R15).
5. **Close or retire the risk-engine gaps**: producers for the three signals, action-bound
   step-up, and a decision on the two-failures-deny behaviour (R6).
6. **Assessment readiness**: keep `labs/agent-lab/src/agentlab/assessment/` stable, make
   evidence generation deterministic (R10), pin Python dependencies and test two interpreters
   (R11). Support the first customer-controlled engagement as the kit prescribes.
7. **Hygiene** in separate, reviewable changes: format `risk-engine`/`policy-engine` and gate
   it in CI (R7), decide on `policy-engine` (wire or archive), stale artifacts (R12).
8. **Frontend tooling**: migrate off Create React App and upgrade Jest (R8). Not urgent for
   security per the existing analysis, but it blocks `npm audit` ever being green.

## 13. Ownership and escalation

| Owner | Decides |
|---|---|
| **CTO** | Architecture, code quality, test and CI policy, dependency updates and advisories, branch workflow, build/release mechanics, infrastructure and reliability, technical feasibility statements, the technical content of assessment tooling and reports |
| **Founder** | Customer discovery, sales, partnerships, pricing and commercial terms, what is promised to a customer, which customers to pursue, outside methodology review outreach, legal entity and contracts |
| **Joint** | Scope of any customer engagement, delivery dates and commitments, cost of infrastructure, major roadmap or wedge changes (`AGENTS.md` §8, §13 kill criteria), security-sensitive disclosures (any finding about a third party's product, vulnerability reports), anything that changes a public claim about what Crypton does, production access for testing |

Escalate to the founder before: contacting a third party about a vulnerability; accepting
access to a customer's systems; agreeing a delivery date; changing what the website, README
or pilot pack claims. The decision log in `AGENTS.md` §15 is append-only: add a row when an
assumption is tested; do not reopen a settled decision without new evidence.
