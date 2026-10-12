# Crypton Standard Operating Procedure v1.0

**Who uses this:** the lead assessor on an engagement, stage by stage.
**When:** from the moment a prospect signs an Authorization-to-Test
through report delivery and the remediation walkthrough.
**Why it exists:** so the 13-stage pipeline in
`docs/methodology/03-assessment-methodology-v0.1.md` and the day-by-day
playbook in `docs/methodology/06-assessment-execution-playbook.md`
become a single, checkable procedure with explicit completion criteria
per stage — not a narrative an assessor has to re-derive judgment calls
from each time.

This SOP assumes the default engagement shape from
`docs/business/ASSESSMENT_PACK.md` §B: 1 workflow, up to 3 connected
tools, 1 staging environment, 3 testing days. A larger or smaller scope
changes the effort estimates, not the stage order or completion
criteria.

**Standing rule across every stage (per `VALIDATION-REPORT.md` §3,
"version drift"):** before any testing begins, verify the exact
installed version of every framework/library in scope directly (`pip
show`, `npm ls`, or the ecosystem equivalent) — never trust a version
number stated in the customer's documentation, intake answers, or your
own memory of a prior engagement. This has caused a real, caught error
once already in Crypton's own research history (EXP-003B) and is not
optional.

---

## Stage 0 — Legal Gate (customer engagements only)

