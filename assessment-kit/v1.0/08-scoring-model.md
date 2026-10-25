# Crypton Maturity Scoring Model v1.0

**Who uses this:** the assessor, at Stage 12 (Findings), to produce the
maturity ratings that anchor the Executive Summary and Leadership
Summary (`09-customer-deliverable-templates.md`).
**When:** once per pillar, per engagement, after all in-scope checklist
items for that pillar have a final PASS/FAIL/INCONCLUSIVE/NOT_APPLICABLE
result.
**Why it exists:** gives leadership a plain-language maturity read
without inventing a numeric score the underlying evidence doesn't
support. **No numeric scores, no weighted formulas, no point systems
exist anywhere in this model, by design** — a number implies a
precision the seven-factor severity rubric and the four-outcome
evidence model were deliberately built to avoid
(`docs/ASSESSMENT_METHODOLOGY.md`'s severity rubric: "not as a point
score... a judgment call").

---

## The Five Ratings, Plus One Non-Rating

**Five substantive ratings** apply to a pillar whose underlying
property actually exists in the customer's architecture:

| Rating | What it means |
|---|---|
| **Excellent** | Every in-scope checklist item under this pillar reached PASS with High confidence, and no recognized coverage gap (per Handbook §7/`03-assessment-checklists.md`'s "Checklist Items Not Yet Available") leaves an obviously-relevant property untested for this specific architecture. |
| **Good** | Every in-scope, testable checklist item under this pillar reached PASS, but at least one of: some items are NOT_APPLICABLE for a legitimate, verified architectural reason (not simply unexamined); confidence on at least one PASS is Moderate rather than High; a real, currently-EVOLVING checklist item (e.g. FL-03/FL-04-shaped patterns) was checked and found clean but is not yet a fully field-proven test. |
| **Adequate** | At least one FAIL exists under this pillar, but every FAIL is LOW or INFORMATIONAL severity, or exactly one MEDIUM-severity FAIL exists with a documented, real compensating control. |
| **Needs Improvement** | At least one MEDIUM-severity FAIL with no meaningful compensating control, or any HIGH/CRITICAL-severity FAIL, or a pattern of multiple LOW-severity FAILs under one pillar that, taken together, indicate a systemic gap rather than isolated issues. |
| **Insufficient Evidence for Rating** | A majority of this pillar's in-scope checklist items ended INCONCLUSIVE. **This is not a negative rating and must never be presented or read as one** — it states that Crypton could not obtain enough evidence to rate this pillar in this engagement, for a specific, stated reason (access, environment, documentation). See "When to Withhold a Rating," below. |

**One non-rating — Not Applicable:** used when the pillar's underlying
property does not exist in this architecture at all (no delegation, no
persistence, no human-approval step), mirroring the finding-level
NOT_APPLICABLE outcome. **This is not a sixth point on the same scale as
the five ratings above** — it is a statement that the scale doesn't
apply here, the same distinction the four-outcome finding model already
draws between FAIL (evidenced gap) and NOT_APPLICABLE (nothing to
evaluate). Never let "Not Applicable" visually read as better or worse
than "Adequate" in a report table — list it separately or footnote it,
not interleaved as if it were a rung on the same ladder.

**Assignment rule:** the rating for a pillar is set by its **worst
qualifying condition**, not an average of its checklist items — one
HIGH-severity FAIL makes the pillar "Needs Improvement" even if four
other properties under that pillar are clean PASSes. This mirrors the
severity rubric's own logic (a single serious gap is not offset by
unrelated things going well) and avoids the false precision a blended
score would imply.

---

## Per-Pillar Application

| Pillar | Primary checklist item(s) that drive the rating | Notes |
|---|---|---|
| Identity | Area 1 (Agent Identity & Ownership) | — |
| Authority | Areas 2, 4, 5, 7 (whichever are in scope) | Rate using the worst of the in-scope areas under this pillar — see "Known overlap" note below. |
| Delegation | Areas 2 (shared), 3; Delegation Gate Delivery (EVOLVING) if tested | If no delegation exists in the workflow, this pillar is rated **NOT_APPLICABLE for the engagement**, stated plainly — not scored as if delegation existed and passed. |
| Human Approval | Area 6; Self-Reported Risk Independence (EVOLVING) if tested | If no approval step exists in the workflow at all, rate NOT_APPLICABLE and separately note (per the area-6 methodology rule) that Authority-pillar controls are what governs the action instead — a missing approval step is not automatically a Human Approval FAIL if the architecture never claimed to have one. |
| Execution | Area 4 (shared with Authority — the pre-execution-boundary ordering specifically) | — |
| Persistence | Area 7 (shared with Authority) | If the workflow has no persistence/checkpointing at all, rate NOT_APPLICABLE. |
| Auditability | Area 8 | — |

**Known overlap note:** because Authority spans four operational areas
and shares boundaries with Execution and Persistence
(`VALIDATION-REPORT.md` §4), a single FAIL can lower more than one
pillar's rating simultaneously — this is expected and correct, not
double-counting. State the shared root cause once in the Technical
Findings section and reference it from every affected pillar's rating,
rather than writing it up as multiple separate findings.

---

## Overall Maturity

**Overall maturity is a narrative synthesis, never a computed roll-up
of the per-pillar ratings.** State it as a short paragraph, anchored by
two explicit rules:

1. **The weakest rated pillar with a real (non-INFORMATIONAL) FAIL sets
   the ceiling for overall maturity language.** A customer with six
   pillars at "Excellent" and one at "Needs Improvement" (HIGH-severity
   FAIL) has an overall posture that reads as "Needs Improvement,
   concentrated in [pillar] — the other six pillars are strong and not
   the concern here," never as "mostly excellent."
2. **Breadth matters as much as depth.** An engagement where three of
   seven pillars are "Insufficient Evidence for Rating" cannot honestly
   report an overall maturity rating at all — state instead: "Overall
   maturity cannot be responsibly summarized in one rating this
   engagement — see the per-pillar table; N of 7 pillars have
   sufficient evidence, M do not, for the reasons stated in each."

## When Uncertainty Affects Scoring

- **A Moderate-confidence PASS still counts as PASS**, but caps the
  pillar's rating at "Good," never "Excellent" — confidence is part of
  the rating input, not a footnote applied after the fact.
- **An INCONCLUSIVE result never lowers a rating** the way a FAIL does.
  INCONCLUSIVE means "we don't know," not "it's broken" — conflating the
  two would penalize a customer for, e.g., a legitimately narrow staging
  environment, which is exactly the kind of dishonest scoring this model
  exists to prevent.
- **A EVOLVING checklist item (H10, delegation-gate-delivery,
  self-reported-risk) contributes to a rating only as a positive or
  negative finding when it was actually tested** — its EVOLVING status
  does not exempt a real FAIL found through it from lowering the
  rating; it does mean the write-up must state plainly that this
  specific check is newer and less field-proven than the STABLE
  checklist items (per `03-assessment-checklists.md`'s confidence
  rules for each EVOLVING item).

## When to Withhold a Rating

Withhold a pillar's rating entirely (report "Insufficient Evidence for
Rating," not a guessed middle tier) when:

- A majority of that pillar's in-scope checklist items ended
  INCONCLUSIVE.
- The pillar's applicability itself could not be determined (e.g.
  whether delegation exists in the architecture at all could not be
  confirmed).
