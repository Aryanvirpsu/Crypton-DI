# Crypton Threat Model v0.1

**Part of EXP-004.** Status: v0.1, derived from real evidence and from the
already-settled failure-class list in `CLAUDE.md` §7 / `docs/strategy/2026-09-10-action-integrity-thesis.md`
§6. This document does not change or re-argue that settled list — see
"Relationship to CLAUDE.md §7" below. It reframes the same ground truth
as **attacker objectives** (what the attacker is trying to achieve),
which is the framework-independent language the [Security Property Catalog](01-security-property-catalog.md)
needs on its threat side, and adds an explicit **current coverage
matrix** — which objectives Crypton has real, reproducible evidence
about (from EXP-001 and EXP-003A/B/C), and which remain open gaps for
future experiments. Coverage gaps are reported honestly, not hidden;
per the standing rule across every EXP-00X lab, "a weaker honest result
is preferable to a stronger artifact."

---

## The eleven attacker objectives

```
Acquire authority        Bypass approval
Expand authority         Modify execution
Reuse authority          Escape delegation
Forge authority          Persist privilege
Replay authority
Confuse authority
Lose attribution
```

Each objective below states: the objective in one sentence, the pillar(s) it threatens, the corresponding CLAUDE.md §7 failure class (where one exists), and its current evidence status.

---

### Acquire authority
**Objective:** obtain an authorization the attacker was never granted, by impersonating an identity or fabricating a credential rather than by escalating one it already legitimately holds.
**Threatens:** Pillar 1 (Identity), Pillar 2 (Authority).
**Nearest §7 class:** not directly named — closest is Credential Inheritance, but that describes a child *legitimately* receiving too much, not an attacker forging entry. This objective is genuinely adjacent to, not identical with, any of the ten.
**Status: COVERAGE GAP.** None of EXP-003A/B/C test identity forgery, because none of the three target frameworks (OpenAI Agents SDK, Microsoft Agent Framework, LangGraph) provide a cryptographically-bound agent identity primitive at all — identity is uniformly the integrating application's responsibility (Security Property Catalog, Pillar 1). There is nothing framework-native to attack yet. This is itself a real, evidenced observation (a NOT_APPLICABLE-shaped result, source-confirmed by omission across all three targets), not an untested hunch — but it means "Acquire authority" can only be tested against Crypton's own reference identity layer (`labs/agent-lab/src/agentlab/identity.py`) or a real customer's actual identity binding, not against any of the agent frameworks studied so far.

### Expand authority
**Objective:** cause an action to execute against a broader scope than was authorized (more targets, more resources, more effect).
**Threatens:** Pillar 2 (Authority), Pillar 3 (Delegation).
**§7 class:** **Scope Escalation** (`isolate X` authorized → `isolate X, Y, Z` executed) — 1:1 match.
**Status: COVERAGE GAP** in EXP-003A/B/C specifically. H1 (Argument Binding) tests whether *arguments* drift, which is adjacent but distinct from whether the *scope/target set* itself expands beyond a single resource. No EXP-003 scenario constructs a multi-target action where the executed target set could grow past what was proposed. Recommended as a concrete EXP-005 hypothesis (see Assessment Methodology's "universal hypotheses" list — this should become H7 once formalized, not smuggled into H1's existing evidence).

### Reuse authority
**Objective:** use a single, already-spent authorization to justify more than one execution.
**Threatens:** Pillar 4 (Human Approval), Pillar 5 (Execution).
**§7 class:** overlaps **Semantic Replay** and **Retry-Induced Overreach**.
**Status: PASS, well-evidenced.** This is exactly H4 (Persistence/Resume Binding), tested with genuine persistence across all three targets:
- EXP-003A: `RunState.from_string`/`from_json` reconstruction, repeated resume of a completed run.
- EXP-003B: `FileCheckpointStorage`, multi-pending partial resolution, repeated resume of completed state.
- EXP-003C: real-file `SqliteSaver` (fresh connection + fresh compiled graph on reload), repeated resume of a completed thread.
All three PASS: exactly-once execution held in every case. **Caveat:** this covers the *checkpoint/resume* reuse vector specifically. The agent-loop-level "retry/replanning produces a fresh execution repeating the same logical authorization" framing in §7's Semantic Replay (e.g., a model retries after a transient tool error and re-proposes the same already-approved action without a fresh approval round) has not been isolated as its own scenario in any EXP-003 lab — recommend as a distinct future hypothesis rather than assuming H4's result already covers it.

