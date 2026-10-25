# Crypton Finding Library v1.0

**Who uses this:** the assessor, during Stage 9 (as a pattern reference
for what to test for) and Stage 12 (as a template for writing up a
newly-discovered instance of a known pattern).
**When:** every engagement — check whether a new observation matches an
existing library pattern before treating it as novel.
**Why it exists:** so Crypton's assessors recognize a recurring failure
class instantly instead of re-deriving it from first principles every
engagement, without ever fabricating a finding that hasn't actually
been validated.

**Inclusion rule (per `VALIDATION-REPORT.md` §5, Deliverable 12 Part
E):** only findings that survived independent re-review, are
customer-facing (not internal code-hygiene notes about a research
target), and generalize as a *pattern* beyond the one specimen they were
found in are included here. Three entries (a fourth, FL-01, was retired
2026-09-27 — see "Excluded From This Library" below). This is not a
small number by accident — the corpus that could produce entries is
itself small (`VALIDATION-REPORT.md` §3), and a short, honest library is
more useful than a padded one.

**A finding library entry describes a pattern to test for. It is never
itself evidence about a customer's system** — see
`01-assessor-handbook.md` §9's rule on Crypton-reference demonstrations.
Finding a customer's system matches a library entry's pattern still requires
collecting real evidence from that customer's own system, through the
full sixteen-field package (`06-evidence-standard.md`).

---

## FL-01 — RETIRED (2026-09-27)

**Former title:** Approval/Execution Drift via Benign Middleware
Composition.

This entry (built from `finding-001d`/`finding-001e`, the `langchain`
`after_model` middleware-ordering mechanism) is **retired** and no
longer a customer-facing finding pattern. Applying the §9a triage rule
(`01-assessor-handbook.md`) to the underlying observation: the mutator
(`HostEscalationMiddleware`) is trusted, developer-authored application
code in the same trust domain as the approval gate it was composed
with; it is permitted by the framework's own `after_model` contract to
alter tool calls (proven by `PIIMiddleware`'s shipped use of the
identical mechanism); and LangChain never claims approval-argument
binding survives a chain of trusted `after_model` hooks composed
afterward. No untrusted principal, model, tool, or input crossed any
framework-claimed boundary. Final classification:
`NO VULNERABILITY — EXPECTED FRAMEWORK BEHAVIOR`.

Full reasoning, and what is retained as calibration history:
`labs/experiments/001-langgraph-approval-drift/CLOSURE.md`. See also
"Excluded From This Library" below and `VALIDATION-REPORT.md` §1/§5.

FL-02 (below) is a different, still-valid, still-active mechanism —
`update_state()` argument mutation, which requires an actor to already
hold write access to the target's own checkpoint store, a genuinely
separate trust question this retirement does not affect.

---

## FL-02 — Checkpoint/Resume State Mutation Before Approval Consumption

**Title:** A framework's legitimate, documented state-mutation API can
alter a pending approval's bound arguments before it is consumed,
without the framework claiming otherwise.

**Description:** where an agent framework provides checkpoint/resume
persistence for a paused (awaiting-approval) workflow, and that
framework also exposes a public, documented API for mutating persisted
state, nothing structurally prevents a caller with write access to the
checkpoint store from changing the pending action's arguments between
the moment a human is shown the proposal and the moment execution
actually resumes. The framework has not misrepresented its own
contract — it never claimed pending interrupts are frozen against its
own state-mutation API — which is exactly what makes this an
application-level trust-boundary question, not a framework defect.

**Affected property:** Pillar 2 (Authority), Pillar 6 (Persistence).
**Threat:** Modify execution.

**Typical causes:**
- A checkpoint/state store whose write access is not scoped to only the
  process/principal that originally proposed the pending action.
- No re-verification of the proposal's arguments against what was
  actually shown to the human, at resume time.

**Detection guidance:** complete the Persistence Graph and Trust
Boundaries worksheets (`05-assessment-worksheets.md` §5, §7) for every
checkpoint/resume mechanism in scope; specifically ask "who else can
write to this pending state, and is that access independently
controlled?"

**Evidence required:** source review confirming the state-mutation API
exists and is reachable by more than the original proposer, or a
controlled test mutating pending state and observing the executed
result diverge from the originally-shown proposal.

