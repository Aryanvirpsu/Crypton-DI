# Crypton Assessor Handbook v1.0

**Who uses this:** any Crypton assessor, from day one on the job.
**When:** read in full before your first engagement; keep it open
during every engagement as the standing reference for judgment calls.
**Why it exists:** so a new assessor who has never seen EXP-001–005 can
still make the same calls a founder-trained assessor would make,
because the calls are written down, not tribal knowledge.

This handbook is grounded entirely in validated work: `docs/ASSESSMENT_METHODOLOGY.md`,
`docs/business/ASSESSMENT_PACK.md`, `docs/methodology/` (EXP-004), and
the findings/adaptations re-validated in `VALIDATION-REPORT.md`
(EXP-006). Where this handbook states a rule, it is quoting or directly
applying one of those documents, not inventing new doctrine. Where a
capability is not yet validated, it is labeled **FUTURE CAPABILITY**
and must not be represented to a customer as something Crypton can
deliver today.

> **2026-09-30 outcome-vocabulary note:** `docs/ASSESSMENT_METHODOLOGY.md`
> is now the canonical source for outcome labels —
> **ENFORCED / VIOLATED / UNREPRESENTABLE / UNVERIFIABLE / NOT_APPLICABLE**,
> plus a seventh evidence-sufficiency gate (Gate 7: security-claim /
> trust-boundary triage). This handbook's §9/§9a triage reasoning below
> predates that formalization and still uses PASS/FAIL/INCONCLUSIVE in its
> prose — the *reasoning* is unchanged and still the right judgment process
> (the §9a triage questions below are exactly what Gate 7 now codifies
> structurally in `assessment/models.py`), but map its outcomes going
> forward as: PASS -> ENFORCED, a FAIL that survives §9a triage ->
> VIOLATED, a FAIL that doesn't -> not a reportable violation (see the
> methodology doc's `migrate_legacy_outcome`), INCONCLUSIVE -> UNVERIFIABLE
> (or UNREPRESENTABLE if the underlying concept genuinely isn't
> representable, not merely unobserved). Do not file a canonical
> `Finding.status` using this handbook's older PASS/FAIL/INCONCLUSIVE
> labels directly — `assessment/models.py::Finding` will reject them.

---

## 1. Mission

Crypton determines whether authority actually flows the way an
organization believes it does through an AI-agent workflow that takes
real actions — before a customer is told an authorization decision is
safe, Crypton finds out whether it actually is, using evidence, not
assumption. The commercial vehicle for this, as of v1.0, is the Agent
Authorization & Delegation Assessment (`docs/business/ASSESSMENT_PACK.md`).
The longer-run research direction (action-integrity testing for
autonomous security agents, `CLAUDE.md` §8) is **not** what this
handbook operationalizes — this handbook is for the assessment service
that is actually sellable and deliverable today.

## 2. Assessment Philosophy

