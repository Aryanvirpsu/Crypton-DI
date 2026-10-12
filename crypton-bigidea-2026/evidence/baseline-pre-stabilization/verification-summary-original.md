# Verification Summary — 2026-10-12

Nothing in the repository was modified, staged or committed; all new files are under `crypton-bigidea-2026/`. No product feature was added. `demo/run_demo.py` is a read-only driver over existing APIs.

## Five questions
1. **Version:** commit `ca2f52f` on `refactor/agent-identity-foundation` **plus a large uncommitted working tree** (45 modified tracked files, 22 untracked paths, including EXP-007, EXP-003*, validation-benchmark, pilot-pack). `evidence/repo-version.txt`.
2. **What works (ran and passed):** Agent Lab 453/453; EXP-007A 16/16; EXP-003C 12/12; EXP-003 OpenAI 17/17; EXP-003 Microsoft 12/12; crypton-id Rust non-DB tests 22/22; SDK jest 10/10. **542 passing, 0 logic failures.**
3. **Behavior reproduced:** 3-hop signed delegation with attenuation, tool/resource-escape denial, ancestor revocation, and issuance-time refusal (`CRYPTON-DEMO-001`); controller-substitution test that separates weak from secure policies (`CRYPTON-DEMO-002`).
4. **Security finding Crypton can demonstrate:** an **ENFORCED** result for delegation attenuation in its own reference implementation, plus a calibrated FAIL/PASS discrimination on a lab policy. **No new vulnerability in a third-party system was demonstrated this session.** The only third-party vulnerable→fixed evidence (Validation Benchmark) was not reproduced here.
5. **Prototype-stage / unimplemented:** see `feature-inventory.md`. Notably: real WebAuthn in the agent loop (fake verifier), gateway enforcement (0 tests), DB-backed identity service (not run), dashboard (not run), scanner/SaaS/monitoring (none).

## Test result
```
Test result:
542 passed
16 failed   (all in services/crypton-id; every one is "DATABASE_URL must be set" - Postgres not available)
0 skipped
14 collection errors, all from my own first attempt using the wrong venv for the two exp-003
    framework suites (ModuleNotFoundError: agents / agent_framework). Retained in test-results.txt.
    Rerun with each suite's own .venv, no installs or edits: 17 passed and 12 passed.
Not run: labs/validation-benchmark (needs pinned third-party venvs), services/crypton-console,
         gateway has 0 tests.
```
Runtime: agent-lab 32.4 s, others < 5 s each, Rust compile excluded.

**Interpretation — what this establishes:** the Python lab code, delegation/approval/chain logic, the EXP-007A harness, three framework-behavior experiments, EdDSA JWT unit logic in Rust, and SDK HTTP-client mocks behave as their tests specify, on this machine, today.

**Limitations — what this does NOT establish:**
- The 16 Rust failures hide the untested part of the identity service (routes, devices, recovery, admin). Passing 22 of 38 is not "the Rust service works".
- All tests are Crypton-authored; passing shows internal consistency, not that the tests find real bugs. The Validation Benchmark is the answer to that and was not re-run.
- Results are for the dirty working tree. Not reproducible from the commit SHA.
- Agent Lab approvals use a fake WebAuthn verifier; nothing here exercises a real authenticator.
- Single Windows machine, Python 3.14.7.

## Discrepancy worth knowing
`labs/exp-007-principal-continuity/FROZEN.md` (2026-09-24) records agent-lab at **38 failed / 284 passed** because `pytest-asyncio` was unavailable, and said repo-wide regression status was "not established." In this environment (pytest-asyncio 1.4.0 present) the same suite is **453 passed, 0 failed**. The earlier failures were environmental, consistent with that note, but that note itself is now stale.

## Defects/claims affected found while verifying
| Item | Classification | Affects an application claim? |
|---|---|---|
| Dirty tree / untracked evidence dirs | Process gap | Yes: "exact revision" is not a clean commit. **Fix: commit or tag before submitting.** |
| CLAUDE.md §4 says gateway "Built" | Unverified claim | Yes if stated. Gateway has 0 tests. Don't claim enforcement gateway. |
| CLAUDE.md §3 "Postgres + Redis, Built" | Not verified | Rust DB tests couldn't run. |
| FROZEN.md regression caveat stale | Doc drift | No. |
| Validation Benchmark not reproduced | Evidence gap | Yes if cited as "reproduced". Cite as "recorded in repo". |
| Demo recording `demo-recording.mp4` | **Not produced** | Package item missing: I can't record video from this session. `demo/raw-output.txt` is the substitute; record the screen yourself running `demo/run_demo.py`. |

## Acceptance gate
- [x] Exact repository revision recorded (with the dirty-tree caveat)
- [x] Current automated tests executed
- [x] Raw test output retained (including failed first attempts)
- [x] Verified capabilities separated from planned
- [x] At least one delegation workflow reproduced (authentication leg = EdDSA JWT unit tests; Agent Lab delegation e2e)
- [x] Expected boundary explicitly stated
- [x] Actual behavior recorded
- [x] Defensible assessment result (ENFORCED; limits stated)
- [x] Reproducible from documented steps — **from this working tree**, not from the SHA
- [x] Sample customer-facing finding produced (`demo/sample-report.md`)
- [x] Known limitations documented
- [x] No claim relies on memory (each claim has a run or file reference)
- [x] No commercial-platform capability implied
- [ ] **Demo recording** — not produced (see above)
