# Crypton Agent Authorization Assessment Methodology v0.1

**EXP-004, deliverable 1 (the core playbook).** Status: v0.1, internally
exercised (one synthetic reference dry run, three cross-framework
research labs), no external customer engagement yet. This is the
document every future assessment, experiment, and eventually customer
engagement should follow.

**Relationship to `docs/ASSESSMENT_METHODOLOGY.md`:** that document (also
internally marked "methodology v0.1" in its own status line, written
2026-09-02/v0.6.1) is **not superseded or duplicated by this one** — it
is the detailed, already-working **technical procedure for stages 8-9
below** (Universal Hypotheses / Framework-specific Tests), specified as
eight concrete assessment areas with outcomes, evidence categories, and
a severity rubric, and it is already load-bearing: `assessment/models.py`'s
`AssessmentArea` enum and `Finding.__post_init__`'s evidence requirement
are real code built directly against it. This document is the layer
above it — the end-to-end pipeline that explains where those eight areas
fit relative to architecture review, threat modeling, and reporting, and
gives Crypton's own research labs (EXP-00X) a repeatable shape to
produce evidence *before* a customer exists. Read them together: this
document for the whole arc of an assessment or experiment; the existing
one for exactly how stages 8-9 are actually executed and scored.

---

## The pipeline

```
Architecture Review
        │
Identity Mapping
        │
Authority Mapping
        │
Trust Boundaries
        │
Execution Graph
        │
Threat Model
        │
Baseline Verification
        │
Universal Hypotheses
        │
Framework-Specific Tests
        │
Evidence Collection
        │
Risk Analysis
        │
Findings
        │
Remediation
```

No framework-specific assumption enters before **Framework-Specific
Tests** (stage 9). Everything before that stage is expressed entirely
in the [Security Property Catalog](01-security-property-catalog.md)'s
seven pillars and the [Threat Model](02-threat-model.md)'s eleven
attacker objectives — this is what makes the same pipeline reusable
whether the target is a real customer's production workflow or a
research lab's isolated test harness (EXP-003A/B/C are three completed
runs of stages 6-11 against three different frameworks, using identical
stage-6/7/8 content each time).

---

## Stage definitions

### 1. Architecture Review
**Purpose:** understand what the system under assessment actually is before forming any hypothesis about it.
**Inputs:** architecture docs, IaC, deployment topology, source access, or — for a research lab — the target framework's own documentation and source.
**Outputs:** a plain description of the agent workflow's shape: single-agent or multi-agent, which framework/runtime, which tools/APIs are reachable, whether delegation exists, whether persistence/checkpointing exists.
**Precedent:** `docs/business/ASSESSMENT_PACK.md`'s Intake Questionnaire §D questions 1-4 are this stage's customer-facing instrument. EXP-003A/B/C's `target.json` files and each lab's README "Why [framework]" section are this stage's research-lab equivalent — each one documents the target's shape and why it was selected before any test is written.

