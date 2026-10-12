# Known Limitations and Deferred Work Register — as of the 2026-10-12 freeze

## A. Limitations that affect what may be claimed
| # | Limitation | Consequence for claims |
|---|---|---|
| L1 | All Agent Lab / EXP results test Crypton-authored code and tests. | Shows internal consistency and that the harness discriminates, not that Crypton finds unknown bugs. |
| L2 | Both benchmark cases are retrospective, one in-process probe per property, scripts written after the advisory and release diff were public. | "Distinguishes a known-vulnerable from known-fixed release", not "found a vulnerability". |
| L3 | No real WebAuthn authenticator exercised anywhere. Agent Lab approvals use `FakeWebAuthnVerifier`; `crypton-id` has `webauthn-rs` code paths but no automated end-to-end ceremony test. | Do not claim verified passkey authentication. |
| L4 | `crypton-gateway` is a 275-line reverse proxy with 0 tests; it does not verify tokens or authorize agents. | Do not claim a gateway/PEP. |
| L5 | Rust DB tests ran against one ephemeral Postgres 16.15 on Windows; Redis-backed paths not exercised; no deployment (Vercel/Railway) tested. | "Passes in a local isolated DB", not "production-verified". |
| L6 | `libs/crypton-sdk` jest tests mock HTTP. | Client logic only. |
| L7 | Agent Lab: SQLite, in-process, one machine, private keys returned to caller (no key custody), revocation propagation timing not measured. | Reference implementation, not a deployed system. |
| L8 | No customer, design partner or external engagement; the sample report is a self-assessment. | Say so. |
| L9 | React console (`services/crypton-console`), `crypton-main`, and the authority-model module's behavior beyond its own tests were not separately exercised/reviewed. | Unverified. |
| L10 | Windows-only runs; Python 3.14.7 (newer than the `>=3.11` floor in pyproject; older interpreters untested this pass). | |

## B. Deferred engineering work (not started; nothing here is a confirmed defect unless stated)
| # | Item | Why deferred |
|---|---|---|
| D1 | Unused imports in `agentlab/approval.py` (`WebAuthnVerificationError`) and `agentlab/delegation.py` (`signing_keys`); 3 unused locals (`fixtures.py` e002, `test_authority_model_presentation.py` all_branches_text, `test_exp003c.py` r3). | Security-critical modules / possible side-effect semantics; verify no re-export or import-order reliance, then remove with the full suite as regression. |
| D2 | Rust clippy: 4 collapsible `if`, 1 empty line after doc comment, **`AdminUser.cred_id` and `.role` never read** (`src/jwt.rs:117-121`). | The dead `role` field in an admin extractor deserves a review that role enforcement happens where intended before anyone "cleans" it. Not confirmed as a defect. |
| D3 | `sqlx-postgres 0.7.4` flagged by cargo as containing code rejected by a future Rust version. | Dependency upgrade is a behavior-risking change; out of scope. |
| D4 | Experiment/test runs overwrite tracked `labs/exp-003-*/evidence/*.json` with fresh timestamps and random IDs (24 files churn per run). | Make evidence generation deterministic or write to an untracked dir; reverted after this run. |
| D5 | Dependency pinning inconsistencies: `agent-lab/requirements.txt` uses `>=` ranges; exp-003 OpenAI/MS list unpinned `pytest`/`pytest-asyncio`; only exp-003C has a full lock file. | Add lock files; behavior-neutral but needs per-lab verification. |
| D6 | `crypton-id` and gateway: add real tests; implement (or keep removed) the gateway PEP; add an automated WebAuthn ceremony test. | Product work, outside this pass. |
| D7 | Validation Benchmark: more independent cases (the `mcp-atlassian` CVE-2026-77243 case was earlier deliberately not forced); external methodology review (EXTERNAL-REVIEW-BRIEF.md) still unsent. | Founder/outreach action per CLAUDE.md §15. |
| D8 | `CLAUDE.md` §3 ("Persistent traces", "Human-approval boundaries") and §4 "Data: Postgres + Redis — Built" wording could be tightened to mention lab-only scope; Redis untested here. | Doc refinement; founder-owned doc. |
| D9 | `docs/` and `assessment-kit/` were committed as-is in the baseline commit without a content review for stale numbers (e.g. old test counts). | Not audited in this pass. |
| D10 | Retired-finding PoC zip preserved outside the repo at `../Crypton-DI-snapshots/removed-from-tree-langchain-poc.zip`; decide whether to delete. | Owner decision. |
