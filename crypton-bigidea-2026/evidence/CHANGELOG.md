# Stabilization Changelog (branch `verification/big-idea-stabilization`)

Behavioral fixes and cleanup are kept in separate commits. **There were no behavioral code fixes**: no confirmed code defect was found.

## `db31183` chore: preserve baseline (no behavior change)
Committed the existing uncommitted work (EXP-003/005/007 labs, validation benchmark, authority-model, assessment kit, pilot pack, methodology docs, audits) so later changes are reviewable. Root `.gitignore` gained Python cache/venv entries. One binary zip (a retired-finding PoC bundle duplicating agent-lab sources, staged but never committed) was left out and preserved outside the repo.

## `e7efbc3` docs: claims and environment docs
- `CLAUDE.md` + `AGENTS.md` (identical copies): gateway reclassified from "Policy Enforcement Point, Built" to "plain reverse proxy; PEP role Planned" (its own `main.rs` header and README say it does not verify JWTs or authorize agents; 0 tests). Enforcement attributed to Agent Lab's in-process check. WebAuthn row split: `crypton-id` uses `webauthn-rs`, no automated end-to-end ceremony test; Agent Lab uses `FakeWebAuthnVerifier` (simulation). Decision Log row added.
- exp-003 / exp-003C READMEs: run inside the lab's own `.venv` (wrong venv => collection `ModuleNotFoundError`, which happened during the first baseline run); exp-003C now lists venv setup with its lockfile; last-verified results added. exp-007 README: environment note.
- `exp-007-principal-continuity/FROZEN.md`: stale 2026-09-24 "38 failed / not established" caveat superseded (453 pass; source fingerprint unchanged).

## `1b93abe` chore(cleanup): remove 14 unused imports
Ruff F401 over `authority_model/{diff,envelope,fixtures}.py` and test files in agent-lab, exp-003 (OpenAI), exp-003C. Behavior-neutral; suites rerun (453 / 17 / 12 pass) before commit and again at freeze.

## Findings that needed no change
- 16 Rust `crypton-id` failures: environment only (no `DATABASE_URL`); all pass against an isolated Postgres. No source change.
- `cargo fmt --check` clean on both Rust crates. `tsc --noEmit` clean on the SDK.

## Not changed on purpose (see KNOWN-LIMITATIONS-AND-DEFERRED.md)
Unused imports in `approval.py` and `delegation.py`; 3 unused local variables; 6 Rust clippy warnings; the frozen exp-001 script.
