# Omen Handoff — Crypton

Audit date: 2026-08-19. Updated same day after the baseline commit landed
and pushed. Old machine → new HP Omen (Windows). Practical reference
only — no roadmap, no marketing language.

## Status: committed and pushed (superseded §1 below)

- **Branch**: `refactor/agent-identity-foundation`, pushed to `origin`
  with upstream tracking set
- **Final commit**: `60740921b4935b5aebcb296c6efb691b9dd98195` —
  "refactor: establish agent identity foundation and Agent Lab v0.3"
- **Tag**: `agent-lab-v0.3` (annotated, pushed) → same commit
- **Working tree**: clean. `labs/agent-lab/` is now fully tracked (source,
  tests, docs — `.venv/`, `__pycache__/`, `.pytest_cache/`, the runtime
  SQLite DB, and trace files remain gitignored, as intended).
- Local HEAD == `origin/refactor/agent-identity-foundation` HEAD, verified
  after push.
- **Known pre-existing issue, not fixed by this commit**: a hardcoded dev
  `JWT_SECRET` that was already committed in `services/crypton-id/run.bat`
  / `start.sh` on `origin/main` prior to this session remains in git
  history (this commit replaces the mechanism going forward with a
  gitignored, auto-generated `.env.local` secret, but does not rewrite
  history). Rotate that value on any real deployment still using it.

## 1. Git state at time of original audit (historical — see status above)

- **Branch**: `refactor/agent-identity-foundation`
- **HEAD**: `4e614750542eeb5760a8d6ec6a0c989f3cb9215f` ("changed landing page")
- **This branch has no remote counterpart.** `git branch -vv` shows no
  `[origin/...]` tracking ref for it. It sits at the exact same commit as
  `main`, and `main` **is** up to date with `origin/main` (`git diff main
  refactor/agent-identity-foundation` is empty). So the *committed* history
  is safe — but see below, almost none of the current work is committed.
- **Working tree is NOT clean.**
  - **343 files staged** (25 added, 2 added-then-further-modified, 247
    deleted, 45 modified, 24 renamed) — this is the `crypton-demo` +
    `crypton-admin` → `crypton-console` consolidation, the `crypton-id`/
    `crypton-gateway` cleanup described in `DECISIONS.md`, the new CI
    workflow, new docs, and `graphify-out/` removal. **None of it is
    committed.**
  - **2 files modified on top of staging** (unstaged): `DECISIONS.md`,
    `labs/README.md`.
  - **1 untracked entry**: `labs/agent-lab/` — the entire directory. Not
    gitignored (verified with `git check-ignore`), just never `git add`-ed.
- **Agent Lab work is NOT tracked at all.** `git ls-files labs/` returns
  only `labs/README.md`. Every file under `labs/agent-lab/` — 15 source
  modules (~2050 lines), 8 test files (90 passing tests), `LEARNING.md`,
  `README.md`, `pyproject.toml` — exists only on this machine's disk. A
  `git clone` on the Omen gets **none of it**.
- **Nothing important is pushed beyond the `main` tip.** `origin/main` ==
  current HEAD, but HEAD predates the entire staged diff. A plain clone on
  the Omen reproduces the *old* repo (duplicate `crypton-demo`/
  `crypton-admin` apps, no CI workflow, no `DECISIONS.md`/`docs/`
  additions, old SDK, no Agent Lab reference) — not what's on disk here.
- Ignored-but-relevant local paths seen: `.claude/settings.local.json`,
  `.claude/worktrees/` (Claude Code local config, not app config — no
  action needed). No `.run/` present (nothing currently running).
- Other local branches on origin not otherwise referenced here: `dev`,
  `feat/gateway-hardening`, `feat/policy-engine`, `feat/risk-engine`, two
  dependabot branches. Not touched by this audit.

**Bottom line: commit + push (or otherwise transfer the working tree, not
just `git clone`) before relying on git alone to move to the Omen.** This
audit does not do that for you — see Final Decision.

## 2. Component status

