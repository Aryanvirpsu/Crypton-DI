# Crypton Agent Authorization & Delegation Assessment — Methodology (v0.6.2)

**Status: methodology v0.1 — internally exercised (one synthetic reference
dry run, `assessment-engagement/reference-agentlab-dryrun/`), customer
validation pending.** No engagement has yet been run against an external
customer's system; the eight-area procedure below has not been field-tested
against workflow shapes Crypton did not design. Treat every claim in this
document as "this is the intended procedure," not "this procedure has been
proven to work on a real, unfamiliar system."

> **2026-09-30 remediation pass:** the outcome vocabulary below was
> replaced — PASS/FAIL/INCONCLUSIVE/NOT_APPLICABLE is now
> ENFORCED/VIOLATED/UNREPRESENTABLE/UNVERIFIABLE/NOT_APPLICABLE — and the
> evidence-sufficiency model gained a seventh gate (security-claim /
> trust-boundary triage). This is a forward-looking change to how a *new*
> customer engagement record is built; it does not rewrite any historical
> research artifact (EXP-003's evidence JSON, EXP-005's findings, the
> Validation Benchmark's verdicts, etc. keep their original vocabulary
> exactly as recorded — see "Migrating a historical result" below). Full
> rationale: `docs/audits/2026-09-30-evidence-report-readiness-audit.md`
> §5.3, §11.1-2.

**What this is:** the procedure Crypton follows when assessing how authority
actually flows through a real, action-taking AI-agent workflow, across eight
core areas. **What this is not:** a claim that Crypton can automatically
inspect an arbitrary customer's system. Most of the procedures below are
manual — document inspection, source review, an assessor-run staging
test — and manual review is a **legitimate, expected assessment method**,
not a fallback for when automation isn't ready.

Crypton's assessment tooling (`labs/agent-lab/src/agentlab/assessment/`)
supports this methodology by modeling findings and evidence, and rendering
them into a report. It does not decide ENFORCED/VIOLATED on a customer's
system — an assessor does, using the evidence this document says to collect.

This document supersedes any earlier positioning language elsewhere in this
repository that implied Crypton's delegation-chain primitive is something no
competitor has, or that the assessment-service market is white space. See
`DECISIONS.md`'s v0.6.1 section for the explicit correction: named
competitors (iC Consult, Article 11, among others) already operate in this
lane, and Crypton's differentiation is **open and unproven**, to be earned
through delivered engagements, not asserted from architecture.

> **Cross-reference (added with EXP-004, 2026-09-16):** this document is
> the detailed technical procedure for what
> `docs/methodology/03-assessment-methodology-v0.1.md` calls stages 8-9
> (Universal Hypotheses / Framework-Specific Tests) of Crypton's full
> assessment pipeline. It is not superseded — `AssessmentArea` and
> `Finding` in `assessment/models.py` are built directly against the
> eight areas below, unchanged. Read `docs/methodology/` for the
> end-to-end pipeline (architecture review through remediation), the
> framework-independent Security Property Catalog and Threat Model these
> eight areas implement, and the cross-framework evidence (EXP-003A/B/C)
> that exercised this same procedure's stages 6-11 against three
> real agent frameworks before any customer engagement existed.

---

## Prompt injection, jailbreaks, and behavioral red-teaming

Explicitly **not** part of this methodology's lead assessment. If a
workflow's exposure to prompt injection, tool injection, jailbreaks, or
unsafe argument content is in scope for an engagement, it is scoped and
delivered as a **separate add-on**, using a different methodology this
document does not define.

---

## The five outcomes

Every finding reaches exactly one of these. There is no sixth outcome, and
"requires manual review" is not one of them — manual review is a *method*
that can produce any of the five below, not an excuse to default to
UNVERIFIABLE.

