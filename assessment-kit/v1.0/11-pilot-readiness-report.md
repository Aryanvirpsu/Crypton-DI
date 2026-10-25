# Crypton Pilot Readiness Report v1.0

**Who uses this:** Crypton's founder/leadership, to decide whether to
book and deliver a first real, paid customer engagement using this kit.
**When:** now, at kit v1.0 completion (EXP-006), and again before any
future kit revision changes the answer.
**Why it exists:** answers the specific question `VALIDATION-REPORT.md`
§8 raises but doesn't itself resolve — not "is the research mature" but
"should Crypton actually go sell and deliver an engagement with what
exists today."

---

## Can a complete assessment be performed?

**Yes, with the scope and limitations stated below — this is the
central finding of this report.** Every stage of the SOP
(`02-standard-operating-procedure.md`) has a defined procedure, a
defined artifact, and a defined completion criterion, sourced from
either code-enforced schema (`assessment/models.py`), a previously
field-mechanics-tested rendering pipeline
(`assessment-engagement/reference-agentlab-dryrun/`), or validated
research evidence (`VALIDATION-REPORT.md`). A complete engagement —
intake through report delivery and remediation walkthrough — can be run
today using only the documents in this kit, for a customer whose
architecture resembles the validated corpus (Python, one of the four
studied frameworks or a structurally similar one, real staging access
available).

**What "complete" does not mean:** it does not mean every pillar will
receive a high-confidence rating on every engagement (`08-scoring-model.md`'s
"Insufficient Evidence for Rating" and NOT_APPLICABLE outcomes are
legitimate, expected results, not failures of the process); it does not
mean the engagement will find findings at the same rate as the research
corpus (a real customer's system, freshly reviewed, may be cleaner or
messier than any of the five research targets); and it does not mean
every one of the eleven threat objectives will be tested (Threat
Coverage, Handbook §7, is reported honestly per engagement, never
implied complete).

## What assumptions remain?

1. **The customer's target resembles the validated corpus's shape** —
   Python, an agent framework (ideally one of the four studied), real
   action-taking, some combination of identity/authority/delegation/
   approval/persistence/audit mechanisms actually present to test. A
   customer whose system is radically different (a different language
   ecosystem, no framework at all, a closed-source vendor product with
   no source access) is real, uncosted adaptation work — see
   `VALIDATION-REPORT.md` §3, "target diversity."
2. **Staging access will actually be granted and will actually work**
   — every stage past Architecture Review depends on this; the SOP's
   Stage 1 gate (confirm access works before scheduling Day 1) is the
   operational control for this assumption, not yet tested against a
   real customer's actual staging environment.
3. **A single assessor can deliver the full 3-day engagement** — every
   effort estimate in the SOP assumes this; untested for a
   multi-assessor team (Internal QA, Persona 4).
4. **The customer's technical point of contact is available and
   forthcoming** — the Discovery Kit's interview guides assume genuine
   cooperation; a customer providing minimal or evasive answers changes
   the engagement's evidence profile toward more INCONCLUSIVE results,
   which the kit handles honestly but which changes the deliverable's
   value.

## What skills are required?

- Source-code reading fluency in the customer's actual implementation
  language (Python at minimum, validated; others untested).
- Working knowledge of at least one of the four validated target
  frameworks' authorization/delegation/persistence primitives, or the
  ability to learn a new framework's primitives quickly using the
  Framework Adaptation Guide's stated procedure
  (`docs/methodology/04-framework-adaptation-guide.md`).
- Comfort running a controlled staging test against a live (non-production)
  system — constructing a specific request, observing the actual result,
  not just reading documentation.
- Severity judgment calibrated to the seven-factor rubric — this is a
  trained skill, not an automatic one; a first-time user of the rubric
  should have their severity calls reviewed by someone else until
  calibrated (see Training Requirements,
  `14-training-requirements.md`).
- Customer-facing communication discipline — stating uncertainty
  plainly, not overclaiming, handling live pushback per Handbook §14 —
  this is as much a required skill as the technical work.

## What tooling is required?

Per `docs/business/ASSESSMENT_PACK.md` §F's standing rule ("start
manual... do not build an automated scanner"), tooling requirements are
deliberately minimal:

- A terminal, source access (read-only is sufficient for most stages),
  and standard traffic-capture tools (Burp/Caido or equivalent) for
  staging tests.
- A way to compute integrity hashes for evidence artifacts (a script or
  standard utility — not yet standardized, see Internal QA Persona 1).
- Access to `labs/agent-lab/` for Crypton-reference demonstrations
  (teaching aid only, never customer evidence).
- No automated scanner. No custom Crypton product is required to
  deliver an engagement — the kit's own deliverables (worksheets,
  checklists, templates) are the tooling.