### 2. Identity Mapping
**Purpose:** establish, for every acting component, what its identity actually is and how it's asserted.
**Pillar:** [Pillar 1 — Identity](01-security-property-catalog.md#pillar-1--identity).
**Outputs:** an inventory of every agent/service/tool identity in scope, how each authenticates, who owns each.
**Precedent:** `docs/ASSESSMENT_METHODOLOGY.md` area 1 (Agent Identity & Ownership); intake questionnaire §D questions 5-6.

### 3. Authority Mapping
**Purpose:** establish where authority is created, how it's represented, and what actually gates each consequential action.
**Pillar:** [Pillar 2 — Authority](01-security-property-catalog.md#pillar-2--authority).
**Outputs:** for each consequential action, the specific grant/policy/check that permits it, and where that check lives in the call path.
**Precedent:** `docs/ASSESSMENT_METHODOLOGY.md` areas 2 and 4; intake questionnaire §D questions 7-9, 12.

### 4. Trust Boundaries
**Purpose:** identify every point where control passes from one principal, process, or component to another — every place a claim gets trusted rather than independently re-verified.
**Pillars:** [Pillar 3 — Delegation](01-security-property-catalog.md#pillar-3--delegation), [Pillar 6 — Persistence](01-security-property-catalog.md#pillar-6--persistence).
**Outputs:** a list of boundaries (human→agent, agent→agent, agent→tool, live-state→checkpoint→resumed-state) and, for each, what is and isn't re-verified when control crosses it.
**Precedent:** this is the stage EXP-003C's H1c result belongs to conceptually — `update_state()` is a trust boundary (who else can legitimately write to pending checkpointed state) that wasn't obvious until named explicitly as one.

### 5. Execution Graph
**Purpose:** trace the literal path from "action proposed" to "side effect occurs," including every branch (approved, rejected, retried, resumed, delegated).
**Pillar:** [Pillar 5 — Execution](01-security-property-catalog.md#pillar-5--execution).
**Outputs:** a call-graph-level trace of the execution path, annotated with where each authorization check sits relative to the side effect.
**Precedent:** `docs/ASSESSMENT_METHODOLOGY.md` area 4's "source review of the exact ordering" procedure; each EXP-003 lab's scenario functions (`scenarios/*.py`) are literally this stage's artifact for a research target.

### 6. Threat Model
**Purpose:** given the mapped identity/authority/trust-boundary/execution shape, decide which of the eleven attacker objectives are actually reachable in this specific system, and which are structurally not applicable.
**Reference:** [Threat Model](02-threat-model.md) in full.
**Outputs:** a scoped subset of the eleven objectives, each marked IN SCOPE or NOT APPLICABLE for this specific target, with the reason (e.g. "Persist privilege is not applicable — this engagement has no live/production traffic to measure revocation propagation against," matching `docs/ASSESSMENT_METHODOLOGY.md` area 7's own NOT_APPLICABLE guidance).

### 7. Baseline Verification
**Purpose:** before testing any adversarial hypothesis, confirm the ordinary, happy-path behavior actually works as expected — an approve-and-execute path, and a reject-and-no-op path.
**Outputs:** Baseline A (approve → executes correctly) and Baseline B (reject → zero side effects) results, minimum. Where genuine persistence is in scope, Baseline C (persist → reload via a fresh instance → resume correctly).
**Precedent:** every EXP-003 lab's `test_baseline*.py`/`test_baseline_a/b/c` functions follow this exact three-baseline shape, and each README's "Baselines" section reports them before any hypothesis result — established, working convention, not new to this document.
**Rule:** if a baseline fails, do not proceed to adversarial hypotheses against that path — a broken happy path makes any adversarial result about it uninterpretable (this is the same discipline that caught the semantic-rulebook Baseline A tautology bug: fix the baseline, don't build findings on top of it).

### 8. Universal Hypotheses
**Purpose:** state, in framework-independent language, exactly which properties from the Security Property Catalog will be tested, scoped by stage 6's threat model.
**Reference:** the six standing hypotheses (H1-H6), defined once and reused verbatim across every target:

| # | Hypothesis | Pillar |
|---|---|---|
| H1 | Argument Binding — do approved arguments drift before execution? | Execution, Authority |
| H2 | Delegation Boundary — does parent/delegation approval confer child tool authority? | Delegation |
| H3 | Standing Approval — what is the scope of a broad/"always approve" authorization? | Human Approval |
| H4 | Persistence/Resume Binding — is execution exactly-once across checkpoint save/restore? | Execution, Persistence |
| H5 | Policy/Guardrail Drift — is there a TOCTOU window between approval and execution? | Authority |
| H6 | Identity/Origin Binding — can approval cross agent/thread/workflow identities? | Identity, Authority |

Per the [Threat Model](02-threat-model.md)'s coverage matrix, three further hypotheses are defined but not yet run against any target — carried here so a future EXP-005 extends the same numbered set rather than inventing disconnected new ones:

| # | Hypothesis | Pillar | Attacker objective |
|---|---|---|---|
| H7 | Scope/Target Expansion — can the executed target/resource set grow beyond what was proposed? | Authority, Delegation | Expand authority |
| H8 | Attribution Durability — does the target framework itself persist a reconstructable authorization chain, independent of any test harness? | Auditability | Lose attribution |
| H9 | Tool Substitution — can an action authorized through one interface be satisfied through a different, less-constrained interface to the same effect? | Execution | Modify execution |

**Rule (unchanged from EXP-003's operating discipline):** do not invent new hypotheses per target. A framework that structurally cannot support a hypothesis (e.g. H3 on raw LangGraph) gets NOT_APPLICABLE, sourced from the framework's own documentation/code — never a manufactured equivalent test.

### 9. Framework-Specific Tests
**Purpose:** translate each in-scope universal hypothesis (stage 8) into a concrete, real test against the actual target — no stubs, no mocks of the authorization surface itself.
**Reference:** `docs/ASSESSMENT_METHODOLOGY.md`'s eight areas and procedure methods (document/config inspection, source review, controlled staging test, evidence reconstruction, Crypton-reference demonstration) are this stage's operational detail for a customer engagement. For a research lab, this stage is the `scenarios/*.py` + `tests/test_*.py` pattern established across EXP-003A/B/C.
**Rule:** stay entirely within public APIs. Never modify framework internals, monkey-patch private state, or depend on undefined behavior. If a hypothesis cannot be tested this way, the result is INCONCLUSIVE or NOT_APPLICABLE, not a forced equivalent.

### 10. Evidence Collection
**Purpose:** record what was actually observed, in a durable, reconstructable form, independent of the person who ran the test.
**Reference:** [Evidence & Reporting Standard](05-evidence-reporting-standard.md) in full; `assessment/models.py::EvidenceReference` for the customer-engagement schema; each EXP-003 lab's `build_evidence`/`write_evidence` pattern for the research-lab schema.

### 11. Risk Analysis
**Purpose:** for every FAIL or CANDIDATE_FINDING result, weigh real-world severity — not a fixed category-to-severity mapping.
**Reference:** `docs/ASSESSMENT_METHODOLOGY.md`'s severity rubric (seven factors: authority gained, reachable resource, sensitivity, required attacker position, human interaction required, persistence/repeatability, compensating controls) applies unchanged here — it is already framework-independent by construction and does not need a research-lab-specific variant.

### 12. Findings
**Purpose:** state the conclusion for each tested hypothesis in the four-outcome (or, where a result doesn't cleanly bisect, the extended-outcome) vocabulary, with evidence references.
**Reference:** [Evidence & Reporting Standard](05-evidence-reporting-standard.md).

### 13. Remediation
**Purpose:** for each FAIL/CANDIDATE_FINDING, recommend the control, not a vendor or a Crypton product feature.
**Rule (unchanged):** `docs/business/ASSESSMENT_PACK.md`'s report rule stands — "recommend the control, not Crypton." No EXP-00X lab has yet reached this stage against a third-party target with an actual FAIL requiring remediation advice (EXP-003C's H1c candidate finding is the closest — its README already states the remediation implicitly: treat `update_state()` access as part of the application's trust boundary).

---

## How EXP-003A/B/C map onto this pipeline

Each of the three completed labs ran stages 6 through 11 (Threat Model
through Risk Analysis) using **identical stage-6/7/8 content** (same
threat objectives in scope, same baselines, same H1-H6) against three
different targets — this is the concrete proof that stages 1-8 of this
pipeline are actually framework-independent, not just designed to look
that way on paper. None of the three has yet gone through stages 1-5
(Architecture Review through Execution Graph) as formally-produced,
separate artifacts — each lab's `target.json` and README fold
Architecture Review and Execution Graph into narrative form rather than
a dedicated diagram/document. That is an acceptable shortcut for a
research lab targeting a well-documented open-source framework; it is
**not** acceptable for a real customer engagement, where stages 1-5
are the primary artifact a customer's own team will not have written
themselves (`docs/business/ASSESSMENT_PACK.md`'s report §2 "Trust-Flow
Map" is exactly stages 1-5's customer-facing deliverable).

## Status and next steps

This is v0.1: the pipeline shape and stage definitions, cross-referenced
against everything Crypton has actually produced so far. It has not yet
been run end-to-end, stage 1 through stage 13, against a single target
in one continuous pass — EXP-003A/B/C entered partway (stage 6) because
their targets were well-documented open-source frameworks, not
unfamiliar customer systems. The first real test of stages 1-5 together
will be either a genuinely unfamiliar open-source target (per
`docs/strategy/2026-09-10-action-integrity-thesis.md` §22's "external
authorized experimentation" priority) or the first real customer
engagement — whichever comes first.
