# Crypton Assessment Kit v1.0

**EXP-006.** Crypton's internal operating manual for delivering a real
customer Agent Authorization & Delegation Assessment. Built entirely
from validated work — EXP-001, EXP-003A/B/C, EXP-004, EXP-005, and the
existing repository (`docs/`, `labs/`, `assessment-engagement/`) — after
an independent re-review of every prior finding and methodology
decision (`VALIDATION-REPORT.md`). Nothing here invents a capability
EXP-001-005 didn't validate; every place that would require one is
labeled **FUTURE CAPABILITY**.

**Success criterion this kit is built against:** a competent security
engineer with no prior exposure to Crypton can execute a complete
customer assessment using only these documents.

**Start here if you are new:** `14-training-requirements.md` gives the
exact reading order and checkpoints. The table below is a reference
index, not a reading-order recommendation.

---

## Read Before Anything Else

| Document | Who | When | Why |
|---|---|---|---|
| [`VALIDATION-REPORT.md`](VALIDATION-REPORT.md) | Everyone | Before trusting any claim elsewhere in this kit | Independent, adversarial re-review of every EXP-001-005 finding and every EXP-004/005 methodology adaptation. States plainly what survived, what was downgraded, what was dropped, and the kit's honest maturity verdict (READY WITH LIMITATIONS). Every other document in this kit reflects this report's conclusions. |

## The Kit

| # | Document | Who | When | Why |
|---|---|---|---|---|
| 1 | [`01-assessor-handbook.md`](01-assessor-handbook.md) | Every assessor | Before the first engagement; kept open during every engagement | Mission, philosophy, scope, terminology, the seven pillars, threat model, evidence philosophy, decision rules, confidence model, quality expectations, customer communication, uncertainty handling, escalation, ethics — the standing reference for every judgment call. |
| 2 | [`02-standard-operating-procedure.md`](02-standard-operating-procedure.md) | Lead assessor | Stage by stage, intake through delivery | The 13-stage pipeline as a literal, checkable procedure — purpose, inputs, prerequisites, activities, artifacts, evidence, decision rules, quality gate, completion criteria, effort, failure conditions, exit criteria, per stage. |
| 3 | [`03-assessment-checklists.md`](03-assessment-checklists.md) | Assessor | During SOP Stages 3-9 | Every pillar's properties as executable checklists — objective, evidence, questions, positive/negative tests, pass/fail/NA/inconclusive criteria, confidence rules, required artifacts. No subjective judgment left undefined. |
| 4 | [`04-customer-discovery-kit.md`](04-customer-discovery-kit.md) | Assessor/engagement lead | SOP Stage 1, before Day 1 | Seven questionnaires plus two interview guides, every question with purpose, expected answer, red flags, follow-ups. |
| 5 | [`05-assessment-worksheets.md`](05-assessment-worksheets.md) | Assessor | SOP Stages 2-9, live | Ten worksheets (Identity, Authority, Delegation, Execution, Trust Boundaries, Approval, Persistence, Audit, Tool, Data-Flow graphs), each with instructions, a worked completion example from the validated corpus, and required evidence. |
| 6 | [`06-evidence-standard.md`](06-evidence-standard.md) | Assessor, engagement lead | Continuously (Stages 9-10), finalized Stage 12 | The sixteen-field canonical evidence package every finding carries, extending the validated eleven-link standard; evidence quality requirements; chain-of-custody rules for real engagements. |
| 7 | [`07-finding-library.md`](07-finding-library.md) | Assessor | Stage 9 (pattern reference), Stage 12 (write-up template) | Four validated, reusable finding patterns (only what survived `VALIDATION-REPORT.md`'s re-review), plus a Known-Safe Patterns reference appendix. |
| 8 | [`08-scoring-model.md`](08-scoring-model.md) | Assessor | Stage 12 | The five-rating (plus one non-rating) maturity model per pillar — no numeric scores, ever — and the rules for withholding a rating when evidence is insufficient. |
| 9 | [`09-customer-deliverable-templates.md`](09-customer-deliverable-templates.md) | Assessor | Stage 13 | Eleven ready-to-fill templates: Executive Summary, Technical Report, Architecture Review, Risk Summary, Finding Detail, Evidence Appendix, Remediation Roadmap, Engineering Appendix, Leadership Summary, Assessment Certificate, Management Presentation. |
| 10 | [`10-internal-qa-review.md`](10-internal-qa-review.md) | Kit maintainer | At kit assembly, and before every future revision | Five-persona critique (junior assessor, senior consultant, skeptical customer, CTO, peer reviewer) of the kit itself, with real fixes applied and cited, and deliberately-not-fixed items forwarded to the Gap Analysis. |
| 11 | [`11-pilot-readiness-report.md`](11-pilot-readiness-report.md) | Leadership | Now, before booking a first engagement | Can a complete assessment be performed, what assumptions/skills/tooling/training are required, what remains research vs. operational, and the conditioned **GO** recommendation. |
| 12 | [`12-operational-readiness-review.md`](12-operational-readiness-review.md) | Leadership | Alongside 11 | Function-by-function readiness across technical methodology, procedure, deliverables, legal/commercial machinery, pricing, tooling, staffing, training. |
| 13 | [`13-gap-analysis.md`](13-gap-analysis.md) | Leadership, kit maintainer | Now, and after each engagement | Every open gap consolidated from every other document in this kit, categorized (methodology / corpus diversity / operational / deliberately-out-of-scope) and prioritized. |
| 14 | [`14-training-requirements.md`](14-training-requirements.md) | New assessor, onboarding lead | Before a new assessor's first engagement | The six-module curriculum, with checkpoints, from "never seen Crypton" to "ready for a supervised engagement." |
| 15 | [`15-roadmap-v1.1.md`](15-roadmap-v1.1.md) | Kit maintainer | After the first real engagement, or after new research | Prioritized, evidence-gated roadmap — what moves next, what's explicitly not planned, and why. |

---

## How This Kit Relates to the Rest of the Repository

This kit does not replace anything — it operationalizes what already
existed:

- `docs/ASSESSMENT_METHODOLOGY.md` and `docs/business/ASSESSMENT_PACK.md`
  remain the underlying technical procedure and the legal/commercial
  pack. This kit is built on top of them, cites them throughout, and
  changes neither.
- `docs/methodology/` (EXP-004) remains the framework-independent
  vocabulary (seven pillars, eleven threat objectives, H1-H9, the
  eleven-link evidence standard) this kit's checklists and evidence
  standard extend.
- `labs/experiments/001-langgraph-approval-drift/`,
  `labs/exp-003-openai-agents-sdk/`, `labs/exp-003-microsoft-agent-framework/`,
  `labs/exp-003c-langgraph/`, and `labs/exp-005-openhands-sdk/` remain
  the primary evidence this kit's Finding Library and Validation Report
  cite — this kit never duplicates their content, only references it.
- `assessment-engagement/reference-agentlab-dryrun/` remains the one
  sanctioned self-assessment dry run proving the report-rendering
  mechanics work — never evidence about a customer's system.

## What This Kit Is Not

- Not a claim that Crypton's research corpus is large or diverse
  (`VALIDATION-REPORT.md` §3 states this plainly).
- Not a claim that every security property that matters is covered
  (`VALIDATION-REPORT.md` §4, Handbook §4's "Out of Scope").
- Not a compliance framework, an automated scanner, or a certification
  body's audit process.
- Not final — v1.0 is deliberately scoped to what EXP-001-005 actually
  validated, with an explicit, evidence-gated path to v1.1
  (`15-roadmap-v1.1.md`).
