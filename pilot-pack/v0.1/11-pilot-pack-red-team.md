# Pilot Pack Red Team

Five personas attack the offer as it now stands (`00-README.md` through
`10-pre-engagement-checklist.md`). For each objection: whether the pack
already answers it, what was changed because of it, and what's still
unresolved. The point of this document is to say what still sucks, not
to declare victory.

---

## 1. CISO — "Why should I trust you?"

**Objection, sharpened:** everything in `07-why-trust-crypton.md` is
about methodology discipline and honesty. None of it is a track record.
Every piece of evidence in `04-evidence-map.md` is either (a) something a
different company found, that Crypton merely read and mapped, or (b)
Crypton's own research against open-source frameworks and its own
reference implementation — never against a real, unfamiliar, paying
customer's production-adjacent system. Why should a CISO extend the
first real trust?

**Does the pack already answer this?** Better than it did before this
review, still not completely. It doesn't claim a track record it doesn't
have — `08-mvp-definition.md` and `11-pilot-readiness-report.md`
(referenced) both say plainly that zero customer engagements have
happened. Two things now reduce what a CISO has to take on faith beyond
that: the default engagement mode requires no credentials, production
access, or source at all (Mode 1, `06-access-and-safety.md`); and the
Crypton Validation Benchmark (`labs/validation-benchmark/v0.1/`) is
real, checkable evidence — now **two independent cases**, different
vendor, different language, different property each — that Crypton's
tests correctly tell a documented vulnerable dependency state from its
documented fixed state, on systems Crypton didn't build them to fit.
That's a real answer to "why should I trust your methodology at all,"
and a partial answer to "does it generalize" (see the Skeptical
Researcher / Track A note below for why two is still not many); it still
isn't an answer to "why should I trust you with *my* system specifically
before anyone else has," which is a fair question to leave open.

**Change made:** built the Crypton Validation Benchmark v0.1 to two
cases — VB-001 (MCP Python SDK / CVE-2026-52869) and VB-002
(`@bytebase/dbhub`'s SQLite connector / CVE-2026-61788) — specifically to
answer the "it's all circular" objection with something stronger than
one self-contained example. Did not fabricate a customer track record —
that objection's core still stands.

**Unresolved:** this is real and stays real until a first engagement is
delivered. The honest position, stated plainly rather than argued around:
*trust the boundedness of what's being asked, not a resume.* A CISO who
needs a track record before a first pilot, specifically, is asking for
something this pack cannot honestly provide yet.

---

## 2. Agent startup CTO — "Why can't my engineers do this?"

**Objection, sharpened:** your test catalog describes things a
competent senior engineer already knows to check — session binding,
re-checking auth on every dispatch path, approval binding before
execution. What do you have that we don't already have in-house?

**Does the pack already answer this?** Yes, and it's deliberately
modest: `01-positioning.md`'s "why wouldn't a company just test this
themselves" section already concedes that internal testing is often
enough, and names the actual conditions under which it isn't (nobody
internally owns this specific question with evidence, the team closest
to the retry/delegation code is the least likely to distrust it, a
structured cross-framework catalog already exists rather than being
invented ad hoc). `09-competitive-positioning.md`'s "when Crypton is
genuinely redundant" section says the same thing from the other
direction.

**Change made:** none — the honest answer was already the modest one,
and making it stronger than that would be the kind of overclaim this
whole pack is trying to avoid.

**Unresolved:** Crypton's advantage here is focus and an outside,
adversarial vantage point, not a technique the CTO's own team is
structurally incapable of running. A CTO with a security-literate team
and the discipline to actually red-team their own retry/delegation paths
may not need this. That's a real, standing limit on the addressable
market, not a rhetorical concession.

---

## 3. Security engineer — "Isn't this just authorization testing?"

**Objection, sharpened:** session binding is bearer-token hijacking.
Dispatch-path re-checking is complete mediation. Approval-binding drift
is TOCTOU. None of this is new; you've just relabeled classical
authorization/security concepts with agent-flavored names.

