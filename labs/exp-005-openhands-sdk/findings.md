# EXP-005 Findings — OpenHands Software Agent SDK

All findings below are from **static source review** of commit
`97dbce5c37865dd7a3020d2b9662f4a76d11fa71` (openhands-sdk `1.49.0`),
cloned 2026-09-16 from `https://github.com/OpenHands/software-agent-sdk`
(MIT license). No live instance was run, no LLM was called, no
production system was touched. Each finding is written through the
[Evidence & Reporting Standard](../../docs/methodology/05-evidence-reporting-standard.md)'s
eleven-link chain. Severity is reasoned via `docs/ASSESSMENT_METHODOLOGY.md`'s
seven-factor rubric (authority gained, reachable resource, sensitivity,
required attacker position, human interaction required,
persistence/repeatability, compensating controls).

---

## FINDING-005-1 — Risk-gated confirmation can be satisfied entirely by the model's own self-report

1. **Hypothesis:** Closest to H1 (Argument Binding) in spirit but not an exact fit — the
   Universal Hypothesis set (H1-H9) has no hypothesis that names "is the
   risk signal gating confirmation independently verified, or
   self-reported by the same actor being gated." This gap is itself
   Phase 8 material (see `README.md` §9, proposed **H10**).
2. **Threat:** "Bypass approval" / "Confuse authority" —
   [Threat Model](../../docs/methodology/02-threat-model.md#bypass-approval)
   — a HIGH-risk action executes without triggering the human-approval
   gate a customer believes is protecting them, because the gate's
   input is unverified.
3. **Test:** source review — traced the full call path from tool-call
   generation to confirmation decision:
   - `openhands-sdk/openhands/sdk/agent/agent.py:1089-1114`
     (`_extract_security_risk`) — `raw = arguments.pop("security_risk", None)`
     pulls the risk level out of the **LLM's own generated tool-call
     arguments**, only when a `security_analyzer` is configured.
   - `openhands-sdk/openhands/sdk/security/llm_analyzer.py:20-29`
     (`LLMSecurityAnalyzer.security_risk`) — returns `action.security_risk`
     verbatim: *"This method checks if the action has a security_risk
     attribute set by the LLM and returns it."* No independent
     re-derivation, no cross-check against the actual command/argument
     content.
   - `openhands-sdk/openhands/sdk/security/confirmation_policy.py:43-61`
     (`ConfirmRisky.should_confirm`) — gates confirmation purely on the
     risk value it receives; it has no way to know whether that value
     came from an independent analyzer or the model itself.
4. **Evidence:** the three file:line citations above, commit-pinned;
   `openhands-sdk/openhands/sdk/security/ensemble.py:1-62`'s own
   docstring and example (`EnsembleSecurityAnalyzer(analyzers=[PolicyRailSecurityAnalyzer(), PatternSecurityAnalyzer()])`)
   — notably the maintainers' own usage example for the "combine
   analyzers" primitive does **not** include `LLMSecurityAnalyzer`,
   consistent with treating self-report as a source needing
   corroboration, not a trusted signal on its own; and
   `https://docs.openhands.dev/sdk/guides/security` (fetched 2026-09-16),
   which documents `LLMSecurityAnalyzer` as a standalone, directly
   attachable analyzer (`conversation.set_security_analyzer(LLMSecurityAnalyzer())`)
   with no warning that it provides no independent verification.
5. **Observed Behaviour:** when `LLMSecurityAnalyzer` is the configured
   (or sole) security analyzer and `ConfirmRisky()` is the confirmation
   policy — the SDK's own documented "risk-based confirmation" recipe —
   the sole input to the human-approval decision is a field the same
   model that proposed the action also populated in that action's own
   tool call. A model that mis-self-assesses (confusion, distribution
   shift) or is steered by adversarial content in tool output
   (prompt-injection-adjacent, though the mechanism here is simpler:
   it does not require jailbreaking the model's *task* reasoning, only
   its *risk self-labeling*) can emit `security_risk: "LOW"` alongside
   an arbitrarily consequential action, and `ConfirmRisky()` will not
   trigger confirmation for it.
6. **Expected Behaviour:** per this catalog's spirit (Pillar 7,
   Auditability: *"without trusting the agent's own self-report"*) and
   per Crypton's own committed 2026-09-15 technical bet (`CLAUDE.md`
   Decision Log: *"adversarially validating that a synthesized
   per-request invariant actually matches request intent... not raw
   NL→policy synthesis-and-enforce on its own"*), a risk signal that
   gates a human-approval decision should be corroborated by a source
   independent of the actor whose action is being evaluated — exactly
   the role `PatternSecurityAnalyzer`/`PolicyRailSecurityAnalyzer`
   (deterministic, no model inference) already play elsewhere in this
   same SDK.
