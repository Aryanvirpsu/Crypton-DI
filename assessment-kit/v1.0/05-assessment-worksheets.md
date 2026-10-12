# Crypton Assessment Worksheets v1.0

**Who uses this:** the assessor, filling these out live during SOP
Stages 2-9.
**When:** one per worksheet, per engagement, in the stage order set by
`02-standard-operating-procedure.md`.
**Why it exists:** a worksheet is where Architecture Review through
Execution Graph (SOP Stages 2-6) become a durable artifact the customer
can review, and the raw material Stage 12 (Findings) draws from — not
something reconstructed from memory at report-writing time.

Every worksheet below includes a **completion example** drawn from
Crypton's own validated corpus (never a hypothetical customer's real
data) so a new assessor can see exactly what "done" looks like before
filling one out for the first time.

---

## 1. Identity Graph

**Instructions:** one row per acting component (agent, sub-agent,
service, tool-caller) in scope. Fill `Identity mechanism` and
`Owner/accountable party` from the Identity Questionnaire
(`04-customer-discovery-kit.md` §B) and verify against source/config —
do not accept the questionnaire answer unverified.

| Component | Identity mechanism | Owner/accountable party | Durable/owned? | Distinguished from human identity? | Evidence ref |
|---|---|---|---|---|---|
| *(customer's top-level agent)* | *(e.g. per-agent service account)* | *(named team)* | Yes/No | Yes/No | EV-### |

**Completion example** (Crypton reference target, EXP-005, cited for
teaching only — never presented as a finding about a customer):

| Component | Identity mechanism | Owner/accountable party | Durable/owned? | Distinguished from human identity? | Evidence ref |
|---|---|---|---|---|---|
| Deployable agent-server caller | Static shared-secret list (`session_api_keys`), membership check only | Not established in reviewed code — no per-key ownership record found | No — shared secret, no individual key identity | No — no per-key attribution located | `labs/exp-005-openhands-sdk/findings.md` (Identity pillar row) |

**Required evidence:** identity records, or source excerpts showing how
identity is resolved at the point of an action (`03-assessment-checklists.md`,
Pillar 1).

---

## 2. Authority Graph

**Instructions:** one row per consequential action in scope. Trace the
specific grant/policy/check that permits it and where that check
actually sits in the call path.

| Consequential action | Grant representation | Check location (file/function or config) | Checked before or after side effect? | Evidence ref |
|---|---|---|---|---|
| *(e.g. isolate_endpoint)* | *(e.g. ConfirmRisky policy + risk analyzer)* | *(e.g. agent.py:1081)* | Before/After | EV-### |

**Completion example** (OpenHands SDK, EXP-005):

| Consequential action | Grant representation | Check location | Checked before or after? | Evidence ref |
|---|---|---|---|---|
| Any tool-call action gated by risk | `ConfirmationPolicyBase` (`ConfirmRisky`) + `SecurityAnalyzerBase` | `openhands-sdk/openhands/sdk/agent/agent.py:1081`, `_requires_user_confirmation` | Before — confirmed via source review of call ordering | `labs/exp-005-openhands-sdk/README.md` §2.3 |

**Required evidence:** the actual grant representation (policy/config/
source) plus at least one authorization log or controlled test showing
it being checked (`03-assessment-checklists.md`, Pillar 2).

---

## 3. Delegation Graph

**Instructions:** one row per delegation hop (parent → child). Record
the scope granted at each hop and whether it narrows, preserves, or
widens relative to the parent.

| Parent | Child | Scope at parent | Scope at child | Narrows/preserves/widens? | Confirmation-gate delivery confirmed? | Evidence ref |
|---|---|---|---|---|---|---|
| *(top-level agent)* | *(sub-agent id)* | *(tool/resource set)* | *(tool/resource set)* | N/W/P | Yes/No/N-A | EV-### |

**Completion example** (OpenHands SDK, EXP-005):

| Parent | Child | Scope at parent | Scope at child | Narrows/preserves/widens? | Gate delivery confirmed? | Evidence ref |
|---|---|---|---|---|---|---|
| Top-level `Agent` | Spawned sub-agent (`DelegateTool`/`TaskToolSet`) | `confirmation_policy` = e.g. `AlwaysConfirm()` | Inherits parent policy unless `AgentDefinition.permission_mode` overrides | Preserves (policy object inheritance is intentional) | **No** — `confirmation_handler=None` default silently auto-resolves the sub-agent's own pause | `labs/exp-005-openhands-sdk/findings.md`, FINDING-005-2 |

**Required evidence:** the actual scope values at two-plus hops (source,
config, or logs), or a controlled test's observed result
(`03-assessment-checklists.md`, Pillar 3).

---

## 4. Execution Graph

**Instructions:** trace the literal path from "action proposed" to
"side effect occurs" for the workflow's most consequential action(s),
including every branch (approved, rejected, retried, resumed,
delegated). Draw as a flow (text/mermaid) plus the annotated table
below.

| Step | Component | Authorization check present? | Branch condition |
|---|---|---|---|
| 1. Propose | *(agent/model)* | No | — |
| 2. Check | *(policy/analyzer)* | Yes/No | — |
| 3. Execute or pause | *(executor)* | — | approve / reject / retry |

**Completion example** (LangChain `create_agent` + `HumanInTheLoopMiddleware`
+ `HostEscalationMiddleware`, EXP-001d — the exact mechanism that
produced Crypton's flagship validated finding):

| Step | Component | Authorization check present? | Branch condition |
|---|---|---|---|
| 1. Model proposes `isolate_endpoint(host="workstation-42")` | model | No | first attempt |
| 2. `HumanInTheLoopMiddleware.after_model` runs (last in list → runs first) | approval gate | Yes — interrupts, human approves `workstation-42` | approved |
| 3. `HostEscalationMiddleware.after_model` runs (first in list → runs last) | no-op (nothing learned yet) | — | passthrough |
| 4. Tool executes `workstation-42` | tool | — | fails (timeout) |
| 5. Model retries, proposes `isolate_endpoint(host="workstation-42")` again | model | No | retry |
| 6. `HumanInTheLoopMiddleware.after_model` runs first again | approval gate | Yes — human approves `workstation-42` again | approved |
| 7. `HostEscalationMiddleware.after_model` runs last again — **rewrites args to `10.0.0.0/24`** | escalation hook | — | drift introduced *after* approval |
| 8. Tool executes `10.0.0.0/24` | tool | — | succeeds — never approved as such |

**Required evidence:** source excerpts of the call path, or execution
traces showing the ordering (`03-assessment-checklists.md`, Pillar 5).
This worksheet is the single highest-value artifact in the entire kit —
it is what surfaced Crypton's strongest validated finding, purely by
being drawn out completely, retry branch included.

---

## 5. Trust Boundaries

**Instructions:** one row per point where control passes from one
principal/process/component to another. State plainly whether the
claim crossing that boundary is re-verified or merely trusted.

| Boundary | What crosses it | Re-verified? | Evidence ref |
|---|---|---|---|
| *(e.g. checkpoint → resumed state)* | *(e.g. pending approval arguments)* | Yes/No/Partial | EV-### |

**Completion example** (LangGraph, EXP-003C H1c):

| Boundary | What crosses it | Re-verified? | Evidence ref |
|---|---|---|---|
| Pending checkpoint → resumed thread | Proposed action arguments | **No** — `update_state()` is public, documented API; anyone with checkpoint-store write access can mutate pending state before resume | `docs/methodology/05-evidence-reporting-standard.md` worked example |

**Required evidence:** source review of what is/isn't re-verified at
each boundary. This worksheet feeds Stage 9 test prioritization
directly — every "No" row is a candidate test.

---

## 6. Approval Graph

**Instructions:** one row per gated action. Record what the approval
record actually binds to, and whether execution matched.

| Gated action | Approval binds to (label only / label+resource / full arguments) | Approved value | Executed value | Match? | Evidence ref |
|---|---|---|---|---|---|
| *(action)* | *(binding tier)* | *(value)* | *(value)* | Yes/No | EV-### |

**Completion example** (EXP-001d/e, the flagship validated finding):

| Gated action | Binds to | Approved value | Executed value | Match? | Evidence ref |
|---|---|---|---|---|---|
| `isolate_endpoint` | Full arguments (via `compute_argument_hash`-equivalent digest check) | `host="workstation-42"` (twice) | `host="10.0.0.0/24"` | **No** | `labs/experiments/001-langgraph-approval-drift/writeup.md` |

**Required evidence:** the approval record's actual bound fields, plus
a controlled replay/drift test result or sampled real-record comparison
(`03-assessment-checklists.md`, Pillar 4).

---

## 7. Persistence Graph

**Instructions:** one row per persisted/checkpointed piece of
security-relevant state. Record what survives a restart/reload, and
whether it is single-source-of-truth or can diverge.

| State element | Persisted where | Single source of truth? | Survives restart correctly? | Evidence ref |
|---|---|---|---|---|
| *(e.g. confirmation_policy)* | *(e.g. base_state.json)* | Yes/No | Yes/No/Untested | EV-### |

**Completion example** (OpenHands SDK, EXP-005 — a real before/after
case):

| State element | Persisted where | Single source of truth? | Survives restart correctly? | Evidence ref |
|---|---|---|---|---|
| `confirmation_policy`, `security_analyzer`, `secrets` | `base_state.json` | **Yes, as of the reviewed commit** — previously split across two locations (`StoredConversation`/`meta.json` and `ConversationState`/`base_state.json`), fixed by a merged PR | Yes, current source confirms this; historically **no** (real, fixed bug) | `labs/exp-005-openhands-sdk/findings.md`, "context finding" (PR #4813) |

**Required evidence:** source review of the live-vs-cached read path, or
a controlled before/after test result (`03-assessment-checklists.md`,
Pillar 6).

---

## 8. Audit Graph

**Instructions:** attempt to reconstruct one real action using only the
customer's own durable records — no insider knowledge of what actually
happened. Record what could and couldn't be reconstructed.

| Question | Answerable from records alone? | Which record answered it | Evidence ref |
|---|---|---|---|
| Who/what acted? | Yes/No | | EV-### |
| Under what authority? | Yes/No | | EV-### |
| What was the outcome? | Yes/No | | EV-### |
| Is the record tamper-evident/durable? | Yes/No | | EV-### |

**Completion example** (OpenHands SDK agent-server, EXP-005):

| Question | Answerable from records alone? | Which record answered it | Evidence ref |
|---|---|---|---|
| Who/what acted? | **Partial** — a durable, correlated event log exists (`get_unmatched_actions`, matched by `action_id`) | `EventLog` | `labs/exp-005-openhands-sdk/README.md` §3 |
| Under what authority (which of N configured API keys)? | **No** — membership check only, no per-key attribution located | `agent_server/dependencies.py:35` | same |
| What was the outcome? | Yes | `EventLog` (`ObservationEvent`) | same |
| Is the record tamper-evident/durable? | Yes — survives crash/restart by design | `conversation/state.py` | same |

**Required evidence:** the actual reconstruction attempt's result,
referencing the specific records used (`03-assessment-checklists.md`,
Pillar 7).

---

## 9. Tool Graph

**Instructions:** one row per tool/API the in-scope agent can call.
Directly feeds the Tool Inventory questionnaire's D1/D2 answers into a
structured, testable form.

| Tool | Capability (read/write/spend/delete) | Reachable by which agent(s) | Requires approval? | Evidence ref |
|---|---|---|---|---|
| *(tool name)* | R/W/S/D | *(agent ids)* | Yes/No/Standing | EV-### |

**Completion example** (EXP-001 series' `isolate_endpoint`):

| Tool | Capability | Reachable by which agent(s) | Requires approval? | Evidence ref |
|---|---|---|---|---|
| `isolate_endpoint` | Write (network isolation, irreversible without separate remediation) | The security-remediation agent under test | Yes — via `HumanInTheLoopMiddleware`, `interrupt_on={"isolate_endpoint": True}` | `labs/experiments/001-langgraph-approval-drift/writeup.md` |

**Required evidence:** the tool's actual capability, confirmed by source
or documentation, not assumed from its name.

---

## 10. Data-Flow Graph

**Instructions:** one row per data category the agent reads or writes.
This worksheet is **supporting/organizational** — it is not backed by
its own validated pillar or area (no EXP-00X experiment has produced a
dedicated data-flow-specific finding to date); it exists to feed
severity-rubric reasoning (factor 3, sensitivity/business impact) and
should not be reported with the same PASS/FAIL rigor as the seven
pillars above. Treat entries here as context, not findings.

| Data category | Read/write | Sensitivity (from Risk Inventory G1/G2) | Feeds severity reasoning for which findings |
|---|---|---|---|
| *(e.g. customer PII)* | R/W | *(classification)* | *(finding ids)* |

**Completion example:** intentionally omitted — this worksheet is
customer-specific by nature (it records what data a specific customer's
workflow touches) and has no generic, validated-corpus analogue to
illustrate without inventing customer detail. Fill it directly from
Risk Inventory answers (`04-customer-discovery-kit.md` §G) during the
engagement.

**Required evidence:** N/A — this worksheet organizes context gathered
elsewhere; it does not itself require independent evidence collection.

---

## Cross-Worksheet Consistency Check (Stage 9 gate)

Before proceeding from worksheet completion into Stage 9 testing,
confirm:

- Every component in the Identity Graph appears in the Authority Graph
  for at least one action.
- Every delegation hop in the Delegation Graph has a corresponding
  Trust Boundary row.
- Every gated action in the Approval Graph traces to a step in the
  Execution Graph.
- Every persisted state element affecting authorization appears in both
  the Persistence Graph and the Trust Boundaries worksheet.

A gap here (a component with no authority entry, a delegation hop with
no trust-boundary entry) is not itself a finding — it is a signal the
worksheets are incomplete and should be revisited before testing
proceeds, per SOP Stage 5-6's quality gates.