**Purpose:** confirm Crypton is authorized to test, and protected if
something goes wrong.
**Inputs:** signed engagement agreement, customer's confirmation of
system ownership/authorization.
**Prerequisites:** none — this is the first stage.
**Activities:** confirm the engaging entity (LLC) is in place; obtain a
signed Authorization-to-Test naming the exact target systems,
environments, testing window, and methods; confirm the customer warrants
they own or are authorized to permit testing of the named target
(`docs/business/ASSESSMENT_PACK.md` §C).
**Artifacts produced:** signed Authorization-to-Test.
**Evidence required:** none (this stage produces the authorization
evidence, it doesn't consume it).
**Decision rules:** all three conditions (entity, signed authorization,
confirmed ownership) must be true, with no exceptions, before proceeding.
**Quality gate:** a second person (or, for a solo engagement, a
documented self-check against the §C checklist) confirms all three
conditions before Stage 1 begins.
**Completion criteria:** signed Authorization-to-Test on file, naming
exact scope.
**Estimated effort:** 0.5-1 day, typically before the paid testing
window starts.
**Failure conditions:** customer cannot confirm ownership/authorization
of the target system; entity/liability protection is not in place.
**Exit criteria:** do not proceed to Stage 1 until this stage's
completion criteria are met — this is a hard block, not a
recommendation.

---

## Stage 1 — Intake (before Day 1)

**Purpose:** produce the raw material for Architecture Review before
testing starts, so Day 1 begins with understanding, not discovery.
**Inputs:** customer contact, willingness to complete a questionnaire.
**Prerequisites:** Stage 0 complete.
**Activities:** send and collect the Customer Discovery Kit's
questionnaires (`04-customer-discovery-kit.md`); confirm staging access
actually works, not just "granted on paper"; confirm a technical point
of contact is available for the full testing window; confirm scope is
exactly one workflow, up to three connected tools — anything broader is
a scope-change conversation, not something to silently absorb.
**Artifacts produced:** completed intake questionnaire responses.
**Evidence required:** none yet — this stage collects the customer's
own claims, which Stage 2 onward will verify, not just record.
**Decision rules:** if staging access cannot be confirmed working before
Day 1, do not schedule Day 1 — reschedule until it is confirmed.
**Quality gate:** every questionnaire section has a substantive answer,
not a placeholder; red flags noted in the Discovery Kit (`04-customer-discovery-kit.md`)
are followed up before Day 1, not discovered live.
**Completion criteria:** questionnaire responses collected; staging
access confirmed working; technical point of contact confirmed
available.
**Estimated effort:** 2-5 business days of customer turnaround time;
0.5 day of assessor review time.
**Failure conditions:** customer cannot provide staging access at all
(engagement may need to be rescoped to document/source review only —
see the Static-Review-Only note in Stage 7); no technical point of
contact available for the window.
**Exit criteria:** do not schedule Day 1 until completion criteria are
met.

---

## Stage 2 — Architecture Review

**Purpose:** understand what the system under assessment actually is
before forming any hypothesis about it.
**Inputs:** intake questionnaire responses, architecture docs, source
access, technical point of contact.
**Prerequisites:** Stage 1 complete.
**Activities:** confirm/expand the intake questionnaire with the
technical point of contact; identify the framework/runtime (consult
`04-framework-adaptation-guide.md`'s architecture summary if it's one of
the four validated targets: OpenAI Agents SDK, Microsoft Agent
Framework, LangGraph, OpenHands Software Agent SDK — a fifth or
unfamiliar framework starts from that guide's adaptation procedure);
identify which tools/APIs are reachable, whether delegation exists,
whether persistence/checkpointing exists.
**Artifacts produced:** a plain-language architecture description
(Worksheet: Identity Graph starting point, `05-assessment-worksheets.md`).
**Evidence required:** architecture/configuration evidence (diagrams,
IaC, deployment topology) or document/config inspection notes.
**Decision rules:** if the framework is unfamiliar (not one of the four
validated targets), budget additional time per the Framework Adaptation
Guide's stated cost structure — do not assume existing test code
transfers.
**Quality gate:** the architecture description is reviewable by the
customer's own team without correction — if they'd correct something,
the review isn't done.
**Completion criteria:** single-agent-vs-multi-agent, framework/runtime,
reachable tools/APIs, delegation presence, and persistence presence are
all stated with a source (not assumed).
**Estimated effort:** 0.5 day (Day 1 morning).
**Failure conditions:** the customer cannot produce enough
architecture information to proceed (no docs, no available engineer,
no source access) — do not proceed past this stage; record the
engagement as INCONCLUSIVE at scale, escalate per Handbook §14.
**Exit criteria:** architecture description complete and reviewed.

---

## Stage 3 — Identity Mapping

**Purpose:** establish, for every acting component, what its identity
actually is and how it's asserted.
**Inputs:** architecture description (Stage 2), identity/registration
system access.
**Prerequisites:** Stage 2 complete.
**Activities:** inventory every agent/service/tool identity in scope;
determine how each authenticates; determine who owns each; review the
provisioning and deprovisioning process.
**Artifacts produced:** Identity Graph worksheet (`05-assessment-worksheets.md`).
**Evidence required:** identity records, or source excerpts showing how
identity is resolved at the point of an action.
**Decision rules:** PASS requires a durable, owned record with a real
provisioning process, not merely that a registry table exists; FAIL if
identity is an unverified, caller-supplied string; NOT_APPLICABLE if the
workflow has no independent "agent" concept at all.
**Quality gate:** every entry in the Identity Graph has a cited source
(document, config, or code reference), not an assumption from a
conversation.
**Completion criteria:** Identity Graph complete for every agent in
scope; a PASS/FAIL/INCONCLUSIVE/NOT_APPLICABLE conclusion drafted for
assessment area 1 (may be finalized later in Stage 12).
**Estimated effort:** 0.5-1 day (part of Day 1).
**Failure conditions:** provisioning process/records cannot be
obtained — record INCONCLUSIVE with the specific reason, do not guess.
**Exit criteria:** Identity Graph complete and cited.

---

## Stage 4 — Authority Mapping

**Purpose:** establish where authority is created, how it's
represented, and what actually gates each consequential action.
**Inputs:** Identity Graph, source/config access to the authorization
check.
**Prerequisites:** Stage 3 complete.
**Activities:** for each consequential action, find the specific
grant/policy/check that permits it and where it sits in the call path;
review the grant representation (policy table, IAM policy, OAuth scope,
MCP permission manifest).
**Artifacts produced:** Authority Graph worksheet.
**Evidence required:** the actual grant representation (policy/config/
source) plus at least one authorization log or controlled test showing
it being checked.
**Decision rules:** PASS requires a real, explicit, checkable grant that
is actually checked before the action; FAIL if authority is ambient
(any agent can do anything a shared credential permits); NOT_APPLICABLE
only if there is exactly one agent with one fixed permission set and no
delegation concept applies.
**Quality gate:** every consequential action in scope has a named,
cited check — "we assume it's checked somewhere" is not acceptable at
this stage.
**Completion criteria:** Authority Graph complete; each consequential
action's gating check identified and cited (or explicitly noted as not
found, which is itself a Stage 12 finding candidate).
**Estimated effort:** 1 day (Day 1 afternoon into Day 2).
**Failure conditions:** grant representation and its check cannot be
inspected — INCONCLUSIVE, reason stated.
**Exit criteria:** Authority Graph complete and cited.

---

## Stage 5 — Trust Boundaries

**Purpose:** identify every point where control passes from one
principal, process, or component to another — every place a claim gets
trusted rather than independently re-verified.
**Inputs:** Authority Graph, Architecture Review.
**Prerequisites:** Stage 4 complete.
**Activities:** list every boundary (human→agent, agent→agent,
agent→tool, live-state→checkpoint→resumed-state); for each, determine
what is and isn't re-verified when control crosses it. This is the
stage most likely to surface something the customer's own team hasn't
thought to name — see `VALIDATION-REPORT.md` §1/§2 for two
independently-validated examples of exactly this (LangGraph's
`update_state()` trust boundary, EXP-003C; OpenHands' delegation
confirmation-handler boundary, EXP-005).
**Artifacts produced:** Trust Boundaries worksheet.
**Evidence required:** source review of what is/isn't re-verified at
each boundary.
**Decision rules:** a boundary where a claim crosses without
re-verification is not automatically a FAIL — it becomes a finding
candidate to test explicitly in Stage 9, framed via the seven-factor
severity rubric once evidenced.
**Quality gate:** every boundary identified in the Trust Boundaries
worksheet has a stated re-verification answer (yes/no/partial), not a
blank.
**Completion criteria:** Trust Boundaries worksheet complete, reviewed
against the Authority and Identity graphs for consistency.
**Estimated effort:** 0.5 day (Day 1 close or Day 2 open).
**Failure conditions:** none specific — this stage produces candidate
findings, it does not itself fail; a thin result (few boundaries found)
should be double-checked against the Architecture Review for
completeness before being accepted.
**Exit criteria:** Trust Boundaries worksheet reviewed and complete.

---

## Stage 6 — Execution Graph

**Purpose:** trace the literal path from "action proposed" to "side
effect occurs," including every branch (approved, rejected, retried,
resumed, delegated).
**Inputs:** source access to the tool-execution call path.
**Prerequisites:** Stage 5 complete.
**Activities:** trace the call-graph-level path for the workflow's most
consequential action(s), annotated with where each authorization check
sits relative to the side effect.
**Artifacts produced:** Execution Graph worksheet.
**Evidence required:** source excerpts of the call path, or execution
traces showing the ordering.
**Decision rules:** the ordering question (check-then-execute, or
execute-then-log) directly feeds assessment area 4 (Pre-Execution
Boundary) in Stage 9 — do not conclude PASS/FAIL here, only trace and
document; the conclusion is drawn in Stage 9 against evidence.
**Quality gate:** every branch (approve, reject, retry, resume,
delegate) that exists in the real system is represented in the graph —
a graph missing the retry branch, for instance, is incomplete (see
`finding-001d`'s entire mechanism, which lives specifically in a retry
branch a less thorough trace would have missed).
**Completion criteria:** Execution Graph complete for the in-scope
workflow's consequential action(s).
**Estimated effort:** 0.5-1 day (Day 2).
**Failure conditions:** source access insufficient to trace the call
path — INCONCLUSIVE, reason stated.
**Exit criteria:** Execution Graph complete and reviewed.

