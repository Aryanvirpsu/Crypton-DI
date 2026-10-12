# Roadmap Toward Crypton Assessment Kit v1.1

**Who uses this:** whoever plans work after the first real
engagement(s) using Kit v1.0.
**When:** after the first pilot engagement, or after any new research
(a hypothetical future EXP-007+) produces evidence that changes a v1.0
conclusion.
**Why it exists:** v1.0 is deliberately scoped to what EXP-001-005
actually validated (`VALIDATION-REPORT.md`). This is the explicit,
prioritized list of what moves next, and — as important — what does
**not** move until real evidence justifies it.

**Standing rule for this roadmap, inherited from the whole kit's
philosophy:** an item moves from this roadmap into the core kit only
when real evidence (a second target, a real engagement's outcome, a
resolved business decision) justifies it — never on a schedule, and
never because "it's been a while."

---

## Tier 1 — Do immediately, low effort, no new evidence required

1. **Decide and document the re-test/re-assessment policy** in
   `docs/business/ASSESSMENT_PACK.md` — a business decision, not a
   research one (`13-gap-analysis.md`).
2. **Standardize the integrity-hash workflow** referenced in
   `06-evidence-standard.md` — pick a utility, document the one-line
   command (`13-gap-analysis.md`).

## Tier 2 — Do after the first real engagement (evidence-gated)

3. **Feed the first engagement's actual results back into this kit** —
   which checklist items produced real signal, which produced noise,
   which worksheets were awkward in practice, whether the effort
   estimates in `02-standard-operating-procedure.md` held. Run the same
   Internal QA five-persona exercise (`10-internal-qa-review.md`)
   against the *real* delivered report this time, not just the
   templates.
4. **Decide whether to consolidate the Discovery Kit's distributed
   "what to prepare" guidance into a single customer-facing checklist**,
   based on whether the first customer(s) actually struggled with the
   distributed version (`13-gap-analysis.md`).
5. **If a second target confirms H10 (Signal Independence) or the
   delegation-gate-delivery checklist item**, promote both from EVOLVING
   to STABLE in `03-assessment-checklists.md`, per the same 2-target
   promotion-gate discipline `VALIDATION-REPORT.md` §2 already applies
   consistently.
6. **If a second engagement needs multi-assessor coordination**, write
   the procedure then, from the real coordination problem encountered —
   not speculatively now.

## Tier 3 — Methodology-document edits (recommended, not yet executed)

7. **Split the Threat Model's outcome vocabulary into two explicit axes**
   (evidence-coverage vs. target-mitigation-strength) in
   `docs/methodology/02-threat-model.md`, per
   `VALIDATION-REPORT.md` §6, recommendation 4. Low risk, high clarity
   value, doesn't require new evidence — the only reason this is Tier 3
   rather than Tier 1 is that it edits a document outside
   `assessment-kit/`, and that edit should be reviewed/approved by
   whoever owns `docs/methodology/` before landing, per that document
   set's own "propose, don't unilaterally rewrite" discipline
   (`docs/methodology/README.md`'s existing pattern for exactly this
   kind of proposed-but-not-yet-merged change).
8. **Define a static-review-only variant of Baseline Verification**
   (SOP Stage 7) — per `13-gap-analysis.md`'s prioritization, build this
   the first time a real engagement actually needs it (no live staging
   access), not speculatively.

## Tier 4 — Research (requires new experimentation — out of scope for
whoever executes this roadmap without first explicitly re-opening
research work, consistent with this exercise's own "research is
complete, do not perform another experiment" boundary)

9. Tool substitution (H9), scope/target-set expansion (H7), forged
   authority, Attribution Durability (H8) as a target-framework
   property, concurrent/parallel multi-agent race conditions.
10. A genuinely independent fifth research target outside the
    LangChain/LangGraph ecosystem (`VALIDATION-REPORT.md` §3's
    named framework-diversity threat).
11. A non-Python target, to test whether the kit's framework-independent
    *shape* (pipeline, pillars, objectives) actually holds outside the
    ecosystem it was derived from, or whether that claim itself needs
    revisiting.
12. Multi-tenant isolation and tool/dependency supply-chain integrity as
    candidate new pillars/areas — currently have no property definition,
    no test method, no evidence; this is genuinely new research, not an
    extension of existing work.
13. Live-execution confirmation of EXP-005's two static-only findings
    (FINDING-005-1/FL-03, FINDING-005-2/FL-04) against a running
    OpenHands SDK instance.

## What Is Explicitly Not On This Roadmap

- **No plan to build an automated scanner** — `docs/business/ASSESSMENT_PACK.md`
  §F's rule stands; automate only a repeated manual step that provably
  cost real time across multiple real engagements, never speculatively.
- **No plan to expand into prompt injection/jailbreak testing** as part
  of the core kit — that remains a deliberately separate, currently
  undefined add-on, and building it is a distinct scoping decision, not
  a v1.1 line item.
- **No plan to change pricing or SOW terms** — those are
  `docs/business/ASSESSMENT_PACK.md`'s domain, informed by real
  engagement outcomes, not by this kit's technical roadmap.