## What training is required?

See `14-training-requirements.md` for the full curriculum. Summary: a
new assessor needs (1) a full read of this kit in stage order, (2) a
shadowed or simulated run-through of the SOP against a non-customer
target (Crypton's own Agent Lab reference implementation, or a
validated-corpus target like OpenHands SDK, used as a training exercise
— never represented to a trainee as "this is how a customer engagement
generates findings," since those targets already have known answers),
and (3) at least one severity-rubric calibration exercise reviewed by
someone else before running one unsupervised on a real engagement.

## What parts remain research?

Per `VALIDATION-REPORT.md` §7's freeze recommendation:

- H7 (Scope/Target Expansion), H8 (Attribution Durability), H9 (Tool
  Substitution) — defined, never run. **FUTURE CAPABILITY.**
- H10 (Signal Independence) and the delegation-gate-delivery checklist
  item — single-target evidenced, EVOLVING, usable in an engagement
  today but not yet a field-proven universal expectation.
- A static-review-only substitute for Baseline Verification (SOP Stage
  7) — identified as a gap, not yet designed.
- Multi-tenant isolation, tool supply-chain integrity, concurrent/
  parallel multi-agent race conditions — no pillar, checklist, or test
  method exists yet. **FUTURE CAPABILITY.**
- Prompt injection / jailbreak / behavioral red-teaming add-on
  methodology — explicitly out of scope, no validated methodology
  exists. **FUTURE CAPABILITY.**

## What parts are operational?

- The four-outcome evidence model, the eight operational areas, the
  seven-factor severity rubric — **FROZEN**, code-enforced or
  process-proven across five experiments.
- The 13-stage SOP, the seven-pillar checklists (STABLE items), the
  Discovery Kit, the Worksheets, the Evidence Standard, the Finding
  Library (four entries), the Scoring Model, and the customer
  deliverable templates — all real, usable today, built directly from
  validated work, with every unvalidated extension explicitly labeled.
- The legal/commercial machinery (Authorization-to-Test, SOW, pricing)
  — already exists in `docs/business/ASSESSMENT_PACK.md`, unchanged by
  this kit, itself still marked "not yet field-tested against an
  external customer" by its own status line.

## GO / NO-GO Recommendation

# **GO — with explicit, disclosed scope limits, not a blanket GO.**

**Rationale:** every stage of a real engagement has a defined,
evidence-grounded procedure; the evidentiary machinery is mature enough
to freeze; the research corpus, while narrow, has produced genuinely
novel, customer-relevant reusable findings (FL-02, FL-03/FL-04-class
patterns — FL-01 was retired 2026-09-27 as `NO VULNERABILITY`, see
`labs/experiments/001-langgraph-approval-drift/CLOSURE.md`) that a
customer's own team plausibly has not found themselves.
Delaying further in pursuit of a larger research corpus before ever
running a real engagement would optimize for research completeness over
the actual goal (`VALIDATION-REPORT.md`'s own standing rule: earn
evidence through real engagements, not endless pre-validation).

**The GO is conditioned on:**

1. **Scope the first engagement toward the validated corpus's shape**
   — a Python-based, one-of-the-four-frameworks (or structurally
   similar) target, with real staging access. Do not take a first
   engagement radically outside this shape without pricing in real
   adaptation time and disclosing the reduced confidence this implies.
2. **Disclose kit maturity to the first customer, plainly, as part of
   the SOW conversation** — this is Crypton's first real engagement
   using this kit; STABLE and EVOLVING checklist items should be
   labeled as such in the delivered report, not presented with the same
   confidence as FROZEN machinery. This is not a weakness to hide; per
   Handbook §2, a weaker honest result beats a stronger unverified one,
   and this discipline should extend to how Crypton describes its own
   readiness, not only its findings.
3. **Run the Internal QA five-persona check (`10-internal-qa-review.md`)
   against the assembled report before delivery**, for at minimum the
   first three engagements, until the templates are field-proven rather
   than merely internally reviewed.
4. **Track every open assumption from this report against what actually
   happens in the first engagement**, and feed the result into v1.1
   (`15-roadmap-v1.1.md`) — the first real engagement is itself the next
   piece of validation evidence Crypton's corpus needs.

**This is not an unconditional GO.** A first engagement significantly
outside the validated shape (non-Python, no framework, no staging
access, adversarial/uncooperative customer team) should be treated as a
**NO-GO for this kit as-is** — not because Crypton can't eventually
assess such a target, but because doing so today would mean delivering
against assumptions this report explicitly flags as untested, without
disclosing that to the customer, which the Handbook's own core
philosophy (§2) exists to prevent.