| Component | Status | Verified how |
|---|---|---|
| `crypton-id` (Rust) | Present, builds blocked locally | `cargo check` → `BLOCKED: MSVC linker` (see §3) |
| `crypton-gateway` (Rust) | Present, cut to proxy+CORS+health+request-ID (310-line `main.rs`, no middleware modules on disk) | Read `src/main.rs`, matches `DECISIONS.md` |
| `crypton-sdk` (TS) | IMPLEMENTED, VERIFIED | `tsc --noEmit` clean, `jest` 10/10 pass, `npm run build` (esm+cjs) clean |
| `crypton-console` (React) | IMPLEMENTED, VERIFIED | `react-scripts test` 7/7 pass, production build succeeds |
| `crypton-main` (React, marketing) | IMPLEMENTED, VERIFIED (build only) | Production build succeeds; no test files exist (`--passWithNoTests` in CI, matches) |
| Postgres | Config present, not verified running | `infra/docker-compose.yml` defines `postgres:16-alpine`; not started this pass |
| Redis | Config present, not verified running | `infra/docker-compose.yml` defines `redis:7-alpine`, no password (documented reason in compose file); not started this pass |
| CI (`.github/workflows/ci.yml`) | IMPLEMENTED, UNVERIFIED on GitHub | File exists and is staged (new), but never pushed → has never actually run on GitHub Actions |
| `labs/agent-lab` | IMPLEMENTED, VERIFIED — **and entirely untracked by git** | 90/90 pytest pass; one fake CLI run succeeded end-to-end (see §3) |

## 3. Build/test verification actually run this pass

**Rust — BLOCKED: MSVC linker.** `rustc`/`cargo` are installed
(1.97.1), but `link.exe` is not on PATH and `cargo check` in
`services/crypton-id` fails at the link step (`icu_normalizer_data`,
`generic-array` build scripts). Per instructions, not troubleshot here.
Note for the Omen: this is unrelated to the old `.cargo/config.toml`
OpenSSL path (see §4) — the codebase itself has no `openssl-sys`
dependency (`rustls-tls` used throughout `crypton-id` and
`crypton-gateway`); the MSVC C++ Build Tools / Windows SDK are what's
actually missing, not OpenSSL.

**SDK (`libs/crypton-sdk`)**:
```
npm run typecheck   → clean
npm test             → Test Suites: 1 passed, Tests: 10 passed
npm run build        → esm + cjs, clean
```

**Console (`services/crypton-console`)**:
```
npm test -- --watchAll=false   → Test Suites: 2 passed, Tests: 7 passed
npm run build                   → Compiled successfully (77.26 kB gzip main bundle)
```

**Marketing site (`services/crypton-main`)**:
```
npm test    → no test files (exits 1 without --passWithNoTests, matches CI's flag)
npm run build → Compiled successfully (75.19 kB gzip main bundle)
```

**Agent Lab (`labs/agent-lab`)**:
```
pytest -q   → 90 passed, 1 warning (HMAC key length warning on a
              deliberately short test secret, not a real issue)
```
Fake deterministic CLI run (`python -m agentlab.cli
--bootstrap-support-agent --task "..." --dev-user user-alice --fake`,
run with `PYTHONPATH=src` and a throwaway `JWT_SECRET` — no `.env` exists
on this machine, see §4) completed end-to-end: registered a canonical
`agt_...` agent, created a session, ran a task to `completed`, wrote a
trace file. Confirms v0.1–v0.3 are not just present in source but
actually executable.

## 4. What Git will NOT bring to the Omen

