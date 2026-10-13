# Crypton Assessment Checklists v1.0

**Who uses this:** the assessor, during Stages 3-9 of the SOP
(`02-standard-operating-procedure.md`), as the literal working
checklist for each property.
**When:** one pass per pillar/property that is IN SCOPE per the Stage 8
threat model.
**Why it exists:** so a PASS/FAIL/INCONCLUSIVE/NOT_APPLICABLE call is
reached by matching evidence against a pre-written criterion, not by
individual judgment about what "good" looks like. Every criterion below
is either quoted from, or a direct operational restatement of,
`docs/ASSESSMENT_METHODOLOGY.md`'s eight areas — nothing here
introduces a new bar for PASS/FAIL that document doesn't already set.

**Organization:** one checklist per **operational area** (the eight
areas are the precise, code-enforced filing unit —
`assessment/models.py::AssessmentArea`), grouped under the **pillar**
it primarily belongs to (Handbook §6). Where an area spans more than
one pillar, it is cross-referenced, not duplicated.

**Freeze status reminder** (full detail: `VALIDATION-REPORT.md` §7):
items marked **FROZEN**/**STABLE** are the validated, expected-every-engagement
checklist. Items marked **EVOLVING** are real, worth asking, but are
single-target-evidenced — do not present a EVOLVING item's absence as
an established industry norm the customer is failing to meet.

---

## Pillar 1 — Identity

### Property: Agent Identity & Ownership (Area 1) — STABLE

**Objective:** can the customer answer, for any agent that can act,
*which specific registered application it is* and *who owns/is
accountable for it*?

**Evidence:** identity records (agent registry, service account
inventory, MCP server/tool registry); the provisioning process (who can
register a new agent, on whose authority); the deprovisioning process;
architecture/configuration showing agent identity is distinguished from
human identity and from a single execution instance.

**Questions (ask the customer, verify, do not accept unverified):**
1. How does each agent authenticate today? (human's key, service
   account, OAuth token, per-agent identity?)
2. Who "owns" each agent internally? Who can authorize a new one?
3. Can an agent's identity be reassigned or reused after
   decommissioning without a new registration event?
4. Is identity resolved from a durable record, or asserted by the
   caller at request time?

**Positive test:** locate the identity-resolution code path; confirm it
reads from a durable, owned record (not a caller-supplied field);
confirm a real provisioning process exists (not just a table that could
theoretically be populated).

**Negative test:** attempt to trace one in-scope action back to a
specific, owned agent identity using only the customer's own records
(not insider knowledge of what you, the assessor, just did) — if this
cannot be done, the identity claim is not evidenced.

**Pass criteria:** identity is a durable, owned record; a real
provisioning process exists; evidenced by identity records or source
excerpts.
**Fail criteria:** identity is an unverified string/label a caller can
assert with no registration/ownership check.
**Not Applicable criteria:** the workflow has no independent "agent"
concept at all (a single script, no multi-agent structure) — verified
by architecture review, not assumed from a short questionnaire answer.
**Inconclusive criteria:** provisioning process/records could not be
obtained (access withheld, no documentation exists to inspect) — state
the specific reason.
**Confidence rules:** High if both the resolution mechanism (source) and
the provisioning process (documented or interviewed) are confirmed;
Moderate if only one of the two is confirmed; do not report High
confidence from documentation claims alone.
**Required artifacts:** Identity Graph worksheet, at least one
`EvidenceReference` of type `identity_records` or `source_excerpts`.

---

## Pillar 2 — Authority

> Spans four operational areas by design (`VALIDATION-REPORT.md` §4) —
> this is the broadest pillar; file each finding against its specific
> area below, use "Authority" only for customer narrative.

### Property: Delegation & Authority Flow (Area 2) — STABLE

**Objective:** can the customer show, for any consequential action,
*who authorized this agent to do it* — a real, traceable grant, not
ambient permission?

**Evidence:** policy/delegation definitions; source excerpts of the
authorization check; authorization logs showing real grants being
checked; architecture showing how authority is granted.

**Questions:**
1. Where is authority created — a config file, a database row, an IAM
   policy, an OAuth scope?
2. What actually gates the action — is the grant checked, or merely
   assumed present?
3. Can one agent grant authority to another? Under what representation?

**Positive test:** source review of the grant-checking code path; a
controlled staging test requesting an in-scope action *with* a grant
present, observing it succeed as expected.
**Negative test:** a controlled staging test requesting the same action
*without* a grant present (or with a grant for a different
agent/resource), observing whether it is actually denied.

**Pass criteria:** a real, explicit, checkable grant exists and is
checked before the action, evidenced by the grant representation plus
at least one authorization log or controlled test result.
**Fail criteria:** authority is ambient (any agent can do anything a
shared credential permits), or the check is missing/bypassable.
**Not Applicable criteria:** exactly one agent, one fixed, human-operated
permission set, no delegation concept applies — verified by
architecture review.
**Inconclusive criteria:** the grant representation and its check could
not be inspected.
**Confidence rules:** High only when both the grant representation
*and* a real check event (log or controlled test) are evidenced — a
grant representation alone (no observed check) caps confidence at
Moderate.
**Required artifacts:** Authority Graph worksheet, evidence of the grant
representation, evidence of at least one check event.

### Property: Scope Attenuation / Privilege Inheritance (Area 3) — STABLE
(also Pillar 3 — Delegation; see there for delegation-specific detail)

Cross-referenced below under Pillar 3.

### Property: Pre-Execution Authorization Boundary & Bypass (Area 4) — STABLE
(also Pillar 5 — Execution; full checklist under Pillar 5)

**Objective:** is authorization actually evaluated, and does it return
ALLOW, *before* a protected action's side effect occurs?

Cross-referenced below under Pillar 5 (this is where "Execution" and
"Authority" overlap most directly — see `VALIDATION-REPORT.md` §4).

### Property: Authorization Bypass Paths (Area 5) — STABLE

**Objective:** beyond the happy path, do specific, realistic bypass
attempts actually get denied?

**Evidence:** controlled test results per bypass category, or source
review confirming the relevant check exists and covers the case.

**Questions:**
1. Can an action beyond granted scope be requested and denied?
2. Can an action be requested via a different agent identity, claiming
   another's authority, and be denied?
3. Can an expired or revoked grant be reused, and is it denied?
4. Is there a race window between revocation and an in-flight action?
5. Can arguments be substituted after a check was performed?
6. Can a downstream component be made to trust a claim the upstream
   component never actually verified?

**Positive test:** attempt each applicable bypass category above
against staging; confirm denial by direct observation (a side effect
that would exist if it ran, an API response, downstream system state),
not merely that an error was returned that the caller could plausibly
suppress or ignore.
**Negative test:** N/A — every attempt in this property *is* the
negative test; there is no separate positive/negative split here.

**Pass criteria:** per bypass category actually attempted and denied.
**Fail criteria:** per category that succeeded.
**Not Applicable criteria:** per category that doesn't apply to the
architecture (e.g. no multi-hop delegation exists, so parent-substitution
doesn't apply).
**Inconclusive criteria:** a category that could not be tested (no
staging access, relevant code path could not be identified).
**Confidence rules:** report per-category, not as one blended
confidence for the whole area — a report with 5 categories PASS and 1
INCONCLUSIVE should say so precisely, not average to "mostly PASS."
**Required artifacts:** the specific request made and specific
response/observed behavior, per category attempted — a general
statement that "bypass testing was performed" without per-category
results is insufficient for either PASS or FAIL.

### Property: Revocation / Expiry Propagation (Area 7) — STABLE
(also Pillar 6 — Persistence; full checklist under Pillar 6)

Cross-referenced below under Pillar 6.

---

## Pillar 3 — Delegation

### Property: Scope Attenuation / Privilege Inheritance (Area 3) — STABLE

**Objective:** where authority passes through more than one hop, can it
be shown that authority only ever narrows, never widens?

**Evidence:** the delegation chain's actual structure (parent/child
relationships); the scope at each hop; source review of the narrowing
check, or a controlled test attempting to widen scope at a hop.

**Questions:**
1. Can authority be delegated? Can delegation be chained more than one
   hop?
2. At each hop, is the granted scope a subset of (or equal to) the
   granting hop's scope, or can it be a superset?
3. Can a delegated child's authority exceed the parent's?
4. Can a parent's delegation be revoked in a way that's observable/
   enforceable against already-delegated children? (**EVOLVING** —
   single-target-evidenced only; see H7/`VALIDATION-REPORT.md` §2.)

**Positive test:** compare the scope values at two or more hops
(source, config, or logs); confirm subset/equal relationship holds.
**Negative test:** a controlled staging test attempting to grant or use
broader authority at a downstream hop than an upstream one possessed.

**Pass criteria:** every observed hop narrows or preserves scope, with
evidence.
**Fail criteria:** any hop widens scope, or scope comparison is not
enforced at all (each hop independently trusted).
**Not Applicable criteria:** no multi-hop delegation exists (a single
human-to-agent grant, nothing downstream of it).
**Inconclusive criteria:** the chain structure or hop scopes could not
be obtained.
**Confidence rules:** High requires actual scope *values* compared at
two-plus hops, not a documentation claim that narrowing is "supposed to
happen."
**Required artifacts:** the actual scope values at two-plus hops
(source, config, or logs), or a controlled test's observed result.

### Property: Delegation Gate Delivery — **EVOLVING, proposed EXP-005, not yet core**

**Objective:** when a delegated sub-agent's own authorization gate
requires human confirmation, is that requirement actually delivered to
a human, or silently auto-resolved by the delegation mechanism itself?

**Evidence:** source review of the delegation executor's handling of a
sub-agent's pending-confirmation state; whether a callback/handler is
required, optional, or absent by default.

**Questions:**
1. Does the delegation/sub-agent mechanism have its own confirmation
   gate, independent of the parent's?
2. If so, what happens by default (no extra integration work) when that
   gate fires — is a human actually asked, or is the pause silently
   resolved?
3. Does the framework's own reference example/documentation demonstrate
   correct wiring of this path?

**Positive test:** source review of the delegation executor's
default-parameter behavior when no handler/callback is supplied.
**Negative test:** attempt (source review or, if live access exists, a
controlled staging test) to trigger a sub-agent confirmation pause with
no handler wired, and observe whether it executes anyway.

**Pass criteria:** the default (no extra wiring) behavior is to block or
fail closed when a sub-agent's confirmation-requiring action pauses.
**Fail criteria:** the default behavior silently auto-approves/resolves
the pause with no human ever asked — validated example: OpenHands SDK's
`DelegateExecutor`/`TaskManager`, `confirmation_handler=None` default
(`VALIDATION-REPORT.md` §1, FINDING-005-2).
**Not Applicable criteria:** the workflow has no delegation/sub-agent
mechanism at all.
**Inconclusive criteria:** the delegation executor's default-parameter
behavior could not be determined from available source/documentation.
**Confidence rules:** this property is EVOLVING — cap confidence at
Moderate even on a clean source-verified result, and state in the
report that this checklist item is a Crypton v1.0 proposal, not yet a
field-proven universal expectation, per `VALIDATION-REPORT.md` §2.
**Required artifacts:** source excerpt of the default-parameter
behavior; a stated confidence caveat noting EVOLVING status.

---

## Pillar 4 — Human Approval

### Property: Human-Approval Binding — Replay / TOCTOU / Drift (Area 6) — STABLE

**Objective:** where a human confirms a consequential action, does that
confirmation bind to the *exact* action, get consumed at most once, and
match what actually executes?

**Evidence:** approval records showing what was approved (ideally a
hash/fingerprint of exact arguments, not just an action label);
execution records showing what actually ran; source review of the
consumption/binding logic, or a controlled replay/drift test.

**Questions:**
1. What exactly does the approval record bind to — tool name only? Tool
   + resource? Tool + resource + full argument set?
2. Can an approval be replayed a second time after it's been consumed?
3. Can the action's arguments change between approval and execution
   without a fresh approval?
4. Is there a standing/broad ("always approve") authorization, and if
   so, is its scope explicit (documented) or implicit?

**Positive test:** approve one specific action; confirm it executes
exactly as approved; attempt to replay that same approval a second time
and confirm it is denied/consumed.
**Negative test:** approve one specific action, then attempt to execute
a *different* action (changed arguments, changed resource) under the
same approval; observe whether it succeeds. This is the exact test
shape that produced `finding-001d`/`finding-001e` (`VALIDATION-REPORT.md`
§1) and EXP-003C's H1c candidate finding — this is Crypton's
highest-yield test in the entire kit; do not skip it under time
pressure.

**Pass criteria:** binding covers exact arguments; consumption is
provably single-use; no drift found between approved and executed
records.
**Fail criteria:** binding is name-only (any argument value treated as
"approved"); an approval is reusable; drift is found between approved
and executed.
**Not Applicable criteria:** the workflow has no human-approval step at
all (note this plainly — areas 2/4/5's grant-level controls are what
governs the action instead, and the absence itself is worth reporting).
**Inconclusive criteria:** approval/execution records could not be
compared, or the binding mechanism's source could not be reviewed.
**Confidence rules:** High requires either a controlled replay/drift
test with an observed result, or a sampled comparison against real
execution records — a source-review-only conclusion (no live test, no
sampled comparison) caps confidence at Moderate, consistent with
`VALIDATION-REPORT.md` §3's static-analysis threat.
**Required artifacts:** the approval record's actual bound fields, plus
either a controlled test result or a sampled real-record comparison.

### Property: Self-Reported Risk Independence — **EVOLVING, proposed EXP-005 (H10), not yet core**

**Objective:** if a risk/authorization signal gates a human-approval
decision, is that signal derived independently of the actor whose
action is being evaluated?

**Evidence:** source review of what produces the risk/priority signal
feeding the approval-gating decision.

**Questions:**
1. What determines whether an action requires confirmation — a fixed
   policy, or a computed risk level?
2. If computed, what computes it — a deterministic, independent
   analyzer, or the same model/agent that proposed the action,
   self-reporting?
3. If self-reported, is that disclosed as a limitation anywhere the
   customer's team would see it before relying on it?

**Positive test:** trace the exact source of the risk value from
generation to the confirmation-policy check; confirm it originates from
a component independent of the proposing actor (e.g. a deterministic,
pattern-based analyzer).
**Negative test:** confirm whether the proposing actor's own output is
the sole input to the risk-gating decision, with no independent
corroboration — validated example: OpenHands SDK's `LLMSecurityAnalyzer`
used alone (`VALIDATION-REPORT.md` §1, FINDING-005-1).

**Pass criteria:** the risk signal is derived independently of the
proposing actor, or is corroborated by at least one independent,
deterministic source.
**Fail criteria:** the risk signal is exclusively self-reported by the
proposing actor with no independent corroboration, and this is not
disclosed as a limitation to the integrator/customer.
**Not Applicable criteria:** the workflow has a fixed
(non-risk-computed) confirmation policy — there is no risk signal to
evaluate.
**Inconclusive criteria:** the source of the risk signal could not be
traced.
**Confidence rules:** this property is EVOLVING (single-target
evidence) — state that explicitly in the report; do not present this
as an established industry-wide expectation.
**Required artifacts:** source excerpt tracing the risk signal's origin;
a stated confidence caveat.

---

## Pillar 5 — Execution

### Property: Pre-Execution Authorization Boundary & Bypass (Area 4) — STABLE

**Objective:** is authorization actually evaluated, and does it return
ALLOW, *before* a protected action's side effect occurs — never
inferred, logged, or checked only after the fact?

**Evidence:** source excerpts of the call path from "action requested"
to "action executed"; execution traces showing the ordering; a
controlled staging test attempting to observe (or induce) execution
without a prior ALLOW.

**Questions:**
1. Does the authorization check's return value actually gate the call
   to the action, or does the action run regardless and get logged/
   audited afterward?
2. Can the check be skipped or raced?

**Positive test:** source review of the exact ordering.
**Negative test:** a controlled staging test — request a genuinely
unauthorized action and confirm, by direct observation (a side effect
that would exist if it ran, an API response, downstream system state),
that it did not occur.

**Pass criteria:** source review and/or a controlled test confirm the
action's side effect cannot occur without a prior ALLOW.
**Fail criteria:** the ordering is reversed (action first, check/log
after), or the check can be skipped/raced.
**Not Applicable criteria:** essentially never applies — every
action-taking workflow has *some* boundary to assess, even if the
finding is that none exists (which is FAIL, not NOT_APPLICABLE).
**Inconclusive criteria:** neither source access nor a staging
environment was available to test the ordering.
**Confidence rules:** a documentation claim ("we enforce before
execution") is never, by itself, sufficient for PASS — High confidence
requires source excerpt or observed test result.
**Required artifacts:** source excerpt showing the ordering, or a
controlled test's observed (not assumed) result.

---

## Pillar 6 — Persistence

### Property: Revocation / Expiry Propagation (Area 7) — STABLE

**Objective:** when authority is revoked or expires, does enforcement
actually reflect that on the very next check — not after a delay, a
cache TTL, or a background sweep?

**Evidence:** source review of how revocation/expiry is checked (live
vs. cached record); a controlled staging test measuring the actual
delay between revocation and enforcement.

**Questions:**
1. Does the authorization check read a live record, or a cached one?
2. What is the measured delay (if any) between revocation and denial
   taking effect?
3. Does reconstructing an agent/workflow from a checkpoint constitute a
   fresh authorization grant, or correctly preserve the pre-checkpoint
   state?

**Positive test:** revoke a grant (or wait out an expiry), then
immediately attempt the previously-valid action; observe the outcome
and any measurable delay.
**Negative test:** N/A — the positive test above *is* the adversarial
test; there is no separate negative variant.

**Pass criteria:** the very next check after revocation/expiry denies,
with no meaningful propagation delay.
**Fail criteria:** a revoked/expired grant remains usable for a
meaningful window.
**Not Applicable criteria:** the engagement has no live/production
traffic against which timing can be measured at all (e.g. a one-shot
synthetic dry run).
**Inconclusive criteria:** revocation could not actually be
triggered/observed (no write access, no staging environment).
**Confidence rules:** High requires an observed before/after result;
source review of the live-vs-cached read path alone caps confidence at
Moderate.
**Required artifacts:** a controlled test's observed before/after
result, or source review of the live-vs-cached read path.

**Related, validated context (not a separate checklist item — informs
severity reasoning when this property FAILs):** Crypton's corpus
contains one real, historical, third-party-confirmed instance of this
exact failure class (OpenHands SDK's authorization state reverting to
stale values on server restart, fixed by a merged PR —
`VALIDATION-REPORT.md` §1, "context finding"). Cite this as evidence
that the failure class is real and has occurred in production agent
software when reasoning about a FAIL's severity/plausibility — never
cite it as evidence *about the customer's own system*.

---

## Pillar 7 — Auditability

### Property: Evidence / Accountability Reconstruction (Area 8) — STABLE

**Objective:** after the fact, can "who did what, under whose authority,
and was it allowed" be reconstructed from durable records — not
inferred, not reliant on ephemeral logs, not requiring the agent/system
itself to be trusted to have told the truth?

**Evidence:** the customer's actual decision/audit log (authorization
decisions, not just application logs); execution traces; confirmation
these are durable (retention, append-only or tamper-evident).

**Questions:**
1. Is a decision recorded *at the point of the decision*, or
   reconstructed later from unrelated logs?
2. Does the durable record cover authorization decisions specifically,
   or only generic application activity?
3. Is per-caller/per-identity attribution preserved in the record, or
   only "some request happened"? (Validated coverage gap in every target
   Crypton has reviewed to date — see Handbook §7, "Lose attribution.")

**Positive test:** take one action, then attempt to reconstruct exactly
what happened using only the customer's own durable records, without
insider knowledge of what you, the assessor, just did.
**Negative test:** N/A — the reconstruction attempt above is itself the
test; a failed reconstruction is the negative result.

**Pass criteria:** a decision-scoped, durable, reconstructable record
exists and was demonstrated to answer the who/what/authority/outcome
question from records alone.
**Fail criteria:** reconstruction requires ephemeral logs, requires
trusting the agent's own self-report, or durable records don't actually
cover authorization decisions (only generic application logs).
**Not Applicable criteria:** essentially never applies — every workflow
has *some* answer to "can this be reconstructed," even if the answer is
FAIL.
**Inconclusive criteria:** records could not be inspected.
**Confidence rules:** High requires an actual reconstruction attempt
performed and referenced by its specific records used — "the customer
says they log this" is not sufficient for High confidence.
**Required artifacts:** the actual reconstruction attempt's result,
referencing the specific records used.

---

## Checklist Items Not Yet Available (do not use these until validated)

Per Handbook §4 and `VALIDATION-REPORT.md` §4/§6, the following have
**no checklist item in this kit** because no validated test method
exists:

- Tool substitution (H9) — **FUTURE CAPABILITY**
- Scope/target-set expansion at a single hop, distinct from
  multi-hop attenuation (H7 in its full form) — **FUTURE CAPABILITY**
- Forged authority (fabricated approval/grant record via a side
  channel) — **FUTURE CAPABILITY**
- Attribution Durability as a standalone, target-framework-tested
  property (H8) — **FUTURE CAPABILITY**; use the Auditability checklist
  above for what *is* validated (reconstruction from durable records),
  which is a narrower, already-tested claim
- Concurrent/parallel multi-agent race-condition testing — **FUTURE
  CAPABILITY**
- Multi-tenant isolation — **FUTURE CAPABILITY**
- Tool/dependency supply-chain integrity — **FUTURE CAPABILITY**
- Prompt injection, jailbreak, and behavioral red-teaming — explicitly
  out of scope for the core assessment; **FUTURE CAPABILITY** as an
  add-on, no validated methodology exists yet

If a customer engagement's scope would benefit from any of the above,
say so plainly in the proposal/SOW conversation and either descope it
explicitly or flag it as custom, unvalidated work priced and delivered
with corresponding caveats — never silently test using an improvised
method and report it with the same confidence as a validated checklist
item.
