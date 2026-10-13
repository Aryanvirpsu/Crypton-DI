# EXP-003B — Microsoft Agent Framework (Python), Authorization/Delegation Assessment

Second EXP-003 target, run in parallel with EXP-003A.2. Tested against the
real, current stable release verified via `pip index versions` before
installing anything: **agent-framework==1.18.0** (the task prompt assumed
1.17.0 - one minor version stale; tested against the real current release
instead, per instructions not to trust the prompt blindly). See
`target.json`.

Isolated from Crypton's shared assessment code and from
`labs/exp-003-openai-agents-sdk/` (EXP-003A) - nothing is imported across
those boundaries. Per the EXP-003 operating rule, this repeats the minimum
of EXP-003A's approach locally rather than importing it, so the two can be
honestly compared (§ below).

## No official ScriptedModel-equivalent

Unlike `openai-agents.testing.ScriptedModel`, no deterministic test-model
utility is exported by this package. `crypton_exp003b/fixtures.py`
implements one (`ScriptedChatClient`), composed with the framework's own
`agent_framework._tools.FunctionInvocationLayer` mixin - required for
automatic function-invocation/approval handling at all (confirmed
empirically: without it, `Agent` logs "does not support function
invoking" and never processes `approval_mode`). This is a deterministic
MODEL substitute; no framework authorization/workflow/checkpoint logic is
reimplemented.

## Architecture actually found (confirmed by introspection + empirical runs, not assumed)

- Tools: `@agent_framework.tool(approval_mode="always_require")`.
- Approval surfaces as `Content(type="function_approval_request")` inside
  an `AgentResponse`/`WorkflowEvent`, wrapping a `function_call` Content.
  Submit a decision via `<request>.to_function_approval_response(approved=...)`.
- **Direct `Agent.run()`** surfaces approval requests directly in the
  response; a `session=` (`AgentSession`) carries `ToolApprovalState`
  (standing-approval rules) across turns.
- **`WorkflowBuilder(start_executor=one_agent, checkpoint_storage=...)`**
  wraps a single agent as a `Workflow` and correctly surfaces the same
  approval request as a `request_info` `WorkflowEvent` with a stable
  `request_id`, checkpointable via `InMemoryCheckpointStorage`. This
  clean, well-understood pattern is what B1-B6/H1/H3/H4/H5/H6 are built on.
- **`agent_framework.orchestrations.HandoffBuilder`** (true multi-agent
  handoff) requires `require_per_service_call_history_persistence=True`
  on every participant. Its own `request_info` mechanism surfaces
  *handoff* decisions (`HandoffAgentUserRequest`) cleanly, but a
  downstream participant's own `function_approval_request` did **not**
  cleanly surface as a workflow-level `request_info` event in any
  configuration tried in this pass - see "H2" below.
- Standing approval is a first-class, two-tier, **documented** design:
  `ToolApprovalRule(tool_name, arguments=None)` = broad, tool-only scope;
  `arguments={...}` = narrow, exact-argument scope. Two builder helpers,
  `create_always_approve_tool_response` / `..._with_arguments_response`,
  construct the corresponding response directly.

## H2 - what was and wasn't resolved

The clean, well-understood `Agent.as_tool(approval_mode=...)` mechanism
confirmed round 1 (delegation-only pending, nested tool not yet reached -
see `test_handoff.py`). Completing the full cycle (approve delegation ->
confirm the nested tool ALSO needs its own approval) hit a streaming
incompatibility: `Agent.as_tool()` invokes the nested agent with
`stream=True`; even after adding `ResponseStream`/`ChatResponseUpdate`
support to `ScriptedChatClient`, the nested call still raised
`TypeError("'async for' requires an object with __aiter__ method, got
coroutine")`. This is a gap in this lab's test double, not an observed
framework authorization behavior - recorded INCONCLUSIVE, not PASS/FAIL.
Top Phase-2 priority (see final report).

## Four-stage model / schema comparison vs. EXP-003A

Applied successfully: PROPOSED -> PRESENTED FOR APPROVAL -> AUTHORIZED ->
EXECUTED, plus DELEGATION PATH and STATE/RESUME BOUNDARY, all mapped
cleanly onto this framework's real primitives (function_call ->
function_approval_request -> to_function_approval_response -> execution;
Workflow/checkpoint_id for the state boundary).

Differences from EXP-003A's schema, kept local per the operating rule
(not yet promoted - see final report §21-22):
- **Added `state_boundary` {workflow, session, checkpoint}**: this
  framework exposes workflow/checkpoint/session identity as first-class,
  independently observable concepts; EXP-003A's target had no equivalent.
- **`authorization_binding` gained `request_match`/`checkpoint_match`**:
  `request_id` (not the tool call_id) is this framework's primary
  approval-correlation handle - a distinct identifier from the underlying
  function call_id, unlike EXP-003A's target where call_id served both
  roles.
- **Lost:** EXP-003A's `ToolContext`-style per-call identity object
  (`tool_name`/`call_id`/`agent`/`namespace` all readable *inside* the
  tool function body). The nearest equivalent found here
  (`FunctionInvocationContext`) exposes only `add_tools`/`remove_tools` -
  no per-call identity fields. Fixtures here record only the raw
  execution fact; scenario code (which already holds agent/workflow/
  checkpoint/request identity from constructing the run) builds the full
  `AuditRecord` after the fact. This is a genuine schema-generalization
  finding, not a workaround: a shared Crypton `AuditRecord` abstraction
  cannot assume "the tool can self-report its own identity."

## Setup

```bash
cd labs/exp-003-microsoft-agent-framework
python -m venv .venv
.venv\Scripts\pip install -r requirements.txt   # or: pip install agent-framework==1.18.0 pytest pytest-asyncio
.venv\Scripts\python -m pytest -v
```

Run the tests inside this lab's own `.venv`. Under any other environment collection fails with
`ModuleNotFoundError` for `agent_framework`. Last verified 2026-10-12: 12 passed
(Python 3.14.7, agent-framework 1.18.0).

No API key, no network, no hosted model - every scenario uses
`ScriptedChatClient`.

## What was explicitly NOT built

Real model integration; any attack on Microsoft-hosted infrastructure;
promotion of anything into Crypton's shared assessment core (gate not yet
met - see final report); a resolved `HandoffBuilder` + downstream-tool-
approval mechanism (recorded INCONCLUSIVE); Experiment 003-C.
