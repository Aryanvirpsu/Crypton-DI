# EXP-003C — Agent Authorization & Delegation Assessment: LangGraph

Third target in the EXP-003 series (after EXP-003A/OpenAI Agents SDK and
EXP-003B/Microsoft Agent Framework). Reuses the exact same hypothesis set
(H1-H6) against a structurally different execution model, chosen for
architectural diversity per the series' own operating rule.

No new capability, abstraction, or scoring engine was promoted into
shared Crypton code as part of this lab — see "Methodology assessment"
below for what did and didn't generalize.

## Target

| | |
|---|---|
| Package | `langgraph==1.2.11` (+ `langchain==1.4.0`, `langgraph-checkpoint-sqlite`) |
| Verified | via `pip index versions`, both current-latest at test time (2026-09-16), not assumed |
| Model dependence | **None.** Every scenario is deterministic, plain Python graph-node functions - no LLM/chat model anywhere in the loop |
| Network/API key required | No |

See [`target.json`](target.json) for the full environment record.

## Why LangGraph (Phase 0 selection)

Chosen over alternatives to maximize architectural diversity from
EXP-003A/B:

- **OpenAI Agents SDK (EXP-003A):** opinionated agent-loop with a typed
  `ToolApprovalItem`, `needs_approval`, and a framework-enforced per-call
  fingerprint check.
- **Microsoft Agent Framework (EXP-003B):** opinionated `Workflow` model
  with `function_approval_request`/`response` content types and an
  explicit two-tier (`ToolApprovalRule`) standing-approval design.
- **LangGraph:** a *generic* graph/node/edge execution engine.
  `interrupt(value) -> Any` and `Command(resume=value)` are a bare
  pause/resume primitive with **no built-in concept of "tool", "call_id",
  or "approval" at all** - those are 100% application-defined shapes
  passed through `value`. The pluggable `checkpointer` (in-memory or
  SQLite in this lab) is likewise generic durable-state, not an
  approval-specific mechanism.

This is itself a Phase-1 finding, not just a selection rationale: it
means every one of H1-H6 below is really asking two separate questions -
*"does the raw primitive make this failure mode possible?"* (usually:
yes, because it provides no opinion either way) and *"does a reasonable
application built on top of it avoid the failure mode?"* (tested via a
deliberate safe/unsafe pattern pair - see H1).

**Framework suitability finding (addressing the explicit instruction to
recommend an alternative if LangGraph is unsuitable):** raw
`langgraph.types.interrupt()` is **not, by itself, a fair target for
H2-H6** in the same sense EXP-003A/B's opinionated approval systems were
- there is nothing named "approval" to test the binding of. It remains a
valid and necessary target for the *substrate-level* question (what does
the checkpoint/resume primitive itself guarantee, with zero application
help?), which is what this lab tests. The **comparable, opinionated
approval layer** is `langchain`'s `HumanInTheLoopMiddleware` +
`InterruptOnConfig`, built on top of `interrupt()` - and that layer is
**not re-tested here**, because it is already the subject of a real,
pre-existing, confirmed finding:
[`labs/experiments/001-langgraph-approval-drift/`](../experiments/001-langgraph-approval-drift/)
(`finding-001`: approval bound to proposed arguments is not re-verified
against executed arguments when a second, benign middleware reorders the
call between approval and execution). Re-running H1/H5 against that
middleware here would duplicate closed work. Where this lab's raw-
primitive results and `finding-001` relate, it's noted inline (H1, H5).

**Conclusion:** LangGraph is the right choice for the raw-primitive layer
this lab targets, is not artificially forced into playing "approval
framework," and its comparable opinionated layer is already covered by
prior evidence - no alternative framework substitution is warranted.

## Concept mapping (Phase 1)

| Crypton concept | LangGraph raw primitive | Supported? |
|---|---|---|
| Proposed action | Value passed to `interrupt(value)` | SUPPORTED (app-defined shape) |
| Presented for approval | The paused `Interrupt(value, id)` surfaced via `graph.invoke()`'s `__interrupt__` key | SUPPORTED |
| Authorization decision | `Command(resume=value)` | SUPPORTED (app-defined shape; no framework-typed "approve"/"reject") |
| Executed action | Whatever the resuming node does with `state`/the resume value | SUPPORTED, but **binding is entirely application-chosen** - see H1 |
| Delegation path | Parent node invoking a compiled child (sub)graph, each with its own `interrupt()` | SUPPORTED, confirmed independent (H2) |
| State/resume boundary | `thread_id` (checkpoint namespace) + pluggable `checkpointer` | SUPPORTED |
| Standing/scoped approval | *(none)* | **NOT SUPPORTED** at the raw-primitive layer; `langchain`'s `InterruptOnConfig` (4 fields: `allowed_decisions`, `description`, `args_schema`, `when`) also has no such field - see H3 |
| Policy/guardrail re-check at resume time | *(none)* | **NOT SUPPORTED** as a framework primitive; constructible in application code - see H5 |
| Call/interrupt identity | `Interrupt.id` | SUPPORTED |

