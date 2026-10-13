# Framework Adaptation Guide v0.1

**EXP-004, deliverable 4.** How the [Security Property Catalog](01-security-property-catalog.md)'s
pillars and the [Assessment Methodology](03-assessment-methodology-v0.1.md)'s
H1-H9 hypotheses map onto three real, already-tested agent frameworks —
and the procedure for adapting the same pipeline to a fourth. No
capability ranking is made anywhere in this document; each framework
makes different, explicit design trade-offs, and the point of this
guide is to make those trade-offs legible, not to declare a winner.

**Correction to EXP-003B's README (flagged here, not yet fixed in
place — a follow-up, not part of this deliverable's scope):** `labs/exp-003-microsoft-agent-framework/README.md`
still describes H2 as INCONCLUSIVE due to a streaming test-double bug.
That bug was later fixed (`ScriptedChatClient._inner_get_response`
corrected to a plain function returning either an `af.ResponseStream`
or an inner coroutine, matching `agent_framework`'s real dispatch
paths), and H2 was resolved end-to-end: delegation approval does not
confer the nested tool's own approval — **PASS/EXPECTED DESIGN**, same
conclusion as EXP-003A and EXP-003C. The table below uses this
corrected, final result; the README itself should be updated to match
in a future pass.

---

## Architecture summary

| | OpenAI Agents SDK (EXP-003A) | Microsoft Agent Framework (EXP-003B) | LangGraph (EXP-003C) |
|---|---|---|---|
| Package tested | `openai-agents==0.22.2` | `agent-framework==1.18.0` | `langgraph==1.2.11` + `langchain==1.4.0` |
| Execution model | Opinionated agent loop | Opinionated `Workflow`/`Agent` with checkpointing | Generic graph/node/edge engine |
| Approval as a typed concept | Yes — `ToolApprovalItem` | Yes — `Content(type="function_approval_request"/"function_approval_response")` | No — generic `interrupt(value)`/`Command(resume=value)`, value is app-defined |
| Per-call identity object | `ToolContext` (rich: tool_name, call_id, agent, namespace, readable inside the tool body) | `FunctionInvocationContext` (thin: `add_tools`/`remove_tools` only — no per-call identity fields) | None — `Interrupt.id` is the only framework-supplied identifier; everything else is app-constructed state |
| Delegation primitive | `Agent.as_tool()` | `Agent.as_tool()`, `orchestrations.HandoffBuilder` | Parent node invoking a compiled child (sub)graph, each with its own `interrupt()` |
| Standing/scoped approval | `always_approve` flag on `RunState.approve()` — broad only, argument-blind | `ToolApprovalRule(tool_name, arguments=None\|{...})` — explicit two-tier (broad vs. narrow) | None at either the raw-`interrupt()` layer or `langchain`'s `InterruptOnConfig` (4 fields, none standing-approval-shaped) |
| Persistence/checkpoint | `RunState.from_string`/`from_json` (async) | `InMemoryCheckpointStorage`/`FileCheckpointStorage` | `InMemorySaver`/`SqliteSaver` (pluggable `checkpointer`) |
| State/resume-boundary identity | None dedicated — `RunState` serialization is the whole unit | `workflow`/`session`/`checkpoint_id`, all independently observable | `thread_id` (checkpoint namespace) |
| Model dependence to exercise authorization mechanics | Requires `agents.testing.ScriptedModel` | Requires custom `ScriptedChatClient` (no official equivalent shipped) | **None** — plain deterministic Python node functions |

---

## Cross-framework hypothesis matrix (H1-H6, corrected/final results)