7. **Security Property:** Pillar 2 (Authority) and Pillar 4 (Human
   Approval) — specifically Universal Hypothesis U-APP-1 ("the approval
   record binds to... arguments actually executed") extended to the
   *risk signal* that determines whether an approval record is even
   requested.
8. **Risk:** **Moderate, bounded by configuration choice.** Per the
   seven-factor rubric: authority gained is real (a HIGH-risk action
   proceeds unconfirmed) but only when the integrator chooses
   `LLMSecurityAnalyzer` alone rather than combining it with a
   deterministic analyzer via `EnsembleSecurityAnalyzer` — both are
   supported, documented configurations, and the SDK does not claim
   `LLMSecurityAnalyzer` alone is adversarially robust. No competing
   safety claim is contradicted (same reasoning class as EXP-003C's
   H1c). Required attacker position: none for the "confused model"
   reading (no attacker needed, just an imperfect self-assessment);
   moderate (ability to influence tool-output content the model reads)
   for the adversarial reading. Persistence/repeatability: every action
   evaluated by this analyzer is exposed, not a one-off. Compensating
   control exists (`EnsembleSecurityAnalyzer` + deterministic children)
   but is opt-in, not default, and not surfaced as a recommendation
   anywhere the security docs page discusses `LLMSecurityAnalyzer` in
   isolation.
9. **Recommendation:** for any customer or integrator using
   `ConfirmRisky()`, do not configure `LLMSecurityAnalyzer` as the sole
   security analyzer for any workflow where a HIGH-risk action has real
   consequences. Compose it inside `EnsembleSecurityAnalyzer` alongside
   at least one deterministic analyzer (`PatternSecurityAnalyzer`,
   `PolicyRailSecurityAnalyzer`), so the self-reported signal is
   corroborated rather than trusted alone. This is a configuration
   recommendation for integrators, consistent with the report rule
   "recommend the control, not a vendor" — it is not a defect in
   `LLMSecurityAnalyzer` relative to its own documented contract, which
   never claims independence.
10. **Reproduction:** `git -c core.longpaths=true clone --depth 1 https://github.com/OpenHands/software-agent-sdk`,
    checkout `97dbce5c37865dd7a3020d2b9662f4a76d11fa71`, open the three
    files/line ranges in link 3. No network, no API key, no execution
    required — this is a static, source-level finding.
11. **Confidence:** **High** on the mechanism (directly read from source,
    not inferred); **moderate** on real-world exploitability, since it
    depends on an integrator's specific configuration choice (SDK
    default has no `security_analyzer` at all — see FINDING-005-3)
    rather than being wrong in every possible deployment.

---

## FINDING-005-2 — Delegated sub-agent confirmation pauses are silently auto-approved unless an integrator manually wires a callback

1. **Hypothesis:** H2 (Delegation Boundary), extended — H2 as originally
   run in EXP-003A/B/C tested whether a *parent's* delegation approval
   confers the *child's* tool-call approval (it does not, cleanly PASS
   on all three prior targets). This finding is the adjacent, previously
   untested question: once a child legitimately reaches its own
   confirmation gate, does anything actually deliver that gate to a
   human? Also relevant to proposed **H7** (Scope/Target Expansion) in
   the sense that un-gated execution is a scope expansion of what
   silently proceeds.