| Outcome | Meaning | When to use it |
|---|---|---|
| **ENFORCED** | The claimed security property was evidenced, held, and every applicable evidence-sufficiency gate (below) is satisfied. | You collected real evidence (source, config, logs, a staging test result, or a Crypton-reference demonstration used analogically), it shows the expected control is in place and functioning, and nothing in the seven-gate check below is missing. |
| **VIOLATED** | The claimed security property was evidenced, every evidence-sufficiency gate is satisfied, **and** Gate 7 (security-claim / trust-boundary triage) affirmatively confirms the observed behavior crosses a real, claimed security boundary. | You collected real evidence showing a gap — the expected control is missing, misconfigured, or bypassable — **and** you have specifically asked and answered "does this actually cross a boundary the system claims to hold," not merely "did something unexpected happen." Evidence sufficiency (Gates 1-6) alone is never enough for VIOLATED — see "Gate 7" below. |
| **UNREPRESENTABLE** | The target system does not contain the semantic state needed to express this property at all. | E.g. no `approver_identity` field exists anywhere in the framework to test — the question has no answer the system could give, not merely "we didn't look." Never established by a single missing log field alone; confirm the underlying semantic concept has no representation anywhere reachable, not just that one record didn't carry it. |
| **UNVERIFIABLE** | The property is representable, but sufficient evidence could not be obtained within this engagement's actual access/scope. | The customer withheld source/config access, a staging environment was unreachable, logs had already rotated out of the retention window, or the process/documentation needed to answer the question doesn't exist to inspect *in this engagement* (the concept itself is real and representable elsewhere). Record *why* in the finding's `limitations` field — a bare UNVERIFIABLE with no stated reason is not acceptable practice. |
| **NOT_APPLICABLE** | The control genuinely does not apply to this workflow. | E.g. no human-approval step exists at all in the workflow, so replay/TOCTOU sub-checks for it don't apply; or the engagement has no live/production traffic to measure a live-timing property against. |

**No ENFORCED or VIOLATED exists without at least one evidence reference
*and* a fully satisfied seven-gate evidence record.** This is enforced
structurally by `assessment/models.py::Finding.__post_init__` — attempting
to construct an ENFORCED or VIOLATED `Finding` with an empty `evidence_ids`
tuple, a missing `evidence_gate_status`, or any gate not explicitly `True`
raises `ValueError`. UNREPRESENTABLE and UNVERIFIABLE do not require
evidence or a satisfied gate record, but *do* require a non-empty
`limitations` stating which of the two applies and why. NOT_APPLICABLE
requires neither.

**UNREPRESENTABLE and UNVERIFIABLE must remain distinct — never collapse
one into the other for convenience.** A missing log field, by itself, is
never sufficient grounds for UNREPRESENTABLE; it is ordinary grounds for
UNVERIFIABLE (the concept is representable, you just couldn't observe it
here). Reach for UNREPRESENTABLE only when you have confirmed the concept
has no representation anywhere in the system, not merely that this one
artifact didn't happen to carry it.

### The seven evidence-sufficiency gates

Before any ENFORCED or VIOLATED conclusion, confirm all seven:

| Gate | Question |
|---|---|
| **G1 Representability** | Does the target system contain the semantic state necessary to express this property at all? |
| **G2 Observability** | Was enough of that state actually captured to evaluate it? |
| **G3 Controlled execution** | Was the relevant condition actually exercised, not just reasoned about? |
| **G4 Effect confirmation** | Was the real resulting effect verified through a sink, ledger, state transition, or other objective evidence — never model/narrative text alone? |
| **G5 Calibration / control** | Was there a negative, baseline, or calibration case demonstrating the test actually measures the intended property? |
| **G6 Reproduction** | Was the behavior reproduced enough to support the strength of the claim? |
| **G7 Security-claim / trust-boundary triage** | Does the observed behavior actually satisfy or violate a security property the system *claims* to hold across a real, relevant trust boundary — not merely that evidence sufficiency (G1-G6) was met? |

**Gate 7 is the gate that separates "evidenced and unexpected" from "a real
violation."** It exists because of a real, documented case in Crypton's own
corpus: the EXP-001d/e finding
(`labs/experiments/001-langgraph-approval-drift/CLOSURE.md`, 2026-09-27)
passed Gates 1-6 cleanly — real framework, real deterministic reproduction
(5/5 and 38/40 runs), a real captured drift — and was still the wrong
conclusion. The mutating code there was trusted, same-trust-domain
application code, explicitly permitted by the framework's own `after_model`
contract (proven by that same framework's own `PIIMiddleware` using the
identical mechanism). No framework-claimed boundary was actually crossed.
**Evidence sufficiency alone (G1-G6) must never promote a finding to
VIOLATED — only an affirmative Gate 7 may.** Before classifying anything as
VIOLATED, name: the actor/code that caused the observed behavior, its trust
level relative to the control being tested, whether it is permitted by the
system's own design/contract, what boundary the system actually claims to
enforce, and whether that specific boundary — not a different, more
convenient one — was crossed. (This is the same discipline as
`assessment-kit/v1.0/01-assessor-handbook.md` §9a's triage rule, applied at
the model-enforcement layer here, not only as an assessor checklist.)

### Migrating a historical result into the canonical vocabulary

`assessment/models.py::migrate_legacy_outcome` encodes exactly this
mapping, so it can be called instead of re-derived by hand:

- `PASS` -> `ENFORCED`, always.
- `NOT_APPLICABLE` -> `NOT_APPLICABLE`, always.
- `FAIL` -> `VIOLATED` **only** when an explicit, positive Gate 7 result is
  supplied. Otherwise the function returns no outcome and flags the record
  as requiring assessor resolution — a FAIL is never mechanically promoted
  to VIOLATED on evidence sufficiency alone.
- `INCONCLUSIVE` -> `UNREPRESENTABLE` if the reason was that the target has
  no semantic state to express the property at all; `UNVERIFIABLE` if the
  property is representable but evidence couldn't be obtained. With no
  reason given, the function returns no outcome — INCONCLUSIVE is never
  mechanically converted.

Research-only vocabulary used inside `docs/methodology/` and the individual
`labs/exp-00*` experiments (PARTIAL PASS, COVERAGE GAP, SPLIT,
PASS_WITH_CAVEAT, CANDIDATE_FINDING, and similar) may continue to be used in
historical and research artifacts — those documents are not rewritten by
this remediation. The five-outcome model above governs only what a
*customer engagement record* (a `Finding` in `assessment/models.py`) is
allowed to resolve to.

## Evidence categories

Evidence entries (`assessment/models.py::EvidenceReference`) use free-text
`evidence_type`, but the expected categories are:

- **architecture/configuration** — diagrams, IaC, deployment topology
- **source excerpts** — the actual code implementing a control
- **policy definitions** — an IAM policy, an OAuth scope list, an MCP
  tool-permission manifest, or equivalent
- **authorization logs** — a decision ledger, an audit trail
- **execution traces** — full request/response/tool-call traces
- **approval records** — human-confirmation records, including their binding
  mechanism
- **identity records** — agent registration/provisioning records
- **live/staging observations** — something the assessor watched happen in
  a real or staging environment
- **controlled test results** — a test the assessor constructed and ran
  against the customer's staging environment, with a predicted and observed
  outcome

## Procedure methods

Each area's procedure below names which of these methods actually produced
the evidence — a report reader should always be able to tell how a
conclusion was reached, not just what it was:

- **Document/config inspection** — reading architecture docs, IaC,
  policy/scope definitions the customer provides.
- **Source review** — reading the customer's actual implementation code for
  the relevant control.
- **Controlled staging test** — the assessor constructs a specific request
  (an in-scope action, a bypass attempt, a replay, etc.) and runs it against
  the customer's staging environment, observing the real result.
- **Evidence reconstruction** — after the fact, reconstructing what happened
  from the customer's own logs/traces/records (not a test the assessor
  triggered).
- **Crypton-reference demonstration** — using Crypton's own Agent Lab
  reference implementation to demonstrate *what an enforced or violated
  version of this control looks like*, as a teaching/comparison aid during
  the engagement. **This never substitutes for evidence about the
  customer's own system** — a Crypton-reference demonstration can motivate
  a question or illustrate a concept to a customer's team, but the finding
  itself must still be evidenced from the customer's own artifacts, or it
  is UNVERIFIABLE.

---

## Severity rubric

Severity (`CRITICAL` / `HIGH` / `MEDIUM` / `LOW` / `INFORMATIONAL`) is set
**only on VIOLATED findings** (occasionally an INFORMATIONAL note on an
otherwise-ENFORCED area) — it is never a property of ENFORCED, and it is
never a fixed mapping from finding *category* to a severity level
("resource escape = High" is exactly the kind of hardcoded rule this rubric
avoids — the same category of gap can be Critical in one workflow and Low
in another, depending on what it actually reaches).

For each VIOLATED finding, weigh these seven factors together — not as a point score,
as a judgment call a second assessor should be able to reproduce from the
same evidence:

1. **Authority gained** — read vs. write vs. an irreversible action vs.
   cross-principal privilege escalation.
2. **Reachable resource/action** — how broad or sensitive is what actually
   becomes reachable through this gap.
3. **Sensitivity/business impact** — regulatory, financial, safety, or
   reputational weight of that resource/action.
4. **Required attacker position** — reachable by any low-trust caller, or
   does exploitation require already holding a privileged agent/insider
   position?
5. **Human interaction required** — does exploitation skip past a required
   human-approval step, or does none exist for this action to begin with?
6. **Persistence/repeatability** — a one-time exposure, or standing/
   repeatable (e.g. an unrevoked, non-expiring grant)?