| Item | Classification | Note |
|---|---|---|
| `labs/agent-lab/` (all source, tests, docs) | **RECREATE via non-git transfer, then `git add`** | Not gitignored — just never committed. Copy the folder or commit+push before cloning. |
| 343 staged + 2 further-modified files (console consolidation, Rust cleanup, CI, docs) | **RECREATE via non-git transfer, then commit+push** | Same as above — currently only on disk. |
| `services/*/.env`, `dev.env` | **RECREATE — do not copy** | None currently exist on this machine (checked; only `.env.example`/`dev.env.example` are tracked). `start-all.ps1` auto-generates `dev.env` with a fresh random `JWT_SECRET` on first run if missing. |
| Any Anthropic/OpenAI API keys | **DO NOT COPY** | Not present in any tracked or untracked file found; `labs/agent-lab/.env.example` has `ANTHROPIC_API_KEY=` blank. Set a real one on the Omen only if you want non-`--fake` LLM runs. |
| `labs/agent-lab/data/agentlab.db` | **OPTIONAL, DO NOT COPY** | Local dev SQLite, gitignored, regenerated automatically by `db.connect()`/`seed()`. Currently 86 KB, contains this pass's test run. |
| `labs/agent-lab/data/traces/*.jsonl` | **OPTIONAL, DO NOT COPY** | Gitignored generated trace files. |
| `services/crypton-id/.cargo/config.toml` | **DO NOT COPY (already being removed)** | Currently staged for deletion — it hardcoded `OPENSSL_DIR=C:\Program Files\OpenSSL-Win64` from this machine. Confirmed the codebase doesn't need it (no `openssl-sys` dep). Let the deletion land; don't recreate it on the Omen. |
| Docker volumes (`postgres_data`, `redis_data`) | **OPTIONAL** | Local dev DB contents. Fresh volumes on the Omen are fine; migrations run automatically via `sqlx` on `crypton-id` startup. |
| TLS certs (`*.pem`/`*.key`/`*.crt`) | **N/A** | None found in the repo tree; gitignore has a rule but nothing matches it currently. |
| `.claude/settings.local.json`, `.claude/worktrees/` | **OPTIONAL** | Claude Code local session state, not app config. |
| `.run/` (PID files, logs) | **N/A** | Doesn't currently exist — nothing is running. |

No secret values are reproduced above or anywhere in this document.

## 5. Omen prerequisites (minimum)

- **Git** — any recent version (this machine: 2.54.0)
- **Node.js + npm** — this machine: Node v26.3.1, npm 11.16.0 (repo has no
  stated floor; CI uses Node 20 — either works, prefer matching CI: Node 20 LTS or newer)