## Hypotheses and results

Same H1-H6 as EXP-003A/B, no new hypotheses invented.

| # | Hypothesis | Result | Evidence |
|---|---|---|---|
| H1 | Argument binding: can approved arguments drift before execution? | **PASS** (state-sourced pattern) / **application-dependent** (resume-payload-sourced pattern is unsafe by construction) / **CANDIDATE_FINDING_APPLICATION_LEVEL** via public `update_state()` | `h1c_update_state_argument_drift.json`; `test_h1a/b/c` |
| H2 | Delegation boundary: does parent/delegation approval confer child tool authority? | **PASS** - independent interrupts, independent decisions, exactly-once execution | `h2_delegation_vs_child_tool_boundary.json` |
| H3 | Standing approval scope | **NOT_APPLICABLE** - no standing-approval primitive exists at this layer (source-verified against installed `InterruptOnConfig` and `HumanInTheLoopMiddleware`) | `h3_no_standing_approval_primitive.json` |
| H4 | Persistence/resume binding: exactly-once execution across checkpoint save/restore | **PASS** - `InMemorySaver` and real-file `SqliteSaver` (fresh connection + fresh compiled graph on reload), repeated resume of a completed thread | `baseline_c_persist_reload_resume.json`, `h4_repeated_resume_of_completed_thread.json` |
| H5 | Policy/guardrail TOCTOU between approval and execution | **PASS** - re-check is entirely application-constructible (no framework primitive); demonstrated with a policy that changes between approval and resume | `h5_policy_recheck_toctou.json` |
| H6 | Identity/origin binding: can approval cross thread identities? | **PASS** (h6a: own-pending-interrupt case) / **PASS_WITH_CAVEAT** (h6b: unknown-thread case surfaces a `KeyError`, not a bypass - see below) | `h6a_cross_thread_resume_distinct_pending.json`, `h6b_resume_unknown_thread_no_checkpoint.json` |

### H1 detail - the state-sourced vs. resume-sourced pattern pair

This lab deliberately builds **two** graphs for the same scenario:

- `build_single_tool_graph` (safe): the resuming node reads execution
  arguments from checkpointed **state**, never from the resume payload.
- `build_single_tool_graph_resume_sourced` (unsafe): the resuming node
  reads execution arguments from whatever the **resume payload itself**
  claims.

The unsafe pattern is trivially defeatable (`test_h1b`) - **by design**,
to characterize what the framework does and does not protect, not
because it's a langgraph defect. `interrupt()`'s resume value is
documented as a generic data channel with no claimed binding guarantee.

The safe pattern is *not* immune to a different, more interesting vector
(`test_h1c`): `update_state()` is a public, documented langgraph API. A
second, legitimate caller invoking it on a still-pending thread, between
the interrupt and the resume, mutates the very state the safe pattern
trusts - producing the same class of drift `finding-001` demonstrated at
the middleware layer, but via a different mechanism (`update_state()`
mutation vs. `after_model` middleware reordering). This is classified as
`CANDIDATE_FINDING_APPLICATION_LEVEL`, not a security issue in langgraph
itself: `update_state()` makes no promise to freeze pending interrupts,
and a real deployment must treat "who may call `update_state()` on a
pending thread" as part of its own trust boundary.

### H6 detail - the unknown-thread edge case

`test_h6b` initially assumed resuming a thread_id with no checkpoint
would silently no-op. The actual, verified behavior is different: with
no checkpoint, the graph executes its entry node against an **empty**
state, and this lab's own node function (which unconditionally reads
`state["tool_name"]`) raises `KeyError`. No execution occurred either
way (no bypass), so the security-relevant assertion still holds - but
per the operating rule to report the real result rather than force the
original expectation, this is recorded as `PASS_WITH_CAVEAT`: an
application-robustness consideration (guard node functions against
partial/missing state), not a framework authorization defect.

## Baselines

- **Baseline A** (`baseline_a_approve`): approve → executes, correct arguments, no pre-approval execution.
- **Baseline B** (`baseline_b_reject`): reject → zero side effects.
- **Baseline C** (`baseline_c_persist_reload_resume`): genuine cross-process-style persistence - `SqliteSaver` backed by a real temp SQLite file, reloaded via a **fresh connection and fresh compiled graph** (not the original Python objects), then resumed.

## Running

```bash
cd labs/exp-003c-langgraph
python -m venv .venv
.venv\Scripts\pip install -r requirements-lock.txt     # exact versions the evidence was produced with
.venv\Scripts\python -m pytest tests/ -v
```

12 tests, no network, no API key, <1s runtime. Run it inside this lab's own `.venv`: under a different
environment, `langgraph.checkpoint.sqlite` may be unavailable and collection fails (this happened on
2026-09-24; see `labs/exp-007-principal-continuity/FROZEN.md`). Last verified 2026-10-12: 12 passed
(Python 3.14.7, langgraph 1.2.11, langgraph-checkpoint-sqlite 3.1.1).