| # | Hypothesis | OpenAI Agents SDK | Microsoft Agent Framework | LangGraph |
|---|---|---|---|---|
| H1 | Argument Binding | **PASS** — SDK-enforced per-call-id fingerprint check, raises `ModelBehaviorError` on reuse-with-drift | **PASS** | **PASS** (state-sourced pattern) / **CANDIDATE_FINDING_APPLICATION_LEVEL** via public `update_state()` on the safe pattern — a vector the other two frameworks' architectures don't expose in the same form |
| H2 | Delegation Boundary | **PASS** — `Agent.as_tool()`, confirmed | **PASS** — `Agent.as_tool()`, confirmed after test-harness repair (see correction note above) | **PASS** — parent/child subgraph `interrupt()`s fully independent |
| H3 | Standing Approval | **PASS/EXPECTED_BY_DESIGN** — broad, argument-blind by design | **PASS** — explicit two-tier design lets the integrator choose broad or narrow | **NOT_APPLICABLE** — no standing-approval primitive exists at this layer, source-verified |
| H4 | Persistence/Resume Binding | **PASS** — reconstructed `RunState`, multi-pending partial resolution, repeated resume of completed state all correct | **PASS** — `FileCheckpointStorage` cross-instance, same set of cases | **PASS** — `InMemorySaver` + real-file `SqliteSaver` (fresh connection + fresh compiled graph), same set of cases |
| H5 | Policy/Guardrail Drift | **PASS** — guardrails re-run at resume, across in-memory and serialize/reload paths | **PASS** (via checkpoint-approval scenario) | **PASS** — but only because the *application* built the re-check (`build_policy_checked_graph`); no framework primitive exists |
| H6 | Identity/Origin Binding | *(not separately isolated in EXP-003A; H1's call-id fingerprint check is the closest evidence)* | **PASS** — `ToolContext`-equivalent identity binding confirmed | **PASS** (own-distinct-pending-interrupt case) / **PASS_WITH_CAVEAT** (unknown-thread case raises `KeyError` rather than silently no-op-ing — an application-robustness note, not a bypass) |

**How to read a NOT_APPLICABLE cell:** it means the property has no
framework-native surface to test, evidenced by source inspection (e.g.
H3 on LangGraph: `InterruptOnConfig`'s exact four fields, checked
against the installed package). It is a different, stronger claim than
"untested" — see the [Threat Model](02-threat-model.md)'s coverage
matrix for what genuinely remains untested (H7-H9) across all three.

---

## What generalized across all three vs. what didn't

**Generalized cleanly, no framework-specific rewrite needed:**
- The H1-H6 hypothesis statements themselves (Assessment Methodology stage 8).
- The Baseline A/B/C pattern (Methodology stage 7).
- The PASS/FAIL/INCONCLUSIVE/NOT_APPLICABLE outcome vocabulary, extended only additively (`PASS_WITH_CAVEAT`, `CANDIDATE_FINDING_APPLICATION_LEVEL`) — never restructured per framework.
- The evidence-record shape in spirit: proposal → approval_request → approval_decision → delegation_path → state_boundary → execution → authorization_binding → outcome → notes, present in some form in all three labs' `evidence.py`/`core.py`.

**Needed real, framework-specific adaptation (each one a genuine finding about the framework, not friction to eliminate):**
- **Per-call identity availability.** EXP-003A's `ToolContext` lets the tool function read its own identity; EXP-003B's `FunctionInvocationContext` doesn't, so the `AuditRecord` has to be built by the *scenario* (which already holds the relevant identity from constructing the run), not by the *fixture*. This is why a shared Crypton `AuditRecord` abstraction cannot assume "the tool can self-report its own identity" — it must be built by whichever layer actually holds that identity, framework by framework.
- **Approval-correlation handle.** EXP-003A uses the tool call_id for both invocation and approval correlation. EXP-003B has a distinct `request_id` for approval correlation, separate from the underlying function call_id. A shared schema needs both fields, not one, or it silently loses information on B-shaped targets.
- **State/resume-boundary richness.** EXP-003B exposes `workflow`/`session`/`checkpoint_id` as three independently observable concepts; EXP-003A's target has no equivalent (a single serialized `RunState` blob is the whole unit); EXP-003C has only `thread_id`. A shared schema's `state_boundary` field has to be a free-form dict, not a fixed set of subfields, until a fourth target proves which subfields actually recur.
- **Model dependence.** EXP-003A/B both require a scripted chat-model test double to exercise authorization mechanics at all (because approval surfaces through the model's tool-call loop). EXP-003C requires none — its authorization primitive (`interrupt()`/`Command`) is entirely decoupled from any model. This means the *cost* of testing a new target varies structurally: a LangGraph-shaped target is cheaper to test than an agent-loop-shaped one, independent of how many hypotheses it needs.

---

## Procedure: adapting this pipeline to a fourth framework

1. **Architecture Review** (Methodology stage 1): read the target's real
   documentation and source for its approval/delegation/persistence
   primitives. Verify the installed version via `pip index versions` (or
   the ecosystem's equivalent) — never trust a version number handed to
   you in a prompt or a stale README, per the standing rule this exact
   guide's own correction note above demonstrates the cost of skipping.
2. **Concept mapping** (Security Property Catalog pillars 1-7): for each
   pillar, determine SUPPORTED / PARTIAL / NOT SUPPORTED at the
   framework-native layer, the way EXP-003C's README did explicitly.
   Do this *before* writing any test — it tells you which of H1-H9 are
   even framework-testable vs. structurally NOT_APPLICABLE.
3. **Baselines** (Methodology stage 7): build Baseline A (approve →
   executes) and Baseline B (reject → no-op) first. Add Baseline C
   (genuine persistence) only if the framework has a real persistence
   primitive to test against — use a real file/connection reload where
   possible (EXP-003C's `SqliteSaver` real-temp-file pattern), not just
   re-invoking the same in-memory Python objects, which would not
   actually exercise cross-process reconstruction.
4. **H1-H6, unchanged.** Do not invent new hypotheses for the new
   target. If a hypothesis needs a framework-specific test double (a
   scripted model, a fake chat client), build the minimum needed to
   exercise the real authorization surface — never mock the
   authorization/approval/delegation code itself.
5. **Evidence schema: extend, don't fork.** Add fields additively (as
   EXP-003B added `state_boundary`/`request_match`/`checkpoint_match`);
   document what's gained and what's lost relative to prior targets (as
   this guide's "What generalized" section does), rather than silently
   dropping information the new target doesn't happen to expose.
6. **Update this table.** A fourth framework's row belongs in the
   Architecture Summary and the Hypothesis Matrix above — this document
   is meant to grow with each new target, not be re-derived from
   scratch each time.
7. **Promotion gate (unchanged from EXP-003's operating rule):** do not
   promote a shared Crypton abstraction into `labs/agent-lab/src/agentlab/assessment/`
   until it has proven useful, unmodified, across **at least two**
   targets. As of EXP-003C, no such abstraction has cleared that bar —
   see [Assessment Methodology](03-assessment-methodology-v0.1.md) and
   each lab's own README "What Crypton may add" section.
