# Crypton Customer Deliverable Templates v1.0

**Who uses this:** the assessor, at SOP Stage 13 (Report Assembly).
**When:** after Stage 12's findings set is complete and internally
reviewed.
**Why it exists:** eleven ready-to-fill templates so report assembly is
template-filling against real findings, not composition from a blank
page under deadline pressure. Extends
`docs/business/ASSESSMENT_PACK.md` §E's 8-section skeleton and
`docs/methodology/07-customer-report-template.md`'s eleven-link
expansion of it — every rule in those two documents still applies;
nothing here supersedes them.

> **This is the canonical, current report definition (2026-09-30
> remediation pass).** `docs/business/ASSESSMENT_PACK.md` §E and
> `docs/methodology/07-customer-report-template.md` are marked HISTORICAL /
> SUPERSEDED for report *structure and outcome vocabulary* — their
> underlying rules (recommend the control not Crypton, the eleven-link
> evidence chain, the seven pillars) are still live and still apply here.
> Outcome labels throughout this document are the canonical five:
> **ENFORCED / VIOLATED / UNREPRESENTABLE / UNVERIFIABLE / NOT_APPLICABLE**
> (`docs/ASSESSMENT_METHODOLOGY.md`), not the four-outcome
> PASS/FAIL/INCONCLUSIVE/NOT_APPLICABLE vocabulary this document used
> before 2026-09-30.
>
> **Canonical report section order**, mapped against the audit-requested
> ten-section structure (`docs/audits/2026-09-30-evidence-report-readiness-audit.md`
> §4/§11) and the code renderer (`labs/agent-lab/src/agentlab/assessment/report.py`):
>
> | Audit's 10 canonical sections | This document's deliverables |
> |---|---|
> | 1. Decision Sheet | §1 Executive Summary + §9 Leadership Summary + §10 Assessment Certificate |
> | 2. Authority Map / Authority Diff | §2 Technical Report's Trust-Flow Map / Authorization Map content |
> | 3. Action Queue | §7 Remediation Roadmap |
> | 4. Findings Summary | §4 Risk Summary |
> | 5. Finding Cards | §5 Finding Detail |
> | 6. Verified Controls | §2 Technical Report's "clean" areas (see "no invented findings" rule above) |
> | 7. Limitations | carried through every template below — never a separate omitted section |
> | 8. Retest Record | §7 Remediation Roadmap's retest-gate rows |
> | 9. Evidence Index | §6 Evidence Appendix |
> | 10. Technical Appendix | §8 Engineering Appendix |
>
> The code renderer (`report.py`) uses the audit's exact 10-heading
> numbering; this document's 11-deliverable numbering is the human-authored,
> narrative-document equivalent for a report assembled by hand rather than
> rendered from a Finding/Evidence data set. Use whichever matches how a
> given engagement's report is actually being produced — they are the same
> canonical structure, not two competing ones.