- Staging access was withdrawn or unreachable for a majority of the
  engagement's testing window, affecting this pillar's tests
  specifically.

**Never fill a withheld rating with a default ("Adequate," as a
middle-of-the-road guess) to avoid an awkward report section.** State
plainly why the rating is withheld, and what would be needed to
complete it (per the specific INCONCLUSIVE reasons already recorded in
Stage 9-10) — this is more useful to a customer than a false middle
rating, and it is the same discipline the entire kit applies to
individual findings, applied one level up.

## What This Scoring Model Is Not

- **Not a compliance score.** No mapping exists (or should be implied)
  between these five ratings and any regulatory framework
  (`01-assessor-handbook.md` §4 — Crypton does not deliver compliance
  attestation).
- **Not comparable across engagements as a league table.** A "Good"
  rating for one customer's Identity pillar and a "Good" rating for
  another's are not directly comparable without reading both
  underlying finding sets — the rating summarizes one engagement's
  evidence, it does not benchmark against an industry baseline Crypton
  has not established (per `VALIDATION-REPORT.md` §3's sample-size
  threat to validity).
- **Not permanent.** A rating reflects the system as reviewed at a
  point in time, against a specific pinned framework version
  (`02-standard-operating-procedure.md`'s version-verification rule).
  It should be re-earned, not assumed to persist, at any future
  re-assessment.
