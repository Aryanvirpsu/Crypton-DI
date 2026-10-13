# Why Trust the Assessment?

Crypton is early. There is no enterprise customer list, no certification,
no partnership, no testimonial — and this document will not manufacture
any of those. What follows is the actual, checkable basis for trusting
the work, not the company's age or resume. If you ask about the team's
background directly, we'll answer honestly; it's just not the argument
being made here, because it isn't the relevant one. Judge the method.

## Transparent methodology

Every property Crypton tests is written down, in advance, with the exact
question it asks, an example test, what counts as PASS/FAIL/N/A, and
where the line between "expected design" and "a real gap" sits —
`03-test-catalog.md`, built directly from the working internal
methodology (`docs/methodology/`, `assessment-kit/v1.0/`), not
constructed after the fact to justify a conclusion. You can read the
test before you agree to it, and check our reasoning against it after.

## Reproducible evidence

No PASS or FAIL is reported without a specific, cited piece of evidence —
a source excerpt, a config value, a trace, a controlled test with a
predicted and observed outcome. This is enforced structurally in
Crypton's own internal tooling (`Finding.__post_init__` raises rather
than allowing an unevidenced PASS/FAIL to be constructed at all), not
just a stated intention. Where evidence can be a deterministic,
reproducible test, it is documented so your own engineers can re-run it
independently — trust is not required if you can just check.

## Explicit threat-model validation before calling something a
vulnerability

Not every surprising or unexpected behavior is a security failure.
Crypton's own methodology requires answering five questions before
filing a finding: who or what caused the observed behavior, what trust
level they hold, whether the system's own design permits it, what
boundary the system actually claims to enforce, and whether that
boundary was actually crossed
(`assessment-kit/v1.0/01-assessor-handbook.md` §9a). A real difference
between what was approved and what executed is not, by itself, evidence
of a vulnerability — see "Methodology calibration," below, for what this
looks like in practice, including against Crypton's own prior work.

## Negative-result reporting

Every report states what was tested and *held*, not only what failed —
see the Negative Results section in `05-sample-assessment.md`. A report
that only ever finds things wrong is a report you should be suspicious
of; ours states PASS and NOT_APPLICABLE results with the same evidence
standard as FAIL.

## Bounded claims

A candidate property stays labeled a candidate until it clears a stated
evidentiary bar — it is never quietly presented as an established
finding because the harness that tests it works. Principal Continuity
(`03-test-catalog.md`'s Experimental section) is the concrete example:
one deterministic test exists, one loosely-related external case exists,
and the internal record states plainly that this is not enough for
promotion — `labs/exp-007-principal-continuity/EVIDENCE.md` documents
two separate, bounded external searches that found no second qualifying
case, and says so rather than stretching a weaker match to close the
gap.

## Retrospective validation against independently patched vulnerabilities

Before ever asking a customer for access, Crypton tested its own
methodology against two things it didn't design: independently disclosed,
independently patched vulnerabilities in two unrelated real systems.
Same discipline both times — a Crypton-authored test script, run
completely unmodified, importing only the target's own public API, never
the target's own test suite:

- **VB-001** — the official MCP Python SDK (CVE-2026-52869): `FAIL` on
  `mcp==1.27.1`, `PASS` on `mcp==1.27.2`.
- **VB-002** — `@bytebase/dbhub`'s SQLite connector (CVE-2026-61788):
  `FAIL` on `0.22.5`, `PASS` on `0.22.6`.

Both include a negative control that would catch the test itself being
broken, not just the target. Full reproduction steps and evidence:
`labs/validation-benchmark/v0.1/`.

Neither is a discovery — the vendors found and fixed both before Crypton
ever looked at them. What they answer is a different, prior question:
does Crypton's own test for a given property actually distinguish a real
vulnerable state from a real fixed one, on a system Crypton didn't build
the test to fit? Doing this **twice**, deliberately against different
vendors, different languages, and different test-catalog properties,
answers a harder version of that question than one case could: not just
"is this circular," but "does it transfer." A methodology that only ever
runs against its own research targets can't answer either; this can, and
both are checkable by anyone who wants to re-run them.

## External grounding

The failure classes Crypton's catalog targets are not hypothetical — see
`04-evidence-map.md` for a set of independently disclosed 2026
vulnerabilities, in real, unrelated, production MCP and agent
infrastructure (an official MCP SDK, a database-connector MCP server, an
AI gateway, an open-source IAM platform, an open-source AI assistant
platform, a support-ticketing MCP integration), each mapped honestly to
the specific catalog property it corroborates — with a standing rule
that none of them are claimed as validating Crypton's own work, only as
evidence the problem shape is real and current.

## Independent verification

Crypton is not the vendor of the IAM system, the policy engine, the agent
framework, or the gateway that a workflow relies on. The assessment's
value is structurally tied to not being the party that built or sells
the control being tested — see `09-competitive-positioning.md` for where
that independence does and doesn't matter.

## Methodology calibration — the LangChain retraction

In September 2026, Crypton carried a candidate finding — a LangChain
agent-middleware composition pattern that let an already-approved
action's arguments change before execution — as a high-confidence,
customer-facing finding (`FL-01` in the finding library), re-validated
as recently as an internal independent re-review weeks earlier.

Applying the five-question triage rule above, properly, changed the
conclusion: the component that altered the arguments was trusted,
developer-authored application code running in the same trust domain as
the approval gate itself, doing exactly what LangChain's own
documented `after_model` contract already permits trusted middleware to
do (the same mechanism LangChain's own shipped PII-redaction middleware
uses, for a different purpose). No untrusted principal, model, tool, or
input ever entered the path, and LangChain never claimed that composing
its own trusted extension points preserves argument-level approval
binding on its own. No framework-claimed boundary was crossed.

The finding was retracted: reclassified `NO VULNERABILITY — EXPECTED
FRAMEWORK BEHAVIOR`, removed from the customer-facing finding library,
and the underlying triage gap turned into a standing rule (§9a) so the
same mistake isn't repeated. Full record:
`labs/experiments/001-langgraph-approval-drift/CLOSURE.md`.

**We're not presenting this as equivalent to discovering a
vulnerability — it isn't one.** It's evidence of something narrower and,
for a customer deciding whether to trust a report, more relevant:
Crypton will retract its own prior conclusion, including a flagship one,
when the analysis says it should, rather than defending a finding
because it was already published. The underlying test family (Family 4,
`03-test-catalog.md`) remains real and unchanged — what changed was the
classification of one specific case, once it was actually checked
properly.
