# Customer Report Template v0.1

> **HISTORICAL / SUPERSEDED (2026-09-30) for report *structure and outcome
> vocabulary*.** The canonical, current report definition is now
> `assessment-kit/v1.0/09-customer-deliverable-templates.md` (10 sections —
> see `docs/audits/2026-09-30-evidence-report-readiness-audit.md` §5.6/§11
> for the reconciliation of this document, `docs/business/ASSESSMENT_PACK.md`
> §E, and the kit document into one lineage). This document's eleven-link
> evidence-chain vocabulary (below) is still live and referenced by the
> canonical doc; only the *report section structure* and the
> PASS/FAIL/INCONCLUSIVE/NOT_APPLICABLE outcome labels used in this
> document's own template text are superseded — see
> `docs/ASSESSMENT_METHODOLOGY.md`'s five-outcome model.

**EXP-004, deliverable 7 — what clients ultimately receive.** This
extends `docs/business/ASSESSMENT_PACK.md` §E's existing 8-section
report skeleton (kept, not replaced) by expressing the Technical
Findings section (§3) through the [Evidence & Reporting Standard](05-evidence-reporting-standard.md)'s
eleven-link chain, and by grounding the Trust-Flow Map and Authorization
Map sections in the [Security Property Catalog](01-security-property-catalog.md)'s
seven pillars so a report reads as an application of Crypton's actual
methodology, not a bespoke document per engagement.

No engagement has used this exact template yet — `assessment-engagement/reference-agentlab-dryrun/`
exercises the underlying report-rendering code (`assessment/report.py`)
against Crypton's own reference implementation, which is a mechanics
proof, not a customer report. Treat this as v0.1: the intended shape,
not a field-tested one.

---

## Template

```markdown
# Agent Authorization Assessment — [Customer]
Prepared by [Entity] · [date] · Confidential

## 1. Executive Summary
- What we assessed (1 workflow, N tools, [framework name], staging).
- Top 3 things leadership needs to know — plain language, no jargon.
- Overall authority-flow risk posture, one paragraph.
- Which of Crypton's eleven threat objectives were in scope for this
  engagement, and which were NOT_APPLICABLE (one line each) — see
  Appendix A.

## 2. Trust-Flow Map
Diagram + narrative: how authority currently moves —
Human -> identity -> agent -> credentials -> tools -> resources.
Structured around the seven pillars (Identity, Authority, Delegation,
Human Approval, Execution, Persistence, Auditability) — mark, per
pillar, whether the mapped flow looks SUPPORTED, PARTIAL, or has a
visible gap. Mark where authority is ambiguous or over-broad.

## 3. Technical Findings
For each finding, all eleven links from the Evidence & Reporting
Standard, written as prose (not eleven bare headers):

- **Title**
- **Hypothesis** (which of H1-H9) and **Threat** (which attacker objective) tested
- **Test** — what was actually done (source review / controlled staging test / document inspection)
- **Evidence** — reference to the evidence appendix (§7)
- **Observed Behaviour** vs. **Expected Behaviour**
- **Security Property** — which pillar(s)
- **Status** — PASS / FAIL / INCONCLUSIVE / NOT_APPLICABLE
- **Risk** (FAIL only) — severity + the seven-factor reasoning, not just a label
- **Recommendation** — the control, never a vendor or Crypton's own product
- **Reproduction** — exact steps, referencing the evidence appendix
- **Confidence** — stated and reasoned, not assumed

Clean areas get a stated PASS with evidence, not silence — an area with
nothing wrong is still a reportable result (`docs/business/ASSESSMENT_PACK.md`'s
existing rule: "if a section is clean, say so").

## 4. Authorization Map
Table: Agent -> identity -> credentials -> tools -> resources -> where
the authorization check sits -> pre- or post-execution -> bypass paths
attempted and their result. This section is Pillar 2 (Authority) and
Pillar 5 (Execution) made concrete for this specific customer's system.

## 5. Delegation Findings (if multi-agent)
- The authority path across agents (Pillar 3).
- Where scope fails to narrow, or widens (H7 if scope/target expansion was tested; otherwise note it as a coverage gap for this engagement, not silently omitted).
- Whether the chain is reconstructable (Pillar 7) and whether revocation propagates (if live/production traffic was available to test it — otherwise NOT_APPLICABLE, stated as such, matching `docs/ASSESSMENT_METHODOLOGY.md` area 7's guidance).

## 6. Human-Approval Analysis
- What requires approval today vs. what should (Pillar 4).
- Is approval bound to the exact request (H1)? Can an approved action be modified after (H1/H5)? Is there a standing/broad approval, and if so what's its actual scope (H3)?
- Is a single approval consumed at most once, including across persistence/resume (H4)?

## 7. Evidence Appendix
Every `EvidenceReference` cited in §3, with source, timestamp, and
integrity hash where practical. HTTP/terminal captures, screenshots,
traces, source excerpts — whatever was actually collected, indexed so
a reader can trace any finding in §3 back to what was actually observed.

## 8. Remediation Summary & Priorities
Ranked fixes, effort vs. impact. Recommend the control, not a vendor —
unchanged rule.

## Appendix A. Threat Coverage
A table, one row per one of the eleven attacker objectives from the
Threat Model: IN SCOPE (with a pointer to the relevant §3 finding(s)) or
NOT APPLICABLE (with the one-line sourced reason). This is what makes
explicit, for the customer, exactly what this engagement did and did
not test — an engagement that tested six of eleven objectives should
say so plainly here, not imply full coverage by omission.
```

---

## Report rules (unchanged from `docs/business/ASSESSMENT_PACK.md` §E, restated for completeness)

- Recommend the *control*, not Crypton. Assessment credibility is the
  asset — don't spend it steering findings to a future Crypton product.
- No invented findings, no filler. If a section is clean, say so.
- Every claim has evidence in the appendix.
- **New with EXP-004:** every finding traces through all eleven links
  of the Evidence & Reporting Standard, and every engagement's Appendix
  A states its actual threat coverage rather than leaving scope
  implicit.

## Relationship to `docs/business/ASSESSMENT_PACK.md`

That document remains the sellable, legal, SOW-facing pack — one-pager,
SOW, intake questionnaire, rules of engagement. This template is
specifically §E's report skeleton, extended to use the vocabulary this
methodology set defines (pillars, H1-H9, the eleven-link evidence
chain, the eleven threat objectives). Nothing in `ASSESSMENT_PACK.md`
needs to change for this template to apply — §E's skeleton is a subset
of this one, not a conflicting one.