### Forge authority
**Objective:** fabricate an authorization record (an approval, a grant, a delegation) that was never actually issued by the real approving principal.
**Threatens:** Pillar 2 (Authority), Pillar 4 (Human Approval).
**§7 class:** not directly named — adjacent to Cross-Agent Confused Deputy but distinct (confused deputy uses a *real* higher-privilege agent's authority; forgery fabricates a record without any real principal acting at all).
**Status: COVERAGE GAP.** Not tested by any EXP-00X lab to date — all EXP-003 scenarios use real, framework-issued approval objects (`ToolApprovalItem`, `FunctionApprovalRequest/Response`, `Interrupt`); none attempt to synthesize a forged one and submit it through a side channel. Genuinely open.

### Replay authority
**Objective:** resubmit a previously-valid authorization artifact (an approval token, a delegation grant, a signed decision) after its legitimate window has passed.
**Threatens:** Pillar 2 (Authority), Pillar 4 (Human Approval), Pillar 6 (Persistence).
**§7 class:** **Semantic Replay**, **Approval / Execution Drift**.
**Status: PASS**, same evidence as "Reuse authority" above (H4) — the two objectives are tested by the same experiments in this catalog's current state, though conceptually distinct (reuse = same authorization used twice; replay = a stale/expired one used again). Both readings are addressed by the H4 exactly-once results. No EXP-003 scenario specifically tests replaying an authorization *after an explicit expiry*, since none of the three labs modeled approval expiry as a first-class concept — worth flagging alongside "Persist privilege" below as a shared gap around time-bounded authority.

### Confuse authority
**Objective:** cause a more-privileged component to act on behalf of a less-privileged one, outside the less-privileged component's own effective authority (the classic confused-deputy pattern).
**Threatens:** Pillar 2 (Authority), Pillar 3 (Delegation).
**§7 class:** **Cross-Agent Confused Deputy** — 1:1 match.
**Status: PASS, partial.** H2 (Delegation Boundary) is directly relevant and clean across all three frameworks (parent delegation approval never confers the child/nested tool's own approval — see [Security Property Catalog Pillar 3](01-security-property-catalog.md#pillar-3--delegation)). This is real evidence against one specific confused-deputy vector: a parent cannot use its own authority to silently authorize a child's more sensitive action. It is **not** a full test of the general confused-deputy class, which also covers cases where a low-privileged agent's *output* (not an approval it holds) manipulates a high-privileged agent into acting — closer to a prompt-injection-adjacent vector explicitly out of scope for this methodology (see `docs/ASSESSMENT_METHODOLOGY.md`'s "Prompt injection... explicitly not part of this methodology's lead assessment"). Marking this PASS *for the delegation-approval-boundary reading specifically*, coverage gap for the broader confused-deputy class.

### Lose attribution
**Objective:** cause an action to execute such that no durable record can later establish who/what caused it or under what authority.
**Threatens:** Pillar 7 (Auditability).
**§7 class:** not named among the ten — §7's list is entirely about authority *mutating*, not about forensic reconstruction *failing*. This is a genuinely distinct objective this threat model adds; it belongs conceptually next to §7's list rather than forced into it.
**Status: COVERAGE GAP as a target-framework property.** Every EXP-00X lab's own evidence-JSON discipline (`build_evidence`/`write_evidence`, consistent across A/B/C) demonstrates Crypton reconstructing *its own test process* well — but no EXP-003 hypothesis asks whether the **target framework itself** provides durable, reconstructable attribution (e.g., does OpenAI Agents SDK/Agent Framework/LangGraph persist "which human decision authorized this specific execution" anywhere a customer's own investigator could find it later, independent of Crypton's test harness). Recommended as a concrete future hypothesis (tentatively H8, alongside Scope Escalation as H7).

### Bypass approval
**Objective:** cause a gated action to execute without a required human approval decision ever being obtained.
**Threatens:** Pillar 4 (Human Approval), Pillar 5 (Execution).
**§7 class:** **Approval / Execution Drift** (partial — drift assumes an approval happened but didn't bind correctly; a clean bypass is a stricter, prior case: no approval at all).
**Status: PASS, well-evidenced.** Every EXP-003 lab's Baseline B (reject → zero side effects) and the `EXECUTED == []` assertions preceding every approval step across all H1-H6 scenarios are direct, repeated evidence that none of the three frameworks executes a gated action before a decision is recorded. This is the most heavily-replicated result across the entire EXP-003 series (asserted in nearly every one of EXP-003C's 12 tests alone, e.g. `test_exp003c.py`'s `assert EXECUTED == []` checkpoints).

### Modify execution
**Objective:** cause the arguments, target, or interface actually used at execution time to differ from what was authorized.
**Threatens:** Pillar 5 (Execution), Pillar 2 (Authority).
**§7 class:** **Argument Drift** (arguments differ) + **Tool Substitution** (interface differs, same effective action).
**Status: SPLIT.**
- Argument Drift: **PASS / CANDIDATE_FINDING_APPLICATION_LEVEL**, this catalog's single best-evidenced objective — `finding-001` (real, confirmed, LangChain middleware ordering), EXP-002's binding-drift demonstration, EXP-003A's SDK-enforced per-call fingerprint (PASS by framework design), EXP-003C's H1a/H1b/H1c triple (state-sourced pattern PASS, resume-sourced pattern fails by design as a deliberate unsafe-pattern control, `update_state()` drift is the one real application-level candidate finding this whole series has produced).
- Tool Substitution: **COVERAGE GAP**, untested by any EXP-00X lab so far. No scenario constructs "approved via tool A, executed via tool B with equivalent effect." Recommended future hypothesis (tentatively H9).

### Escape delegation
**Objective:** a delegated child agent acts with more authority than the parent actually delegated to it — either by receiving over-broad credentials or by discovering privileges the delegation grant never named.
**Threatens:** Pillar 3 (Delegation).
**§7 class:** **Delegation Escalation**, **Credential Inheritance**.
**Status: PARTIAL PASS.** H2 (tested identically to "Confuse authority" above) shows the child's *own tool gate* cannot be bypassed via the parent's delegation approval — real evidence against one escape vector. What remains untested: whether the child, once invoked, actually holds a *narrower* credential/tool set than the parent (true attenuation of the runtime capability, not just of the approval gate). None of EXP-003A/B/C's fixtures model real scoped credentials passed to a delegated sub-agent — they use fixed fake tool sets. Crypton's own `chain.py::verify_attenuation` (Agent Lab reference implementation) is the only place this has actually been exercised, and only against Crypton's own primitive, not a third-party framework. Coverage gap for third-party targets.

### Persist privilege
**Objective:** continue acting under an authority grant after it has been revoked or has expired.
**Threatens:** Pillar 2 (Authority), Pillar 6 (Persistence).
**§7 class:** **Revocation Failure** — 1:1 match.
**Status: COVERAGE GAP.** Consistent with `docs/ASSESSMENT_METHODOLOGY.md`'s own area 7 guidance (revocation/expiry timing is NOT_APPLICABLE for a one-shot synthetic scenario with no live/production traffic to measure propagation delay against), none of EXP-003A/B/C model revocation or expiry as first-class, time-bounded concepts — all three labs test deterministic, single-pass scenarios with no "time passes, then a previously-granted authority is checked again" structure. This is the most consistently open objective across the entire series and the strongest candidate for the next experiment's primary focus (tentatively EXP-005, since it requires a materially different test shape — either real wall-clock/TTL behavior or an explicit revoke-then-recheck scenario — than anything H1-H6 currently construct).

---

## Coverage summary

| Objective | Status | Primary evidence |
|---|---|---|
| Acquire authority | COVERAGE GAP | none — no framework provides an identity primitive to attack |
| Expand authority | COVERAGE GAP | H1 is adjacent but doesn't test scope/target-set expansion |
| Reuse authority | PASS | H4, all three targets |
| Forge authority | COVERAGE GAP | none |
| Replay authority | PASS (checkpoint vector only) | H4, all three targets |
| Confuse authority | PASS (delegation-boundary reading only) | H2, all three targets |
| Lose attribution | COVERAGE GAP (as a target-framework property) | Crypton's own evidence discipline only |
| Bypass approval | PASS | Baseline B + `EXECUTED==[]` pattern, all three targets |
| Modify execution | SPLIT — PASS/CANDIDATE_FINDING (arguments) / COVERAGE GAP (substitution) | `finding-001`, EXP-002, H1 (all targets) |
| Escape delegation | PARTIAL PASS (gate only, not credential scoping) | H2, all three targets; `chain.py` (Crypton reference only) |
| Persist privilege | COVERAGE GAP | none — requires a live/time-bounded test shape not yet built |

**Reading this table honestly:** five of eleven objectives are real coverage gaps, and two more are only partially tested. This is not a weakness to paper over — it is the actual, current state of Crypton's evidence base, and it directly generates the next round of experiment priorities (scope escalation, tool substitution, revocation/expiry, identity forgery, attribution durability, credential-level attenuation on a third-party target). See the [Assessment Methodology](03-assessment-methodology-v0.1.md)'s hypothesis-numbering convention for how these become H7+ in a future EXP-005.

## Relationship to CLAUDE.md §7

CLAUDE.md §7's ten failure classes are the settled, logged list — this
document does not revise them. The eleven objectives above are a
different cut through mostly the same ground truth (attacker-intent
language instead of before/after examples), plus two objectives ("Forge
authority", "Lose attribution") that don't map cleanly onto any of the
ten and are flagged as such rather than force-fit. If EXP-005+ evidence
eventually shows these two deserve to be folded into CLAUDE.md's
official list, that is a Decision Log entry to propose then, with that
evidence cited — not a change made by this document.
