# MVP Definition

## What is Crypton's MVP?

**The assessment service itself, delivered by a person, using a written
methodology, supported by internal test harnesses.** Not a dashboard, not
a SaaS product, not an autonomous scanner. The smallest real thing
Crypton can sell or pilot today is:

```
scope -> authority map -> test -> evidence -> classify -> report -> remediation
```

— run by an assessor, against one customer's one bounded workflow, over
3 fixed testing days, producing a written report a customer's own
engineers can independently check. Every piece of that pipeline already
exists and runs (`assessment-kit/v1.0/`, `docs/methodology/`,
`labs/agent-lab`) — the MVP question isn't "what do we need to build,"
it's "is the service, as it already exists, good enough to deliver to a
real customer." `11-pilot-readiness-report.md`'s answer is elaborated in
`assessment-kit/v1.0/11-pilot-readiness-report.md`: **GO, with explicit,
disclosed scope limits.**

## Customer-Facing MVP

What a customer actually pays for and receives, today:

- The 3-day, $2,500 founding assessment (`02-founding-assessment.md`).
- A written technical report following the real templates in
  `assessment-kit/v1.0/09-customer-deliverable-templates.md` and
  `docs/methodology/07-customer-report-template.md`.
- A live remediation walkthrough call.
- Test coverage drawn from `03-test-catalog.md`'s mature families
  (Principal/Session Binding, Delegation & Scope Attenuation,
  Authorization Boundary & Complete Mediation, Approval-to-Execution
  Binding, and the proven half of Revocation & Authority Freshness).

That's the whole customer-facing product. There is no software a
customer installs, logs into, or integrates.

## Internal Tooling (supports the service; not sold directly)

- **Agent Lab** (`labs/agent-lab`) — a reference implementation of agent
  identity, delegation, authorization, approval, and tracing, used to
  teach/motivate a question during an engagement and to run Crypton's
  own research. 322 passing tests as of this writing. **Never presented
  to a customer as having tested their system** — it hasn't; it's
  reference material (`docs/ASSESSMENT_METHODOLOGY.md`'s explicit rule
  on Crypton-reference demonstrations).
- **The EXP-00X research harnesses** (`labs/exp-003*`, `labs/exp-005*`,
  `labs/exp-007*`) — deterministic test harnesses against real
  open-source agent frameworks, used to build and validate the property
  catalog before it's ever pointed at a customer's system.
- **The assessment data model** (`labs/agent-lab/src/agentlab/assessment/`)
  — structures findings and evidence and renders them into a report; does
  not decide PASS/FAIL on a customer's system, an assessor does.
- **The methodology documents themselves**
  (`docs/methodology/`, `assessment-kit/v1.0/`) — the property catalog,
  threat model, evidence standard, finding library, scoring model,
  checklists, worksheets, and templates that make the service repeatable
  rather than reinvented per engagement.

## NOT MVP (explicitly, until real customers demand it)

- A dashboard.
- A SaaS portal.
- An autonomous/continuous scanner.
- Continuous monitoring of a customer's live system.
- Any product a customer installs or grants standing access to.

None of these are ruled out forever — they're ruled out **now**, because
building them before a single real engagement has happened would be
solving a problem Crypton hasn't yet confirmed exists in the form
software could address.

## What would justify turning a recurring service step into software

Not "it would be more efficient" or "customers might like it." The bar:

1. **The same manual step recurs, unchanged, across multiple real paid
   engagements** — not once, not only in Crypton's own research corpus,
   but across actual customers with different architectures.
2. **The step's inputs and outputs are stable enough to automate without
   losing the judgment call it currently requires** — e.g., evidence
   *collection* (running a specific controlled test against a known
   framework shape) is a much better automation candidate than
   *classification* (the five-question triage judgment in §9a), which is
   explicitly a human reasoning step Crypton does not want to make
   mechanical prematurely.
3. **A customer has asked for standing/continuous coverage specifically**,
   not just a faster one-time assessment — a real pull signal, not an
   assumption about what customers must want.
4. **The manual version has already been run enough times to know what
   "good" output looks like** — so the automated version can be checked
   against a real baseline, not built speculatively ahead of evidence
   (the same discipline `docs/strategy/2026-09-10-action-integrity-thesis.md`
   and the founder operating doc already apply to the research track:
   build only what repeated evidence demands).

Until all four hold for a specific step, that step stays a person running
a written procedure — which is also, not incidentally, the version of
Crypton a first design partner can actually evaluate and hold
accountable.
