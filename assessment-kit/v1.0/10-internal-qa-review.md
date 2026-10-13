# Crypton Assessment Kit — Internal QA Review v1.0

**Who uses this:** the kit's maintainers (Deliverable 12), and anyone
onboarding who wants to see the reasoning behind decisions in files
01-09.
**When:** performed once, at kit assembly (this pass); repeat this
exercise before every future kit revision (v1.1+).
**Why it exists:** proof the kit was actually stress-tested against
five different reader perspectives before being called commit-ready,
not merely assembled and shipped.

**Method:** each persona below reviewed the completed kit
(`VALIDATION-REPORT.md`, `01`-`09`) looking specifically for its own
named failure modes. Every finding is either a **real issue, fixed** (with
the specific edit applied and cited) or a **considered issue, not
fixed** (with the reasoning for leaving it as-is). No finding was
recorded and then silently dropped.

---

## Persona 1 — Junior Assessor (never seen EXP-001-005, using the kit cold)

**Looking for:** missing guidance, ambiguous wording, places where the
kit assumes context it doesn't actually provide.

| Finding | Status | Detail |
|---|---|---|
| The kit's four-outcome model (`assessment/models.py`) has no fifth status, but `01-assessor-handbook.md` cited `CANDIDATE_FINDING_APPLICATION_LEVEL` as a "validated precedent" for extending outcomes, without saying how to actually encode that in a real report's `status` field. A junior assessor would not know whether to invent a new status string. | **Real issue, fixed** | `01-assessor-handbook.md` §9 rewritten to state explicitly: file as `status = FAIL` with the qualifier in prose/`risk_impact`, never a fifth status. `06-evidence-standard.md` field 11 updated to match. |
| `08-scoring-model.md`'s "Five Ratings" table didn't account for pillars where the underlying property doesn't exist at all (no delegation, no persistence) — a junior assessor filling the Risk Summary template's rating dropdown (which already listed "Not Applicable" as an option) would find no definition of what that means or how it differs from the five real ratings. | **Real issue, fixed** | `08-scoring-model.md` restructured into "The Five Ratings, Plus One Non-Rating," with an explicit definition of "Not Applicable" and a rule against treating it as a sixth rung on the same ladder. |
| The SOP's Stage 2 (Architecture Review) and Stage 3 (Identity Mapping) both touch identity-adjacent content — a junior assessor might wonder which stage actually owns filling the Identity Graph worksheet. | **Considered, not fixed** | The stages already have distinct "Artifacts produced" (Stage 2: architecture description; Stage 3: Identity Graph worksheet) — re-reading confirmed this is unambiguous once both stage definitions are read together, and splitting it further would add process overhead without a demonstrated real confusion. Flagged for the kit maintainer to watch for in the first real engagement's retrospective, not pre-emptively changed. |
| `06-evidence-standard.md` says to compute an integrity hash "where practical" for screenshots/recordings without saying how. | **Considered, not fixed** | This is a tooling question (what hash utility, what workflow), not a methodology question — appropriately deferred to `13-gap-analysis.md`'s operational-gaps section and `15-roadmap-v1.1.md` Tier 1 rather than answered here with an invented procedure. |

## Persona 2 — Senior Consultant (has run real client engagements before, elsewhere)

**Looking for:** weak decision rules, situations the kit doesn't
anticipate that come up constantly in real client work.

| Finding | Status | Detail |
|---|---|---|
| Nothing in the kit told an assessor what to do when a customer disputes a finding or severity rating live in the remediation walkthrough — a near-certain occurrence in real engagements. | **Real issue, fixed** | Added a full escalation rule to `01-assessor-handbook.md` §14: walk back to mechanism/evidence, revise if new evidence surfaces, hold the line with explanation (not silent capitulation) if it's a risk-tolerance disagreement rather than new evidence. |
| The Checklist's "Positive test"/"Negative test" split is occasionally a forced fit (Area 5 has no independent "positive" reading distinct from its bypass attempts). | **Considered, not fixed** | Already handled explicitly in `03-assessment-checklists.md` ("N/A — every attempt in this property IS the negative test") rather than forcing an artificial split — this is the correct resolution, not a gap. |
| Re-test/re-assessment policy after remediation (is a follow-up check included, priced separately, informal?) is referenced once in passing (`09-customer-deliverable-templates.md` §11, "any re-test offer") but never defined. | **Real issue, identified, deliberately not invented** | This is a pricing/business decision, not a methodology question — inventing an answer here would violate the "never invent capabilities" rule. Logged explicitly as an open question in `13-gap-analysis.md`. |

## Persona 3 — Skeptical Customer (paying, and reads everything with suspicion)

**Looking for:** poor customer experience, claims that sound better than
the evidence supports, anything that reads as marketing.

