# Closure — finding-001d / finding-001e

**Date:** 2026-09-27
**Final classification:** `NO VULNERABILITY — EXPECTED FRAMEWORK BEHAVIOR`
**Status:** CLOSED. No further disclosure or PoC-strengthening work is planned against LangChain for this mechanism.

## What was reported

finding-001d/e described `langchain`'s `after_model` middleware composing in
reverse list order, such that a small, independently-benign custom middleware
(`HostEscalationMiddleware`) could rewrite an already-approved tool call's
arguments after `HumanInTheLoopMiddleware` had gated it, before execution.
The original write-ups (preserved unchanged below this closure) described
this as a "composition bug," "drift," and a gap worth reporting to the
framework's maintainers as remediation option 2.

## Why it is not a vulnerability

- `after_model` middleware execution order (last-in-list runs first,
  first-in-list runs last) is intentional, documented behavior of
  `langchain.agents.factory`'s graph-building code — not an accident and not
  a defect in that code.
- Middleware supplied by the developer (`HostEscalationMiddleware`,
  `HumanInTheLoopMiddleware` composed alongside it) is trusted application
  code, running with the same trust level as the rest of the agent's own
  wiring — not a separate, lower-trust principal.
- Trusted `after_model` middleware is already permitted, by the framework's
  own contract, to alter or re-route tool calls after they are produced —
  proven in the original write-up by `PIIMiddleware`'s own shipped use of
  exactly this mechanism to rewrite `tool_calls[*].args`.
- Applying the Phase 2 triage rule (see `assessment-kit/v1.0/01-assessor-handbook.md`
  §9a): the mutator is `HostEscalationMiddleware`, trust level is fully
  trusted (developer-authored, same trust domain as the approval gate
  itself), it is permitted by design to do exactly what it did, and
  LangChain's `after_model`/`HumanInTheLoopMiddleware` contract never
  claims that approval-argument binding survives an arbitrary chain of
  trusted `after_model` hooks the developer composes afterward. No
  untrusted principal, model, tool, or input entered the path at any point
  — the entire sequence, from the model's retry to both middlewares' effect,
  ran inside one trust domain that the developer fully controls and
  authored. No framework-claimed authorization boundary was crossed.
- Maintainer triage on this point is accepted. No further disclosure or
  PoC-strengthening work is planned.

## What is retained

The original write-ups (`finding-001d.md`, `finding-001e.md`, `writeup.md`,
this directory's `README.md`) are preserved as a **negative/calibration
result**, unedited except for a closure banner pointing here. Their
technical content — the verbatim quoted `langchain==1.4.0` source, the
deterministic 5/5 mechanism proof, the 38/40 real-model confirmation, the
oracle/capture/trace methodology — remains accurate and useful as a
calibration exercise: it shows Crypton correctly characterizing a real,
reproducible framework mechanism, before correctly closing it as expected
behavior once trust-domain and boundary analysis were applied.

## Methodological lesson retained

A difference between an approved action and an executed action is not
sufficient by itself to call something a vulnerability. The question that
actually decides it is whether an entity outside the trust domain the
framework claims to protect caused the mutation. Here, the same developer
who wrote the approval gate also wrote the middleware that altered the
call afterward — there was never a second principal, and never a claim
from the framework that composition of its own trusted extension points
preserves argument-level approval binding on its own. This is exactly the
distinction Phase 2's triage rule (§9a) now makes explicit for every future
observation, so Crypton does not re-make this classification error on a
future engagement.

## Effect on the Finding Library

`FL-01 — Approval/Execution Drift via Benign Middleware Composition`
(`assessment-kit/v1.0/07-finding-library.md`) is **retired** and moved to
that document's "Excluded From This Library" table. It is not a reusable
customer-assessable pattern under the name "LangChain vulnerability" or
"framework bypass." A related, narrower customer-integration
consideration — *does this deployment recompute the approval binding
immediately before execution, given that its own trusted composition of
hooks/middleware can otherwise change an already-approved action* — is a
legitimate question for an assessor to ask about a customer's own system
architecture, independent of any framework claim, but it is not restated
here as a numbered library finding; see `VALIDATION-REPORT.md`'s revised
entry for the reasoning trail.

`VALIDATION-REPORT.md`'s matrix entries for EXP-001d/EXP-001e are updated
accordingly (see that document).