7. **Compensating controls** — do other layers (row-level ownership checks,
   audit logging, rate limiting, a human review step elsewhere in the
   process) meaningfully reduce real-world exploitability even though the
   primary control failed?

**Rough calibration** (a guide, not a formula):

| Severity | Shape |
|---|---|
| **CRITICAL** | High authority gained (full impersonation, an irreversible write, cross-principal escalation), reachable by a low-privilege caller, no meaningful compensating control. |
| **HIGH** | Significant authority/resource exposure, but one mitigating factor present (requires an already-authenticated low-priv agent, or a real-but-weak compensating control). |
| **MEDIUM** | Bounded exposure, a real compensating control exists, or exploitation requires unusual/multi-step conditions. |
| **LOW** | Narrow, low-impact, well-compensated, or requires extensive pre-conditions unlikely in practice. |
| **INFORMATIONAL** | A hygiene/best-practice observation with no direct exploit path — used to record something worth fixing that isn't itself a vulnerability. |

Document the reasoning for the chosen severity in the finding's
`risk_impact` field — a severity label with no stated reasoning is not
acceptable practice, symmetrically with the evidence requirement above.

---

## The eight areas

> **Cross-reference to the customer-facing SOW (added 2026-09-02):**
> `docs/business/ASSESSMENT_PACK.md` describes the sellable engagement at a
> coarser, 5-pillar grain for one-pagers/SOWs/report templates. The eight
> technical areas below are the assessor's actual working procedure; they
> map onto that pack's pillars as follows — use this table to translate
> between the two documents rather than treating them as two different
> scopes:
>
> | This document's 8 areas | Assessment Pack's 5 SOW pillars |
> |---|---|
> | 1. Agent Identity & Ownership | Agent identity & ownership |
> | 2. Delegation & Authority Flow | Delegation & authority flow |
> | 3. Scope Attenuation / Privilege Inheritance | Delegation & authority flow |
> | 4. Pre-Execution Authorization Boundary & Bypass | Authorization boundary |
> | 5. Authorization Bypass Paths | Authorization boundary |
> | 6. Human-Approval Binding (Replay / TOCTOU / Approved-vs-Executed Drift) | Human approval |
> | 7. Revocation / Expiry Propagation | Delegation & authority flow |
> | 8. Evidence / Accountability Reconstruction | Accountability |
>
> Areas 3 and 5 don't have their own pack pillar because the pack folds
> attenuation into "delegation & authority flow" and bypass paths into
> "authorization boundary" for a customer-facing audience; nothing is
> dropped, only regrouped at a coarser grain for sales/report material.

### 1. Agent Identity & Ownership

**Objective:** can the customer answer, for any agent that can act, *which
specific registered application it is* and *who owns/is accountable for
it* — not merely "some process was running"?

**Required evidence:** identity records (an agent registry, a service
account inventory, an MCP server/tool registry), the provisioning process
(who can register a new agent, on whose authority), the deprovisioning
process, architecture/configuration showing how agent identity is
distinguished from human identity and from a single execution instance.

**Procedure:** document/config inspection of the identity/registration
system; source review of how an agent's identity is resolved at the point
of an action (is it a stable, owned record, or a caller-supplied string?);
interview or process-documentation review for provisioning/offboarding. A
Crypton-reference demonstration can show what a durable, owned
`Agent` record with a real registration lifecycle looks like
(`agents.py`), for comparison — not as evidence about the customer's
system.

**Outcomes:** ENFORCED if agent identity is a durable, owned record with a
real provisioning process (and the seven gates are satisfied); VIOLATED if
agent identity is an unverified string/label any caller can assert *and*
Gate 7 confirms this crosses a boundary the customer actually claims (e.g.
they claim per-agent accountability but the mechanism cannot deliver it);
UNVERIFIABLE if the provisioning process/records could not be obtained;
UNREPRESENTABLE if the architecture has no concept of a durable agent
identity to inspect at all; NOT_APPLICABLE if the workflow has no
independent "agent" concept at all (e.g. a single script with no
multi-agent structure).

**Evidence required for conclusion:** identity records or source excerpts
showing how identity is resolved, plus (for ENFORCED) evidence of an actual
provisioning process, not just that a table/registry exists.

---

### 2. Delegation & Authority Flow

**Objective:** can the customer show, for any consequential action, *who
authorized this agent to do it* — a real, traceable grant, not an assumed
or ambient permission?

**Required evidence:** policy/delegation definitions (however the customer
represents "agent X may do Y"), source excerpts of the authorization check
itself, authorization logs showing real grants being checked, architecture
showing how authority is granted (human-to-agent, and agent-to-agent if
applicable).