| Finding | Status | Detail |
|---|---|---|
| A customer reading `07-finding-library.md`'s "Known-Safe Patterns" table without the surrounding caveat could mistake it for "Crypton has certified these frameworks are secure." | **Considered, not fixed (already mitigated)** | The table is immediately followed by an explicit, bolded rule against exactly this reading ("Do not present these as guarantees about a customer's own system"). Re-read confirmed the mitigation is adequate; no further change made. |
| The Finding Library has only four entries — a skeptical customer could reasonably ask whether four findings represent enough experience to be worth paying for. | **Considered, not fixed (addressed elsewhere, correctly)** | `VALIDATION-REPORT.md` §3/§8 already states this limitation plainly and unprompted (sample-size threat to validity, READY WITH LIMITATIONS verdict) — duplicating that caveat inside the Finding Library itself would be redundant, not additive; the cross-reference is sufficient. |
| The Assessment Certificate template (`09-customer-deliverable-templates.md` §10) could be read by a compliance-focused customer as certifying more than it does, if skimmed. | **Considered, not fixed (already mitigated)** | The template's disclaimer sentence is bolded and placed immediately after the scope statement, before the signature block — re-read confirmed this ordering makes the disclaimer hard to skip, and duplicating the SOW's liability language into the certificate itself would be the "duplicated work" this review is also checking for (Persona 5). |

## Persona 4 — CTO (evaluating whether to buy, thinking about integration cost)

**Looking for:** operational bottlenecks, unclear resourcing, whether
this is actually deliverable with the team Crypton has.

| Finding | Status | Detail |
|---|---|---|
| The SOP's effort estimates assume a single lead assessor throughout — no guidance exists for what changes if more than one person works an engagement (parallelizing Stage 9's testing, for instance). | **Real issue, identified, deliberately not invented** | Crypton has run every validated experiment single-handedly; there is no evidence basis for multi-assessor coordination guidance. Logged as a training/tooling gap in `14-training-requirements.md` rather than speculatively answered. |
| A CTO would ask "what tooling does my team need to give your assessor access to" — this exists implicitly across the Discovery Kit's questionnaires but has no single consolidated list. | **Real issue, fixed conceptually via cross-reference, not a new document** | Rather than create an additional deliverable duplicating information already in `04-customer-discovery-kit.md` (staging access, technical POC availability) and `06-evidence-standard.md` (chain-of-custody/tooling), `13-gap-analysis.md`'s operational-gaps section consolidates this into one "customer-facing tooling/access checklist" pointer rather than inventing new content. |

## Persona 5 — Peer Reviewer (checking internal consistency and rigor)

**Looking for:** inconsistent terminology, duplicated work, places two
documents implicitly disagree.

| Finding | Status | Detail |
|---|---|---|
| `08-scoring-model.md`'s original "Five Ratings" section title was inconsistent with the "Not Applicable" option already present in `09-customer-deliverable-templates.md`'s Risk Summary template dropdown — a real cross-document inconsistency. | **Real issue, fixed** | Same fix as Persona 1's finding above — resolved once, benefits both. |
| Terminology check across all nine kit documents plus `VALIDATION-REPORT.md`: "operational area" / "area" / "assessment area" used to mean the same eight things (`AssessmentArea` enum); "pillar" used consistently for the seven-property teaching layer; "hypothesis" (H1-H10) used consistently; "finding" vs. "checklist item" vs. "property" used consistently (property = what a checklist tests; finding = the recorded conclusion). | **Reviewed, no fix needed** | Spot-checked terminology usage across all files during this pass; found consistent. |
| `02-standard-operating-procedure.md` Stage 9 was renamed "Target-Specific Tests" (from the source methodology's "Framework-Specific Tests") per `VALIDATION-REPORT.md`'s recommendation — confirmed this rename did not silently leave the phrase "Framework-Specific Tests" orphaned or contradicted elsewhere in the kit. | **Reviewed, no fix needed** | Checked; other kit documents refer to the *Framework Adaptation Guide* (a proper document name, correctly left unchanged) rather than the renamed stage, so no contradiction exists. |
| Duplicated-work check: does any deliverable restate content that another deliverable already owns, rather than cross-referencing it? | **Reviewed, no fix needed** | The one near-duplication found (severity rubric content appearing in both `01-assessor-handbook.md` §9 and `02-standard-operating-procedure.md` Stage 11) is intentional and consistent with the Handbook's stated role as the standing quick-reference — both point to the same source (`docs/ASSESSMENT_METHODOLOGY.md`) rather than diverging, so this is cross-reference redundancy (acceptable, expected in a handbook-plus-SOP pair), not duplicated *authorship*. |

---

## Summary of Revisions Applied This Pass

1. `01-assessor-handbook.md` §9 — clarified that extended outcome
   labels (e.g. "candidate finding") are FAIL-status qualifiers in the
   real customer schema, never a fifth base status.
2. `01-assessor-handbook.md` §14 — added an explicit escalation rule
   for a customer disputing a finding or severity rating.
3. `06-evidence-standard.md` field 11 — updated to match fix #1.
4. `08-scoring-model.md` — restructured the ratings section to
   explicitly define "Not Applicable" as a non-rating, distinct from
   the five substantive ratings.

## Issues Identified But Deliberately Not Fixed In This Pass

Logged forward into `13-gap-analysis.md` and `14-training-requirements.md`
rather than answered speculatively, per the "never invent capabilities"
rule:

- Re-test/re-assessment policy after remediation.
- Multi-assessor engagement coordination guidance.
- A consolidated, single customer-facing "what to prepare" checklist
  (currently correctly distributed across existing documents, but worth
  revisiting once real engagements show whether distribution or
  consolidation serves customers better).

No issue found by any of the five personas was dismissed without a
recorded reason. No finding resulted in inventing new methodology
content to paper over a gap — every "not fixed" resolution either points
to an existing, adequate mitigation, or forwards the open question to
the appropriate roadmap document.
