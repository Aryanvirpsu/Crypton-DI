# Handoff verification report

Date: 2026-10-21. Branch `chore/cto-handoff-cleanup`, based on `origin/main` @
`4344036` (merge of PR #14). Companion documents: [`CTO-HANDOFF.md`](CTO-HANDOFF.md),
[`PRE_CLEANUP_BASELINE.md`](PRE_CLEANUP_BASELINE.md), [`GRAPHIFY-NOTES.md`](GRAPHIFY-NOTES.md).

Every result in section 4 was produced by a command run on this commit during this cleanup.
No count was copied from an older report. Raw logs are kept outside the repository
(`..\Crypton-DI-snapshots\cleanup-run-logs\main\`) so the repository does not grow by tool
output.

## 1. Repository health

| Item | State |
|---|---|
| Base | `origin/main` @ `4344036`. The founder's checked-out branch, `verification/big-idea-stabilization` @ `4d0257e`, is an ancestor of `main` and 34 commits behind it; an earlier pass on that branch was discarded as stale before any commit |
| Cleanliness | The cleanup changes 14 files, all documentation, example configuration or CI metadata (section 5). No source, lockfile or test was changed. Tests rewrote tracked evidence files twice; they were restored and the tree contains no such diffs |
| Founder's working copy | Untouched. `git status` there is byte-identical to the baseline (11 modified/untracked entries preserved); a full snapshot exists outside the repository |
| Branches | Every local and remote branch except `chore/rustls-0.23.46-advisories` is fully contained in `origin/main`. Local `main` is 46 commits behind `origin/main` (not touched) |
| Dependencies | Rust: 6 unlisted advisories per lockfile in `crypton-id`, gateway and `risk-engine` (section 3). npm: 97 findings at the root (4 critical), 93 in `crypton-main`'s own lockfile, via build/test tooling; not re-analysed for reachability. Python labs: `>=` ranges, no lock files except EXP-003C |
| Reproducibility | Rust, JavaScript, Docker and Python suites reproduce from a clean checkout with the steps in `CTO-HANDOFF.md` §6. Two reproducibility defects: lab tests rewrite tracked evidence JSON; the reference-engagement CLI does not regenerate the committed files |
| Hosted CI (GitHub API) | Push of `4344036`: `rust`, `runtime`, `frontend` succeeded; `rust-audit` **failed**. Weekly scheduled run: `rust-audit` failed |

## 2. Architecture

See `CTO-HANDOFF.md` §5 for components and boundaries; `docs/architecture.md` and
`docs/deployment-topology.md` are the long-form sources. Short form: clients reach only the
gateway; it verifies `crypton-id` tokens with a public key, enforces nonce, rate-limit and
risk policy, records every decision, and proxies to `crypton-id`, which is the sole token
issuer and talks to Postgres and Redis. `crypton-main` has its own intake API and database;
Agent Lab is a standalone Python lab that verifies the same tokens offline. Operational
dependencies: Postgres 16, Redis 7, Supabase Postgres, Vercel, Railway (targets only).

## 3. Security

**Confirmed findings** (class: confirmed in code or by a run, not hypothesis):

| # | Finding | Severity (author's judgement) | Status |
|---|---|---|---|
| S1 | `rust-audit` fails on `main`: `RUSTSEC-2026-0346/-0347/-0348/-0349` (`rustls` 0.23.45), `-0381` (`webpki-roots`), `-0382` (`rustls-webpki` 0.103.15) in three lockfiles. The two newest were published 2026-10-19/20, after PR #13 last ran | Medium (dependency; reachability of each was not re-analysed; the 2026-10-13 record says the earlier `rustls` findings were compiled into shipped binaries) | Open. PR #13 clears `-0346..-0349`; a lockfile-only follow-up clears the rest (verified on a throwaway worktree: 0 unlisted vulnerabilities, 17 lines added and 17 removed, `cargo check --all-targets` passes for gateway and identity). Not applied here |
| S2 | `crypton-id` `AuthUser` skips the Redis logout-denylist check when Redis is unreachable (`jwt.rs:148`); the gateway fails closed | Medium if identity is ever reachable without the gateway; low in the documented topology | Open, documented |
| S3 | `POST /admin/bootstrap` promotes the first authenticated caller to `super_admin` while no admin exists | Medium operational risk at first deployment; by design | Open, documented |
| S4 | `GATEWAY_SHARED_SECRET` defence is opt-in; `.env.production.example` gave no guidance and shipped `postgres:postgres` | Low | Example file corrected in this change |
| S5 | Gateway CORS origins hard-coded; an unused `ALLOWED_ORIGINS` example misled | Low | Example corrected; code unchanged |
| S6 | CI workflow had no `permissions:` block (default token scope), actions pinned to mutable tags | Low | `permissions: contents: read` added; tag pinning left to the CTO |
| S7 | Step-up not bound to an action; three risk inputs have no producer; two failed assertions in an hour deny all requests on issued tokens | Functional/security gap, documented | Open |

**Secret scan** (patterns for private keys, cloud/API tokens, JWTs and credentialed
connection strings over the tracked tree, excluding `node_modules`, `.venv`, `target`, build
output): no real credential found. Matches are (a) the same two fixed Ed25519 test keypairs
in `services/crypton-id/src/jwt.rs` (`#[cfg(test)]`) and `services/crypton-gateway/src/gateway_tests.rs`,
documented as non-secret; (b) placeholder or local-dev connection strings (`postgres:postgres`)
in examples, CI service containers and `start.sh`/`run.bat`; (c) obviously fake values in tests.
No `.env`, `.pem`, `.key`, database or private-data file is tracked (only `*.example`).
`crypton-bigidea-2026/evidence/secret-scan.txt` is intentionally skipped by Graphify as
sensitive-looking; it was not opened.

**Controls tested in this pass** (all by the runtime suite and gateway tests listed below):
forged/invalid token shapes rejected with recorded reasons; revoked, lost and foreign
credentials rejected; logout denylist; exact public-route allowlist with near-miss bypass
attempts; nonce single use and replay in the victim's scope; rate limits; decision trail with no
secrets; allow refused when the trail cannot be written; identity unreachable around the
gateway (network and shared secret); fail-closed behaviour under Redis and Postgres outage;
startup refusal without the gateway secret; account lock; step-up changing a Challenge
decision; intake API fail-closed configuration, forged/expired tokens, encryption at rest,
least-privilege database roles.

**Not tested / limits**: production platform networking; a physical or platform
authenticator (the runtime suite uses a software ES256 `none`-attestation authenticator that
`webauthn-rs` verifies for real); two-actor recovery and device-added signals; the frontends
against a live gateway; any real browser; Postgres outage during auth; load/performance;
container image vulnerability scanning; whether any advisory is exploitable in a shipped
binary.

## 4. Test results (fresh, 2026-10-21)

Environment: Windows 11 Home (10.0.26200); cargo/rustc 1.97.1; Node 24.19.0, npm 11.17.0;
Python 3.14.7; Docker Desktop 29.7.2; throwaway `postgres:16-alpine` and `redis:7-alpine`
containers on `127.0.0.1:55432` and `127.0.0.1:56379` with generated credentials (removed
afterwards); OpenSSL via vcpkg (`VCPKG_ROOT=C:\dev\vcpkg`); shell scripts through Git Bash.
No production or shared resource was contacted, except read-only calls to crates.io, the
RustSec advisory database, npm, PyPI and the public GitHub API.

"Pre-existing" = the condition was present on `origin/main` before any change in this branch.
Because no source changed, every failure below is pre-existing.

| Group | Command (from the directory shown) | Result | Counts | Exceptions / notes |
|---|---|---|---|---|
| `crypton-id` format | `services/crypton-id`: `cargo fmt --check` | Pass | | |
| `crypton-id` lint | `cargo clippy --all-targets` | Exit 0 | 6 warnings (4 collapsible `if`, 1 empty line after doc comment, unread fields `AdminUser.cred_id`/`.role`) | Pre-existing; left alone in authentication code |
| `crypton-id` tests | `cargo test --no-fail-fast` with `DATABASE_URL`, `REDIS_URL`, JWT key files, `WEBAUTHN_*` | **Pass** | 41 passed, 0 failed | Redis-backed denylist test ran (Redis was provided) |
| Gateway format / lint | `services/crypton-gateway`: `cargo fmt --check`; `cargo clippy --all-targets` | Pass; exit 0 | 0 warnings | |
| Gateway unit tests | `cargo test --no-fail-fast` | **Pass** | 32 passed; 37 ignored (need infra) | |
| Gateway integration | `cargo test -- --ignored` with `GATEWAY_TEST_REDIS_URL`, `GATEWAY_TEST_DATABASE_URL` | **Pass** | 37 passed | Throwaway infra only |
| `risk-engine` format | `risk-engine`: `cargo fmt --check` | **Fail** | 506 diff hunks | Pre-existing; not in CI; not reformatted here |
| `risk-engine` lint / tests | `cargo clippy --all-targets`; `cargo test --no-fail-fast` | Exit 0; **Pass** | 61 warnings; 309 passed, 0 failed, 3 ignored | |
| `policy-engine` | `policy-engine`: `cargo fmt --check`; `cargo clippy`; `cargo test` | **Fail**; exit 0; **Pass** | 50 hunks; 2 warnings; 35 passed | Pre-existing |
| Rust security audit | `"C:\Program Files\Git\bin\bash.exe" scripts/rust-audit.sh`; per-lockfile detail from `cargo audit --file <lock> --json` with the ignore list | **Fail** (exit 1) | `crypton-id` 6, gateway 6, `risk-engine` 6 unlisted vulnerabilities; `policy-engine` 0; 1 allowed warning (`rustls-pemfile` unmaintained) | Pre-existing; matches the hosted CI failure. Same audit on PR #13's lockfiles: 2 each; with the follow-up bump: 0 |
| Real-stack runtime | `"C:\Program Files\Git\bin\bash.exe" tests/runtime/run.sh` (builds both images from source) | **Pass** | 88/88 checks | A first attempt with `MSYS_NO_PATHCONV=1` failed on key-file paths (environment); passed with default path conversion |
| SDK | `libs/crypton-sdk`: `npm run typecheck`; `npm run build`; `npm test` | **Pass** | 11 tests, 2 suites | HTTP and WebAuthn mocked |
| Console | `services/crypton-console`: `CI=true npm test -- --watchAll=false`; `CI=true npm run build` | **Pass** | 7 tests, 2 suites; build succeeds with lint warnings as errors | |
| Public site | `services/crypton-main`: `CI=true npm test -- --watchAll=false --passWithNoTests`; `CI=true npm run build` | **Pass** | 123 tests, 9 suites; build succeeds | |
| Intake API | `services/crypton-main`: `npm run test:api` with `ASSESSMENT_TEST_DATABASE_URL` = throwaway Postgres | **Pass** | 64 tests, 13 suites, 0 skipped | Without that variable the DB group is skipped and reported as skipped |
| Inbox service | `services/crypton-inquiries`: `npm test` | **Pass** | 20 tests | |
| Agent Lab | `labs/agent-lab`: `python -m pytest -q -p no:cacheprovider` (fresh venv from `requirements.txt`) | **Pass** | 453 passed | |
| EXP-007A | `labs/exp-007-principal-continuity`: pytest (agent-lab venv); `python -m exp007.scenario_007a` | **Pass** | 16 passed; runner prints `DISTINCTION CHECK: OK` | |
| EXP-003 OpenAI / Microsoft / LangGraph | each lab's own fresh venv: pytest | **Pass** | 17 / 12 / 12 passed | These runs rewrote tracked `evidence/*.json`; restored |
| Representative assessment dry-run | `labs/agent-lab`, `PYTHONPATH=src`: `python -m agentlab.assessment.reference_dryrun`; plus the 6-observation delegation demo `crypton-bigidea-2026/demo/run_demo.py` | Pass / pass | dry-run exit 0; demo "ALL OBSERVATIONS MATCH EXPECTED" | The dry-run overwrote tracked files with a non-identical result (CTO-HANDOFF R17); restored |
| npm audit | `npm audit --json` at the repo root and in `services/crypton-main` | Findings | 97 (4 critical, 78 high, 10 moderate, 5 low); 93 in `crypton-main` | Dev/build tooling per the 2026-10-13 analysis; not re-analysed |
| Graphify regression | AST extraction on `main`, then check for the three false `Outcome` edges | Reproduced and corrected | 3 false edges found and removed; structural graph 5,083 nodes, 12,403 edges, 229 communities | Semantic layer not regenerated; see `GRAPHIFY-NOTES.md` |
| Follow-up lockfile bump (not committed) | throwaway worktree of PR #13: `cargo update -p rustls-webpki@0.103.15 -p webpki-roots@<ver>`; `cargo audit`; `cargo check --all-targets` | Pass | 0 unlisted vulnerabilities in 3 lockfiles; gateway and identity compile | Patch saved outside the repository; worktree removed |

**Totals passed locally:** 454 Rust tests (41 `crypton-id` + 69 gateway [32 + 37 integration] +
309 `risk-engine` + 35 `policy-engine`), 88 runtime checks, 225 JavaScript tests (11 + 7 + 123 +
64 + 20), 510 Python tests (453 + 16 + 17 + 12 + 12). Nothing failed. Not run: the 3 tests
`risk-engine` marks `#[ignore]`.

**Not run, with reason**
- Validation Benchmark cases and EXP-005 / EXP-001: need pinned third-party installs or a
  model key; unchanged since they were last verified, and not part of the product test surface.
- `start-all.ps1` end to end: starts local services and Docker; documented as not provisioning
  the gateway environment.
- Anything against production, Supabase, Vercel or Railway.

**Earlier discarded pass.** A first pass on the stale base (`4d0257e`) produced matching
counts for the suites that exist in both trees (for example Agent Lab 453, `crypton-id` 38 at
that commit versus 41 now). It is superseded and was not used.

## 5. Cleanup changes

Documentation, example configuration and CI metadata only.

| File | Change | Why | Regression risk |
|---|---|---|---|
| `README.md` | Intro, repository table (adds `crypton-main` backend, `crypton-inquiries`, `supabase/`, `scripts/`), stale "on `dev`" wording, CI status, `start-all.ps1` description and caveat, "where to look" | Claimed `crypton-main` had no backend and CI had never run on hosted runners; both false | None (text) |
| `AGENTS.md` | Gateway row qualifier (`dev`/`main`), identity-base row (software authenticator in the runtime suite), one appended Decision Log row | Factual drift only; strategy untouched; log is append-only | None (text). Founder-owned document |
| `docs/architecture.md`, `docs/repository.md` | `crypton-main` description, new components, actual branch/PR state alongside the old rule | Stale statements | None |
| `docs/testing.md` | Hosted CI status, what CI does not run, fresh results, Windows notes | `rust-audit` had run and was failing; coverage gaps undocumented | None |
| `docs/dependency-audit.md` | 2026-10-21 status update and the verified follow-up bump | Counts were stale; gate red | None |
| `docs/README.md` | Map row for `docs/handoff/` | Discoverability | None |
| `services/crypton-gateway/README.md` | "On `dev`" → "On `main` (and `dev`)" | Stale qualifier | None |
| `dev.env.example` | Removed the dead `ALLOWED_ORIGINS` line, explained | Read by nothing | Low: `start-all.ps1` copies this file to `dev.env`; the removed key was not read by any code (`git grep`) |
| `services/crypton-id/.env.production.example` | Placeholder DB URL; gateway-secret block with `REQUIRE_GATEWAY_SECRET=true` | Weak default and missing hardening guidance in a production template | Low: anyone copying it must now set two values; both are documented requirements |
| `.github/workflows/ci.yml` | `permissions: contents: read` | Least privilege | Low: YAML validated locally; **not run on a hosted runner**. No job writes to the repository |
| `docs/handoff/*` | Four new documents | The handoff | None |

## 6. Known limitations of this verification

Single Windows machine and one Python/Node/Rust version each. Hosted CI results are quoted from
the GitHub API, not reproduced. The security review was a targeted read of the authentication,
gateway, intake and configuration code plus pattern scans, not a full audit; the other Rust
modules, `risk-engine` internals, the frontends and the lab code were covered by their tests,
not by line-by-line review. PR review comments and check details beyond job-level results were
not read (`gh` is unauthenticated here). The production deployment cannot be seen from the repository.

## 7. CTO priorities (ranked)

1. Turn CI green: merge PR #13 with the lockfile follow-up; decide whether `rust-audit` is required.
2. Run the suites CI skips (`risk-engine`, `policy-engine`, Python labs, API, inquiries).
3. Decide the branch model; close PRs #9, #10, #5, #6; update local `main`.
4. Make identity fail closed on Redis outage; default `REQUIRE_GATEWAY_SECRET` on in production configs.
5. Verify one real deployment end to end (isolation, secrets, backups, public-origin intake).
6. Close the risk-engine gaps (signal producers, action-bound step-up, lock-out tuning).
7. Assessment readiness: deterministic evidence output, pinned Python dependencies, reproducible reference engagement.
8. Format and lint gates for the Rust libraries; decide the fate of `policy-engine`.
9. Replace Create React App / upgrade Jest.

## 8. Founder responsibilities

Customer discovery and the kill-criteria checkpoints in `AGENTS.md` §13; sales, pricing and
contract terms; the legal entity and signed Authorization-to-Test before any paid work;
sending `pilot-pack/v0.1/EXTERNAL-REVIEW-BRIEF.md` to outside reviewers; partnership and
design-partner conversations; stating to customers only what the README and pilot pack support;
the production domain cutover decision.

## 9. Joint decisions

Scope and dates of any customer engagement; infrastructure spend; any roadmap or wedge
change; disclosure of any finding about a third party's product; any change to public claims
about what Crypton does; granting or accepting access to a customer's or partner's system.