**Procedure:** source review of the grant-checking code path; document/
config inspection of however delegation is represented (a database table,
an IAM policy, an OAuth scope, an MCP permission manifest); a controlled
staging test requesting an action with and without a grant present, to
observe the actual behavior rather than trust documentation alone.

**Outcomes:** ENFORCED if a real, explicit, checkable grant exists and is
checked before the action; VIOLATED if authority is ambient (any agent can
do anything a shared credential permits) or the check is missing/bypassable
*and* Gate 7 confirms the workflow actually claims per-grant authorization
(not merely that this looks loose by Crypton's own preference);
UNVERIFIABLE if the grant representation and its check could not be
inspected; UNREPRESENTABLE if the architecture has no concept of a
checkable grant at all; NOT_APPLICABLE if the workflow has exactly one
agent with one fixed, human-operated permission set and no delegation
concept applies.

**Evidence required for conclusion:** the actual grant representation
(policy/config/source) plus at least one authorization log or controlled
test showing it being checked, not just that a mechanism exists on paper.

---

### 3. Scope Attenuation / Privilege Inheritance

**Objective:** where authority passes through more than one hop (a human
grants to an agent, which grants to a sub-agent, or a workflow composes
multiple delegated steps), can it be shown that authority only ever
narrows, never widens?

**Required evidence:** the delegation chain's actual structure (parent/
child relationships, however represented), the scope at each hop, and
either source review of the narrowing check or a controlled test attempting
to widen scope at a hop.

**Procedure:** source review of any chain-composition/narrowing logic;
document inspection of the actual grants issued at each hop, compared by
hand for subset relationships; a controlled staging test attempting to
grant or use broader authority at a downstream hop than an upstream one
possessed. A Crypton-reference demonstration
(`chain.verify_attenuation`, exercised end to end in
`assessment-engagement/reference-agentlab-dryrun/`) illustrates exact-set
subset semantics for tools/resources and `child <= parent` for expiry — a
teaching aid, never a substitute for inspecting the customer's own chain.

**Outcomes:** ENFORCED if every observed hop narrows or preserves scope,
with evidence; VIOLATED if any hop widens scope, or if scope comparison is
not actually enforced (each hop's grant is independently trusted with no
narrowing check at all) *and* Gate 7 confirms the workflow actually claims
monotonic narrowing across hops; UNVERIFIABLE if the chain structure or hop
scopes could not be obtained; UNREPRESENTABLE if the architecture has no
multi-hop scope concept to inspect; NOT_APPLICABLE if the workflow has no
multi-hop delegation at all (a single human-to-agent grant, nothing
downstream of it).

**Evidence required for conclusion:** the actual scope values at two or
more hops (source, config, or logs) showing the comparison, or a controlled
test's observed result.

---

### 4. Pre-Execution Authorization Boundary & Bypass

**Objective:** is authorization actually evaluated, and does it actually
return ALLOW, *before* a protected action's side effect occurs — never
inferred, logged, or checked only after the fact?

**Required evidence:** source excerpts of the call path from "action
requested" to "action executed," execution traces showing the ordering, or
a controlled staging test that attempts to observe (or induce) execution
without a prior ALLOW.

**Procedure:** source review of the exact ordering (does the authorization
check's return value gate the call to the action, or does the action run
regardless and get logged/audited afterward?); a controlled staging test —
request a genuinely unauthorized action and confirm, by direct observation
(a side effect that would exist if it ran, an API response, a downstream
system state), that it did not occur, not just that an error was returned
that the caller could plausibly suppress or ignore.

**Outcomes:** ENFORCED if source review and/or a controlled test confirm
the action's side effect cannot occur without a prior ALLOW; VIOLATED if
the ordering is reversed (action first, check/log after) or the check can
be skipped/raced — Gate 7 here is close to automatic, since "authorize
before execution" is close to a universal claim every action-taking
workflow makes implicitly; UNVERIFIABLE if neither source access nor a
staging environment was available to test the ordering; NOT_APPLICABLE
essentially never applies here — every action-taking workflow has *some*
boundary to assess, even if the finding is that none exists (which is
VIOLATED, not NOT_APPLICABLE).

**Evidence required for conclusion:** source excerpt showing the ordering,
or a controlled test's observed (not assumed) result — a documentation
claim ("we enforce before execution") is not, by itself, sufficient
evidence for ENFORCED.

---

### 5. Authorization Bypass Paths

**Objective:** beyond the happy path, do specific, realistic bypass
attempts actually get denied?