**Standing rules across every template below** (repeated because they
matter more than any individual template's structure):
- Recommend the control, not Crypton.
- No invented findings, no filler — if a section is clean, say so.
- Every claim has evidence in the appendix.
- State threat coverage and maturity-rating withholdings explicitly —
  never imply completeness by omission.

---

## 1. Executive Summary

```markdown
# [Customer] — Agent Authorization & Delegation Assessment
## Executive Summary

**What we assessed:** [1 workflow, N tools, framework name, staging
environment] — tested [testing window].

**Top 3 things leadership needs to know:**
1. [Plain-language statement — no jargon]
2. [Plain-language statement]
3. [Plain-language statement]

**Overall authority-flow risk posture:** [one paragraph, plain
language — anchored by the Overall Maturity rule in
`08-scoring-model.md`: state the weakest pillar with a real finding
first, then the broader context]

**Threat coverage:** [N] of 11 attacker objectives were IN SCOPE and
tested this engagement; [M] were NOT APPLICABLE to this architecture
(reason stated); [K] remain untested — see Appendix A.

**What happens next:** [remediation walkthrough date/status, or "to be
scheduled"]
```

---

## 2. Technical Report

The full report — assembles sections 3-8 below plus the Architecture
Review and this Executive Summary into one document, per
`docs/methodology/07-customer-report-template.md`'s template structure.
No separate template body here; see that document for the exact section
order (Executive Summary → Trust-Flow Map → Technical Findings →
Authorization Map → Delegation Findings → Human-Approval Analysis →
Evidence Appendix → Remediation Summary → Appendix A: Threat Coverage).

---

## 3. Architecture Review

```markdown
## Architecture Review

**Workflow:** [one-paragraph description, from Architecture
Questionnaire A1]
**Framework/runtime:** [name + exact verified version]
**Topology:** [single-agent / multi-agent, described]
**In-scope tools:** [list, from Tool Graph worksheet]
**Persistence:** [described, or "none"]

**Trust-Flow Map:**
[diagram + narrative — Human → identity → agent → credentials → tools →
resources, per pillar status table below]

| Pillar | Status |
|---|---|
| Identity | SUPPORTED / PARTIAL / GAP |
| Authority | SUPPORTED / PARTIAL / GAP |
| Delegation | SUPPORTED / PARTIAL / GAP / NOT APPLICABLE |
| Human Approval | SUPPORTED / PARTIAL / GAP / NOT APPLICABLE |
| Execution | SUPPORTED / PARTIAL / GAP |
| Persistence | SUPPORTED / PARTIAL / GAP / NOT APPLICABLE |
| Auditability | SUPPORTED / PARTIAL / GAP |
```

Fill directly from the Identity, Authority, Delegation, and Trust
Boundaries worksheets (`05-assessment-worksheets.md`).

---

## 4. Risk Summary

```markdown
## Risk Summary

| Finding ID | Title | Severity | Pillar(s) | Status |
|---|---|---|---|---|
| AD-01 | ... | CRITICAL/HIGH/MEDIUM/LOW/INFORMATIONAL | ... | VIOLATED |
| ... | | | | |

**Maturity ratings** (per `08-scoring-model.md` — never a numeric
score):

| Pillar | Rating |
|---|---|
| Identity | Excellent / Good / Adequate / Needs Improvement / Insufficient Evidence for Rating / Not Applicable |
| Authority | ... |
| Delegation | ... |
| Human Approval | ... |
| Execution | ... |
| Persistence | ... |
| Auditability | ... |

**Overall maturity:** [narrative synthesis per `08-scoring-model.md` —
never an average]
```

---

## 5. Finding Detail

One instance of this template per finding, using the full sixteen-field
package (`06-evidence-standard.md`):

```markdown
### [Finding ID] — [Title]

**Outcome:** ENFORCED / VIOLATED / UNREPRESENTABLE / UNVERIFIABLE / NOT_APPLICABLE
**Severity:** [VIOLATED only — CRITICAL/HIGH/MEDIUM/LOW/INFORMATIONAL, with
seven-factor reasoning]
**Property ID:** [stable property ID — see `08-exp003-property-id-mapping.md`
for why a local H#/area label alone is not cross-engagement comparable]
**Hypothesis:** [H#/area, local to this engagement/experiment]
**Threat:** [attacker objective]
**Security Property:** [pillar(s)]
**Architecture Context:** [which worksheet, which component/action]

**Test:** [method — source review / controlled staging test / document
inspection]
**Observed Behaviour:** [fact]
**Expected Behaviour:** [what the property requires]
**Root Cause:** [the specific mechanism]
**Gate 7 (security-claim / trust-boundary triage):** [for VIOLATED only —
name the actor/code that caused the observed behavior, its trust level,
whether it is permitted by the system's own design, what boundary the
system actually claims, and whether that specific boundary was crossed.
Evidence sufficiency (the other six gates) is never enough on its own —
see `docs/ASSESSMENT_METHODOLOGY.md`'s "seven evidence-sufficiency gates."]
**Impact:** [concrete, plain-language consequence — VIOLATED only]

**Recommendation:** [the control, never Crypton or a vendor]
**Reproduction:** [exact steps, or "not independently reproducible —
see Confidence"]
**Confidence:** [reasoned statement per `01-assessor-handbook.md` §10]

**Supporting Artifacts:** [list, each with a Chain of Custody entry —
see Evidence Appendix]
```

---

## 6. Evidence Appendix

```markdown
## Evidence Appendix

| Evidence ID | Type | Source | Description | Collected by | Timestamp | Integrity hash |
|---|---|---|---|---|---|---|
| EV-001 | source_excerpt | [file:line] | [what it shows] | [assessor] | [ISO 8601] | sha256:... |
| ... | | | | | | |
```

Every `Evidence ID` cited in any Finding Detail (§5) must appear here.
No evidence entry exists that isn't cited by at least one finding — an
uncited evidence entry is dead weight, not thoroughness.

---

## 7. Remediation Roadmap

```markdown
## Remediation Roadmap

| Priority | Finding | Recommended Control | Effort | Impact if fixed |
|---|---|---|---|---|
| 1 | [id] | [control, not vendor] | Low/Medium/High | [plain language] |
| ... | | | | |

**Sequencing rationale:** [why this order — typically severity first,
then effort-vs-impact for ties]
```

Ranked fixes, effort vs. impact, per `docs/business/ASSESSMENT_PACK.md`
§E's existing rule — this template only adds the explicit two-column
effort/impact structure for scannability.

---

## 8. Engineering Appendix

```markdown
## Engineering Appendix

**For the technical team implementing remediation.**

Per finding requiring a fix:
- [Finding ID]: exact code/config location, suggested implementation
  approach (control-first, framework-agnostic where possible; the
  specific mechanism where the finding is framework-specific), and a
  test the engineering team can run themselves to confirm the fix
  (mirrors the finding's own Reproduction steps, inverted to confirm
  ENFORCED instead of the original VIOLATED — this is exactly what a
  Retest Record entry captures, see `docs/ASSESSMENT_METHODOLOGY.md`).

**Open questions for engineering:** [anything the assessor could not
resolve with the evidence available and needs the customer's own team
to answer — e.g. "does anything else in your stack rely on the current
[behavior]? Removing it may have side effects we couldn't observe from
outside."]
```

---

## 9. Leadership Summary

```markdown
# [Customer] — Leadership Summary

One page. No jargon, no severity labels without plain-language
translation, no evidence appendix references.

**In one sentence:** [what was tested and why it matters to the
business]

**What we found:** [2-4 bullets, plain language, ordered by real-world
impact not technical severity label]

**What this means for the business:** [risk in terms of the Risk
Inventory questionnaire's own framing — financial, safety, regulatory,
reputational — matched to what the customer told us mattered, from
`04-customer-discovery-kit.md` §G]

**What we recommend, in order:** [top 3 remediation priorities,
plain language]

**What we could not assess, and why:** [any Insufficient-Evidence
pillars, stated plainly, not buried]
```

---

## 10. Assessment Certificate

**Explicit constraint, restated because getting this wrong would
violate a standing rule:** Crypton does not provide compliance
attestation or audit sign-off (`docs/business/ASSESSMENT_PACK.md` §B).
This certificate documents that an assessment was **performed**, to a
stated scope, on a stated date — it is never worded as a certification
that a system **is secure**, meets a compliance standard, or has passed
an audit.

```markdown
# Certificate of Assessment Completion

This certifies that [Entity name] performed an Agent Authorization &
Delegation Assessment for [Customer name], covering:

- **Workflow:** [name/description]
- **Scope:** [1 workflow, N tools, framework, staging environment]
- **Testing window:** [dates]
- **Methodology:** Crypton Assessment Kit v1.0, itself built on
  `docs/ASSESSMENT_METHODOLOGY.md` v0.6.1 and
  `docs/methodology/` (EXP-004/EXP-006)
- **Areas assessed:** [list from the eight operational areas actually
  in scope]

**This certificate confirms that an assessment took place and its
scope. It is not a compliance attestation, a security guarantee, a
penetration-test clearance, or a certified audit.** Findings and their
severity are detailed in the accompanying Technical Report; a system's
overall security posture depends on factors outside this assessment's
scope (see "Out of Scope," `01-assessor-handbook.md` §4) and can change
at any time after this assessment's testing window closes.

Issued: [date]
[Entity name] · [signatory]
```

---

## 11. Management Presentation

```markdown
# [Customer] — Assessment Results
## Slide/section outline for the remediation walkthrough

1. **What we tested** — 1 slide, matches Executive Summary's "What we
   assessed."
2. **Top findings, plain language** — 1 slide per CRITICAL/HIGH finding,
   using the Leadership Summary's framing, not raw severity labels.
3. **Maturity by pillar** — 1 slide, the seven-pillar table from Risk
   Summary (§4), ratings only, no raw finding IDs.
4. **What "good" looks like** — 1 slide, drawing on the Finding
   Library's Known-Safe Patterns (`07-finding-library.md`) where
   relevant to this customer's architecture, to make the ask concrete.
5. **Remediation roadmap** — 1 slide, top 3-5 items from §7, with
   effort/impact.
6. **What we could not assess, and why** — 1 slide, if applicable;
   omit only if every pillar received a real rating.
7. **Next steps** — 1 slide: remediation timeline expectations, any
   re-test offer, contact for follow-up questions.

**Presentation rules:** no finding ID jargon on slides shown to a
non-technical audience (keep IDs in the leave-behind Technical Report);
never promise a re-test finds nothing, only that Crypton will apply the
same evidence standard again.
```
