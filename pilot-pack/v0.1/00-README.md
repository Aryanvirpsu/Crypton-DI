# Crypton Pilot Pack v0.1

**Purpose:** give an AI-agent startup CTO, a CISO/security lead, a
platform/security engineer, or a prospective design partner everything
needed, in a few minutes, to decide whether to offer Crypton one bounded
workflow to assess — the **Crypton Founding Agent Authorization
Assessment** ($2,500, 3 testing days, no credentials required by
default).

Crypton is a service-first **Agent Authorization & Delegation
Assessment**: independent testing of whether an action-taking AI agent's
real authority stays correctly bound to the principal, the delegated
task, its scope, human approval, current authorization state, and
downstream execution — with reconstructable evidence throughout. Crypton
is not an IAM provider, authorization engine, MCP gateway, policy
engine, compliance certification, generic AI pentest, or prompt-injection
scanner. See `01-positioning.md` for exactly where the line is and why.

## Read in this order

1. **[01-positioning.md](01-positioning.md)** — one sentence, 30 seconds,
   and the technical boundary (what Crypton is, isn't, and why internal
   testing sometimes suffices).
2. **[02-founding-assessment.md](02-founding-assessment.md)** — the
   actual $2,500 / 3-day offer: what we test, what you provide, what you
   receive, what we don't need, ideal fit vs. not a fit.
3. **[03-test-catalog.md](03-test-catalog.md)** — the properties Crypton
   tests, each with a question, example test, evidence, PASS/FAIL/N/A
   definitions, and honest maturity/boundary notes. The most important
   technical artifact in this pack.
4. **[04-evidence-map.md](04-evidence-map.md)** — independently disclosed
   2026 vulnerabilities in real, unrelated agent/MCP systems, mapped to
   the test catalog, kept strictly separate from Crypton's own internal
   evidence. Includes the **Crypton Validation Benchmark**
   (`labs/validation-benchmark/v0.1/`) — two real, reproducible cases,
   deliberately independent of each other (different vendor, language,
   and test-catalog property each), where an unmodified Crypton test
   correctly returns FAIL against a known-vulnerable dependency version
   and PASS against its patched successor. Breaks the "we designed the
   test, we ran the test, it passed" circularity in every other piece of
   evidence here, and starts to answer whether the methodology
   transfers, not just whether it's self-referential.
5. **[05-sample-assessment.md](05-sample-assessment.md)** — a fictional,
   prominently-labeled illustrative assessment (ExampleCo Support Agent)
   showing report format: one PASS, one FAIL, one expected-behavior N/A,
   negative results, limitations.
6. **[06-access-and-safety.md](06-access-and-safety.md)** — the three
   engagement modes, led by **Customer-Controlled Assessment** (no
   credentials, no production access, no source repository — you run the
   agreed tests and hand over the evidence), with staging and
   observed-live as fallbacks only where they add real value, and every
   rule that bounds what Crypton can touch and for how long.
7. **[07-why-trust-crypton.md](07-why-trust-crypton.md)** — the honest
   credibility argument, including the LangChain finding Crypton
   retracted after applying its own triage rule properly.
8. **[08-mvp-definition.md](08-mvp-definition.md)** — what Crypton's MVP
   actually is today (the assessment service, not software), and what
   would justify building software later.
9. **[09-competitive-positioning.md](09-competitive-positioning.md)** —
   where Crypton overlaps IAM/authorization engines/gateways/pentest/red
   teaming, and where Crypton is honestly redundant.
10. **[10-pre-engagement-checklist.md](10-pre-engagement-checklist.md)** —
    the fillable version of the access/safety/scope rules.
11. **[11-pilot-pack-red-team.md](11-pilot-pack-red-team.md)** — five
    adversarial personas attacking this pack, what was fixed because of
    it, and what's still honestly unresolved.

## Also in this directory (not part of the customer-facing 12)

**[EXTERNAL-REVIEW-BRIEF.md](EXTERNAL-REVIEW-BRIEF.md)** — a condensed
brief for sending to outside technical reviewers (not customers) to
attack the methodology's claimed security boundaries, its evidence
discipline, and its agent-specificity framing, before any customer does.
Not yet sent to anyone as of this writing — see
`11-pilot-pack-red-team.md`'s "pre-customer credibility gate."

## What this pack is not

Not a claim of enterprise customers, certifications, partnerships,
testimonials, or completed engagements — Crypton has none of these yet,
and this pack does not imply otherwise anywhere. Every fictional example
is prominently labeled as such. Every evidence claim states, explicitly,
whether it's something Crypton produced or something a different party
independently disclosed.

## The actual test

> If you were responsible for security at an agent company, does this
> pack give you enough to safely offer Crypton one bounded workflow to
> assess?

The honest answer, worked through in full in
`11-pilot-pack-red-team.md`: **for a team that weighs bounded, reversible
risk rather than requiring a prior track record, yes** — the default
engagement mode asks for no credentials at all
(`06-access-and-safety.md` Mode 1), the methodology is checkable in
advance (`03-test-catalog.md`), it's been shown to work against two real,
independently-patched vulnerabilities on unrelated systems, not just its
own research targets (`labs/validation-benchmark/v0.1/`), and the failure
classes are
documented as real and current independent of Crypton
(`04-evidence-map.md`). For a team that specifically requires a
delivered customer track record before a first engagement, no — and no
amount of rewriting this pack changes that; only running a real
engagement does.
