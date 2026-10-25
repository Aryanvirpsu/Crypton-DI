# Agent Authorization Assessment — OpenHands Software Agent SDK (Simulated Engagement)

Prepared by Crypton (EXP-005, research simulation — **no real customer
engaged, no fee, no Authorization-to-Test executed**) · 2026-09-16 ·
Confidential (internal)

> **This is a simulation.** Per the [Assessment Execution Playbook](../../docs/methodology/06-assessment-execution-playbook.md)'s
> research-lab variant, Step 0 (legal gate) and the real
> remediation-walkthrough delivery are skipped — there is no customer.
> This document exists to test whether `docs/methodology/07-customer-report-template.md`
> produces a usable, real-feeling artifact when filled with genuine
> (not fabricated) findings against a real target's public source. Every
> finding cited below is identical to, and traceable to, `findings.md`
> in this same directory — nothing here was invented for presentation
> purposes.

---

## 1. Executive Summary

**What we assessed:** the authorization surface of the OpenHands
Software Agent SDK (`openhands-sdk` 1.49.0, commit `97dbce5c...1fa71`) —
one workflow (single-conversation action execution with
confirmation-policy/security-analyzer gating) plus both of its
multi-agent delegation primitives (`DelegateTool`, `TaskToolSet`),
reviewed via public source and documentation only, no staging
environment, no live execution.

**Top 3 things leadership needs to know:**

1. If your team configures risk-based confirmation
   (`ConfirmRisky()` + `LLMSecurityAnalyzer()` alone), the human-approval
   gate you believe protects high-risk actions can be satisfied by the
   AI model's own self-reported risk label, with no independent check.
   This is a configuration choice you can fix today — see §6, §8.
2. If your team uses the SDK's own delegation features
   (`DelegateTool`/`TaskToolSet`) to run sub-agents, any confirmation
   pause a sub-agent hits is, by default, silently auto-approved —
   **the human is never actually asked** — unless someone on your team
   independently writes and wires a callback the reference documentation
   does not show. This is the highest-impact finding in this report.