- **Python** — this machine: 3.13.3 (Agent Lab's `.venv` must be rebuilt
  fresh on the Omen; venvs aren't portable across machines)
- **Rust via rustup** — this machine: rustc/cargo 1.97.1, target
  `x86_64-pc-windows-msvc`
- **Visual Studio Build Tools (C++ workload) + Windows SDK** — required
  for `link.exe`. **This was the missing piece on this machine** (`where
  link.exe` found nothing) and blocked all Rust compilation here. Install
  "Desktop development with C++" from the VS Build Tools installer before
  trusting any Rust step on the Omen.
- **Docker Desktop** — for Postgres + Redis via `infra/docker-compose.yml`

Not needed: WSL, Kubernetes, a locally installed Postgres/Redis server
(Docker owns both), OpenSSL (confirmed not a dependency — see §3/§4), or
any other tooling.

## 6. Locked decisions (see `DECISIONS.md` for full text/rationale)

A future session could plausibly break these — don't, without updating
`DECISIONS.md` in the same change:

1. `HumanIdentity` / `AgentIdentity` / `AgentInstance` stay three separate
   entities, never collapsed.
2. LLMs propose tool calls; they never make the authorization decision.
3. Authorization is deterministic and checked immediately before
   execution, not cached from earlier in a session.
4. Every consequential agent action carries a resolvable
   `(human_user_id, agent_id, instance_id)` at execution time.
5. Agent services shouldn't hold token-signing/root credentials just to
   verify identity — **known open gap**, see §7.
6. Postgres owns durable security/authority state; Redis is
   ephemeral-only (challenges, logout denylist) — never system-of-record.
7. Security audit evidence is a separate artifact from ordinary logs.
8. Model providers are replaceable — no code outside a `ModelProvider`
   abstraction depends on a vendor SDK/wire format.
9. Agent frameworks (LangChain/LangGraph/CrewAI/MCP/etc.) may orchestrate
   but never become the authorization boundary.
10. Lab environments (`labs/agent-lab/` and successors) stay isolated —
    own data stores, never point at production Postgres/Redis/`JWT_SECRET`.
11. Every future vulnerability needs attack → oracle → fix → regression
    test, not just a patch.
12. `crypton-id`'s WebAuthn core isn't rewritten without a concrete
    security reason.

## 7. Known technical debt (tracked, not fixed here)

- **Invariant 5 open gap**: `crypton-id` signs JWTs with HS256
  (symmetric), so `labs/agent-lab/src/agentlab/identity.py`'s verifier
  holds the same secret and is technically capable of signing too (its
  `issue_dev_token` does this deliberately, for local dev only, and is
  unused outside dev/test paths). Real fix is asymmetric signing
  (RS256/EdDSA) in `crypton-id`/`crypton-sdk` — out of scope until that
  work is picked up. Mitigation today is operational (per-environment
  secrets), not structural.
- `README.md`'s new "Layout" section (currently staged, not yet
  committed) still describes `labs/` as "Placeholder for the next planned
  component (empty)" — stale as of this audit; `labs/agent-lab/` is a
  real, tested, 2050-line implementation. Not fixed here (doc content
  edits are out of this audit's scope) — worth a one-line fix whenever
  README is next touched.
- OAuth (`crypton-id/src/routes/oauth.rs`) remains real but unmounted
  (`ENABLE_OAUTH=false`), no PKCE, single hardcoded client — experimental
  by design, not a bug.

## 8. Local-only environment — Omen setup

### Health check (run first on the Omen)

```powershell
git --version
node --version
npm --version
python --version
rustc --version
cargo --version
docker --version
docker compose version
where.exe link.exe
```
`where.exe link.exe` must resolve to something under a Visual Studio
`VC\Tools\MSVC\...\bin\...\link.exe` path — if it prints nothing, install
the C++ Build Tools workload before trusting any `cargo` step below.

### Setup sequence

```powershell
# 1. Clone and get on the right commit — git alone is now sufficient,
#    everything (including labs/agent-lab/) is committed and pushed.
git clone https://github.com/Aryanvirpsu/Crypton-DI.git
cd Crypton-DI
git checkout refactor/agent-identity-foundation
git rev-parse HEAD
# expect: 60740921b4935b5aebcb296c6efb691b9dd98195
git tag -l agent-lab-v0.3   # should exist, points at the same commit

# 2. Root + workspace JS dependencies
npm install

# 3. Rust — sanity check before building anything real
rustc --version
cargo --version
where.exe link.exe

# 4. Start Postgres + Redis (Docker Desktop must be running first)
docker compose -f infra/docker-compose.yml up -d --wait

# 5. SDK — typecheck, test, build
cd libs/crypton-sdk
npm run typecheck
npm test
npm run build
cd ../..

# 6. Console — test, production build
cd services/crypton-console
npm test -- --watchAll=false
npm run build
cd ../..

# 7. Agent Lab — fresh venv (do not copy .venv from the old machine)
cd labs/agent-lab
python -m venv .venv
.\.venv\Scripts\pip install -r requirements.txt
copy .env.example .env
# edit .env: set JWT_SECRET (match crypton-id's dev secret if you want
# cross-verification to work), leave ANTHROPIC_API_KEY blank unless doing
# a real (non --fake) run
$env:PYTHONPATH = "src"
.\.venv\Scripts\python.exe -m pytest -q
.\.venv\Scripts\python.exe -m agentlab.cli --bootstrap-support-agent `
    --task "look up order status for customer c-1001" `
    --dev-user user-alice --fake
cd ../..

# 8. Full local stack (existing script — prefer this over ad hoc commands)
.\start-all.ps1 -KillExisting
# generates dev.env automatically on first run if missing (fresh random
# JWT_SECRET), builds Rust in release mode if no binary exists yet,
# starts Postgres/Redis, crypton-id :8080, crypton-gateway :8090,
# crypton-main :3000, crypton-console :3001
.\status.ps1     # health check all services
.\stop-all.ps1   # stop everything
```

## 9. Verification results this pass (exact counts)

| Suite | Result |
|---|---|
| `crypton-id` / `crypton-gateway` (`cargo check`) | **BLOCKED: MSVC linker** |
| `crypton-sdk` typecheck | clean |
| `crypton-sdk` tests | 10/10 passed |
| `crypton-sdk` build (esm+cjs) | clean |
| `crypton-console` tests | 7/7 passed (2 suites) |
| `crypton-console` production build | success |
| `crypton-main` tests | no test files (expected, matches CI flag) |
| `crypton-main` production build | success |
| `labs/agent-lab` pytest | **90/90 passed** |
| `labs/agent-lab` fake CLI run | success — registered agent, session, task→completed, trace written |

## 10. Exact resume point

```
COMPLETED:
- v0.1 — handwritten loop/tools/tracing/identity: VERIFIED (agent.py,
  authz.py's dict-based allowlist, tracing.py, identity.py)
- v0.2 — Sessions/Tasks/state/lifecycle: VERIFIED (sessions.py, tasks.py,
  lifecycle.py, agent_instances table)
- v0.3 — persisted Agent Registry / canonical agt_* IDs: VERIFIED
  (agents.py's register_agent → sqlite `agents` table, agt_{uuid4().hex}
  ids, enable/disable/status lifecycle)

NEXT:
- v0.4 — Delegation / AuthorizationDecision / resource constraints /
  revocation. No `Delegation` class, `AuthorizationDecision` type, or
  `revoke` logic exists anywhere in `labs/agent-lab/src/agentlab/` (grep
  confirmed zero matches). `authz.py` is explicitly documented in its own
  docstring as "just a Python dict for v0.1, not a policy engine" —
  v0.4 has not been started.
```
Not started here, per audit scope.

---

## FINAL DECISION

**SAFE TO MOVE TO OMEN**

Resolved since the original audit:
1. `labs/agent-lab/` (source, tests, docs — 90 passing tests) is now
   tracked and pushed. `.venv/`, `__pycache__/`, `.pytest_cache/`, the
   runtime SQLite DB, and trace files remain correctly gitignored.
2. The 343 staged + 2 unstaged changes from the original audit are now
   one committed, pushed baseline: `60740921b4935b5aebcb296c6efb691b9dd98195`
   on `origin/refactor/agent-identity-foundation`, tagged `agent-lab-v0.3`.
   Local HEAD == remote branch HEAD, verified after push.
3. Rust toolchain remains `BLOCKED: MSVC linker` on *this* machine — still
   informational only for the Omen (install VS Build Tools + Windows SDK
   there; the repo itself has no `openssl-sys` dependency, confirmed
   earlier). `cargo check`/`test` are unverified on this machine, not a
   reason to withhold the source itself.

Outstanding, carried forward (not blockers to moving, but real):
- A hardcoded dev `JWT_SECRET` predating this session is already in git
  history on `origin/main` (`services/crypton-id/run.bat`/`start.sh`,
  pre-existing commits). This baseline commit removes it going forward
  (replaced by a gitignored, auto-generated `.env.local`) but does not
  rewrite history. Rotate that value on any real deployment still using
  it — a history purge is a separate decision, not done here.

1. **Branch**: `refactor/agent-identity-foundation` (pushed, tracking
   `origin/refactor/agent-identity-foundation`)
2. **Final commit**: `60740921b4935b5aebcb296c6efb691b9dd98195` —
   "refactor: establish agent identity foundation and Agent Lab v0.3"
3. **Push status**: local HEAD == remote HEAD; tag `agent-lab-v0.3` pushed
   to the same commit; working tree clean
4. **Uncommitted/untracked important files**: none
5. **Agent Lab current version**: v0.3 (persisted Agent Registry with
   canonical `agt_*` IDs), VERIFIED via 90/90 pytest + a fake CLI run
6. **Exact next development chunk**: v0.4 — Delegation /
   AuthorizationDecision / resource constraints / revocation (not started)
7. **Omen setup command sequence**: §8 above (now a plain `git clone` +
   `checkout` — no manual file transfer needed)
8. **`docs/OMEN_HANDOFF.md` status**: created in the original audit pass,
   updated this pass with the final commit hash and tag