- **Evidence over assumption.** A documentation claim ("we enforce
  before execution") is never, by itself, sufficient for PASS. See
  `06-evidence-standard.md`.
- **A weaker honest result beats a stronger unverified one.** This is
  not a slogan — it is demonstrated, repeatedly, in the validated
  corpus: EXP-001's original finding was voluntarily superseded once a
  more representative test existed (`VALIDATION-REPORT.md` §1); EXP-003B's
  harness bug was caught and fixed rather than reported as-is; EXP-005's
  static-only limitations were stated up front, not discovered by the
  customer later. Do the same on every engagement.
- **Manual review is a legitimate method, not a fallback.** Crypton does
  not run an automated scanner (`docs/business/ASSESSMENT_PACK.md` §F).
  Source review, document inspection, and a controlled staging test are
  all first-class evidence-gathering methods.
- **Recommend the control, not Crypton.** Never steer a finding's
  remediation toward a future Crypton product. This is a repeated,
  explicit rule across every methodology document in this repository
  and it is not negotiable.
- **State the boundary of every claim.** "PASS" means "the specific
  thing we tested held, under the specific conditions we tested it
  under" — not "this system is secure." Every report section should
  read that way.

## 3. Scope

**In scope for the core (non-add-on) engagement**, per
`docs/business/ASSESSMENT_PACK.md` §B and the eight areas in
`docs/ASSESSMENT_METHODOLOGY.md`:

- Agent Identity & Ownership
- Delegation & Authority Flow
- Scope Attenuation / Privilege Inheritance
- Pre-Execution Authorization Boundary & Bypass
- Authorization Bypass Paths
- Human-Approval Binding (Replay / TOCTOU / Drift)
- Revocation / Expiry Propagation
- Evidence / Accountability Reconstruction

Default engagement shape: **1 agent workflow, up to 3 connected tools,
1 staging/test environment, 3 testing days, 1 written report, 1
remediation walkthrough.**

## 4. Out of Scope

- **Prompt injection, jailbreaks, and behavioral red-teaming** — explicitly
  excluded from the lead assessment (`docs/ASSESSMENT_METHODOLOGY.md`).
  Sold, if at all, as a separate add-on with a different methodology
  this kit does not define. **FUTURE CAPABILITY** — no validated
  methodology exists yet for this add-on.
- **Production environment testing, load/DoS testing, social
  engineering, physical security** — out of scope per the SOW
  (`docs/business/ASSESSMENT_PACK.md` §B).
- **Compliance attestation or audit sign-off** — Crypton delivers a
  security assessment, not a certified audit. Never represent a report
  as satisfying a compliance framework.
- **Tool substitution testing (H9) and scope/target-set expansion
  testing (H7)** — defined as hypotheses, never yet run against any
  target (`docs/methodology/02-threat-model.md`). If a customer's
  engagement would benefit from these, say so plainly and mark the
  result NOT_APPLICABLE-or-untested with the reason "not part of
  Crypton's validated test suite as of v1.0," not silently skip them.
- **Multi-tenant isolation and tool supply-chain integrity** — no
  pillar, area, or test currently covers these
  (`VALIDATION-REPORT.md` §4, §6). **FUTURE CAPABILITY.**
- **Concurrent/parallel multi-agent race conditions** — never tested
  against any target to date, despite at least one validated target
  (OpenHands SDK) having real concurrent sub-agent execution.
  **FUTURE CAPABILITY.**

## 5. Terminology & Definitions

| Term | Definition | Source |
|---|---|---|
| **Agent** | A component that can independently initiate a consequential action, distinct from the human or process that ultimately benefits from it. | `docs/ASSESSMENT_METHODOLOGY.md` area 1 |
| **Authority / grant** | A real, checkable representation of "agent X may do Y" — a policy, scope, or delegation object, not an ambient/assumed permission. | area 2 |
| **Scope attenuation** | The rule that authority may only narrow, never widen, as it passes through a delegation hop. | area 3 |
| **Pre-execution boundary** | The point in the call path where an authorization check's return value actually gates the protected action, before any side effect occurs. | area 4 |
| **Approval binding** | The property that a human confirmation is tied to the exact arguments that will execute, not merely a tool/action label. | area 6 |
| **Drift** | Any divergence between what was approved and what actually executed. | area 6, `finding-001d` |
| **Finding** | One conclusion, about one control, reaching exactly one of four outcomes (§6 below), with a unique `finding_id`. | `assessment/models.py::Finding` |
| **Evidence** | A pointer to something actually inspected/collected — never the claim itself. | `assessment/models.py::EvidenceReference` |
| **Pillar** | One of seven framework-independent property categories (Identity, Authority, Delegation, Human Approval, Execution, Persistence, Auditability) — the teaching/customer-communication vocabulary layered over the eight operational areas. | `docs/methodology/01-security-property-catalog.md` |
| **Universal Hypothesis (H1-H9)** | A standing, framework-independent testable claim, reused verbatim across targets rather than invented per engagement. | `docs/methodology/03-assessment-methodology-v0.1.md` |
| **Candidate finding** | A FAIL-shaped result whose severity/exploitability reasoning concludes it's an application-design trust-boundary consideration rather than a framework/product defect — still a real, reportable finding, framed accurately. | `VALIDATION-REPORT.md` §1 (EXP-003C H1c) |
| **INCONCLUSIVE** (now split into UNVERIFIABLE/UNREPRESENTABLE, see the 2026-09-30 note above) | Sufficient evidence could not be obtained, by any method available in the engagement — a legitimate, expected outcome, not a soft failure to explain away. | `assessment/models.py::Outcome` |

## 6. The Seven Security Pillars

Full definitions, questions, and grounding: `docs/methodology/01-security-property-catalog.md`.
Summary table for quick reference:

| # | Pillar | One-sentence definition | Maps to assessment area(s) |
|---|---|---|---|
| 1 | Identity | A durable, owned, non-impersonable identity, distinct from any human/credential/execution instance. | Agent Identity & Ownership |
| 2 | Authority | A permission that is explicitly created, boundedly represented, and cannot silently expand, leak, replay, or outlive its policy. | Delegation & Authority Flow, Pre-Execution Boundary, Authorization Bypass, Revocation/Expiry |
| 3 | Delegation | Authority passed between principals is explicit, chainable, attenuating, and revocable. | Delegation & Authority Flow, Scope Attenuation |
| 4 | Human Approval | A confirmation binds to the exact proposed action, is consumed at most once, matches what executes, and has defined scope/expiry/revocability. | Human-Approval Binding |
| 5 | Execution | What actually runs is deterministic, bound to what was authorized, substitution-resistant, exactly-once. | Pre-Execution Boundary (partial) |
| 6 | Persistence | Checkpointed/serialized state cannot be used to gain, restore, or replay authority beyond a fresh evaluation. | Revocation/Expiry (partial) |
| 7 | Auditability | Every authorization decision and execution can be reconstructed after the fact, with attribution, from durable records, without trusting the agent's self-report. | Evidence/Accountability Reconstruction |

**Known overlap (do not treat the pillars as a strict partition):**
Authority spans four of the eight operational areas; Authority and
Execution both touch "is the check applied before the side effect";
Authority and Persistence both touch revocation/expiry timing
(`VALIDATION-REPORT.md` §4). **File findings against the eight
operational areas** (they are the precise, code-enforced unit); use the
pillars for customer-facing narrative and the trust-flow map.

## 7. Threat Model

Full detail: `docs/methodology/02-threat-model.md`. Eleven attacker
objectives; current validated coverage (post-EXP-006 revalidation):

| Objective | Status | Evidence tier |
|---|---|---|
| Acquire authority | Coverage gap | None — no target studied provides an identity primitive to forge against |
| Expand authority | Coverage gap | H7 never run |
| Reuse authority | PASS (checkpoint-reuse vector) | H4, 3 targets |
| Forge authority | Coverage gap | Never tested |
| Replay authority | PASS (checkpoint-reuse vector, same evidence as Reuse) | H4, 3 targets |
| Confuse authority | Partial PASS (delegation-boundary reading) + 1 new counter-example (FINDING-005-1, self-report gating) | H2, 3 targets; EXP-005 |
| Lose attribution | Coverage gap, now with real-target evidence (not just by omission) | EXP-003 (by omission), EXP-005 (agent-server shared-secret admission) |
| Bypass approval | PASS (top-level path) / FAIL by default (delegated path) | Baseline B, 3 targets; FINDING-005-2 |
| Modify execution | Split — CONFIRMED/APPLICATION DESIGN CHOICE for argument drift via a documented state-mutation API (EXP-003C H1c, FL-02); `finding-001d/e`'s middleware-composition mechanism closed 2026-09-27 as `NO VULNERABILITY — EXPECTED FRAMEWORK BEHAVIOR` (§9a triage; retired from Finding Library, see `labs/experiments/001-langgraph-approval-drift/CLOSURE.md`); coverage gap for tool substitution (H9) | Full corpus |
| Escape delegation | Partial PASS (gate independence) / gap (gate delivery, FINDING-005-2) | H2, 3+ targets; EXP-005 |
| Persist privilege | Coverage gap in live-timing terms; 1 real historical instance found-and-fixed (OpenHands PR #4813) | EXP-005 (context finding) |

**Read this table as a floor, not a ceiling.** Every "coverage gap" row
means Crypton has not validated a test for that objective — it does not
mean the objective doesn't matter for a given customer. Say so plainly
in every engagement's threat-coverage appendix.

## 8. Evidence Philosophy

See `06-evidence-standard.md` for the full canonical package. Core
rules, restated here because they govern every judgment call an
assessor makes in the field:

1. No PASS or FAIL without at least one evidence reference — structurally
   enforced in code, and true in spirit for every deliverable in this
   kit, code-enforced or not.
2. A Crypton-reference demonstration (Agent Lab) can teach or motivate a
   question. **It never substitutes for evidence about the customer's
   own system.**
3. Evidence is a pointer to something inspected, never the claim itself.
   "We reviewed the authorization code" is not evidence; the specific
   file, the specific function, the specific behavior observed, is.
4. Record the method that produced each conclusion (document/config
   inspection, source review, controlled staging test, evidence
   reconstruction, Crypton-reference demonstration) — a report reader
   should always be able to tell *how* a conclusion was reached.

## 9. Decision Rules

- **PASS** requires real evidence showing the expected control in
  place and functioning — never inferred from absence of a failed test.
- **FAIL** requires real evidence showing a gap — never assumed from a
  customer's own uncertainty ("we're not sure if this is checked" is a
  reason to test, not a FAIL by default).
- **INCONCLUSIVE** requires a stated reason (no access granted,
  environment unreachable, logs rotated, process doesn't exist to
  inspect) — a bare INCONCLUSIVE with no reason is not acceptable
  practice.
- **NOT_APPLICABLE** requires the control to genuinely not apply to
  this workflow's architecture — not "we didn't get to it" (that's
  INCONCLUSIVE) and not "the customer didn't want us to check" (that's
  a scoping note, and likely INCONCLUSIVE too, depending on why).