**Severity guidance:** typically bounded toward MEDIUM — exploitation
requires an attacker or process to already hold write access to the
target's own checkpoint store, a meaningfully higher bar than the
retired FL-01 pattern's no-special-access mechanism (see "Excluded
From This Library"). Do not default to this bound without checking the
specific customer's actual access-control posture around their
checkpoint store — a customer with broad, uncontrolled checkpoint-store
write access should be scored higher.

**Remediation guidance:** treat "who can write to pending checkpointed
state" as part of the application's own access-control boundary — scope
checkpoint-store write access to the same principal(s) trusted to
resume the workflow at all. This is a control recommendation for
integrators, not a request that the framework's maintainers change
documented, working-as-designed behavior.

**References:** `docs/methodology/05-evidence-reporting-standard.md`
(worked example), EXP-003C.
**Related findings:** none active (the formerly-related FL-01 is
retired — see "Excluded From This Library").
**Confidence:** High on mechanism (source-verified against the
installed framework's own documented API contract); the severity bound
is a reasoned, evidence-based judgment, not a certainty — re-derive it
per customer.

**Structured fields (2026-09-30, drawn from the prose above — nothing new):**
- **Invariant:** a pending approval's bound arguments must not change between
  the moment a human is shown the proposal and the moment execution resumes,
  regardless of the mechanism used to change them.
- **Preconditions:** the framework provides checkpoint/resume persistence
  for a paused, approval-awaiting workflow, *and* exposes a public,
  documented API for mutating persisted state.
- **Applicability:** frameworks offering a public state-mutation API over
  checkpointed/persisted workflow state (e.g. LangGraph's `update_state()`).
- **Required semantic facts:** checkpoint/resume persistence exists; a
  public state-mutation API exists; the write-access scope of that API/store
  is traceable.
- **Retest condition:** re-verify after checkpoint-store write access is
  scoped to only the principal(s) trusted to resume the workflow, or after
  proposal arguments are re-verified against what was shown to the human at
  resume time.

---

## FL-03 — Authorization Gate Fed by an Unverified Self-Report

**Title:** A risk- or authorization-gating decision that relies solely
on a signal supplied by the same actor whose action is being evaluated
provides no real independent check, even when it is correctly wired
and correctly enforced.

**Description:** some agent systems support risk-adaptive confirmation
— an action's required approval level depends on a computed risk
score. Where that risk score's *sole* source is the acting model's own
self-report (e.g. a field the model itself populates in its tool-call
arguments, interpreted verbatim by the "analyzer"), the entire gating
decision reduces to trusting the actor being gated. This is distinct
from, and structurally worse than, argument-binding drift (FL-02)
— there, an approval exists but gets defeated after the fact; here, the
decision of *whether to even require approval* can be satisfied by the
actor itself, with the enforcement mechanism working exactly as
designed.

**Affected property:** Pillar 2 (Authority), Pillar 4 (Human Approval).
**Threat:** Bypass approval, Confuse authority.

**Typical causes:**
- A "smart"/risk-adaptive confirmation policy layered on top of a
  correctly-functioning binary approve/deny gate, where the risk
  computation itself has no independent implementation option offered,
  or where an independent option exists but isn't used together with
  the self-report option.
- No documentation warning that the risk-adaptive mode, used alone,
  provides no adversarial/error robustness.

**Detection guidance:** trace the exact source of the risk/priority
signal feeding any risk-adaptive confirmation policy in scope
(`03-assessment-checklists.md`, Pillar 4, "Self-Reported Risk
Independence" — EVOLVING). Ask specifically: "if the acting model
mislabels this action's risk, is there anything else that would catch
it?"

**Evidence required:** source review tracing the risk signal from
generation to the confirmation-policy check, confirming whether an
independent, deterministic source is or isn't involved.

**Severity guidance:** MODERATE by default (requires a specific
configuration choice by the integrator to be exploitable — a
correctly-composed deployment using an independent analyzer alongside
the self-report is not affected). Do not default to a fixed severity —
confirm the customer's actual configuration before scoring; this
pattern is config-dependent by nature, not a universal defect.

**Remediation guidance:** never gate a consequential approval decision
on a signal sourced solely from the actor being gated. Compose any
self-reported/model-derived risk signal with at least one independent,
deterministic source, and take the higher (more cautious) of the two —
never the self-report alone.

**References:** `labs/exp-005-openhands-sdk/findings.md`, FINDING-005-1.
**Related findings:** none directly — this is currently a single-instance
pattern (see confidence note).
**Confidence:** High on mechanism (source-verified); this pattern is
currently evidenced against **one** target only — present it to a
customer as "a real, demonstrated pattern worth checking for," not as
an established, cross-industry-validated finding class, per
`VALIDATION-REPORT.md` §2's promotion-gate discipline.

**Structured fields (2026-09-30, drawn from the prose above — nothing new):**
- **Invariant:** an authorization/risk-gating decision must be fed by at
  least one signal source independent of the actor whose action is being
  evaluated.
- **Preconditions:** a risk-adaptive confirmation policy exists, *and* its
  sole configured risk signal source is the acting model's own self-report.
- **Applicability:** agent systems with a risk-adaptive/"smart" confirmation
  policy layered on top of a correctly-functioning binary approve/deny gate.
- **Required semantic facts:** a risk-adaptive confirmation policy exists;
  the risk signal's source is traceable to either the actor itself or an
  independent, deterministic component.
- **Retest condition:** re-verify after an independent, deterministic
  risk-signal source is composed with the self-report and the more cautious
  of the two is taken, never the self-report alone.

---

## FL-04 — Delegated Confirmation Pauses Not Delivered to a Human by Default

**Title:** A multi-agent delegation mechanism can have a real,
independently-functioning confirmation gate at the sub-agent level that
is nonetheless never actually shown to a human, because the delegation
executor's default behavior silently resolves the pause on its own.

**Description:** where a system supports spawning delegated sub-agents,
and each sub-agent carries its own confirmation-policy enforcement
(inherited from or independent of its parent), the delegation *executor*
— the code responsible for running a sub-agent to completion and
returning its result to the parent — is a separate integration point
that must itself forward any confirmation-required pause to an actual
human decision-maker. Where the executor's default behavior (no extra
integration work required) is to treat "confirmation required" as
"proceed anyway" absent an explicitly-wired callback, the sub-agent's
own gate is real but never actually exercised in the default
configuration — including, potentially, in the system's own reference
integration examples.

**Affected property:** Pillar 3 (Delegation), Pillar 4 (Human Approval).
**Threat:** Bypass approval, Escape delegation.

**Typical causes:**
- A delegation/sub-agent orchestration layer with an optional
  confirmation-handling callback parameter that defaults to `None`/no-op
  rather than failing closed.
- A reference example or quick-start guide that demonstrates delegation
  without demonstrating the confirmation-handler wiring, teaching
  integrators an incomplete pattern by default.

**Detection guidance:** complete the Delegation Graph worksheet
(`05-assessment-worksheets.md` §3) and specifically fill the
"Confirmation-gate delivery confirmed?" column for every delegation
hop — do not assume "yes" without tracing the executor's actual
default-parameter behavior in source.

**Evidence required:** source review of the delegation executor's
handling of a sub-agent's pending-confirmation state when no explicit
handler is supplied, ideally corroborated by checking whether any
shipped example/reference integration demonstrates correct wiring.

**Severity guidance:** tends toward HIGH — this pattern, when present,
typically fires on the *documented default path* (no attacker position
required, zero human interaction occurs). Weigh down if the specific delegation feature is opt-in and
rarely used in the customer's actual workflow, or if a wrapper layer
elsewhere in the customer's own integration does supply a real handler.

**Remediation guidance:** the delegation executor should fail closed
(block or raise) rather than silently proceed when a sub-agent's
confirmation-requiring action pauses and no handler is wired — forcing
the integrator to make an explicit choice. Update any reference
integration examples to demonstrate correct wiring, since it is the
most likely place a new integrator copies from.

**References:** `labs/exp-005-openhands-sdk/findings.md`, FINDING-005-2.
**Related findings:** FL-03 (both share the shape "a real control
exists somewhere in the pipeline, but something downstream of it
silently defeats or bypasses its outcome").
**Confidence:** Very high on mechanism (independently re-derived by two
separate reviews — see `VALIDATION-REPORT.md` §1); single-target
evidence for generalization, same caveat as FL-03.

**Structured fields (2026-09-30, drawn from the prose above — nothing new):**
- **Invariant:** a sub-agent's confirmation-requiring pause must reach an
  actual human decision-maker unless the integrator has explicitly chosen
  otherwise.
- **Preconditions:** a delegation/sub-agent orchestration layer exists,
  *and* its executor has a confirmation-handling callback parameter that
  defaults to no-op/`None` rather than failing closed.
- **Applicability:** multi-agent delegation frameworks where a sub-agent's
  own confirmation gate exists but the delegation executor is a separate
  integration point responsible for forwarding it to a human.
- **Required semantic facts:** a delegation executor exists; the executor's
  default behavior on an unhandled pending-confirmation state is traceable
  in source.
- **Retest condition:** re-verify after the delegation executor fails
  closed (blocks/raises) on an unhandled confirmation-requiring pause, and
  after any reference integration examples are updated to demonstrate
  correct wiring.

---

## Known-Safe Patterns (reference material, not vulnerability findings)

These are validated, sourced, *negative* results — configurations
Crypton specifically tried to break and could not, under the tested
conditions. Use these during a customer conversation to illustrate what
"good" looks like, or to calibrate expectations when a customer's
architecture resembles one of these patterns.

| Pattern | What was tested | Result | Reference |
|---|---|---|---|
| LangGraph interrupt-in-tool HITL pattern | `interrupt()` called inside the `@tool` function body itself | Structurally bypass-resistant — every invocation (first attempt and retry) re-enters `interrupt()`, because it's the only path to the tool's effect | `labs/experiments/001-langgraph-approval-drift/finding-001b.md` |
| LangGraph separate-review-node HITL pattern | A dedicated `human_review_node` between the model and tool-execution nodes, per the official `langgraph-ai/langgraphjs` "Review Tool Calls" how-to | Structurally bypass-resistant — no edge into the tool-execution effect exists that doesn't pass through the review node | `labs/experiments/001-langgraph-approval-drift/finding-001c.md` |
| Delegation-boundary independence (parent approval never confers child tool approval) | Tested identically across three structurally different frameworks (OpenAI Agents SDK's `Agent.as_tool()`, Microsoft Agent Framework's `Agent.as_tool()`, LangGraph's independent parent/child `interrupt()`s) | PASS on all three — Crypton's strongest cross-framework result to date | `docs/methodology/04-framework-adaptation-guide.md` |
| Exactly-once execution across checkpoint save/restore | Tested identically across all three EXP-003 targets, genuine cross-process-style persistence (fresh connection/process on reload) | PASS on all three | `docs/methodology/04-framework-adaptation-guide.md` |
| Guardrail/policy re-evaluation at resume (no TOCTOU window) | Tested across all three EXP-003 targets | PASS on all three (LangGraph's PASS required the *application* to build the re-check — no framework primitive existed there) | `docs/methodology/04-framework-adaptation-guide.md` |
| Authorization-state persistence single-source-of-truth (post-fix) | OpenHands SDK's `confirmation_policy`/`security_analyzer`/`secrets` on resume | PASS as of the reviewed commit — was previously FAIL (dual persisted locations), fixed by a real, merged upstream PR | `labs/exp-005-openhands-sdk/findings.md` |
| Agent tool-set verification on resume (tools may only be added, never removed) | OpenHands SDK's `AgentBase.verify()` | PASS — explicit, documented, source-confirmed rule | `labs/exp-005-openhands-sdk/README.md` §2.7 |

**Do not present these as guarantees about a customer's own system** —
each is a validated result about a specific documented pattern or a
specific reviewed commit, not a blanket claim that "LangGraph is safe"
or "OpenHands is safe." Always re-verify the customer's actual
implementation matches the tested pattern before relying on these as
comparison points.

---

## Excluded From This Library (and why)

| Item | Why excluded |
|---|---|
| EXP-001's original `finding-001` | Superseded — author-induced bug in a synthetic specimen, not representative of any real framework/pattern (`VALIDATION-REPORT.md` §1). Retained only as internal methodology-development history. |
| **FL-01** (`finding-001d`/`finding-001e`, retired 2026-09-27) | `NO VULNERABILITY — EXPECTED FRAMEWORK BEHAVIOR`. Applying §9a's triage rule, the mutator was trusted, developer-authored middleware in the same trust domain as the approval gate, permitted by the framework's own contract, and no framework-claimed boundary was crossed. Not a customer-assessable defect pattern. Retained as a negative/calibration result: `labs/experiments/001-langgraph-approval-drift/CLOSURE.md`. |
| EXP-005 FINDING-005-3 (duplicate dead confirmation logic) | Target-maintainer code-hygiene note, not a customer-assessable pattern (`VALIDATION-REPORT.md` §5). |
| EXP-003A H3, EXP-003B H3 (standing approval, argument-blind by design) | Not a gap — EXPECTED DESIGN, no vendor claims otherwise. Documented in `03-assessment-checklists.md`'s Pillar 4 questions as something to *ask about*, not a finding pattern. |