**Does the pack already answer this?** No — and it shouldn't try to
argue the concepts are new, because they aren't, and Crypton's own
internal record already says so explicitly (the founder operating doc's
Decision Log states the underlying delegation primitive is converging on
an IETF standard and was reclassified from "unique" to "credibility
asset, not moat" months before this pack existed). This pack should
concede the same point rather than quietly avoid it.

**Change made:** confirmed no file in this pack claims the underlying
security *theory* is novel — `03-test-catalog.md` and `04-evidence-map.md`
describe these as tested properties and corroborated patterns, never as
Crypton-invented concepts. No new claim was added to argue otherwise.

**Unresolved, stated as the real answer:** correct, this is applied
complete mediation / confused deputy / TOCTOU / bearer-token reasoning,
not new security theory. What's actually different is *surface area and
dynamism*: an agent can create a delegation, choose a retry path, or
compose two pieces of trusted middleware at runtime, driven by
model reasoning rather than a fixed, enumerable set of code paths a
human wrote in advance. That makes the same old failure modes more
frequent and harder to enumerate statically — a legitimate reason to
test for them specifically in agent systems, not a claim that the
failure modes themselves are new.

---

## 4. Procurement / risk — "What exactly am I buying for $2,500?"

**Objection, sharpened:** 3 days for $2,500 is priced low enough to look
under-resourced. What happens if you find nothing? What's the actual
deliverable if the answer across every test is PASS?

**Does the pack already answer this?** It didn't, cleanly, before this
review. `02-founding-assessment.md` listed deliverables but didn't
directly answer "what if the result is clean."

**Change made:** added an explicit "What you get if we find nothing"
section to `02-founding-assessment.md` — the Authority Map, PASS
evidence, and written report are the deliverable regardless of FAIL
count; a clean result across tested families is itself a real,
evidenced outcome, not an empty report.

**Unresolved:** the price is explicitly, admittedly under-market
(stated as such in `02-founding-assessment.md`) to secure a first real
engagement — a procurement reviewer used to pricing-as-a-quality-signal
may read "cheap" as "not serious" regardless of how that's explained.
This is a real tension between "reduce friction for a first pilot" and
"price signals competence," and the pack resolves it by disclosure, not
by pricing higher.

---

## 5. Skeptical researcher — "Which of these properties are genuinely
agent-specific?"

**Objection, sharpened:** same core objection as #3, from a research
angle — name one property in your catalog that wouldn't exist, in some
form, in a pre-LLM distributed system with retries and delegated service
accounts.

**Does the pack already answer this?** Not by claiming one does. Being
honest about this was already forced by #3 above, and the same answer
applies: none of the six catalog families are theoretically novel.

**Change made:** none beyond what #3 already produced — a second
overclaim-avoidance pass would be redundant with the first.

**Unresolved:** what would make a property genuinely agent-specific, and
doesn't exist yet in this catalog, is something like: a failure that can
only occur because an agent's *reasoning*, not just its code path,
created a delegation or retry that no engineer explicitly wrote — e.g. a
model deciding, on its own, to compose two tools in a sequence nobody
anticipated. Crypton's closest real work on this exact question
(Principal Continuity, `03-test-catalog.md`'s Experimental section) is
explicitly not promoted and not sold as a delivered test today. Until it
is, the honest position is: Crypton currently sells rigorous, focused,
cross-framework-tested *application* of established security reasoning
to agent systems, not a demonstrated new category of failure unique to
agents.

---

## The three strongest remaining objections, after every fix above

1. **Zero completed customer engagements**, still. The Validation
   Benchmark answers "does your methodology work on anything you didn't
   design it around" — it does not and cannot answer "have you done this
   for a real customer." Only a first engagement resolves that; no
   further pack revision, benchmark case, or citation will.
2. **The underlying security concepts are not new**, and Crypton does not
   claim otherwise once pressed — the value proposition rests on focus,
   packaging, cross-framework validation, and independent adversarial
   application, not on a technique competitors structurally cannot
   reproduce. This is consistent with, not contradicted by, Crypton's own
   internal strategy doc's prior conclusion on the same question.
3. **Coverage is uneven across the catalog**, and this pack says so
   (Family 5's revocation sub-property is spec-only; Family 6 is a
   stated internal coverage gap). A customer whose primary concern
   happens to land on the weaker families gets meaningfully less than
   the catalog's headline framing might suggest on a first skim — the
   fix is reading past the headline, which the boundary-condition and
   maturity notes in `03-test-catalog.md` are built to force, but a
   skimming reader could still miss it.
4. **Two benchmark cases is evidence the methodology transfers a
   little, not that it transfers generally.** VB-001 and VB-002 are
   independent of each other, but two data points don't establish a
   trend — a customer could reasonably ask for a third, on a different
   property family again, before finding "it generalizes" persuasive.
   This is explicitly still open; see the pre-customer credibility gate
   below.
5. **Nobody outside Crypton has attacked this methodology yet.**
   Every check in this pack, including this red-team exercise, was
   performed by the same people who built the thing being checked. An
   external, substantive review — someone trying specifically to break
   the claimed security boundaries (are they real?), the evidence
   standard (does it actually justify PASS/FAIL/N/A?), and the
   "agent-specific" framing (is it overclaimed?) — has not happened.

## The pre-customer credibility gate

Given objections 1, 4, and 5 above, the working rule for what "as ready
as reasonably possible, before someone has to be the first customer"
means:

> **Two independent Validation Benchmark cases (systems, languages, and
> property families) + one substantive external methodology review =
> pre-customer credibility gate satisfied.**

This does not mean "Crypton is proven" or "industry validated." It means
the things fixable *before* a first customer engagement have actually
been fixed, and what's left (objection 1 — zero customer track record)
is the one thing that can only be resolved by running one. As of this
pack's current state: the benchmark half of the gate is satisfied (two
independent cases, VB-001 and VB-002); the external-review half is not —
no outside reviewer has yet attacked the methodology's claimed security
boundaries, evidence standard, or agent-specificity framing. A ready-to-send
brief for that review exists (`EXTERNAL-REVIEW-BRIEF.md`) but has not yet
been sent to anyone — sending it is a founder action, not something this
pack can do on its own. Status stays **READY WITH CONDITIONS** until that
review actually happens; only then does **READY FOR FIRST DESIGN PARTNER**
become the honest label.
