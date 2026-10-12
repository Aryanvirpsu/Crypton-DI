# Crypton Assessment Kit — Gap Analysis v1.0

**Who uses this:** whoever prioritizes v1.1 work
(`15-roadmap-v1.1.md`) and whoever scopes a first engagement that falls
outside the validated shape.
**When:** now, as the consolidated list; re-run after the first real
engagement.
**Why it exists:** every gap named separately across
`VALIDATION-REPORT.md`, `01-assessor-handbook.md`,
`10-internal-qa-review.md`, and `11-pilot-readiness-report.md`,
collected in one place so nothing gets lost between documents.

## Methodology Gaps (technical — require research, not just writing)

| Gap | Source | Severity if unaddressed |
|---|---|---|
| Tool substitution (H9) never tested | `VALIDATION-REPORT.md` §4/§7 | A real bypass class could be missed entirely on any engagement |
| Scope/target-set expansion (H7) never tested | same | same |
| Forged authority never tested | `VALIDATION-REPORT.md` §1 (Threat Model coverage) | Untested attacker objective |
| Attribution Durability (H8) never tested as a target-framework property | same | Auditability claims may be incomplete |
| Concurrent/parallel multi-agent race conditions never tested despite a validated target (OpenHands) having real concurrent execution | `VALIDATION-REPORT.md` §4 | A real, plausible bug class with zero test coverage |
| Multi-tenant isolation has no pillar/area/test at all | `VALIDATION-REPORT.md` §4/§6 | Cannot assess a real, common enterprise concern |
| Tool/dependency supply-chain integrity has no pillar/area/test | same | same |
| Static-review-only Baseline Verification substitute undefined | `VALIDATION-REPORT.md` §6/§7, `02-standard-operating-procedure.md` Stage 7 | Engagements without live staging access have a real procedural hole |
| H10 (Signal Independence) and delegation-gate-delivery checklist item are single-target evidenced | `VALIDATION-REPORT.md` §2 | Real, usable today, but shouldn't be presented as field-proven |

## Corpus Diversity Gaps (research — require a second/third data point)

| Gap | Source |
|---|---|
| LangChain/LangGraph overlap between EXP-001 and EXP-003C — not five independent data points | `VALIDATION-REPORT.md` §3 |
| Zero non-Python targets | same |
| Zero enterprise/closed-source targets | same |
| Zero live-execution confirmation for EXP-005's two OpenHands findings | same |
| Zero targets outside the "well-documented, popular open-source" selection profile (selection-bias risk) | same |

## Operational Gaps (process/business — do not require new research)

| Gap | Source | Owner action needed |
|---|---|---|
| Re-test/re-assessment policy after remediation is undefined | `10-internal-qa-review.md` Persona 2, `11-pilot-readiness-report.md` | A pricing/business decision, not a methodology one — decide and document in `docs/business/ASSESSMENT_PACK.md`, not invented here |
| Multi-assessor engagement coordination has no procedure | `10-internal-qa-review.md` Persona 4, `11-pilot-readiness-report.md` | Write once Crypton actually runs a multi-assessor engagement — premature to define without evidence |
| Integrity-hash tooling/workflow unstandardized | `10-internal-qa-review.md` Persona 1 | Pick a utility, document the one-line command, low effort |
| No consolidated single "what to prepare" customer-facing checklist (currently correctly distributed across the Discovery Kit) | `10-internal-qa-review.md` Persona 4 | Revisit after the first real engagement shows whether distribution or consolidation serves customers better — do not pre-emptively consolidate without evidence it's needed |

## Coverage Gaps Deliberately Out of Scope (not gaps to close — documented boundaries)

| Item | Why it's a boundary, not a gap |
|---|---|
| Prompt injection / jailbreak / behavioral red-teaming | Deliberate scope exclusion from the lead assessment, sold as a separate, currently-undefined add-on — `docs/ASSESSMENT_METHODOLOGY.md`'s own stated boundary, not something this kit should absorb |
| Compliance attestation / audit certification | Crypton delivers security assessments, not certified audits — `docs/business/ASSESSMENT_PACK.md` §B |
| Production environment testing, load/DoS, social engineering, physical security | Out of scope per SOW §B |

## Prioritization Guidance

Order gaps by **what a real, imminent customer engagement would
actually need**, not by research interest:

1. **First priority (operational, low effort, real risk if skipped):**
   integrity-hash tooling standardization, re-test policy decision.
2. **Second priority (methodology, needed once a second target exists):**
   promote H10/delegation-gate-delivery if a second target confirms
   them; define the static-review-only Baseline Verification substitute
   the first time an engagement actually needs it (don't build it
   speculatively — build it against a real need, consistent with
   Handbook §2's evidence-over-assumption philosophy).
3. **Third priority (research, no immediate customer pressure):** H7/H8/H9,
   corpus-diversity gaps, multi-tenant isolation, supply-chain
   integrity — genuinely valuable, genuinely not blocking a first
   engagement.

Do not let priority-3 items block a GO decision on a first engagement —
see `11-pilot-readiness-report.md`'s GO recommendation, which already
accounts for this prioritization.
