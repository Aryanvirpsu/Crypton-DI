# Evidence & Reporting Standard v0.1

**EXP-004, deliverable 5.** Every finding — from a research lab's
evidence JSON or a real customer engagement's `Finding` record — must be
expressible as this eleven-link chain:

```
Hypothesis → Threat → Test → Evidence → Observed Behaviour →
Expected Behaviour → Security Property → Risk → Recommendation →
Reproduction → Confidence
```

**No exceptions.** A finding missing any link is incomplete, not
lightweight — see "What to do when a link is genuinely unavailable"
below for the honest way to record that, which is different from
skipping the link silently.

---

## The eleven links, defined

| Link | Definition | Where it already exists in code/practice |
|---|---|---|
| **Hypothesis** | Which universal hypothesis (H1-H9) this finding tests. | [Assessment Methodology](03-assessment-methodology-v0.1.md) stage 8 table |
| **Threat** | Which of the eleven attacker objectives this hypothesis defends against. | [Threat Model](02-threat-model.md) |
| **Test** | The specific, concrete action taken against the target — a controlled staging test, a source review, a document inspection. | `docs/ASSESSMENT_METHODOLOGY.md`'s "Procedure methods"; each EXP-003 lab's `scenarios/*.py` |
| **Evidence** | What was actually collected — a reference to something inspectable, not a claim. | `EvidenceReference` (customer schema); each EXP-003 lab's `evidence/*.json` file (research schema) |
| **Observed Behaviour** | What actually happened, stated as fact. | `Finding.observation` (customer schema); the `execution`/`authorization_binding` fields in EXP-003's evidence JSON |
| **Expected Behaviour** | What the security property under test requires. | `Finding.expected_control` (customer schema); implicit in each test's assertions in EXP-003's `test_*.py` files — **should be stated explicitly in prose for every finding**, not left implicit in test code alone (see below) |
| **Security Property** | Which pillar(s) of the [Security Property Catalog](01-security-property-catalog.md) this bears on. | `Finding.area` (customer schema, `AssessmentArea` enum); not yet a first-class field in EXP-003's evidence schema — **gap, addressed below** |
| **Risk** | For a FAIL/CANDIDATE_FINDING only: severity, reasoned via the seven-factor rubric. | `Finding.severity` + `Finding.risk_impact` (customer schema); EXP-003's evidence schema has no equivalent field yet — **gap, addressed below**, though EXP-003C's `notes` field has carried this informally (e.g. H1c's classification as application-level, not framework-level) |
| **Recommendation** | The control to add/fix, never a vendor or a Crypton product pitch. | `Finding.recommendation` (customer schema); implicit in EXP-003 READMEs' prose (e.g. EXP-003C's H1c note on `update_state()` access as a trust boundary) — **should be an explicit field**, addressed below |
| **Reproduction** | How to reproduce the result independently — command, file, exact steps. | The pytest test itself, for every EXP-003 finding (`pytest tests/test_x.py::test_y`); `assessment-engagement/`'s dry-run structure for the customer schema |
| **Confidence** | How certain the conclusion is, and why — not a number, a reasoned statement. | `Finding.confidence` (customer schema, free text); **not present** in any EXP-003 evidence JSON to date — **gap, addressed below** |

---

## Gaps this standard identifies in the current schemas

Writing out the full eleven-link chain against real, already-collected
evidence (see worked example below) surfaces three fields present in
the customer-engagement schema (`assessment/models.py::Finding`) that
have **no equivalent** in any EXP-003 research-lab evidence schema to
date:

