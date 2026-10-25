# Crypton Assessor Training Requirements v1.0

**Who uses this:** anyone onboarding a new assessor; the new assessor
themselves as a self-checkable curriculum.
**When:** before that person's first unsupervised customer engagement.
**Why it exists:** the mission's own success criterion for this entire
kit is that "a competent security engineer, with no prior exposure to
Crypton, can successfully execute a complete customer assessment using
only this kit" — this document is the specific, checkable path from
"has never seen Crypton" to "ready to run an engagement unsupervised."

---

## Module 1 — Foundations (reading, no exercises)

**Read, in order:** `01-assessor-handbook.md`, `02-standard-operating-procedure.md`,
`03-assessment-checklists.md`, `06-evidence-standard.md`,
`08-scoring-model.md`.

**Checkpoint (self-assessable):** can you explain, without re-reading,
why "INCONCLUSIVE" is not a synonym for failure, why severity is never
assigned from a category-to-severity table, and why a Crypton-reference
demonstration never substitutes for customer evidence? If any of these
three feels shaky, re-read Handbook §2, §9, §10 before proceeding.

## Module 2 — Context (reading, no exercises)

**Read:** `07-finding-library.md`, `VALIDATION-REPORT.md` in full.

**Checkpoint:** can you name, for each of the three active Finding
Library entries (FL-02 through FL-04), the one-sentence mechanism (not
just the title)? Can you explain why `VALIDATION-REPORT.md` classified
EXP-001's original finding as SUPERSEDED rather than keeping it in the
library, *and* why FL-01 (finding-001d/e) was separately retired on
2026-09-27 as `NO VULNERABILITY — EXPECTED FRAMEWORK BEHAVIOR` even
though it had been re-validated as CONFIRMED just weeks earlier? The two
retirements are for different reasons — one is "not representative of a
real target," the other is "we never asked who caused the mutation, what
trust level they held, or what boundary the framework actually claims"
(`01-assessor-handbook.md` §9a). If you cannot answer both, you have not
yet internalized why Crypton insists on re-validating its own
conclusions — re-read `VALIDATION-REPORT.md` §1 and
`labs/experiments/001-langgraph-approval-drift/CLOSURE.md` before
proceeding, since this discipline is load-bearing for every future
finding you'll write.

## Module 3 — Customer-Facing Materials (reading, no exercises)

**Read:** `04-customer-discovery-kit.md`, `05-assessment-worksheets.md`,
`09-customer-deliverable-templates.md`, `docs/business/ASSESSMENT_PACK.md`
in full (this kit's Handbook and SOP reference it constantly; read the
source, not just the citations).

**Checkpoint:** given a one-paragraph description of a hypothetical
agent workflow, can you draft the Architecture Questionnaire's first
five answers as if you were the customer, then identify which of the
Discovery Kit's "red flags" would apply? This exercises reading
comprehension of the questionnaires' actual purpose, not just their
existence.

## Module 4 — Training Run (hands-on, supervised or self-directed)

**Exercise:** run SOP Stages 2-9 (Architecture Review through
Target-Specific Tests) against one of two approved training targets:

- **Option A — Crypton's own reference implementation**
  (`labs/agent-lab/`), which has a known-answer set (its own test
  suite, `docs/ASSESSMENT_METHODOLOGY.md`'s "A note on Crypton's own
  test suite" section) you can check your conclusions against
  afterward.
- **Option B — a validated corpus target** (any of EXP-003A/B/C's
  frameworks, or the OpenHands SDK commit reviewed in EXP-005), where
  the existing writeups serve as the known-answer set.

**Rule, stated explicitly because it matters:** this training run's
output must never be represented to a customer, or reused verbatim in a
real report, as if it were customer-specific evidence — per Handbook §9,
this is training material and Crypton-reference/research-corpus
material, not a live engagement.

**Checkpoint:** complete at least the Identity, Authority, and one
Pillar-specific checklist (`03-assessment-checklists.md`) against the
chosen training target, fill the corresponding worksheets
(`05-assessment-worksheets.md`), and compare your PASS/FAIL/INCONCLUSIVE/NOT_APPLICABLE
conclusions and evidence citations against the known-answer set. A
trainee whose conclusions materially diverge from the known answers
should review Module 1-2 before proceeding, not simply move on.

## Module 5 — Severity Calibration (hands-on, requires a second reviewer)

**Exercise:** given 3-5 of the validated corpus's real FAIL-shaped
results (FL-02 through FL-04, or EXP-003C's H1c), independently apply
the seven-factor severity rubric and write out your reasoning, before
looking at the existing severity framing in
`07-finding-library.md`/`VALIDATION-REPORT.md`.

**Checkpoint:** a second person (the training supervisor, or — for a
solo trainee — a deliberate re-attempt after 24+ hours, comparing
against the library's stated severity guidance) reviews whether the
trainee's independent reasoning lands in the same range and, more
importantly, whether the *reasoning* (not just the label) reflects all
seven factors, not a subset. This mirrors the Handbook §9 quality gate
("a second assessor should be able to reproduce the same severity from
the same evidence") applied to training rather than a live engagement.

## Module 6 — Escalation & Communication Drill

**Exercise:** role-play (with a supervisor, a peer, or by writing out
both sides yourself) the two escalation scenarios in Handbook §14 most
likely to occur in a real engagement: a customer disputing a severity
rating in the remediation walkthrough, and discovering mid-engagement
that a majority of an area's tests are heading toward INCONCLUSIVE due
to access limits.

**Checkpoint:** can you articulate, out loud or in writing, the
difference between "revise the finding because new evidence surfaced"
and "hold the line because it's a risk-tolerance disagreement, not new
evidence" (Handbook §14)? Conflating these two is the single most
likely real-world mistake a new assessor makes under social pressure in
a live customer room.

## Completion Criteria

A trainee is ready for their first supervised (not yet unsupervised)
real engagement when all six modules' checkpoints are satisfied. A
trainee is ready for their first **unsupervised** engagement after
completing at least one supervised real engagement, per standard
professional-services practice — this kit does not by itself certify
readiness for a live customer without at least one shadowed run,
regardless of how well the training-run checkpoints go, because no
training target fully substitutes for real customer ambiguity
(withheld access, evasive answers, genuine architectural surprises)
that only a live engagement exercises.

## What This Training Does Not Cover

**FUTURE CAPABILITY, not yet part of this curriculum:** training for
the out-of-scope add-on disciplines (prompt injection/red-teaming
testing), training for non-Python target adaptation (no validated
example exists yet to train against), and training for multi-assessor
coordination (no validated procedure exists yet — see
`13-gap-analysis.md`). Do not represent this curriculum as covering
these; a trainee assigned to one of these situations needs additional,
currently-undefined preparation, and that gap should be surfaced to
leadership before the engagement is booked, not discovered mid-engagement.