---

## Stage 7 — Baseline Verification

**Purpose:** before testing any adversarial hypothesis, confirm the
ordinary, happy-path behavior actually works as expected.
**Inputs:** staging environment access, Execution Graph.
**Prerequisites:** Stage 6 complete.
**Activities:** run Baseline A (approve → executes correctly); run
Baseline B (reject → zero side effects); run Baseline C (persist →
reload via a fresh instance → resume correctly), if a persistence
mechanism is in scope.
**Artifacts produced:** Baseline A/B/C results, each with an observed
outcome recorded.
**Evidence required:** controlled staging test results, or (see
Static-Review-Only Variant below) source review confirming the expected
behavior.
**Decision rules:** **do not proceed to Stage 8 against a failing
baseline path** — a broken happy path makes any adversarial result about
it uninterpretable. This is the same discipline that caught a tautology
bug in a prior (non-EXP-00X) internal test before it produced a false
result.
**Quality gate:** all in-scope baselines (A, B, and C if persistence is
in scope) have a recorded PASS/FAIL, not "assumed working."
**Completion criteria:** Baseline A and B always required; Baseline C
required whenever persistence/checkpointing is in scope for the
workflow.
**Estimated effort:** 0.5 day (Day 2).
**Failure conditions:** a baseline fails — stop, do not proceed to
Stage 8 for that path, report the baseline failure itself as a Stage 12
finding (a broken happy path is itself a serious finding, not a
blocker to route around).
**Exit criteria:** all in-scope baselines pass, or the engagement
explicitly documents why it is proceeding past a failing baseline (rare,
requires engagement-lead sign-off).