1. **Security Property** — no EXP-003 evidence JSON records which
   catalog pillar a finding belongs to as a structured field (only as
   narrative in the lab's README).
2. **Risk** — no EXP-003 evidence JSON has a severity/risk field at all
   (reasonable, since none of EXP-003A/B/C produced a real FAIL against
   a third-party framework — only one `CANDIDATE_FINDING_APPLICATION_LEVEL`,
   H1c, whose risk reasoning lives only in prose).
3. **Recommendation** — same as above; EXP-003C's H1c *implies* a
   recommendation in its notes but doesn't carry it as its own field.

**Resolution for future EXP-00X evidence:** rather than retrofitting
every existing evidence JSON (18 files across EXP-002/003A/B/C, real,
already-verified artifacts that should not be touched without cause),
future evidence-writing code (`build_evidence()` in any new lab) should
add three optional fields — `security_property` (pillar name(s)),
`risk` (`None` unless the outcome is FAIL/CANDIDATE_FINDING, then a
short reasoned statement), and `recommendation` (`None` unless there's
a concrete one to make) — bringing the research schema into full
eleven-link parity with the customer schema going forward, without
breaking any existing file. This is additive, matching the same
discipline the [Framework Adaptation Guide](04-framework-adaptation-guide.md)
already recommends for schema evolution generally ("extend, don't
fork").

---

## Worked example: EXP-003C's H1c, walked through all eleven links

This is the strongest real candidate finding produced by the EXP-003
series to date — using it here both to validate the standard against a
real artifact and to make its full reasoning explicit for the first
time (the existing evidence JSON and README state most of this, but not
in this exact structure).

1. **Hypothesis:** H1 — Argument Binding (do approved arguments drift before execution?).
2. **Threat:** "Modify execution" (arguments differ from what was authorized) — [Threat Model](02-threat-model.md#modify-execution).
3. **Test:** a controlled test constructed a pending, checkpointed thread using the *safe*, state-sourced execution pattern (`build_single_tool_graph`), then invoked LangGraph's public `update_state()` API against the same thread_id before resuming it. Source: `labs/exp-003c-langgraph/tests/test_exp003c.py::test_h1c_update_state_public_api_can_mutate_pending_state_before_resume`.
4. **Evidence:** `labs/exp-003c-langgraph/evidence/h1c_update_state_argument_drift.json` — records the proposed arguments, the state-mutation call, the resume decision, and the actual executed arguments.
5. **Observed Behaviour:** the executed arguments matched the *drifted* values written via `update_state()`, not the arguments originally presented to the human at interrupt time.
6. **Expected Behaviour:** per Universal Hypothesis U-AUTH-1 ([Security Property Catalog](01-security-property-catalog.md#pillar-2--authority)), the arguments actually executed should match the arguments that were authorized — verified at execution time.
7. **Security Property:** Pillar 2 (Authority) and Pillar 6 (Persistence) — specifically, the trust boundary around who may legitimately call `update_state()` on a pending thread ([Assessment Methodology](03-assessment-methodology-v0.1.md) stage 4, Trust Boundaries).
8. **Risk:** bounded. `update_state()` is public, documented API with no competing safety claim — LangGraph never claimed pending interrupts are frozen against it. Exploitability requires a caller already holding write access to the target's own checkpoint store (not a low-trust external position). Per the severity rubric's seven factors: authority gained is real (argument substitution) but requires an already-privileged position to exploit; no compensating control is asserted by the framework, but none was ever claimed either. **Classified as an application-design trust-boundary consideration, not a framework security defect** — this is itself the "Risk" link's conclusion, not a dodge of it.
9. **Recommendation:** any application built on LangGraph's `interrupt()`/`update_state()` primitives for a security-relevant approval flow must treat "who can call `update_state()` on a pending thread" as part of its own access-control boundary — e.g. by restricting checkpoint-store write access to the same principal(s) trusted to run the resuming process at all. This is a control recommendation for integrators, not a fix Crypton would ask LangGraph's maintainers to make (the framework's behavior matches its own documentation).
10. **Reproduction:** `cd labs/exp-003c-langgraph && python -m pytest tests/test_exp003c.py::test_h1c_update_state_public_api_can_mutate_pending_state_before_resume -v`. No network, no API key, <1s.
11. **Confidence:** high. The mechanism is directly observed (not inferred), reproducible deterministically, and grounded in reading `update_state()`'s actual documented contract (no "freeze pending interrupts" claim exists) rather than assumed framework behavior.

This worked example is also the template every future finding — research or customer — should follow when writing up a result: eleven short answers, each one sentence to one paragraph, not eleven headers with one word each.

---

## What to do when a link is genuinely unavailable

Some links legitimately cannot be filled for a NOT_APPLICABLE or
INCONCLUSIVE outcome — e.g. **Risk** and **Recommendation** don't apply
to a PASS or a NOT_APPLICABLE finding (there is no gap to size or fix).
The rule, consistent with `assessment/models.py::Finding`'s existing
enforcement:

- **PASS / FAIL:** all eleven links must be answerable. FAIL additionally
  requires Risk and Recommendation to be non-empty (mirroring
  `Finding.__post_init__`'s evidence_ids requirement).
- **NOT_APPLICABLE:** Risk and Recommendation are `None` by definition
  (there is nothing to remediate against a property that doesn't exist
  at this layer) — but **Evidence** is still required, specifically
  evidence *for why it's not applicable* (e.g. EXP-003C's H3: the
  source-code field enumeration of `InterruptOnConfig` IS the evidence
  that no standing-approval surface exists — NOT_APPLICABLE is not a
  synonym for "didn't look").
- **INCONCLUSIVE:** Evidence, Observed Behaviour, Risk, and
  Recommendation may all be absent — but the record must state,
  specifically, why (no staging access, no source access, the relevant
  process/documentation doesn't exist to inspect). "Requires manual
  review" alone is never an acceptable substitute for this reason,
  consistent with `docs/ASSESSMENT_METHODOLOGY.md`'s existing rule.

## Relationship between the two schemas going forward

Neither schema replaces the other:

- `assessment/models.py::Finding`/`EvidenceReference` remains the
  code-enforced schema for real customer engagements — it should gain
  the additive fields noted above only if a real engagement's reporting
  need demands them, not preemptively.
- Each EXP-00X lab's own `build_evidence()`/`write_evidence()` remains
  free to vary in shape per target (per the [Framework Adaptation
  Guide](04-framework-adaptation-guide.md)'s "extend, don't fork"
  guidance) — but every finding it produces, PASS or FAIL or
  NOT_APPLICABLE or INCONCLUSIVE, should be traceable through this
  document's eleven links, even where several links live in the lab's
  README prose rather than the JSON file itself, as this worked example
  demonstrates is already achievable today without any code change.
