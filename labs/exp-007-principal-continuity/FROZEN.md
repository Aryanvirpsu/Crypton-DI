# EXP-007A — FROZEN

- Date: 2026-09-24
- Base commit: `ca2f52f` (package is **untracked / uncommitted**; nothing committed or pushed)
- Source fingerprint: SHA-256 `EE69BBFE9F1C6927404D5B70DD059D7045CB1B598E86E293C403D93105C47EAC` over `exp007/*.py` then `tests/*.py`, sorted by path, LF-normalised, joined by `\n--\n`
- Canonical run: `python -m exp007.scenario_007a` from `labs/exp-007-principal-continuity` (exit 0 = gate passes)
- Tests: `python -m pytest` in the same directory: 16 passed
- Determinism: 10 standalone runs with PYTHONHASHSEED 1..10 -> exit 0 every time, 1 distinct `--json` evidence hash; plus in-process x10 and cross-process/hash-seed tests
- Negative controls: `vulnerable_policy` and `authenticated_only_policy` substituted as the secure policy -> gate reports failures, runner exits 1 (tested)

Freeze checklist: baseline passes; unauthenticated substitution distinguishable; authenticated substitution distinguishable; NO_CAPABILITY calibration distinct; vulnerable allows leg d; secure STEP_UPs leg d with `CONTROLLER_NOT_AUTHORIZED`; negative control caught; determinism gate passes; trace reviewer-readable; replay limitation documented.

Replay protection is deferred to EXP-007B. Nothing here claims replay is handled.

Changes to frozen files require a new dated entry here.

## 2026-09-24 (later): regression-status caveat (docs only; source fingerprint unchanged)

EXP-007A freeze criteria passed. Repository-wide regression status is **not established**: unrelated suites could not execute cleanly in this environment (`agent-lab`: `pytest-asyncio` unavailable, 38 failed / 284 passed; `exp-003c`: `langgraph.checkpoint.sqlite` unavailable, collection error). These failures were not shown to predate EXP-007A and are not claimed to.