- When a result doesn't cleanly bisect into the outcomes, extend
  additively with a **qualifier**, never an invented base outcome.
  Validated precedents (`PASS_WITH_CAVEAT`,
  `CANDIDATE_FINDING_APPLICATION_LEVEL`) came from EXP-003's research-lab
  evidence schema, which is free-form by design — the real, code-enforced
  customer schema (`assessment/models.py::Outcome`, five values as of
  2026-09-30: ENFORCED/VIOLATED/UNREPRESENTABLE/UNVERIFIABLE/NOT_APPLICABLE)
  has no sixth value to add in a live engagement. **In a real customer
  report, file a "candidate finding" as `outcome = VIOLATED` only if Gate 7
  (security-claim/trust-boundary triage) affirmatively confirms a real
  boundary was crossed, with the qualifier stated in
  `risk_impact`/the write-up prose** (e.g. "VIOLATED — classified as an
  application-design trust-boundary consideration, not a framework defect;
  see Risk"); if Gate 7 does *not* confirm a crossed boundary, it is not a
  reportable violation at all — see `docs/ASSESSMENT_METHODOLOGY.md`'s
  EXP-001d/e discussion of exactly this distinction, and
  `VALIDATION-REPORT.md` §1 for EXP-003C's H1c as a worked example that
  predates Gate 7's formalization. Do not leave `outcome` ambiguous or
  invent a new enum value hoping the report renderer will accept it.
- **Severity** is set only on FAIL (occasionally an INFORMATIONAL note
  on an otherwise-passing area), reasoned through the seven-factor
  rubric (`docs/ASSESSMENT_METHODOLOGY.md`), written out — not assigned
  from a category-to-severity lookup table, which does not exist and
  should not be invented.
- **Never represent Crypton's own test suite** (`labs/agent-lab/tests/`)
  as having evaluated a customer's system. It is reference material and
  methodology inspiration only.

## 9a. Triage: Surprising Behavior vs. Actual Boundary Violation

Added 2026-09-27 after `labs/experiments/001-langgraph-approval-drift/CLOSURE.md`
(finding-001d/e, originally classified CONFIRMED, closed as
`NO VULNERABILITY — EXPECTED FRAMEWORK BEHAVIOR` once this question was
asked properly). A difference between an approved action and an executed
action — or any other observed drift, mutation, or unexpected state
change — is **not sufficient by itself** to classify something as an
authorization or security failure. Before filing FAIL, or drafting a
disclosure to a target's maintainers, answer all five questions, in
order:

1. **Who or what caused the mutation?** Name the specific actor: a
   specific middleware/hook, a specific caller, a specific principal —
   not "the framework" in the abstract.
2. **What trust level does that actor have?** Is it untrusted input, an
   external/lower-privileged principal, or trusted application code
   running in the same trust domain as the control being examined (e.g.
   developer-authored middleware sitting alongside the developer's own
   approval gate)?
3. **Is that actor permitted, by the system's own design, to make this
   change?** Check the actual contract (source, docs, or a first-party
   shipped example doing the same thing) — not intuition about what
   "should" be allowed.
4. **What security boundary does the system actually claim to enforce?**
   State it precisely. A framework that never claimed a given invariant
   (e.g. "approval-argument binding survives an arbitrary chain of
   trusted extension hooks composed afterward") cannot have violated it.
5. **Did the observed behavior cross that claimed boundary?** Only a
   "yes" here, reached after questions 1-4, supports FAIL/CONFIRMED. A
   "no" — the actor was trusted, permitted by design, and no
   framework-claimed boundary was crossed — supports
   `NO VULNERABILITY — EXPECTED FRAMEWORK BEHAVIOR`, even when the
   observed drift is real, reproducible, and worth documenting for
   calibration.

**Rule of thumb:** `observed drift -> identify mutator -> classify trust
-> identify claimed boundary -> determine whether boundary was crossed ->
classify impact`. Run this before extending an existing library pattern,
before drafting a disclosure, and before upgrading a `CANDIDATE_FINDING`
or `APPLICATION DESIGN CHOICE` note to `CONFIRMED`. This does not replace
§9's four base outcomes or the seven-factor severity rubric — it is the
gate a candidate FAIL must pass *before* those apply, so Crypton does not
inflate a normal, documented framework behavior into a vulnerability
claim.

## 10. Confidence Model

Every finding states confidence as a reasoned statement, not a number
(`docs/methodology/05-evidence-reporting-standard.md`, link 11).
Calibration, grounded in the validated corpus's own range of results:

| Confidence | What it means | Validated example |
|---|---|---|
| **High** | Directly observed (source read, test executed, trace captured), reproducible deterministically, mechanism understood, not inferred. | `finding-001d` — deterministic 5/5, mechanism quoted verbatim from installed package source |
| **High (mechanism) / Moderate (real-world exploitability)** | The technical mechanism is certain; whether it matters in a given deployment depends on a configuration or deployment choice outside the assessor's control. | FINDING-005-1 — self-report gating is certain; real-world impact depends on whether the customer composes it with a deterministic analyzer |
| **Moderate** | Real evidence exists but with a stated, specific limitation (small sample, scripted rather than real model, single target). | EXP-003's scripted-model-only H1-H6 results (pre-EXP-001e-style real-model confirmation) |
| **Low / INCONCLUSIVE** | Evidence could not be obtained, or was static-only where live confirmation would materially change confidence. | EXP-005's H1/H4/H6 hypothesis results — static source review only, explicitly not confirmed live |

**Rule:** never round a Moderate or Low confidence up for a cleaner
report. A customer trusts Crypton's confidence statements exactly
because they are calibrated honestly, including when that means saying
"we are not fully sure."

## 11. Quality Expectations

- Every finding traces through all eleven links of the Evidence &
  Reporting Standard (`06-evidence-standard.md`).
- Every checklist item (`03-assessment-checklists.md`) is completed with
  a stated outcome and evidence reference or a stated reason for its
  absence — no blank rows in a delivered report.
- Every report section states plainly when an area is clean ("if a
  section is clean, say so" — `docs/business/ASSESSMENT_PACK.md`).
- No finding is invented, padded, or restated to appear more numerous
  than it is. A three-finding report is not a worse deliverable than a
  ten-finding report if three is what the evidence supports.
- Every claim in a customer deliverable has a corresponding entry in
  the evidence appendix.

## 12. Customer Communication

- Lead with what was tested and the plain-language risk posture, before
  technical detail (Executive Summary structure, `09-customer-deliverable-templates.md`).
- State threat coverage explicitly (Appendix A of the report template)
  — an engagement that tested 6 of 11 objectives says so; it never
  implies full coverage by omission.
- Recommend the control, not Crypton, in every remediation.
- When a finding's interpretation is genuinely debatable (see
  `VALIDATION-REPORT.md` §4 on FINDING-005-1-shaped ambiguity), present
  the mechanism and the reasoning, not just the label — a customer's
  engineering team is allowed to disagree with a severity call as long
  as they can see exactly what evidence produced it.
- Never promise a finding count, a clean report, or a specific outcome
  before testing is complete.

## 13. Handling Uncertainty

- Default to INCONCLUSIVE with a stated reason over a guessed PASS or
  FAIL — this is not a weaker report, it is a more honest one, and it
  is the single most consistent practice across the entire validated
  corpus.
- If a hypothesis or checklist item genuinely cannot be evaluated with
  the methods available (no source access, no staging environment, the
  relevant process doesn't exist), say precisely why — "requires manual
  review" alone is never acceptable; name what specifically is missing.
- If a finding's classification is debatable (CONFIRMED vs. APPLICATION
  DESIGN CHOICE vs. EXPECTED DESIGN), apply the seven-factor severity
  rubric and the exploitability-position test explicitly, and write out
  the reasoning — see `VALIDATION-REPORT.md` §1 for four worked
  examples of exactly this distinction being made and justified.
- If new evidence surfaces mid-engagement that contradicts an earlier
  finding, revise the finding — never preserve a conclusion because it
  was already written down (this is the standing discipline demonstrated
  by EXP-003B's harness-bug correction and this handbook's own source
  report, `VALIDATION-REPORT.md`).

## 14. Escalation Rules

- **Scope creep:** if a serious issue appears outside the named scope,
  stop testing it and report it — do not test further without an
  explicit scope amendment (`docs/business/ASSESSMENT_PACK.md` §C).
- **No staging access / withheld source:** record as INCONCLUSIVE with
  the specific reason; escalate to the engagement lead before the
  report is finalized if this affects a majority of the engagement's
  scope — a report that is mostly INCONCLUSIVE is a signal the
  engagement itself needs a conversation with the customer, not just a
  finding to write up.
- **A finding implicates a third-party framework/product defect (not
  the customer's own integration):** verify against the *installed*
  version, cite the exact source, and frame the finding as an
  application-design/trust-boundary consideration unless the evidence
  clearly shows otherwise — see `VALIDATION-REPORT.md` §1's EXP-003C
  H1c and FINDING-005-1 entries for the calibration on when this
  framing applies.
- **Uncertain whether something is authorized to test:** stop and
  confirm against the signed Authorization-to-Test before proceeding
  (`docs/business/ASSESSMENT_PACK.md` §C) — never assume.
- **A prior Crypton finding or methodology component seems wrong in
  light of new evidence:** raise it for revalidation using the same
  taxonomy and rigor as `VALIDATION-REPORT.md` — do not silently work
  around it or silently keep using it.
- **A customer disputes a finding or severity rating**, live in the
  remediation walkthrough or in writing: do not defend the label —
  walk back to the underlying mechanism and evidence (the sixteen-field
  package, `06-evidence-standard.md`) and let the customer's team engage
  with that directly. If their pushback reveals evidence the assessor
  didn't have (e.g. an undisclosed compensating control), re-run the
  seven-factor severity reasoning with the new information and revise
  the finding — this is the same "revise, don't defend a stale
  conclusion" discipline as the prior bullet, applied to a live
  customer conversation instead of internal review. If their pushback
  is a genuine difference of risk tolerance rather than new evidence
  (e.g. FINDING-005-1-shaped ambiguity, Handbook §4/`VALIDATION-REPORT.md`
  §4), say so plainly: the mechanism stands, the severity judgment is
  Crypton's documented reasoning, and the customer is free to weigh it
  differently for their own risk decisions — do not silently change a
  severity rating to avoid an uncomfortable conversation.

## 15. Research Ethics

- Every engagement operates under a signed Authorization-to-Test, named
  target systems, and a defined testing window
  (`docs/business/ASSESSMENT_PACK.md` §C) — no exceptions, no informal
  verbal go-aheads.
- No destructive actions; no production data exfiltration beyond the
  evidence actually needed to support a finding.
- Stay entirely within public APIs and the customer's authorized staging
  environment. Never modify framework internals, monkey-patch private
  state, or depend on undefined behavior (standing rule across every
  EXP-00X lab, carried into every customer engagement).
- Findings are shared only with the customer — responsible disclosure,
  no public write-up of a customer engagement without separate,
  explicit permission.
- When citing a third-party framework's behavior as evidence (as in
  `finding-001d`'s quoted `langchain` source, or EXP-005's OpenHands
  citations), quote and cite precisely, pin the exact version, and never
  represent inferred or remembered behavior as directly observed.
- Prior-session or prior-experiment claims are re-verified, not assumed
  — see `VALIDATION-REPORT.md`'s entire method, and the standing rule
  ("never trust a version number given in a prompt") that this corpus
  has followed (and once, in EXP-003B, had to self-correct after
  skipping).