## Cross-framework comparison (not a ranking)

| Property | OpenAI Agents SDK (EXP-003A) | Microsoft Agent Framework (EXP-003B) | LangGraph (EXP-003C) |
|---|---|---|---|
| Approval is a first-class typed concept | Yes (`ToolApprovalItem`) | Yes (`FunctionApprovalRequest/Response`) | No - generic `interrupt(value)`; approval is an app-defined shape |
| Argument-binding enforcement | Framework-enforced (per-call-id fingerprint check, raises on reuse-with-drift) | Application-responsibility (no equivalent fingerprint found) | Application-responsibility (state-sourced vs. resume-sourced pattern is entirely the app's choice); public `update_state()` opens a second drift vector even in the safe pattern |
| Delegation boundary (parent approval vs. child tool approval) | Independent (`Agent.as_tool()`) | Independent (`Agent.as_tool()`, after harness repair) | Independent (parent `interrupt()` vs. child subgraph's own `interrupt()`) |
| Standing/scoped approval | Yes - `always_approve` flag, argument-blind (broad only) | Yes - explicit two-tier (`ToolApprovalRule` broad vs. tool_with_arguments narrow) | **No primitive at either the raw-interrupt or `HumanInTheLoopMiddleware` layer** |
| Guardrail/policy re-check at resume time | Yes - guardrails re-run at resume | Not directly comparable in this lab's tests | No framework primitive; constructible in application code (`build_policy_checked_graph`) |
| Persistence/exactly-once across checkpoint reload | PASS | PASS | PASS (`InMemorySaver` + real-file `SqliteSaver`) |
| Cross-thread/identity binding | PASS (per-run identity) | PASS (`ToolContext` identity) | PASS (own-pending-interrupt case); unknown-thread edge case raises rather than silently no-ops |
| Model/API dependence to exercise authorization mechanics | Requires `ScriptedModel` (still a model in the loop) | Requires `ScriptedChatClient` (still a model in the loop) | **None** - authorization mechanics are fully exercised with zero LLM/chat model |

No framework is ranked "better" here - each makes different, explicit
design trade-offs (opinionated typed approval + enforcement vs. a
generic, unopinionated primitive that pushes more responsibility to the
application). The finding is about what each framework *guarantees by
default* vs. what it leaves to the integrator.

## Methodology assessment (Phase 10) - did Crypton's approach generalize?

- **Generalized unchanged:** the H1-H6 hypothesis set itself: every
  hypothesis was meaningfully answerable (including "NOT_APPLICABLE" as
  a first-class, evidence-backed answer, not a cop-out) against a third,
  structurally distinct framework without inventing new hypotheses.
  The PASS/FAIL/INCONCLUSIVE/NOT_APPLICABLE finding-discipline and the
  evidence-JSON schema (`AuthorizationBinding`, `build_evidence`) needed
  no structural change - only additive `Outcome` variants
  (`PASS_WITH_CAVEAT`, `CANDIDATE_FINDING_APPLICATION_LEVEL`) to
  honestly represent results that were not clean binary passes.
- **Needed adaptation:** the four-stage
  proposed/presented/authorized/executed model had to be understood as
  entirely application-defined here, not framework-supplied - so
  "evidence" for this lab is inherently about *pairs of patterns* (safe
  vs. unsafe construction) rather than a single framework-given
  mechanism's behavior, unlike EXP-003A/B where the framework itself
  was the object under test.
- **Failed to generalize / not applicable:** the model-dependence
  testing infrastructure built for A/B (`ScriptedModel`,
  `ScriptedChatClient`) was entirely unneeded - LangGraph's raw
  primitive requires no chat model at all to exercise authorization
  mechanics, so none of that scaffolding was reused or adapted.
- **No shared Crypton abstraction was promoted** out of this lab. Per
  the series' promotion gate (only promote if proven useful on
  *multiple* targets), the evidence-JSON shape is similar in spirit
  across A/B/C but was not literally shared code, and no single
  reusable authorization-testing harness component has yet been used
  unmodified across all three frameworks - that remains a candidate for
  a future, explicitly-scoped promotion pass, not something this lab
  should force.

## Final verdict

**EXPECTED DESIGN**, confidence: high.

No CANDIDATE SECURITY ISSUE or CONFIRMED SECURITY ISSUE was found
against langgraph or langchain's `HumanInTheLoopMiddleware`/
`InterruptOnConfig` themselves. One application-level trust-boundary
consideration was surfaced (H1c, `update_state()`-mediated argument
drift against the "safe" state-sourced pattern) and classified as
`CANDIDATE_FINDING_APPLICATION_LEVEL` - real, reproducible, and worth
documenting for anyone building on `interrupt()`/`update_state()`, but
not a framework defect: `update_state()` makes no competing safety
claim. H3 is NOT_APPLICABLE, source-confirmed rather than assumed. All
other hypotheses PASS or PASS_WITH_CAVEAT, each with an honestly-
reported real result (including one, H6b, that contradicted this lab's
own initial expectation and was corrected rather than forced).