**Static-Review-Only Variant (when live staging access is not
available or not authorized for this engagement — see `VALIDATION-REPORT.md`
§6, recommendation 5):** Crypton has **not yet validated** a formal
substitute for Stage 7 when no live execution is possible (this
happened during EXP-005 and the stage was simply skipped, which
`VALIDATION-REPORT.md` names as a real methodology gap). Until a formal
substitute is defined and validated (tracked in
`15-roadmap-v1.1.md` Tier 3 as a roadmap item), the interim rule is:
**explicitly record "Baseline
Verification not performed — static review only, no validated
substitute as of Kit v1.0" in the report**, and downgrade every
downstream finding's confidence accordingly (per the Confidence Model,
Handbook §10) rather than silently proceeding as if baselines were
confirmed.

---

## Stage 8 — Threat Model

**Purpose:** given the mapped identity/authority/trust-boundary/
execution shape, decide which of the eleven attacker objectives are
actually reachable in this specific system, and which are structurally
not applicable.
**Inputs:** Stages 2-7 artifacts.
**Prerequisites:** Stage 7 complete (baselines pass, or the
Static-Review-Only variant is explicitly invoked).
**Activities:** using `docs/methodology/02-threat-model.md`'s eleven
objectives, mark each IN SCOPE or NOT APPLICABLE for this specific
target, with a one-line sourced reason for each NOT APPLICABLE.
**Artifacts produced:** a scoped threat model — the customer-reviewable
artifact that ends Day 1 in the standard 3-day cadence.
**Evidence required:** the reasoning for each NOT APPLICABLE must be
sourced (e.g. "no standing-approval primitive exists — confirmed by
reading the exact source field enumeration"), never a shrug.
**Decision rules:** an objective is NOT APPLICABLE only when its
absence has actually been verified, not merely unexamined — an
unexamined objective is a coverage gap to disclose in Appendix A of the
final report, not a silent NOT APPLICABLE.
**Quality gate:** the scoped threat model is reviewed with the customer's
technical point of contact before Day 2 testing begins in the standard
cadence — Day 2 should not start from a disputed premise.
**Completion criteria:** all eleven objectives marked, each with a
one-line reason.
**Estimated effort:** 0.5 day (Day 1 close).
**Failure conditions:** none specific — this stage scopes, it does not
itself pass or fail.
**Exit criteria:** scoped threat model reviewed and, where practical,
confirmed with the customer before proceeding.

---

## Stage 9 — Universal Hypotheses & Target-Specific Tests

**Purpose:** state which properties from the Security Property Catalog
will be tested (scoped by Stage 8), then translate each into a concrete,
real test against the actual target.

> **Naming note (per `VALIDATION-REPORT.md` §2/§6):** this stage was
> named "Framework-Specific Tests" in `docs/methodology/03-assessment-methodology-v0.1.md`.
> It is renamed here to **"Target-Specific Tests"** because EXP-005's
> best finding lived at the seam between a framework's own primitives
> and its reference integration's default wiring — a target is not
> always cleanly separable into "framework" vs. "the application built
> on it," and this stage's scope explicitly includes testing a
> framework's own reference/example integration choices, not only its
> raw primitives.

**Inputs:** scoped threat model (Stage 8), Execution Graph, staging
access.
**Prerequisites:** Stage 8 complete.
**Activities:** confirm which of H1-H9 apply (plus H10, currently
EVOLVING — see `03-assessment-checklists.md`), and in what order they'll
be run; run each in-scope hypothesis as a controlled staging test,
source review, or document inspection, per the eight operational areas'
procedure methods; record results as they come in, not from memory at
end of day.
**Artifacts produced:** per-hypothesis test results with evidence
references.
**Evidence required:** stay entirely within public APIs and the
customer's authorized staging environment. Never modify framework
internals or the customer's production systems. If a hypothesis can't
be tested this way, the result is INCONCLUSIVE, not a forced
equivalent.
**Decision rules:** apply the four-outcome vocabulary per checklist
(`03-assessment-checklists.md`) — no fifth outcome, no default to
INCONCLUSIVE when a real test could be run instead.
**Quality gate:** every hypothesis marked IN SCOPE in Stage 8 has a
recorded result by end of Stage 9 — no hypothesis silently dropped.
**Completion criteria:** all in-scope hypotheses and area-level tests
have a recorded PASS/FAIL/INCONCLUSIVE/NOT_APPLICABLE with evidence
references.
**Estimated effort:** 1.5-2 days (Day 2 full, into Day 3 morning).
**Failure conditions:** a majority of in-scope tests end INCONCLUSIVE
due to access limitations — escalate per Handbook §14; this signals an
engagement-scoping problem, not just individually weak evidence.
**Exit criteria:** complete result set for all in-scope hypotheses/
areas.

---

## Stage 10 — Evidence Collection

**Purpose:** record what was actually observed, in a durable,
reconstructable form, independent of the person who ran the test.
**Inputs:** raw evidence gathered during Stage 9 (captures, screenshots,
source excerpts, trace files).
**Prerequisites:** Stage 9 in progress or complete (evidence collection
happens continuously, formalized here).
**Activities:** finalize the evidence manifest; ensure every PASS/FAIL
finding has at least one `EvidenceReference` (`06-evidence-standard.md`).
**Artifacts produced:** evidence manifest (one entry per `EvidenceReference`).
**Evidence required:** N/A — this stage produces evidence records, it
does not itself require external evidence.
**Decision rules:** every PASS/FAIL finding's `evidence_ids` must be
non-empty — structurally required, matching `assessment/models.py`.
**Quality gate:** spot-check a sample of evidence entries against their
source (does the cited file/log/trace actually say what the entry
claims) before proceeding to Stage 11.
**Completion criteria:** evidence manifest complete, every finding's
evidence references resolve to something real and inspectable.
**Estimated effort:** ongoing through Day 2-3, 0.5 day dedicated
finalization.
**Failure conditions:** an evidence reference cannot be resolved back
to its source — fix before proceeding, do not deliver an unverifiable
citation.
**Exit criteria:** evidence manifest complete and spot-checked.

---

## Stage 11 — Risk Analysis

**Purpose:** for every FAIL or CANDIDATE_FINDING result, weigh
real-world severity.
**Inputs:** finalized findings with evidence (Stage 10).
**Prerequisites:** Stage 10 complete.
**Activities:** for each FAIL, apply the seven-factor severity rubric
(authority gained, reachable resource, sensitivity, required attacker
position, human interaction required, persistence/repeatability,
compensating controls) — `docs/ASSESSMENT_METHODOLOGY.md`.
**Artifacts produced:** severity rating + written reasoning per FAIL
finding.
**Evidence required:** the same evidence already collected in Stage 10;
this stage reasons over it, it does not collect new evidence.
**Decision rules:** severity is never assigned from a fixed
category-to-severity mapping — the same category of gap can be Critical
in one workflow and Low in another. Document the reasoning in the
finding's `risk_impact` field; a severity label with no stated reasoning
is not acceptable practice.
**Quality gate:** a second reviewer (or, for a solo engagement, a
documented self-check reapplying the seven factors independently) can
reproduce the same severity from the same evidence.
**Completion criteria:** every FAIL has a severity and a written
rationale.
**Estimated effort:** 0.5 day (Day 3).
**Failure conditions:** none specific — this stage always completes if
Stage 10 is complete.
**Exit criteria:** all FAILs have severity + rationale.

---

## Stage 12 — Findings

**Purpose:** state the conclusion for each tested hypothesis/area in
the eleven-link chain, with evidence references.
**Inputs:** Stages 9-11 outputs.
**Prerequisites:** Stage 11 complete.
**Activities:** write each finding through the eleven-link Evidence &
Reporting Standard (`06-evidence-standard.md`); close out any loose end
from earlier stages (an INCONCLUSIVE that could actually be resolved
with one more staging test, a NOT_APPLICABLE that needs a clearer
sourced reason) here, before writing the report — not left for the
customer to notice.
**Artifacts produced:** the complete, internally-reviewed findings set.
**Evidence required:** already collected (Stage 10); this stage
assembles it into the finding format.
**Decision rules:** apply `assessment/models.py::Finding`'s structural
requirements exactly — non-empty `finding_id`, `title`, `observation`,
`expected_control`; non-empty `evidence_ids` for PASS/FAIL.
**Quality gate:** every finding independently reviewable — a second
reader should be able to follow Hypothesis → Threat → Test → Evidence →
Observed → Expected → Security Property → Risk → Recommendation →
Reproduction → Confidence without asking the original assessor a
clarifying question.
**Completion criteria:** complete findings set, each traceable through
all eleven links.
**Estimated effort:** 0.5-1 day (Day 3).
**Failure conditions:** a finding cannot be completed to all eleven
links — resolve before delivery (return to Stage 9/10 if evidence is
missing) rather than deliver an incomplete finding.
**Exit criteria:** findings set complete, ready for report assembly.

---

## Stage 13 — Report Assembly, Delivery & Remediation

**Purpose:** deliver the customer-facing artifact and walk the customer's
engineers through it.
**Inputs:** complete findings set (Stage 12).
**Prerequisites:** Stage 12 complete.
**Activities:** assemble the report using
`09-customer-deliverable-templates.md`; recommend the control, not
Crypton, in every remediation; deliver within the SOW's agreed window;
hold the live remediation walkthrough.
**Artifacts produced:** the full customer deliverable set (Executive
Summary through Remediation Roadmap, `09-customer-deliverable-templates.md`).
**Evidence required:** already collected; this stage presents it.
**Decision rules:** no invented findings, no filler; if a section is
clean, say so; every claim has evidence in the appendix.
**Quality gate:** run the Internal QA review's five-persona check
(`10-internal-qa-review.md`) against the assembled report before
delivery, at minimum for the first several engagements while the kit is
still STABLE rather than field-proven (`VALIDATION-REPORT.md` §7).
**Completion criteria:** report delivered within the agreed window;
remediation walkthrough scheduled or held.
**Estimated effort:** 1 day report assembly + 1 session (typically
30-60 min) for the walkthrough, both outside the 3 testing days.
**Failure conditions:** none specific — if Stage 12 is genuinely
complete, this stage is assembly and delivery, not further judgment.
**Exit criteria:** report delivered, remediation walkthrough held or
scheduled, engagement closed.

---

## Effort Summary

| Stages | Typical calendar placement | Cumulative effort |
|---|---|---|
| 0-1 | Before Day 1 | 2.5-6 days (mostly customer turnaround) |
| 2-5 | Day 1 | 1 day |
| 6-8 | Day 1 close / Day 2 open | 1.5-2 days |
| 9-11 | Day 2-3 | 2.5-3 days |
| 12-13 | Day 3 close + follow-up | 1.5-2 days |

Matches the SOW's stated "~3 testing days + report + remediation call"
(`docs/business/ASSESSMENT_PACK.md` §A) once pre/post-engagement stages
are counted separately, as they are here.
