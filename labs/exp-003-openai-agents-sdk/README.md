# EXP-003A — OpenAI Agents SDK (Python), Authorization/Delegation Assessment

Crypton's active wedge is **Agent Authorization & Delegation Assessment**.
This lab is Phase 1 of TARGET 003-A: `openai-agents==0.22.2` (import name
`agents`), the open-source Python SDK. Everything here runs locally,
deterministically, with no API key and no network access — see
`target.json`.

Isolated from Crypton's production/shared code
(`labs/agent-lab/src/agentlab/assessment/`) and from the other research
lab in this repo (`semantic-rulebook/`, a separate git repository). No
imports cross those boundaries.

## What is being assessed

Function-tool approval, `Agent.as_tool()` delegation boundaries, nested
approvals, sticky (`always_approve`) authorization scope, `RunState`
serialization/resume, argument binding, tool identity/namespace binding,
and approval/guardrail ordering — against real, unmodified SDK code, not
a model of it.

## Setup

```bash
cd labs/exp-003-openai-agents-sdk
python -m venv .venv
.venv\Scripts\pip install -r requirements.txt
.venv\Scripts\python -m pytest -v
```

No `OPENAI_API_KEY` needed. Every agent is built with
`agents.set_tracing_disabled(True)` and every model is
`agents.testing.ScriptedModel` — deterministic, no network call.

## Structure

- `crypton_exp003/` — the assessment library (audit sink, fake tools,
  evidence schema, small SDK-usage helpers). See "What Crypton may add"
  below for what's reusable and what isn't (yet).
- `scenarios/` — controlled workflow construction (agents, tools,
  scripted model steps) per hypothesis.
- `tests/` — assertions + evidence generation. One passing pytest run is
  the actual acceptance artifact; `evidence/*.json` is the durable record.
- `evidence/` — one JSON file per scenario, schema in
  `crypton_exp003/evidence.py`.

## The four-stage comparison

Every scenario is built to answer, for one tool call:

```
WHAT WAS PROPOSED  ->  WHAT WAS PRESENTED FOR APPROVAL  ->  WHAT WAS APPROVED  ->  WHAT ACTUALLY EXECUTED
```

`authorization_binding` in each evidence file records whether these agree
(tool/argument/call_id/namespace/delegation match) as booleans, or `null`
+ a note when the public API doesn't expose enough to judge — never a
guessed boolean.

## Results summary (see final report for full detail)

All five hypotheses (H1-H5) and all three baselines (A/B/C) passed — 16
of 16 tests, real evidence for every one. This SDK's approval/delegation/
resume machinery held up against every realistic (non-private-internal)
mutation vector tried in this pass. See `evidence/*.json` for the exact
mechanism behind each PASS, and the final report for full hypothesis
write-ups, what was NOT constructible, and what's recommended for Phase 2.

## What Crypton may add (kept inside this experiment)

`AuditSink`/`AuditRecord`, the evidence schema (`build_evidence`/
`AuthorizationBinding`), and the small scenario-runner helpers in
`tools.py` are all reusable in shape. None have been promoted to
Crypton's shared assessment core (`labs/agent-lab/src/agentlab/
assessment/`) — that promotion is deliberately deferred until a SECOND
target (a different SDK/framework) proves the abstraction actually
generalizes, not just that it worked once here.

## What was explicitly NOT built

- No real model integration (Phase 2+ only, if ever needed).
- No attack against OpenAI infrastructure/APIs/accounts.
- No new invariant families, no stateful/temporal policy language.
- No promotion of anything into Crypton's shared assessment core yet.
- No true "same bare tool name in a different namespace" construction —
  not found to be reachable via the public API without deeper MCP-specific
  investigation (recorded as INCONCLUSIVE in H3's evidence, not skipped
  silently).