**Required evidence:** controlled test results for each bypass category
attempted, or source review confirming the relevant check exists and covers
the case.

**Procedure:** a controlled staging test suite covering the bypass
categories relevant to the customer's actual architecture — chosen from,
but not limited to: requesting an action beyond granted scope; requesting
via a different agent identity claiming another's authority; reusing an
expired or revoked grant; a race between a revocation and an in-flight
action; parameter/argument substitution after a check was performed; an
attempt to have a downstream component trust a claim the upstream
component never actually verified. Crypton's own `test_delegation_chain.py`
attack-case set (permission amplification, resource escape, parent
substitution, unauthorized issuer, cycle, depth overflow, tampered
delegation, revoked-key retroactivity) is methodology inspiration for
*which categories to try* — it is Crypton's own control suite, not an
automated customer check (see "A note on Crypton's own test suite" below).

**Outcomes:** ENFORCED per bypass category actually attempted and denied;
VIOLATED per category that succeeded (Gate 7 is close to automatic here —
"this specific bypass category should be denied" is the claim being
directly tested); UNVERIFIABLE for a category that could not be tested (no
staging access, or the relevant code path could not be identified for
source review); UNREPRESENTABLE for a category whose precondition the
architecture cannot even construct; NOT_APPLICABLE for a category that
doesn't apply to the architecture (e.g. no multi-hop delegation exists, so
parent-substitution doesn't apply — see area 3).

**Evidence required for conclusion:** the specific request made, the
specific response/observed behavior, for each category — a general
statement that "bypass testing was performed" without per-category results
is not sufficient for either ENFORCED or VIOLATED.

---

### 6. Human-Approval Binding (Replay / TOCTOU / Approved-vs-Executed Drift)

**Objective:** where a human confirms a consequential action, does that
confirmation bind to the *exact* action (not just a tool name), get
consumed at most once, and match what actually executes?

**Required evidence:** approval records showing what was approved (ideally
a hash/fingerprint of exact arguments, not just an action label), execution
records showing what actually ran, and either source review of the
consumption/binding logic or a controlled replay/drift test.

**Procedure:** source review of what an approval record actually binds to
(tool name only? tool + resource? tool + resource + full argument set?);
a controlled staging test — approve one specific action, then attempt to
replay that same approval a second time, and attempt to execute a
*different* action under the same approval (changed arguments, changed
resource) — observe whether either succeeds; evidence reconstruction
comparing a sample of the customer's own approval records against their
corresponding execution records for drift.

**Outcomes:** ENFORCED if binding covers exact arguments, consumption is
provably single-use, and no drift is found between approved and executed
records; VIOLATED if binding is name-only (any argument value is treated as
"approved"), an approval is reusable, or drift is found between what was
approved and what executed — **and** Gate 7 confirms the drift actually
crosses a boundary the system claims (see the EXP-001d/e closure in "The
seven evidence-sufficiency gates" above for the exact case this guards
against: a real, reproducible drift that was still not a violation, because
the mutating code was trusted and the framework's own contract never
claimed otherwise); UNVERIFIABLE if approval/execution records could not be
compared or the binding mechanism's source could not be reviewed;
UNREPRESENTABLE if the architecture has no approval-binding concept to
inspect; NOT_APPLICABLE if the workflow has no human-approval step at all
(then areas 2/4/5's grant-level controls are what governs the action, and
that absence is worth noting in the report even though this area doesn't
apply).

**Evidence required for conclusion:** the approval record's actual bound
fields, plus either a controlled replay/drift test result or a sampled
comparison against real execution records.

---

### 7. Revocation / Expiry Propagation

**Objective:** when authority is revoked or expires, does enforcement
actually reflect that on the very next check — not after a delay, a cache
TTL, or a background sweep the customer may not have configured correctly?

**Required evidence:** source review of how revocation/expiry is checked
(live against a current record, or against a cached/stale copy), or a
controlled staging test measuring the actual delay between a revocation
action and enforcement reflecting it.

**Procedure:** source review of whether the authorization check reads a
live record or a cached one; a controlled staging test — revoke a grant (or
wait out an expiry), then immediately attempt the previously-valid action,
and observe the actual outcome and any measurable delay.

