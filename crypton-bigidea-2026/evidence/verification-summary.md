# Full Test and Benchmark Report — code freeze `1b93abe887c9e28f3217cb67e0e707a5a9f618f6`

Run 2026-10-12 by `crypton-bigidea-2026/run_all.sh`. Raw output for every row: `regression/<suite>.txt`; one-line-per-suite table: `regression/summary.tsv`. Environment: see `repo-version.txt`. All exit statuses are the process exit codes.

## 1. Regression results vs. the original baseline

Baseline = the first verification run earlier on 2026-10-12 (`baseline-pre-stabilization/`): **542 passed, 16 failed**.

| Suite | Command (from the directory shown) | Exit | Baseline | Final | Notes |
|---|---|---|---|---|---|
| Agent Lab | `labs/agent-lab`: `.venv/Scripts/python.exe -m pytest -v -rs -p no:cacheprovider` | 0 | 453 pass | **453 pass, 0 fail, 0 skip** | |
| EXP-007A tests | `labs/exp-007-principal-continuity`: `<agent-lab venv python> -m pytest -v` | 0 | 16 | **16 pass** | stdlib-only target; pytest from the agent-lab venv |
| EXP-007A runner | same dir: `python -m exp007.scenario_007a` | 0 | exit 0 | exit 0; `vulnerable_policy`: FAIL, `secure_policy`: PASS | expected by design |
| EXP-003C LangGraph | `labs/exp-003c-langgraph`: `.venv/.../python -m pytest -v` | 0 | 12 | **12 pass** | own venv |
| EXP-003 OpenAI Agents SDK | `labs/exp-003-openai-agents-sdk`: same | 0 | 17 | **17 pass** | own venv |
| EXP-003 Microsoft AF | `labs/exp-003-microsoft-agent-framework`: same | 0 | 12 | **12 pass** | own venv |
| Rust `crypton-id` | `services/crypton-id`: `cargo test --no-fail-fast` with `DATABASE_URL` -> isolated Postgres 16.15 | 0 | 22 pass, **16 fail** | **38 pass, 0 fail** | the 16 are the previously failing DB tests |
| Rust `crypton-gateway` | `services/crypton-gateway`: `cargo test --no-fail-fast` | 0 | 0 tests | 0 tests | no tests exist; it is a proxy, not a PEP |
| JS SDK | `libs/crypton-sdk`: `npx jest` | 0 | 10 | **10 pass** | HTTP is mocked |
| JS SDK typecheck | `libs/crypton-sdk`: `npx tsc --noEmit` | 0 | not run | clean | new check |
| Demo CRYPTON-DEMO-001 | `labs/agent-lab` venv: `python crypton-bigidea-2026/demo/run_demo.py` | 0 | ok | all 6 observations match | |
| **Totals** | | | **542 pass / 16 fail** | **558 pass / 0 fail** | |

Accounting: 558 − 542 = 16 = exactly the Rust tests that moved from failing to passing. **No test was added, removed or skipped**, and every other suite's count is unchanged. The cleanup commit (unused imports only) was verified by rerunning agent-lab (453), exp-003 OpenAI (17) and exp-003C (12) before it was committed, and again in this freeze run.

## 2. The 16 Rust failures
Root cause: **environmental**. Every one panicked with `DATABASE_URL must be set` (sqlx `#[sqlx::test]` needs a database). With an isolated PostgreSQL 16.15 container (ephemeral, loopback-only, throwaway credentials, no volume), all 38 tests pass at the unmodified code. **No Rust source defect was found, and no Rust code was changed.** Redis was not needed.

## 3. External validation benchmark (independent regression reproduction)

Versions were checked against the GitHub Advisory Database (`benchmark/advisory-*.json`), not only the repo's README, and installed into fresh isolated environments from the public registries (hashes/lockfiles in `benchmark/`).

| Case | Package / advisory | Version | Crypton test verdict | Exit | Expected |
|---|---|---|---|---|---|
| 01 Principal/session binding | `mcp` (PyPI), CVE-2026-52869 / GHSA-jpw9-pfvf-9f58, CVSS 7.1 | 1.27.1 (vulnerable, `<= 1.27.1`) | **FAIL — VULNERABLE** (baseline 400, cross-principal 400) | 0 | FAIL |
| 01 | | 1.27.2 (first patched) | **PASS — SECURE** (baseline 400, cross-principal 404) | 0 | PASS |
| 02 Authorization boundary (readonly) | `@bytebase/dbhub` (npm), CVE-2026-61788 / GHSA-mwwr-p57h-56pf, CVSS 7.4 | 0.22.5 (vulnerable, `< 0.22.6`) | **FAIL — VULNERABLE** (write under `readonly:true` changed the row) | 0 | FAIL |
| 02 | | 0.22.6 (first patched) | **PASS — SECURE** (write rejected: "attempt to write a readonly database") | 0 | PASS |

Exit code 0 on all four is by design (the negative control held, so the verdict is trustworthy). Values match the JSON previously committed in each case's `evidence/` folder. Run on Python 3.14.7 / Node 24.19.0 (the original run used earlier interpreters).

**What this does and does not show.**
- *Independent regression reproduction* — yes: the vulnerable/fixed pairs come from the vendors' public advisories; anyone can rerun the same scripts against the same pinned packages (`labs/validation-benchmark/v0.1/REPRODUCE.md`).
- *Crypton's own detection capability* — limited: both cases are retrospective, and each script was written after reading the advisory and release diff. They show the tests can tell a known-vulnerable release from its known-fixed successor, not that Crypton would have discovered either flaw. Each probes one property in-process.

## 4. Invariants
No authorization, delegation, approval or cryptographic code was modified (only unused imports in `authority_model/*` and in tests). The security-invariant suites — `test_delegation_chain.py`, `test_attenuation.py`, `test_enforcement_boundary.py`, `test_approval*.py`, `test_signing_keys.py`, EXP-007A — pass unchanged: 453/453 and 16/16.

## 5. Acceptance criteria
| Criterion | Status |
|---|---|
| All available tests pass or remaining failures are documented | **Met**: 558/558; gateway has 0 tests (documented) |
| No regression in security invariants | **Met** (same counts; no security code edited) |
| External benchmark independently reproducible | **Met** for the 4 runs above (retrospective; see §3) |
| No credentials/sensitive material in the evidence package | **Met** per `secret-scan.txt`. A local `.env.local` exists on disk but is git-ignored and excluded; its contents were never read |
| Working tree clean at freeze | **Met** at `1b93abe` (see repo-version.txt) |
| Every technical claim links to verified evidence | See `../application/technical-evidence-summary.md`; unverified items are labelled |

## 6. Limits of this report
Single Windows machine; Rust DB tests used one ephemeral Postgres and no Redis; jest mocks HTTP; no real WebAuthn authenticator was exercised anywhere; Agent Lab results concern Crypton's own lab code; benchmark scripts probe one property per target. Tests rewrite committed `evidence/*.json` files with new timestamps and random IDs on every run (reverted to the committed copies after the run; see deferred work).
