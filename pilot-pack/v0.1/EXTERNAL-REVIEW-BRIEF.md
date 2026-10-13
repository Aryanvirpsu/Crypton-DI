# Crypton — External Methodology Review Brief

**Purpose of this document:** ask three reviewers, each from a different
angle, to try to break a specific, narrow set of claims — not to review
Crypton as a company, a business, or a pitch. Please read this as an
invitation to attack, not to validate. If nothing here survives scrutiny,
that is a useful and welcome result.

**What Crypton is, in one paragraph:** a service that independently tests
whether an AI agent's real, executed actions stay bound to the principal,
delegation, approval, and authorization state that were supposed to
govern them — for one specific, bounded, agent workflow at a time. It is
not an IAM provider, policy engine, or gateway; it tests whether the
guarantees those controls are supposed to provide actually survive real
agent execution (retries, delegation hops, persisted/resumed state,
composed middleware). Full positioning: `01-positioning.md`.

**What's attached / referenced:**
- `03-test-catalog.md` — the properties tested, each with a precise
  question, an example test, and explicit PASS/FAIL/N/A definitions.
- `labs/validation-benchmark/v0.1/` — two cases (VB-001, VB-002, below)
  where a Crypton-authored test is run, unmodified, against a
  known-vulnerable and a known-patched version of a real dependency.
- `07-why-trust-crypton.md` — the credibility argument this brief is
  meant to test, including a case where Crypton retracted its own prior
  finding after applying its classification rule properly.

## The two things being claimed

**Claim 1 (methodology validity):** Crypton's test-catalog properties
correspond to real, checkable security boundaries in agent-based
systems — not invented categories, and not a restatement of ordinary
application bugs with "agent" attached to the name.

**Claim 2 (evidence discipline):** Crypton's PASS/FAIL/INCONCLUSIVE/
NOT_APPLICABLE classification, and the five-question triage rule it
applies before calling something a security failure (§9a,
`01-assessor-handbook.md` — summarized below), correctly separates a
real gap from expected, permitted behavior.

## The two Validation Benchmark cases, summarized

| | VB-001 | VB-002 |
|---|---|---|
| Target | Official MCP Python SDK (`mcp` on PyPI) | `@bytebase/dbhub` (npm), SQLite connector |
| CVE | CVE-2026-52869 | CVE-2026-61788 |
| Property tested | Principal / Session Binding | Authorization Boundary & Complete Mediation |
| Mechanism | A session ID alone, with no check on who authenticated, let a different principal act inside another's session | A `readonly: true` option was accepted and threaded through the code but never actually enforced at the database level |
| Vulnerable version | `1.27.1` — test result: FAIL | `0.22.5` — test result: FAIL |
| Fixed version | `1.27.2` — test result: PASS | `0.22.6` — test result: PASS |

Same script, unmodified, both runs, each case. Neither vulnerability was
discovered by Crypton — both were independently disclosed and patched
before Crypton wrote a line of test code against them. Full detail,
including each case's own stated limitations: the two case `README.md`
files under `labs/validation-benchmark/v0.1/`.

## The five-question triage rule being tested (§9a)

Before Crypton classifies an observed behavior as a security failure:
(1) who or what caused it, (2) what trust level that actor has, (3)
whether the system's own design permits it, (4) what boundary the system
actually claims to enforce, (5) whether that boundary was actually
crossed. This rule was added after Crypton had to retract a prior
candidate finding (a LangChain agent-middleware case) that had been
carried as high-confidence for weeks before this question was asked
properly — full account in `07-why-trust-crypton.md`.

## Three specific questions, one per reviewer

We are not asking for a general opinion. We are asking each reviewer to
attack one specific thing, from their own vantage point:

**For a systems-security reviewer (are these legitimate boundaries?):**
Do the six properties in `03-test-catalog.md` (Principal/Session Binding,
Delegation & Scope Attenuation, Authorization Boundary & Complete
Mediation, Approval-to-Execution Binding, Revocation & Authority
Freshness, Accountability & Provenance) correspond to real, distinct
security boundaries — or do any of them collapse into each other, or
into a boundary that doesn't actually exist in the way the catalog
describes it? Where would you say the catalog is imprecise or wrong
about what it's actually testing?

**For a reviewer focused on evidence/verification methodology (does the
evidence justify the verdicts?):** Walk through VB-001 and VB-002's own
test scripts and README files. Does the evidence collected (the
negative controls, the before/after comparison, the exit-code discipline)
actually justify the PASS/FAIL verdicts drawn from it, or is there a gap
between what was measured and what was concluded? Separately: does the
five-question triage rule (§9a) actually distinguish a real boundary
violation from expected behavior, or does it just relocate the same
judgment call to a different, equally subjective place?

**For a reviewer skeptical of AI-specific security framing (is this
overclaimed as agent-specific?):** Crypton does not claim these are new
security concepts — session binding, complete mediation, and TOCTOU-style
approval drift are classical. The claim is narrower: that applying them
specifically across the full agent authority path
(`principal → agent → delegation → approval → tool → resource → effect`)
is a distinct, useful specialization, the way a pentesting firm's value
isn't discovering SQL injection. Is that claim actually defensible, or is
"agent-specific" doing more work in the positioning than the substance
supports? Where, if anywhere, is there something in this catalog that
genuinely could not exist in a pre-agent distributed system?

## What we're asking for back

Written criticism — as harsh as it needs to be — of any of the above. We
will document what was raised and what changed as a result, whether or
not permission is given to say who raised it. If you're willing to let us
state publicly that you reviewed this methodology, that's welcome and
we'll ask separately how you'd like that framed; if not, we will not
imply a review happened or that anyone specific endorsed anything. What
matters to us internally is that someone outside Crypton genuinely tried
to break this before a paying customer does.