**Outcomes:** ENFORCED if the very next check after revocation/expiry
denies, with no meaningful propagation delay; VIOLATED if a revoked/expired
grant remains usable for a meaningful window (a cache TTL, a stale session,
a background job that hasn't run yet) and Gate 7 confirms the customer
actually claims immediate revocation (not a documented, accepted propagation
delay); UNVERIFIABLE if revocation could not actually be triggered/observed
in the engagement (no write access, no staging environment);
UNREPRESENTABLE if the architecture has no revocation concept to measure at
all; NOT_APPLICABLE if the engagement has no live/production traffic
against which timing can be measured at all (e.g. a one-shot synthetic dry
run — see `assessment-engagement/reference-agentlab-dryrun/`'s AD-07 for
exactly this case).

**Evidence required for conclusion:** a controlled test's observed
before/after result, or source review of the live-vs-cached read path.

---

### 8. Evidence / Accountability Reconstruction

**Objective:** after the fact, can "who did what, under whose authority,
and was it allowed" be reconstructed from durable records — not inferred,
not reliant on ephemeral logs, not requiring the agent/system itself to be
trusted to have told the truth?

**Required evidence:** the customer's actual decision/audit log
(authorization decisions, not just application logs), execution traces,
and confirmation that these are durable (retention, append-only or
otherwise tamper-evident) rather than debug output that rotates or can be
edited.

**Procedure:** document/config inspection of what's actually persisted
and for how long; source review confirming a decision is recorded *at the
point of the decision*, not reconstructed later from unrelated logs; a
controlled test — take one action, then attempt to reconstruct exactly
what happened using only the customer's own durable records, without
insider knowledge of what you just did.

**Outcomes:** ENFORCED if a decision-scoped, durable, reconstructable record
exists and was demonstrated to answer the who/what/authority/outcome
question from records alone; VIOLATED if reconstruction requires ephemeral
logs, requires trusting the agent's own self-report, or durable records
don't actually cover authorization decisions (only generic application
logs) — Gate 7 here is close to automatic, since durable accountability is
close to a universal claim; UNVERIFIABLE if records could not be inspected;
UNREPRESENTABLE if the architecture has no decision-logging concept
whatsoever to inspect; NOT_APPLICABLE essentially never applies — every
workflow has *some* answer to "can this be reconstructed," even if the
answer is VIOLATED.

**Evidence required for conclusion:** the actual reconstruction attempt's
result, referencing the specific records used.

---

## A note on Crypton's own test suite

`labs/agent-lab/tests/test_delegation_chain.py`, `test_enforcement_boundary.py`,
`test_approval.py`, and related files are **Crypton's own control suite** —
proof that Crypton's reference implementation understands and correctly
handles these failure modes. They are valuable as:

- internal reference cases,
- methodology inspiration (which categories of bypass/failure to test for,
  above),
- control examples to show a customer's team what a specific ENFORCED/VIOLATED
  looks like,
- evidence that Crypton understands the failure modes it is assessing for.

**They are not, and must never be represented as, an automated customer
assessment check.** Running Crypton's own test suite against Crypton's own
SQLite database and reporting the result as if it evaluated a customer's
system is exactly the mistake this methodology exists to prevent. A finding
about a customer's workflow requires evidence *from that workflow* — see
each area's "Evidence required for conclusion" above.

`assessment-engagement/reference-agentlab-dryrun/` is the one sanctioned
exception: it is explicitly labeled "Crypton Reference Agent Lab —
Methodology Dry Run," self-assesses Crypton's own reference implementation,
and exists solely to prove the *reporting mechanics* work end to end with
real (not fabricated) evidence — never to stand in for, or be cited as,
results about an external customer's system.

---

## The Authority Model (v0.1, added 2026-10-01)

`labs/agent-lab/src/agentlab/assessment/authority_model/` is a *derived*
layer built on top of everything above — it never replaces or duplicates
an engagement, finding, or evidence record; it normalizes already-existing
ones into a structure that can answer "what authority did this actor have,
and did the observed exercise of it match?"

**Layers, never collapsed:** RAW EVIDENCE -> **Authority Ledger**
(`ledger.py` — a time-ordered sequence of typed events, each referencing
only the Principal/Actor/Grant/Approval/ExecutionContext/Action/Resource/
Effect objects the evidence actually supports) -> **Authority Envelope**
(`envelope.py` — the effective, permitted authority, derived by
intersecting whatever grant/approval dimensions the ledger evidences,
never a general policy evaluator) -> **Authority Diff**
(`diff.py` — expected envelope vs. an explicitly-named set of observed
actions, classified into one of ten divergence types or `NONE`) ->
Finding/Action.

