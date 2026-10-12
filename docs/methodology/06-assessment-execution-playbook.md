# Assessment Execution Playbook v0.1

**EXP-004, deliverable 6.** The literal, day-by-day sequence an assessor
follows to run one engagement — customer or research lab — through the
[13-stage pipeline](03-assessment-methodology-v0.1.md). Where a step is
already fully specified elsewhere (legal terms, the intake
questionnaire, the severity rubric), this playbook points to that
document rather than restating it — this file is the **order of
operations**, not a second copy of the content.

This playbook assumes the customer-engagement shape defined in
`docs/business/ASSESSMENT_PACK.md` §B (3 testing days, 1 workflow, up
to 3 connected tools, 1 staging environment). A research-lab run
(EXP-00X, no customer) follows the same stage order with steps 0 and 9
(legal gate, remediation call) skipped entirely — see "Research-lab
variant" at the end.

---

## Step 0 — Before any paid work (customer engagements only)

**Blocking, no exceptions.** Per `docs/business/ASSESSMENT_PACK.md` §C:
1. Confirm the engaging entity (LLC) is in place.
2. Signed Authorization-to-Test on file for this specific engagement, naming the exact target systems/environments.
3. Confirmed the customer actually owns or is authorized to permit testing of the named target.

Do not proceed to Day 1 until all three are true. This is not a
formality — see `docs/business/ASSESSMENT_PACK.md` §C's rules-of-engagement
for the exact terms an Authorization-to-Test must include.

## Step 1 — Intake (before Day 1)

Send and collect `docs/business/ASSESSMENT_PACK.md` §D's intake
questionnaire. This produces the raw material for **Pipeline Stage 1
(Architecture Review)** before testing starts, so Day 1 begins with
understanding, not discovery.

Confirm before scheduling Day 1:
- Staging/test environment access actually works (not just "granted on paper").
- A technical point of contact is available for the full testing window.
- Scope is exactly one workflow, up to three connected tools (SOW default) — anything broader is a scope-change conversation, not something to silently absorb.

## Day 1 — Architecture through Threat Model (Pipeline Stages 1-6)

1. **Architecture Review** (Stage 1): confirm/expand on the intake
   questionnaire with the technical point of contact. Identify the
   framework/runtime (`docs/methodology/04-framework-adaptation-guide.md`
   tells you what's already known about OpenAI Agents SDK, Microsoft
   Agent Framework, and LangGraph specifically; a fourth framework
   starts from that guide's adaptation procedure).
2. **Identity Mapping** (Stage 2): walk the identity/ownership inventory.
3. **Authority Mapping** (Stage 3): walk the grant/policy representation and where each check actually sits.
4. **Trust Boundaries** (Stage 4): identify every point a claim gets trusted rather than re-verified — this is the stage most likely to surface something the customer's own team hasn't thought to name (EXP-003C's `update_state()` trust boundary is the concrete example of what this stage is for).
5. **Execution Graph** (Stage 5): trace the actual call path, proposed-to-executed, for the in-scope workflow's most consequential action(s).
6. **Threat Model** (Stage 6): using the [Threat Model](02-threat-model.md)'s eleven objectives, mark each IN SCOPE or NOT APPLICABLE for *this* target, with a one-line reason for each NOT APPLICABLE (mirroring how H3 was marked NOT_APPLICABLE for LangGraph — a sourced reason, not a shrug).

**End-of-day-1 artifact:** a trust-flow map (this becomes
`docs/business/ASSESSMENT_PACK.md` §E report §2) and a scoped threat
model — both reviewable by the customer's team before testing begins,
so Day 2 doesn't start from a disputed premise.

## Day 2 — Baselines through Framework-Specific Tests (Pipeline Stages 7-9)

1. **Baseline Verification** (Stage 7): Baseline A (approve → executes),
   Baseline B (reject → no-op), Baseline C (persist/reload/resume) if a
   persistence mechanism is in scope. **Do not proceed past a failing
   baseline** for that path — see the Methodology's stage-7 rule.
2. **Universal Hypotheses** (Stage 8): confirm which of H1-H9 apply
   given Day 1's threat model, and in what order they'll be run.
3. **Framework-Specific Tests** (Stage 9): run each in-scope hypothesis
   as a controlled staging test, source review, or document inspection
   per `docs/ASSESSMENT_METHODOLOGY.md`'s eight areas and procedure
   methods. Record results as they come in, not from memory at end of
   day — evidence collected same-day is more reliable than evidence
   reconstructed later.

**Operating rule carried from every EXP-00X lab:** stay entirely within
public APIs and the customer's authorized staging environment. Never
modify framework internals or the customer's production systems. If a
hypothesis can't be tested this way, the result is INCONCLUSIVE, not a
forced equivalent — and INCONCLUSIVE with a stated reason is a
legitimate, complete result, not a failure to finish.

## Day 3 — Evidence, Risk, Findings (Pipeline Stages 10-12)

1. **Evidence Collection** (Stage 10): finalize the evidence manifest —
   every PASS/FAIL finding has at least one `EvidenceReference`
   (structurally enforced by `Finding.__post_init__`).
2. **Risk Analysis** (Stage 11): for every FAIL, apply the seven-factor
   severity rubric (`docs/ASSESSMENT_METHODOLOGY.md`'s severity
   section) — reasoned, not templated.
3. **Findings** (Stage 12): write each finding through the [Evidence &
   Reporting Standard](05-evidence-reporting-standard.md)'s eleven-link
   chain. Any loose end from Day 1-2 (an INCONCLUSIVE that could
   actually be resolved with one more staging test, a NOT_APPLICABLE
   that needs a clearer sourced reason) gets closed out here, before
   writing the report — not left for the customer to notice.

**End-of-day-3 artifact:** a complete, internally-reviewed findings set,
ready for report assembly.

## Report assembly + delivery (Pipeline Stage 13, Remediation)

Assemble the report using the [Customer Report Template](07-customer-report-template.md).
**Recommend the control, not Crypton** — unchanged rule from
`docs/business/ASSESSMENT_PACK.md` §E. Deliver within the SOW's agreed
window, then hold the live remediation walkthrough (`docs/business/ASSESSMENT_PACK.md`
§B's "1 remediation discussion").

---

## Research-lab variant (EXP-00X, no customer)

Same 13 stages, same order, with these differences:

- **Step 0 and the report-assembly/remediation step are skipped entirely** — no legal gate needed against Crypton's own research environment; no customer report is produced (a README and a final-report chat response substitute for it).
- **Day 1-3 timing is illustrative, not a hard boundary** — a research lab runs stages 1-12 across as many sessions as the target needs (EXP-003C's actual work spanned multiple sessions), but the *order* (architecture → identity → authority → trust boundaries → execution graph → threat model → baselines → hypotheses → tests → evidence → risk → findings) has held across all three completed labs and should not be reordered without a specific reason.
- **Intake (Step 1) becomes `target.json` + version verification** — `pip index versions` (or the ecosystem equivalent) in place of a customer intake questionnaire; the standing rule to never trust a version number given in a prompt applies here specifically (both EXP-003A and EXP-003B independently verified their target's version rather than assuming the task prompt's number, and EXP-003B's assumption was in fact stale by one minor version).
- **"Risk" (Stage 11) is usually skipped or minimal** — research labs against well-documented open-source frameworks rarely produce a real FAIL requiring severity scoring; where they do (EXP-003C's H1c), the [Evidence & Reporting Standard](05-evidence-reporting-standard.md)'s worked example shows how to reason through it anyway.