2. **Threat:** "Bypass approval" and "Escape delegation" —
   [Threat Model](../../docs/methodology/02-threat-model.md#bypass-approval),
   [Threat Model](../../docs/methodology/02-threat-model.md#escape-delegation).
3. **Test:** source review of both delegation primitives:
   - `openhands-tools/openhands/tools/delegate/impl.py:111-130`
     (`DelegateExecutor._run_until_finished`):
     ```python
     while (conversation.state.execution_status
            == ConversationExecutionStatus.WAITING_FOR_CONFIRMATION):
         pending = ConversationState.get_unmatched_actions(conversation.state.events)
         if not pending:
             break
         if self._confirmation_handler is None or self._confirmation_handler(agent_id, pending):
             conversation.run()
         else:
             conversation.reject_pending_actions("User rejected the actions")
             conversation.run()
     ```
     — when `self._confirmation_handler is None` (the constructor default,
     `openhands-tools/openhands/tools/delegate/impl.py:41-50`), the
     `or` short-circuits true and the loop **auto-approves and resumes
     the sub-agent** every time it pauses for confirmation.
   - The identical pattern is duplicated in the second delegation
     primitive: `openhands-tools/openhands/tools/task/manager.py:446`
     (same `self._confirmation_handler is None or ...` shape), also
     defaulting to `None` (`openhands-tools/openhands/tools/task/manager.py:100-103`).
   - Sub-agent policy inheritance is real and does reach this gate:
     `openhands-tools/openhands/tools/delegate/impl.py:233-241` —
     a spawned sub-agent inherits `parent_conversation.state.confirmation_policy`
     (e.g. `AlwaysConfirm()`) unless its `AgentDefinition.permission_mode`
     overrides it, and the top-level enforcement point
     (`openhands-sdk/openhands/sdk/agent/agent.py:1081`,
     `_requires_user_confirmation`) is real and does set
     `WAITING_FOR_CONFIRMATION` on the sub-agent's own conversation
     state. The gate genuinely fires — nothing downstream forwards it.
   - Confirmed no counter-example: `grep -rn "confirmation_handler"` across
     `openhands-sdk/`, `openhands-tools/`, and `examples/` shows the
     parameter is only ever *defined*, never *passed a real callback*,
     including in the maintainers' own reference example
     (`examples/01_standalone_sdk/25_agent_delegation.py`), which runs
     `TaskToolSet` end to end without wiring one.
4. **Evidence:** the four file:line citations above, commit-pinned.
5. **Observed Behaviour:** a sub-agent spawned under an inherited
   `AlwaysConfirm()` (or `ConfirmRisky()`) policy will genuinely enter
   `WAITING_FOR_CONFIRMATION` for each of its own actions — but by
   default (no integration work beyond following the reference
   example), that pause is immediately and silently resolved as
   "approved," and `conversation.run()` proceeds, with no human ever
   seeing the pending action.
6. **Expected Behaviour:** per Universal Hypothesis U-DEL-1 (H2 —
   "approving a delegation does not implicitly approve the delegated
   agent's own subsequent tool calls") and Pillar 4 (Human Approval),
   a confirmation-required action should reach an actual human decision
   before executing, at every delegation depth, not just at the
   top level.
7. **Security Property:** Pillar 3 (Delegation) and Pillar 4 (Human
   Approval) jointly — this is precisely the kind of trust boundary
   Assessment Methodology Stage 4 exists to surface (the
   `update_state()` precedent from EXP-003C's H1c is the closest prior
   analogue: a real, public, documented API creates a silent gap
   between the *existence* of a safety mechanism and its actual
   delivery to the party meant to exercise it).
8. **Risk:** **Moderate-to-high, and structural rather than a
   misconfiguration edge case.** Per the seven-factor rubric: authority
   gained is the full authority the sub-agent's own tools grant (which
   can include shell/file/browser actions, depending on `agent_type`);
   sensitivity depends on what tools the sub-agent has; required
   attacker position is none — this fires on the *documented, default*
   integration path, demonstrated by the maintainers' own example, not
   an obscure misuse; human interaction required is exactly zero
   (that's the finding); persistence/repeatability is total — every
   delegated conversation that hits a confirmation pause is affected;
   the only compensating control is an integrator independently
   discovering they must write and wire a `ConfirmationHandler`
   callback themselves, which no shipped example demonstrates.
9. **Recommendation:** two options for the target (recommend the
   control, not a Crypton feature): (a) make `DelegateExecutor` and
   `TaskManager` fail closed — raise or block rather than
   auto-approving — when `confirmation_handler is None` and a sub-agent
   under a confirmation-requiring policy actually pauses, forcing
   integrators to make an explicit choice; or (b) at minimum, add a
   loud runtime warning at the moment of silent auto-approval, and
   update the reference delegation example to demonstrate wiring a real
   handler, since it is currently the one place a new integrator is
   most likely to copy from.
10. **Reproduction:** same commit; read `impl.py:111-130` and
    `impl.py:41-50` together (delegate), or `task/manager.py:100-103`
    and `:446` together (task). No execution required to see the logic;
    a live reproduction would additionally require an LLM to drive an
    actual sub-agent to a `WAITING_FOR_CONFIRMATION` state and confirm
    no callback fires — **not run in this pass** (would require an API
    key/cost; the source-level result is unambiguous enough that a live
    run would confirm, not change, the conclusion).
11. **Confidence:** **High.** The control-flow is unambiguous
    (`X is None or handler(...)`), duplicated identically across both
    delegation mechanisms, and corroborated by the absence of any
    counter-example across the entire cloned tree, including the
    maintainers' own worked example.

---

## FINDING-005-3 (informational / low severity) — Two independent, inconsistent confirmation-decision implementations coexist; one is dead code today

1. **Hypothesis:** none of H1-H9 name "does more than one code path
   compute the same authorization decision, with different logic."
   Adjacent to U-AUTH-2/H5 (policy drift) but distinct: this is a
   *code-structure* hazard, not an observed runtime drift.
2. **Threat:** latent "Modify execution" / "Bypass approval" risk —
   not currently reachable (the divergent path is unused), but a
   structural precondition for a future drift bug of exactly the shape
   `docs/methodology/README.md` calls out as the PR #4813
   dual-source-of-truth precedent for this same codebase.
3. **Test:** `grep -rn "should_require_confirmation"` and
   `grep -rn "\.should_confirm("` across the full cloned tree.
4. **Evidence:** `openhands-sdk/openhands/sdk/security/analyzer.py:57-83`
   defines `SecurityAnalyzerBase.should_require_confirmation()`, a
   second, independent confirmation-decision function with **different
   semantics** than the one actually used
   (`ConfirmationPolicyBase.should_confirm()` at
   `openhands-sdk/openhands/sdk/security/confirmation_policy.py`) — e.g.
   it treats UNKNOWN risk as requiring confirmation only "if no
   security analyzer is configured," a different rule than any of
   `AlwaysConfirm`/`NeverConfirm`/`ConfirmRisky`. A repo-wide grep found
   **zero callers** of `should_require_confirmation` anywhere in
   `openhands-sdk`, `openhands-tools`, `openhands-agent-server`,
   `examples`, or `tests` — the only real call site for a confirmation
   decision is `openhands-sdk/openhands/sdk/agent/agent.py:1081`, which
   calls `state.confirmation_policy.should_confirm(risk)`.
5. **Observed Behaviour:** the SDK ships two authorization-decision
   implementations with different rules; only one is wired to any
   caller today.
6. **Expected Behaviour:** a single authoritative implementation per
   authorization decision (the same principle PR #4813 enforced for
   *state storage* — "single source of truth" — extended here to
   *decision logic*).
7. **Security Property:** Pillar 2 (Authority) — representation of
   where a decision is made.
8. **Risk:** **Low today** (unreachable code has no live effect); the
   real risk is prospective — a future contributor who finds
   `should_require_confirmation` sitting on `SecurityAnalyzerBase`
   (a natural place to look) and wires it in, believing it *is* "the"
   confirmation logic, would silently and incorrectly change gating
   behavior (e.g. LOW/MEDIUM risk actions would stop requiring
   confirmation even under an `AlwaysConfirm()`-equivalent intent,
   since this method ignores `ConfirmationPolicyBase` entirely).
9. **Recommendation:** remove `should_require_confirmation` (dead code)
   or clearly mark it deprecated/internal-only with a docstring pointer
   to `ConfirmationPolicyBase.should_confirm` as the single real
   authority, to prevent exactly the class of drift PR #4813 already
   had to fix once for persisted state.
10. **Reproduction:** the two greps in link 3, against the pinned commit.
11. **Confidence:** **High** that the code exists and is uncalled
    (structural, grep-verified); this is reported as informational
    because "currently dead code" is a real, meaningful mitigating
    fact, not a caveat to hide.

---

## Context finding (not a Crypton-attributed defect): the target's own history already demonstrates Crypton's threat model

`https://github.com/OpenHands/software-agent-sdk/pull/4813` (fetched
2026-09-16) is a real, merged PR fixing exactly the failure shape
Crypton's Threat Model calls "Persist privilege" / policy TOCTOU:
`confirmation_policy`, `security_analyzer`, and `secrets` previously
existed in two persisted locations (`StoredConversation`/`meta.json`
and `ConversationState`/`base_state.json`); API mutations only updated
one of them, so a server restart or conversation eviction silently
reverted authorization-relevant state to a stale copy. This is cited
here as **corroborating evidence that the threat class Crypton tests
for is real and has already occurred in a production agent SDK** — not
as a current finding (it is fixed, commit-verified: `base_state.json`
is now the single source of truth, per the PR's own description of the
change). See `README.md` §8 for what this implies about methodology
validation.