**Provenance is first-class.** Every identity- or binding-sensitive field
(principal identity, approver identity, a grant's delegation edges, an
approval's binding to a specific action) is a `Fact` — a value plus
*how* it was established (`OBSERVED`/`CONFIGURED`/`DERIVED`/
`CUSTOMER_ASSERTED`/`UNKNOWN`) and whether it's even knowable at all
(`AVAILABLE`/`UNREPRESENTABLE`/`UNVERIFIABLE`/`UNKNOWN`). A missing
principal or approver is represented as exactly that — missing, with a
stated reason — never inferred or defaulted. See `fact.py`'s module
docstring.

**Normalization code is fixture-specific, not a framework.**
`fixtures.py` contains exactly three builder functions (one per existing
evidence source it has been proven against — Agent Lab's own reference
dry run, Mastra Candidate 4a, EXP-003A's argument-binding case) and no
adapter registry, plugin system, or ingestion pipeline. Build a fourth only
when a fourth real target needs it.

**Rendering is text-only and generated, never hand-authored.**
`render.py`'s `render_authority_map` renders only what the model actually
contains — an unresolved fact prints the literal word `UNKNOWN`,
`UNREPRESENTABLE`, or `UNVERIFIABLE`, never silently disappearing, since
its absence can itself be security-relevant.

**Validated against three fixtures, not one:** a rich Agent Lab case
(AD-02 — a real `SCOPE_EXPANSION` divergence, the issued grant exceeding
the workflow's stated need), Mastra Candidate 4a (a real external-framework
`APPROVAL_SCOPE_MISMATCH` — an ambiguous-scope approval resolving two
actions, not at most one), and EXP-003A's H1a (a real external-framework
`NONE` — argument binding held, demonstrating the model represents a
clean system as faithfully as a violated one). Outputs for each:
`authority-ledger.json`, `authority-envelope.json`, `authority-diff.json`,
`authority-map.txt` under `assessment-engagement/authority-model-fixtures/`.

**Report integration is light-touch.** `Engagement.authority_map` (a plain
string field) is the only report-renderer surface this layer touches —
`report.py`'s "Authority Map / Authority Diff" section renders it verbatim
or states "not populated," never fabricating content to fill the section.

**Report surface (added 2026-10, same pass as above):**
`presentation.py` (`AuthorityView` — Header/expected-observed paths/
Evidence Strip/Action Strip, a projection, never a second source of truth)
and `render_text.py` (the provenance visual grammar: `────` OBSERVED,
`- - -` CONFIGURED, `········` CUSTOMER_ASSERTED, `[derived]` DERIVED,
`[invariant]` the presentation-only `ASSESSED_INVARIANT` category, `?`
UNKNOWN, `════`/`+==` the unexpected-authority marker). Outputs per
fixture: `authority-map.txt` (structural), `authority-diff.txt` (the
Header + expected/observed comparison + Evidence Strip + Action Strip),
`authority-view.json` (the projection). `property_mapping.py` mirrors
`docs/methodology/08-exp003-property-id-mapping.md` in code specifically
so a property's displayed name is always sourced from that stable mapping
— never inferred from a bare local H-number, which is the precise
cross-framework comparison error the 2026-09-30 audit found.

**Presentation accuracy rule:** expected/permitted authority and observed
target state are separate semantic categories. Expected/permitted
authority may originate from an assessed invariant, configured policy, a
source contract, or customer-stated intended policy; observed authority
must always originate from runtime evidence. An assessed invariant (e.g.
"an approval should resolve at most one pending action") is never rendered
as UNREPRESENTABLE/UNVERIFIABLE — that would wrongly imply the security
property itself is missing. A target's inability to represent *which*
action an approval binds to is a separate, genuinely evidenced fact about
the target, rendered on the OBSERVED side only.

---

## FROZEN

> **Authority Model v0.1 — FROZEN**
> **Authority Report Surface v0.1 — FROZEN**
>
> Frozen means: no schema expansion without evidence from a real target;
> no new divergence types without a demonstrated case; no
> presentation-model expansion solely for aesthetics. Correctness fixes
> remain allowed (this is how the 2026-10 freeze pass itself was able to
> fix the EXP-003A property-label and Mastra invariant/representation
> conflation issues without reopening scope). This does **not** imply
> industry-standard status, and does **not** imply external field
> validation has already occurred.
>
> **Explicit limitation:** v0.1 is validated against internal/reference
> evidence (Agent Lab's own AD-02) and existing third-party evidence
> fixtures already captured by prior research (Mastra Candidate 4a,
> EXP-003A's H1a) — it has **not** yet been validated through a live,
> worksheet-based authority reconstruction of an unfamiliar customer
> system. That is the next validation milestone: **First External
> Authority Reconstruction.** No further engineering roadmap beyond what
> that milestone itself requires is implied by this freeze.