3. This is not hypothetical risk-modeling: the SDK's own GitHub history
   contains a real, merged fix (PR #4813) for a closely related class
   of bug (authorization state silently reverting to stale values on
   restart) — meaning the maintainers have already had to fix one
   instance of "authorization state didn't do what was assumed."

**Overall authority-flow risk posture:** the SDK's core, single-agent
confirmation mechanism is well-designed and does what it claims — a
real pre-execution pause, correctly ordered ahead of tool execution,
with argument-bearing correlation. Risk concentrates at two specific
seams: (a) where the confirmation gate's *input signal* comes from an
unverified source, and (b) where delegation composes with confirmation
and the composition silently drops the human out of the loop. Neither
is a defect in the core mechanism; both are configuration/integration
gaps a team can close without waiting on the SDK's maintainers.

**Threat coverage:** 3 of 11 objectives tested with a definite result
this pass (Persist privilege — historically failed, now fixed;
Bypass approval — split result; Confuse authority — partial, with one
new finding); 2 more partially evidenced (Escape delegation, Lose
attribution); 6 remain untested this pass (Acquire/Expand/Forge/Replay/Reuse
authority, Modify execution's tool-substitution half) — see Appendix A
for the full, honest accounting.

---

## 2. Trust-Flow Map

```
Human/Integrator -> (no crypto identity) -> Conversation/Agent
   -> confirmation_policy (opt-in; NeverConfirm() by default)
   -> [optional] security_analyzer -> risk signal
        -> LLMSecurityAnalyzer: signal = model's own self-report (GAP)
        -> Pattern/PolicyRail analyzers: signal = independent, deterministic
   -> pre-execution gate (agent.py:1081, sound)
   -> tool execution (bash/file/browser/MCP)
   -> [if delegated] sub-agent inherits confirmation_policy
        -> sub-agent's own gate fires correctly
        -> DelegateExecutor/TaskManager: confirmation_handler=None by
           default -> pause silently auto-resolved (GAP)
```

| Pillar | Status |
|---|---|
| Identity | **Gap** — shared static secret, no per-caller attribution |
| Authority | **Partial** — sound representation and single enforcement point; input signal to that enforcement point is unverified in one documented configuration (§6) |
| Delegation | **Partial** — own-gate independence is sound; gate *delivery* is broken by default (§6, top finding) |
| Human Approval | **Partial** — real at the top level; silently defeated one hop down by default |
| Execution | **Supported** — pre-execution ordering is correct; tool-substitution not tested this pass |
| Persistence | **Supported (currently)** — was previously a real gap (PR #4813), fixed and verified in the current source |
| Auditability | **Partial** — durable, reconstructable event log; no cryptographic attribution of *who* |

---

## 3. Technical Findings

See [`findings.md`](findings.md) for the complete eleven-link chain per
finding. Summarized:

### FINDING-005-1 — Risk-gated confirmation satisfiable by model self-report
- **Hypothesis/Threat:** proposed H10 (Signal Independence) / Bypass approval, Confuse authority
- **Test:** source review, 3 file:line citations
- **Observed vs. Expected:** `ConfirmRisky()` gates on a risk value that, under `LLMSecurityAnalyzer` alone, is the model's own unverified self-report, vs. expected independent corroboration
- **Security Property:** Authority, Human Approval
- **Status:** **CANDIDATE_FINDING** (config-dependent, not a universal defect)
- **Risk:** Moderate — real authority-bypass potential, bounded by requiring a specific configuration choice
- **Recommendation:** compose `LLMSecurityAnalyzer` inside `EnsembleSecurityAnalyzer` with a deterministic analyzer; never use it alone for consequential workflows
- **Confidence:** High (mechanism), Moderate (real-world exploitability, config-dependent)

### FINDING-005-2 — Delegated confirmation pauses silently auto-approved by default
- **Hypothesis/Threat:** H2 (extended) / Bypass approval, Escape delegation
- **Test:** source review, 4 file:line citations across both delegation primitives, confirmed by absence of any counter-example in the maintainers' own reference example
- **Observed vs. Expected:** sub-agent `WAITING_FOR_CONFIRMATION` genuinely fires but is auto-resolved by `DelegateExecutor`/`TaskManager` when no callback is wired (the shipped default), vs. expected delivery to an actual human decision
- **Security Property:** Delegation, Human Approval
- **Status:** **CANDIDATE_FINDING**, structural (affects the documented default path, not an edge case)
- **Risk:** Moderate-to-high — full authority of whatever tools the sub-agent holds, zero human interaction required, affects every delegated conversation that hits a confirmation pause
- **Recommendation:** target should fail closed (block/raise) rather than auto-approve when no handler is wired under a confirmation-requiring policy, and update the reference delegation example to demonstrate correct wiring
- **Confidence:** High

### FINDING-005-3 — Duplicate, inconsistent confirmation-decision logic (dead code today)
- **Hypothesis/Threat:** none of H1-H9 directly; adjacent to policy-drift concerns
- **Test:** repo-wide grep, zero callers found for the unused path
- **Status:** **INFORMATIONAL**
- **Risk:** Low today; real prospective risk if a future contributor wires the wrong function in
- **Recommendation:** remove or clearly deprecate `SecurityAnalyzerBase.should_require_confirmation`
- **Confidence:** High

Clean areas, stated explicitly: the core pre-execution gate ordering
(action proposed → check → execute) is correct and was not found to be
bypassable through any path reviewed; tool-set changes across a
persisted resume are explicitly and correctly verified (tools may only
be added, never silently removed); the historical dual-source-of-truth
persistence bug (PR #4813) is confirmed fixed in the reviewed commit.

---

## 4. Authorization Map

| Agent | Identity | Credentials | Tools | Where check sits | Pre/post execution | Bypass attempted | Result |
|---|---|---|---|---|---|---|---|
| Top-level `Agent` (library) | None (app-supplied) | Whatever the integrating app grants | Any registered `Tool` | `agent.py:1046` (`_requires_user_confirmation`) | **Pre**-execution | Traced risk-signal source | Self-report-only gap found (FINDING-005-1) when `LLMSecurityAnalyzer` used alone |
| Deployable server caller | `session_api_keys` membership | Same shared secret set | All server-exposed operations | `dependencies.py:35` and equivalents | Pre-request | Checked for per-key attribution | Not found — flagged, not "bypassed" (no live bypass attempted) |
| Delegated sub-agent | Inherits parent's (none) | Inherits/overridden per `agent_type` | Sub-agent's own registered tools | Sub-agent's own `agent.py:1046` instance | Pre-execution (at the sub-agent) | Traced pause-delivery path | **Confirmed gap** — pause silently auto-resolved by default (FINDING-005-2) |

---

## 5. Delegation Findings

- **Authority path across agents:** parent spawns children via
  `DelegateExecutor`/`TaskManager`; each child is a fully independent
  `LocalConversation` with its own `ConversationState`, own event log,
  own confirmation gate. The gate is not shared or bypassed by the
  parent's own approval — this specific vector (parent approval
  implicitly covering child actions) is **sound**, consistent with
  Crypton's H2 results on every other framework tested to date
  (OpenAI Agents SDK, Microsoft Agent Framework, LangGraph — see
  `docs/methodology/04-framework-adaptation-guide.md`).
- **Where scope fails to narrow, or widens:** no scope/target-set
  expansion test was run this pass (H7 — coverage gap, stated as such,
  not implied clean). Policy *inheritance* (child gets parent's
  `confirmation_policy` unless a specific `agent_type` overrides it) is
  intentional and reasonable design, not itself a widening.
- **Is the chain reconstructable? Does revocation propagate?**
  Reconstructable: yes — each conversation's event log is durable and
  independently readable. Revocation: **NOT_APPLICABLE** — this
  simulation had no live/production traffic to test revocation
  propagation timing against, consistent with `docs/ASSESSMENT_METHODOLOGY.md`
  area 7's own guidance for a one-shot review.

---

## 6. Human-Approval Analysis

- **What requires approval today vs. what should:** by default,
  **nothing** requires approval (`ConversationState.confirmation_policy`
  defaults to `NeverConfirm()`) — a disclosed, documented posture, not
  a hidden default, but worth stating plainly for a leadership audience:
  approval is opt-in. Once opted in (`AlwaysConfirm()`/`ConfirmRisky()`),
  the mechanism is sound at the top level.
- **Is approval bound to the exact request?** Yes, at the top level
  (argument-bearing `ActionEvent` correlation). One hop into delegation,
  the question becomes moot — the approval step itself doesn't happen
  (FINDING-005-2).
- **Can an approved action be modified after?** No live mutation
  API analogous to LangGraph's `update_state()` was located in the
  reviewed surface; not exhaustively ruled out (INCONCLUSIVE, stated as
  such, not claimed clean).
- **Standing/broad approval scope:** `NeverConfirm()`/`AlwaysConfirm()`
  are binary, argument-blind, explicit (H3, same shape as OpenAI Agents
  SDK's `always_approve`). `ConfirmRisky()` narrows by *risk level*, not
  by argument — and that risk level can, in one documented
  configuration, come from the actor being gated (FINDING-005-1).
- **Consumed at most once, including across resume?** Structurally
  supported by the current single-source-of-truth persistence design
  (post-PR #4813); not confirmed with a live resume test this pass
  (INCONCLUSIVE for H4 specifically, stated as such).

---

## 7. Evidence Appendix

All evidence is static source citation, commit-pinned to
`97dbce5c37865dd7a3020d2b9662f4a76d11fa71` (openhands-sdk `1.49.0`),
plus one external, dated GitHub PR. No screenshots, HTTP captures, or
execution traces exist for this pass — see `target.json` for the exact
reason (no live execution attempted, disclosed up front, not a
limitation discovered after the fact). Full citations are in
[`findings.md`](findings.md); repeating them here would duplicate, not
add, information — the report rule against filler applies to Crypton's
own reports as much as to invented findings.

---

## 8. Remediation Summary & Priorities

| Priority | Finding | Fix | Effort | Impact |
|---|---|---|---|---|
| 1 | FINDING-005-2 (delegation auto-approve) | Fail closed when `confirmation_handler is None` under a confirmation-requiring policy; fix the reference example | Low (a few lines + one doc example) | High — closes a silent, default-path human-approval bypass |
| 2 | FINDING-005-1 (self-report risk) | Document that `LLMSecurityAnalyzer` alone provides no independent verification; recommend `EnsembleSecurityAnalyzer` composition in the security guide | Low (documentation) to Medium (if a "warn when used alone" runtime check is added) | Moderate — closes a config-dependent gap, doesn't require an architecture change |
| 3 | FINDING-005-3 (dead duplicate logic) | Remove or deprecate `should_require_confirmation` | Very low | Low today; prevents a future regression |

**These are recommendations for the OpenHands SDK's own maintainers and
its integrators — not a Crypton product pitch.** Per the standing rule,
Crypton's credibility asset in a real engagement is the assessment
itself, not a funnel to a future platform.

---

## Appendix A. Threat Coverage

| Objective | Status this engagement | Reason / pointer |
|---|---|---|
| Acquire authority | NOT APPLICABLE (scope) | No identity primitive exists to forge against at the library layer; server-layer shared-secret admission was reviewed but not live-tested for bypass — see §4 |
| Expand authority | NOT TESTED | No multi-target/scope-expansion construct exercised this pass (H7) |
| Reuse authority | INCONCLUSIVE | Structurally supportive persistence design; no live resume test run — see §6 |
| Forge authority | NOT TESTED | No forged-approval-object test attempted |
| Replay authority | NOT TESTED | Same reasoning as Reuse authority |
| Confuse authority | **IN SCOPE — FINDING-005-1** | See §3 |
| Lose attribution | PARTIALLY TESTED | Event-log durability confirmed; per-caller attribution gap identified, not exploited live — see §2, §4 |
| Bypass approval | **IN SCOPE — FINDING-005-2** | See §3 |
| Modify execution | PARTIALLY TESTED | Pre-execution ordering confirmed sound; tool-substitution (H9) not tested |
| Escape delegation | PARTIALLY TESTED | Own-gate independence confirmed sound; gate-delivery gap identified — see §5 |
| Persist privilege | **IN SCOPE — historical, now fixed** | PR #4813, see §1, §3 |

This engagement tested a genuine minority of the eleven objectives to a
definite result. That is reported plainly here, not implied otherwise
by omission, per the Customer Report Template's own rule.
