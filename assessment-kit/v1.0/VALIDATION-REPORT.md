# Crypton Assessment Kit Validation Report

**EXP-006, Deliverable 12.** Independent re-review of every finding and
methodology decision from EXP-001 through EXP-005, performed with no
presumption that a prior conclusion is correct. This report **gates**
the rest of `assessment-kit/v1.0/`: the Finding Library (07), the
Standard Operating Procedure (02), and the Scoring Model (08) all
reflect this report's conclusions, not the other way around — where
this report downgrades, drops, or reclassifies something, the kit
documents built afterward already reflect that, they do not separately
re-argue it.

**Author's standing:** written as an independent reviewer with no
investment in Crypton's conclusions being right, per the mission's
explicit instruction. Every classification below states the evidence
that produced it and is falsifiable by the same evidence — a future
reviewer with the same source access should be able to check this
report's own conclusions, not just trust them.

---

## 1. Finding Revalidation Matrix

Taxonomy (mission-specified): CONFIRMED / EXPECTED DESIGN / APPLICATION
DESIGN CHOICE / DOCUMENTATION GAP / QUALITY ISSUE / FALSE POSITIVE /
SUPERSEDED / REQUIRES RE-TEST.

| ID | Original claim | Re-review finding | Classification | Changed from original framing? |
|---|---|---|---|---|
| **EXP-001 finding-001** | LangGraph agent (`target_agent.py`) skips re-approval on retry, broadening scope from one host to a /24 | Re-opened `target_agent.py`: the bypass edge in `route_after_plan` is authored by the experimenter, marked `# THE BUG` in the source's own comment. It is not a discovered flaw in LangGraph, in any documented LangGraph pattern, or in any shipped middleware — it is a hand-built specimen constructed to demonstrate a failure *class* is codeable. The project's own later work (finding-001b, finding-001c) already tested this against LangGraph's two first-party-documented HITL topologies and found **neither reproduces the bug** — both are structurally bypass-resistant by construction (no second edge into the tool-execution effect exists in either documented pattern). | **SUPERSEDED** — by finding-001d/001e, which found the real, representative version of this same failure class in real shipped code (see next row). Retained in the record as a methodology-development artifact (it is the reason 001b/c were run at all, and its own supersession by better evidence is itself a demonstration of the "weaker honest result over a stronger artifact" discipline this team already practices), but **excluded from the customer-facing Finding Library** — a synthetic, author-induced specimen has no reusable diagnostic value for what to look for in a real customer's system. | Yes — original write-up already flagged the "deterministic stand-in for an LLM" caveat, but did not yet know (at time of writing) that 001b/c would clear both real topologies. This re-review makes the supersession explicit and final. |
| **EXP-001b, finding-001b** | Interrupt-in-tool HITL pattern does not reproduce the drift | Re-checked: cited against a live, first-party LangChain docs page (`interrupts`, "Interrupts in tools", fetched 2026-09-10, exact code quoted). Structural claim ("only one path to the tool's effect") is verifiable directly from the graph construction shown in the file — no edge exists to add a bypass around. | **CONFIRMED** — a real, sourced, negative (safe-pattern) result. Belongs in the assessor's reference material as a *known-safe pattern*, not a vulnerability finding. | No change. |
| **EXP-001c, finding-001c** | Separate-review-node HITL pattern does not reproduce the drift | Re-checked: cited against the official `langchain-ai/langgraphjs` repository's own "Review Tool Calls" how-to (found via `gh search code`, not a blog), ported faithfully with an edge-by-edge fidelity table showing zero authored bypass edges. | **CONFIRMED** — second real, sourced, negative result. Same status as 001b. | No change. |
| **EXP-001d, finding-001d** | Composing real `langchain` `HumanInTheLoopMiddleware` with a benign custom `after_model` middleware (`HostEscalationMiddleware`) in natural list order produces approval/execution drift, deterministically 5/5, due to an undocumented reverse-order composition property of `langchain.agents.factory` | Re-checked: the reverse-order claim is quoted verbatim from the *installed* `langchain==1.4.0` package's own graph-building code (`graph.add_edge(...)` / the `for idx in range(len(middleware_w_after_model) - 1, 0, -1)` loop), not inferred or assumed. The A/B control (swap only the list order) flips the result to 5/5 PASS with no other change — isolates the mechanism to ordering specifically, not to `HostEscalationMiddleware`'s existence. Oracle (`agentlab.approval.compute_argument_hash`, imported unmodified from Crypton's own reference implementation) was independently sanity-checked 6/6 before and after. | ~~**CONFIRMED.**~~ **SUPERSEDED (2026-09-27) — reclassified `NO VULNERABILITY — EXPECTED FRAMEWORK BEHAVIOR`.** Applying the §9a triage rule (`01-assessor-handbook.md`): the mutator (`HostEscalationMiddleware`) is trusted, developer-authored application code in the same trust domain as the approval gate itself, permitted by the framework's own `after_model` contract to alter tool calls (proven by `PIIMiddleware`'s shipped use of the identical mechanism), and LangChain never claims approval-argument binding survives a chain of trusted `after_model` hooks composed afterward — so no framework-claimed boundary was crossed. No untrusted principal, model, tool, or input entered the path. Maintainer triage on this point is accepted; no further disclosure work is planned. Full reasoning: `labs/experiments/001-langgraph-approval-drift/CLOSURE.md`. The prior DOCUMENTATION GAP secondary tag is withdrawn along with it — it was reasoning about the same mischaracterized boundary. | **Yes.** The 2026-09-27 review found the original CONFIRMED classification had not asked who caused the mutation, what trust level they held, or what boundary the framework actually claims — see CLOSURE.md and §9a. Retained as a negative/calibration result (methodology-development value), excluded from the customer-facing library. |
| **EXP-001e, finding-001e** | A real model (`gpt-4o-mini`, two temperatures) triggers 001d's mechanism 38/40 (95%) times, via naive retry, never by the model itself choosing to broaden scope (0/40) | Re-checked: batch table, conditional (`model_broadened` × verdict) table, and the 2/40 INCONCLUSIVE anomaly (multi-tool-call turn, a stated harness limitation, not silently folded into PASS or FAIL) are internally consistent and separately disclosed. The provider-switch note (Gemini free-tier quota wall → OpenAI) is reported plainly rather than omitted, which is itself evidence of the reporting discipline being real, not just claimed. | ~~**CONFIRMED**, merged with 001d~~ **SUPERSEDED (2026-09-27) — same reclassification as finding-001d** (this is 001d's mechanism confirmed to fire on a real model; closing 001d closes this with it). The 38/40 rate remains an accurate, useful measurement of how often the (non-vulnerable) mechanism fires — it does not reopen the vulnerability question. See CLOSURE.md. | **Yes**, same basis as finding-001d above. |
| **EXP-003A H1 (Argument Binding)** | PASS — SDK-enforced per-call-id fingerprint, raises on reuse-with-drift | Consistent with the Framework Adaptation Guide's cited mechanism (a real SDK-internal check, not a Crypton assumption). No new evidence surfaced in this review to contradict it. | **CONFIRMED** (unchanged) | No change. |
| **EXP-003A/B/C H2 (Delegation Boundary)** | PASS across all three (with EXP-003B's correction: initially INCONCLUSIVE due to a test-double bug, later resolved to PASS after the harness fix was itself verified) | The EXP-003B correction is itself a good example of the methodology working as intended — a harness bug was caught, named, and fixed, and the corrected result (not the original INCONCLUSIVE) is what the Framework Adaptation Guide now reports, with the correction explicitly flagged rather than silently overwritten. | **CONFIRMED** (unchanged), with the EXP-003B self-correction noted as evidence the methodology's own error-catching worked, not swept under. | No change to the current conclusion; this re-review explicitly credits the correction as a positive quality signal rather than treating it as a blemish. |
| **EXP-003A H3 (Standing Approval)** | PASS/EXPECTED_BY_DESIGN — `always_approve` is broad, argument-blind by design | This is explicitly a *design property*, not a gap — the SDK never claims argument-level standing approval. | **EXPECTED DESIGN** (re-labeled from the original "PASS/EXPECTED_BY_DESIGN" free-text tag to the mission's formal taxonomy — same substance). | Re-labeled only, substance unchanged. |
| **EXP-003C H3 (Standing Approval)** | NOT_APPLICABLE — no standing-approval primitive exists at the raw-`interrupt()` or `InterruptOnConfig` layer, source-verified (exact field enumeration cited) | Re-checked reasoning: NOT_APPLICABLE here is explicitly distinguished from "untested" — the source was read and the primitive genuinely does not exist. This distinction (a sourced absence vs. an unexamined gap) is exactly the discipline the Evidence & Reporting Standard requires and this review found it correctly applied. | **CONFIRMED** as a correctly-reasoned NOT_APPLICABLE (not itself a "finding," but the classification of the classification is validated). | No change. |
| **EXP-003C H1c (`update_state()` argument drift)** | CANDIDATE_FINDING_APPLICATION_LEVEL — a real, documented, public LangGraph API can mutate pending checkpointed state between proposal and resume; classified as an application-design trust-boundary consideration, not a framework defect, because `update_state()` never claimed pending interrupts are frozen against it | Re-checked the risk reasoning against the seven-factor severity rubric (`docs/ASSESSMENT_METHODOLOGY.md`): authority gained is real, but exploitability requires already holding write access to the target's own checkpoint store — not a low-trust external position. This is a materially different exploitability profile than, e.g., finding-001d (which fires on nothing more than a naive model retry, no special access required). The original classification already reflected this — it did not overclaim. | **APPLICATION DESIGN CHOICE** (formal taxonomy label for what was already, informally, "not a framework defect, an integration trust-boundary"). Confidence: unchanged, **high** on mechanism, **moderate-bounded** on real-world exploitability, exactly as originally stated. | Re-labeled only; this review found no basis to upgrade or downgrade the underlying conclusion. |
| **EXP-003C H6 (Identity/Origin Binding)** | PASS (own-distinct-pending-interrupt case) / PASS_WITH_CAVEAT (unknown-thread case raises `KeyError` rather than silently no-op) | The `KeyError`-vs-no-op distinction is an application-robustness note, not a security bypass (an exception that halts is a safer failure mode than a silent no-op) — the original "PASS_WITH_CAVEAT" framing already got this right. | **EXPECTED DESIGN** (the caveat itself; the core PASS is CONFIRMED). | Re-labeled only. |
| **EXP-005 FINDING-005-1** (risk-gated confirmation satisfiable by LLM self-report) | Moderate risk, config-dependent (`LLMSecurityAnalyzer` alone vs. `EnsembleSecurityAnalyzer`-composed) | Re-verified the three cited file:line locations describe exactly what the finding claims: `_extract_security_risk` pops a model-authored field; `LLMSecurityAnalyzer.security_risk()` returns it verbatim with no independent check; `ConfirmRisky.should_confirm()` cannot distinguish an independently-derived risk value from a self-reported one. No counter-mechanism (e.g. a hidden secondary check) was found on re-review. This review additionally checked whether this is arguably **EXPECTED DESIGN** rather than a gap — i.e., did the SDK ever claim `LLMSecurityAnalyzer` is adversarially robust on its own? No such claim was found in the fetched security guide; the guide presents it as a legitimate standalone option with no caveat. | **CONFIRMED**, secondary **DOCUMENTATION GAP** (same shape as finding-001d's secondary tag: the underlying mechanism is real and undisclosed as a limitation, not falsely represented as safe). This is judged the **most generalizable** finding produced across all five experiments — see §5. | No change from EXP-005's own original framing; this review found the original hedging (config-dependent, not universal) already correct and did not need loosening or tightening. |
| **EXP-005 FINDING-005-2** (delegated confirmation pauses silently auto-approved by default) | Moderate-to-high, structural (fires on the documented default path) | Re-verified via the same greps this review re-ran independently: `confirmation_handler` is defined with a `None` default in both `DelegateExecutor` and `TaskManager`, the `is None or handler(...)` short-circuit exists identically in both, and no call site across `openhands-sdk/`, `openhands-tools/`, `openhands-agent-server/`, or `examples/` passes a real callback — including the maintainers' own delegation example. This review specifically checked whether a caller-side wrapper elsewhere in the deployable `agent-server` might supply a handler transparently (which would materially change the finding) — no such wiring was found in the reviewed `agent_server` routes. | **CONFIRMED.** Strongest single finding across the whole EXP-001–005 corpus by the "fires on the documented default path, zero attacker position required" test. | No change; re-verification strengthens confidence from "high" to **very high** (independent re-derivation of the same grep result, not just re-reading the original conclusion). |
| **EXP-005 FINDING-005-3** (duplicate, inconsistent confirmation-decision logic; dead code today) | Low/informational | Re-confirmed zero call sites for `should_require_confirmation` across the full cloned tree. This is real, but it is a code-hygiene/maintainability observation, not a currently-exploitable control failure — no live authorization decision is affected today. | **QUALITY ISSUE** (formal taxonomy label — matches the original "informational" framing exactly, now correctly categorized rather than left as a severity label doing double duty as a classification). **Excluded from the customer-facing Finding Library** (see §5) — it is a maintainer-facing code-hygiene note about the *target*, not a reusable assessment pattern for Crypton to apply against other customers. | Re-labeled and rescoped (kept as a documented observation in EXP-005's own record; not promoted into the reusable library). |
| **EXP-005 "context finding," PR #4813** | Real, merged, third-party PR corroborating the "Persist privilege" threat class | Re-verified this is not a Crypton finding at all — it is cited evidence about the *target's own history*, used to corroborate that Crypton's threat model maps to a real, independently-discovered engineering failure. No reclassification applies because it was never presented as a Crypton finding; this review confirms it should continue to be labeled explicitly as "corroborating context," not folded into the finding count. | **N/A — not a Crypton finding.** Retained as corroborating evidence only. | No change. |

**Summary:** 1 finding downgraded to SUPERSEDED and removed from the
reusable library (EXP-001 original); 2 findings merged into 1
(finding-001d + finding-001e); 1 finding reclassified from
"informational" to formal QUALITY ISSUE and excluded from the
customer-facing library (FINDING-005-3); **1 additional entry (§1
update, 2026-09-27) downgraded from CONFIRMED to SUPERSEDED /
`NO VULNERABILITY — EXPECTED FRAMEWORK BEHAVIOR` and removed from the
customer-facing library** (the merged finding-001d+e entry, FL-01 —
see `labs/experiments/001-langgraph-approval-drift/CLOSURE.md` and
§9a of `01-assessor-handbook.md`); all other findings/classifications
survive re-review either unchanged or with a formal taxonomy label
applied to substance that was already correct. **No finding was found
to be a FALSE POSITIVE.** No finding required a REQUIRES RE-TEST
classification in this pass — where evidence was static-only and
genuinely unconfirmed live (EXP-005's H1/H4/H6 hypothesis results),
the original work already labeled them INCONCLUSIVE rather than
claiming a result re-review would need to walk back.

---

## 2. Methodology Adaptation Matrix

| Adaptation | Reason introduced | Evidence | Generalizes? | Promote to core methodology? | Confidence |
|---|---|---|---|---|---|
| Pillar 5 (Execution) named as its own catalog pillar, split out of "Pre-Execution Boundary" | EXP-003's H1/H2/H4 results (exactly-once semantics, substitution-resistance, determinism) didn't have a dedicated home in the original 8-area methodology | Evidenced across **3** targets (EXP-003A/B/C, identical H4 test shape, all PASS) | Yes — tested identically across 3 structurally different frameworks, meeting the Framework Adaptation Guide's own 2-target promotion bar with room to spare | **Promote — already effectively promoted** in `docs/methodology/01-security-property-catalog.md`; this review finds no reason to reverse it | High |
| Pillar 6 (Persistence) named as its own catalog pillar, split out of "Revocation/Expiry" | EXP-003C's H1c result (checkpoint/resume mutating pending authority) had no dedicated vocabulary in the original 8 areas | Evidenced across **3** targets for the base persistence-binding property (H4, all PASS), plus **1** target (EXP-003C) for the specific mutation-vector sub-finding | Base property: yes (3 targets). Mutation-vector sub-finding: **no** — only observed once, on LangGraph specifically; EXP-005's OpenHands review found the *analogous* boundary (`base_state.json` as single source of truth) already fixed via PR #4813 by the time it was reviewed, so this review could not re-confirm the *vulnerable* pattern on a second live target, only its *absence* after a fix | **Promote the pillar itself** (well-evidenced); **do not promote H1c's specific mutation-vector as a universal expectation** — keep it as a named example of the class, not a claim every framework has this exact hole | High (pillar), Medium (specific vector's generality) |
| H10 (Signal Independence), proposed in EXP-005 | FINDING-005-1's mechanism (risk gate satisfiable by the gated actor's own self-report) has no home in H1-H9 | Evidenced against **1** target (OpenHands SDK) | **Not yet established** — a single-target hypothesis has not cleared the 2-target generalization bar Crypton's own Framework Adaptation Guide sets for promoting a shared abstraction | **Do not promote to the core H1-H9 set yet.** Keep as a documented, numbered candidate (H10) pending a second target. This is not a rejection of the hypothesis's validity — the *mechanism* is confirmed (§1) — it is a statement that "is this a generalizable universal hypothesis worth testing on every future target" has one data point, not two. | Medium — the underlying mechanism is well-evidenced; the *generalization claim* specifically is what's unproven |
| Pillar 3 (Delegation) checklist addition — "is the delegated party's own gate actually delivered to a human?", proposed in EXP-005 | FINDING-005-2's mechanism (real per-agent gate, not delivered by the delegation executor) has no explicit checklist question in the existing Delegation pillar | Evidenced against **1** target (OpenHands SDK, two independent delegation primitives within that one target) | Partially — the same underlying pattern was confirmed independently in **two structurally distinct code paths within one target** (`DelegateTool` and `TaskToolSet`), which is stronger than a single instance but still one *target* | **Promote the checklist question itself** (asking it costs nothing and the failure mode is now demonstrated to be real, not hypothetical) — but **do not yet claim a "should" default answer** (e.g. "delegation frameworks typically get this wrong") without a second target | Medium-High for "worth asking"; Low for any generalized prevalence claim |
| Threat Model outcome vocabulary split into two axes (evidence-coverage vs. target-mitigation-strength), proposed in EXP-005 | The mission's four-way outcome vocabulary (Supported/Partially mitigated/Not mitigated/Not applicable) forced apart two questions the existing five-way vocabulary (PASS/PARTIAL PASS/SPLIT/COVERAGE GAP/NOT_APPLICABLE) had conflated | This is a **documentation/reporting-clarity** adaptation, not an empirical claim — it does not need multi-target replication to validate, only internal consistency, which this review confirmed by re-deriving EXP-005's own dual-column table independently and finding it internally coherent | Yes, by construction — it is a vocabulary distinction, not a target-specific empirical result | **Promote.** Recommend updating `docs/methodology/02-threat-model.md`'s coverage table to carry both axes going forward. | High |
| Baseline Verification (Stage 7) has no defined static-review-only substitute, identified as a gap in EXP-005 | EXP-005 ran static-review-only (an explicit constraint of that experiment, not of every future engagement) and found Stage 7 simply had nothing to produce | This is a **process gap identification**, not an adaptation that was actually implemented and tested — EXP-005 skipped the stage rather than inventing a substitute | N/A — nothing was built to test for generalization | **Do not mark this "promoted"** — it is an open recommendation, not yet a defined, evidenced procedure. See §6 (Recommended Changes) for the concrete proposal, which itself still needs to be run once before being called validated. | N/A (a gap, not a claim) |
| Stage 9 ("Framework-Specific Tests") rename/scope-broadening to acknowledge framework-vs-application seams, proposed in EXP-005 | FINDING-005-2 (the strongest EXP-005 result) lived at the seam between a framework primitive (real, sound) and its own reference integration's default wiring (unsound) — the existing stage name and Framework Adaptation Guide implicitly assumed a cleaner separation | Evidenced by **1** target, but the observation is about the *pipeline's own language*, not an empirical claim about targets in general — the risk of a target having no clean framework/application boundary is structural (true of essentially any SDK-plus-reference-app shape), not something that needs replication to be plausible | Reasonably — the argument for why this matters doesn't depend on OpenHands specifically | **Promote the renaming/scope clarification** (low-cost, clearly correct once stated); do not claim it changes test *content*, only test *scope framing* | Medium-High |

**Overall Part B conclusion:** of the six adaptations/proposals produced
by EXP-004/EXP-005, two are well-evidenced generalizations already
meeting Crypton's own 2-target promotion bar (Pillars 5 and 6) and are
correctly already part of the core catalog; two are single-target,
single-instance observations that should be **retained as proposals,
not promoted to "core, expect-this-everywhere" status** (H10, the
Pillar 3 checklist addition); and two are vocabulary/process
clarifications that don't require multi-target replication and can be
promoted on internal-consistency grounds alone (the outcome-vocabulary
split, the Stage 9 rename). None should be treated as settled,
field-proven methodology — see §7 (Freeze Recommendation).

---

## 3. Threats to Validity

| Threat | Likelihood | Impact | Mitigation (already in place) | Residual risk |
|---|---|---|---|---|
| **Sample size** — 4 distinct targets total across the entire corpus (OpenAI Agents SDK, Microsoft Agent Framework, LangGraph, OpenHands SDK), plus 1 pattern-level study (EXP-001's LangChain middleware composition) | High (certain — this is a fact, not a probabilistic risk) | High — no claim in this kit should be read as "true of agent frameworks generally" | Every methodology doc and this report itself state coverage gaps explicitly rather than implying completeness (Threat Model's own coverage table, §1's per-finding scope statements) | **Real, unresolved.** Four to five targets is a research corpus, not a validated population sample. Every "generalizes" verdict in §2 is bounded to "generalizes across the targets actually tested," not "generalizes to agent systems broadly." |
| **Framework diversity** — LangChain/LangGraph appears in **two** of the five experiments (EXP-001 and EXP-003C), meaning the corpus is not five independent data points | Medium | Medium — could overstate how broadly "approval/execution drift" style bugs recur, since two experiments share underlying tooling lineage | None specific — not previously flagged as a named risk in prior EXP-00X docs | **Real, newly named by this review.** Recommend the next research target (if research resumes post-pilot) deliberately avoid the LangChain/LangGraph ecosystem to get a genuinely independent fifth data point. |
| **Target diversity** — all five targets are Python, all are open-source, none are enterprise/closed-source, none are non-US-vendor, none are Node.js/TypeScript/Go/Rust-native agent stacks, none are MCP-server-centric deployments | High | Medium-High for a customer-facing pilot — real customer stacks may be built in languages/ecosystems Crypton has never actually reviewed | Assessment methodology (7 pillars, 11 objectives) is designed framework-independent in *concept*; Framework Adaptation Guide documents the real, non-trivial cost of adapting to each new concrete target | **Real, material for pilot planning.** A first customer engagement outside Python/open-source agent stacks should be treated as adaptation work, budgeted accordingly, not assumed to be a drop-in application of existing test code. |
| **Research/confirmation bias** — all research conducted with an interest in Crypton's underlying thesis (that authorization/action-integrity failures are real and findable) being correct | Medium | Medium-High if unchecked | **Real, evidenced mitigating practice exists**: finding-001b/c were run specifically to try to disprove finding-001's generalizability and returned negative (safe-pattern) results that were published as such, not suppressed; EXP-005's target.json states "attempt to DISPROVE the methodology" as an explicit goal and this report itself continues that practice | **Partially mitigated, not eliminated.** The practice of publishing negative results is real and repeatable evidence of some bias resistance, but a single team cannot fully self-audit; recommend the Internal QA review (Deliverable 10) and any future work include an actual second reviewer, not only a second persona simulated by the same author. |
| **Selection bias** — targets chosen because they were popular, well-documented, and likely to reward investigation (a form of "look under the streetlight") | Medium-High | Medium — findings may be biased toward "well-known frameworks with good docs and active development," which is not obviously representative of a typical customer's actual (often more ad hoc, less documented) internal agent code | Target selection criteria have been stated explicitly each time (EXP-005's `README.md` §1 lists the exact criteria and candidates considered, not just the winner) | **Real, unresolved.** A customer's actual bespoke, undocumented internal agent workflow is arguably the *more* likely place to find authorization gaps (less engineering rigor than a widely-used open-source SDK), and the corpus has never tested one. |
| **Tooling limitations / scripted models** | Medium | Low-Medium | EXP-003A/B required `ScriptedModel`/`ScriptedChatClient` test doubles to exercise authorization mechanics at all; EXP-001's *core* mechanism (001d) was proven deterministically first, then **independently reconfirmed with a real model** (001e) before being trusted — the strongest evidence-discipline example in the corpus | **Low for EXP-001's specific finding** (real-model-confirmed); **not fully resolved for EXP-003A/B's H1-H6 results**, which remain scripted-model-only — a real model's tool-call formatting/timing quirks were never tested against those three targets the way EXP-001e tested them against LangChain. |
| **Static-only analysis (EXP-005)** | High (certain, by design of that experiment) | Medium-High for the two headline EXP-005 findings specifically | Stated explicitly in `target.json` and every EXP-005 deliverable; both findings marked with reasoned, not blanket, confidence ("high on mechanism, moderate on real-world exploitability") | **Real, unresolved.** FINDING-005-1 and FINDING-005-2 have never been confirmed against a live-running instance. This is the single largest "REQUIRES RE-TEST"-adjacent gap in the corpus, even though neither finding was formally given that label (both were static source citations strong enough to stand on their own for a *static-review-tier* engagement — see §6's recommendation to make this tiering explicit in the SOP). |
| **Missing runtime evidence generally** | Medium | Medium | Same as above; also applies to EXP-003's H1/H4/H6 results that were marked INCONCLUSIVE rather than assumed | Same as above. |
| **Harness assumptions** | Medium | Low | EXP-001e's single-tool-call-per-turn assumption caused 2/40 INCONCLUSIVE, disclosed rather than hidden or coerced into a PASS/FAIL | Low residual — the discipline of disclosing rather than coercing is itself the mitigation, and it worked here. Future harnesses should be built to handle multi-tool-call turns rather than treating them as an anomaly, to reduce future INCONCLUSIVE rates. |
| **Version drift** | High over time (low right now) | Medium-High if the kit is used without re-verification | Every experiment pins and states the exact version tested (`openai-agents==0.22.2`, `agent-framework==1.18.0`, `langgraph==1.2.11`+`langchain==1.4.0`, `openhands-sdk==1.49.0`) and the standing rule ("never trust a version number given in a prompt, verify the installed one") is followed consistently across all experiments, including a self-correction where it was skipped once (EXP-003B) and caught | **Real, growing over time.** None of the corpus's PASS/FAIL conclusions should be assumed to still hold against a customer using a materially newer version of any of these four frameworks without re-verification. This is a standing operational requirement, carried into the SOP (`02-standard-operating-procedure.md`) as a mandatory pre-engagement step, not an optional nicety. |
| **Unverified assumptions** | Medium | Low-Medium | Explicitly flagged per-instance throughout (e.g. EXP-005's `agent.verify()` tool-addition rule read from source but never live-tested; EXP-003C's "third HITL variant... not tested here" note in finding-001c) | Low-Medium — the discipline of flagging rather than silently assuming is consistently present across the corpus; the residual risk is simply that flagged-but-unverified items remain unverified until someone runs the test. |

**Overall Part C conclusion:** the corpus's *evidence discipline* (cite,
don't assume; disclose limitations; publish negative results; verify
installed versions) is consistently strong and itself a mitigating
factor across nearly every threat above. The corpus's *breadth* (target
count, framework diversity, language/ecosystem diversity, live-execution
coverage) is the genuine, unresolved weakness — not a flaw in how any
single experiment was run, but a real limit on how far any of its
conclusions should be assumed to generalize. This directly shapes the
Operational Readiness verdict in §5 and the Pilot Readiness
recommendation in `11-pilot-readiness-report.md`.

---

## 4. Methodology Weaknesses (Stress Test)

**Would this methodology miss an authorization flaw?** Yes, plausibly,
in at least three specific ways evidenced by this corpus itself: (1)
it has never tested concurrent/parallel multi-agent execution for race
conditions — OpenHands' `DelegateTool` runs sub-agents on separate OS
threads concurrently, and no hypothesis in H1-H9 asks whether two
in-flight delegated actions can interleave in a way that defeats a
check that assumes sequential execution; (2) tool substitution (H9) and
scope/target-set expansion (H7) remain untested against every target
reviewed to date — acknowledged gaps, not silent ones, but still real
blind spots; (3) forged-authority and replay-after-expiry (two of the
eleven threat objectives) have never been exercised against a real
target, only reasoned about.

**Would two assessors reach different conclusions?** On severity: very
likely to differ at the margins. The seven-factor rubric
(`docs/ASSESSMENT_METHODOLOGY.md`) is explicitly "a judgment call a
second assessor should be able to *reproduce* from the same evidence,"
not a formula — and a judgment call, by definition, is not guaranteed
identical between two competent people. The rubric mitigates this
(shared factors, a written reasoning requirement) but does not
eliminate it. On PASS/FAIL/INCONCLUSIVE/NOT_APPLICABLE: the base
four-outcome model is more mechanical (evidence-gated, per
`Finding.__post_init__`), but this corpus's own history shows the
outcome vocabulary strains at real edges — EXP-003 introduced
`PASS_WITH_CAVEAT` and `CANDIDATE_FINDING_APPLICATION_LEVEL` as ad hoc
extensions because the clean four-way split didn't fit every real
result. Two assessors facing a genuinely borderline result could
reasonably reach for different extensions, or none.

**Are any steps subjective?** Yes, named plainly: severity assignment
(judgment call, mitigated not eliminated, above); the boundary of what
counts as "in scope" for a given engagement (a scoping decision, not a
technical test); and — more subtly — the decision of when INCONCLUSIVE
is truly warranted versus when more effort would have produced a real
PASS/FAIL (the methodology's own rule, "requires manual review is not
an outcome," pushes hard against this, but an assessor under time
pressure could still under-invest and over-use INCONCLUSIVE as a soft
exit).

**Can evidence be interpreted multiple ways?** Yes — FINDING-005-1 is
the clearest example in this corpus: reasonable engineers could
disagree about whether an LLM self-reporting its own risk level is a
"security control that failed to be independently verified" (Crypton's
framing) or "a best-effort heuristic never meant to be a security
boundary on its own" (a defensible reading of the SDK's own
documentation, which never explicitly claims `LLMSecurityAnalyzer`
alone is adversarially robust, but also never explicitly disclaims it).
This report's classification (CONFIRMED + DOCUMENTATION GAP, §1) takes
a position on this, but a customer's engineering team could reasonably
push back, and the SOP (`02-standard-operating-procedure.md`) instructs
assessors to expect and be ready to defend this kind of pushback with
the underlying mechanism, not the label.

**Are any security properties overlapping?** Yes, by the catalog's own
admission — the Security Property Catalog's reconciliation table
already shows Pillar 2 (Authority) spans four of the original eight
assessment areas (Delegation & Authority Flow, Pre-Execution Boundary,
Authorization Bypass, Revocation/Expiry), which is a sign the pillar
boundary is not crisp. This is a real, structural fuzziness, not a
typo — Authority and Execution both claim "is the check applied
before the side effect," and Authority and Persistence both claim
territory around revocation/expiry timing. The eight-area breakdown in
`docs/ASSESSMENT_METHODOLOGY.md` is the operationally precise level
(it is what a real finding actually gets filed against); the
seven-pillar catalog is a coarser, teaching-oriented regrouping on top
of it — assessors should file findings against the eight areas, and
use the seven pillars for customer communication, not the reverse.

**Are important properties missing?** Yes, by design in some cases and
by gap in others. By design: prompt injection, jailbreaks, and
behavioral red-teaming are explicitly out of scope
(`docs/ASSESSMENT_METHODOLOGY.md`'s own stated boundary) — a deliberate
scoping choice, not an oversight, but one worth restating here because
FINDING-005-1 sits close enough to this boundary (a self-reported
signal that *could* be steered by injected content) that a customer
may reasonably ask why it's in scope while injection generally is not.
The honest answer: FINDING-005-1 is in scope because it's about
*trusting an unverified signal for an authorization decision*, which is
squarely Authority/Approval; it is not, itself, a test of whether the
model can be tricked via crafted input, which remains the excluded
add-on discipline. By gap, not design: multi-tenant isolation (can one
customer's agent instance affect another sharing infrastructure) has
no home in the current eight areas or seven pillars at all, and neither
does supply-chain/dependency integrity of the tools an agent calls.
Both are flagged here as open gaps for `docs/methodology/` to consider,
not silently absorbed into an existing pillar where they don't cleanly
fit.

**Does the methodology depend too heavily on specific frameworks?**
The *pipeline shape* (13 stages, 7 pillars, 11 objectives) does not —
it is stated and has been exercised in framework-independent language
across 4 structurally different targets. The *executable test
implementations* absolutely do — every concrete test (`ScriptedModel`,
`ScriptedChatClient`, `thread_id`/`RunState`/`FileCheckpointStorage`
handling) is real, non-reusable engineering work specific to one
target, exactly as the Framework Adaptation Guide already states
plainly ("the cost of testing a new target varies structurally"). This
is not a flaw to fix; it is a fact to budget for in every new
engagement's SOW.

**Does it overfit to the research corpus?** Some real risk: H1-H6 were
derived by noticing patterns while testing three specific frameworks,
not derived top-down from first principles before seeing any target.
Whether they are the *right* six universal hypotheses for an arbitrary
enterprise customer's bespoke, possibly framework-free agent stack is
unproven — the corpus has never tested a hand-rolled, no-framework
system (other than EXP-001's deliberately-synthetic specimen, which
does not count as a naturalistic example). This is named as an open
question for the roadmap, not resolved here.

---

## 5. Finding Library QA (Deliverable 12 Part E, cross-referenced into Deliverable 7)

| Finding | Reusable? | Scope | Customer-facing? | Include in library? | Disposition |
|---|---|---|---|---|---|
| EXP-001 original (finding-001) | No — synthetic, author-induced | Research artifact | No | **No** | Excluded; retained in `labs/experiments/001-.../` as methodology-development history only |
| EXP-001b/c (safe-pattern confirmations) | Yes, as reference material | LangChain/LangGraph-specific | Yes, as reassurance/comparison material during an engagement on this framework | **Yes — separate "known-safe patterns" appendix**, not the vulnerability-finding list proper | See `07-finding-library.md` §"Known-Safe Patterns" |
| EXP-001d + EXP-001e (merged) | ~~Yes — highest-value entry in the corpus~~ **No, as of 2026-09-27** — closed as `NO VULNERABILITY — EXPECTED FRAMEWORK BEHAVIOR` (§9a triage; see `labs/experiments/001-langgraph-approval-drift/CLOSURE.md`) | Framework-specific mechanism (`langchain` `after_model` composition), correctly-permitted trusted-middleware behavior, not a customer-assessable defect pattern | No | **No** — retired | Was `07-finding-library.md` entry FL-01; now in that document's "Excluded From This Library" table |
| EXP-003C H1c (`update_state()` drift) | Yes | Framework-specific (LangGraph checkpoint API) | Yes, with the application-design-choice framing preserved | **Yes** | `07-finding-library.md`, entry FL-02 |
| EXP-005 FINDING-005-1 (self-report risk) | **Yes, and the most conceptually generalizable entry** — the underlying anti-pattern (an authorization/risk decision gated on a signal supplied by the party being gated) is not specific to OpenHands or even to security analyzers | Application-specific instance, generalizable pattern | Yes | **Yes** | `07-finding-library.md`, entry FL-03 |
| EXP-005 FINDING-005-2 (delegation auto-approve default) | Yes, as a pattern ("does the delegation mechanism actually deliver a sub-agent's confirmation pause to a human, or silently resolve it?") even though the specific code is OpenHands-only | Application-specific instance, generalizable question | Yes | **Yes** | `07-finding-library.md`, entry FL-04 |
| EXP-005 FINDING-005-3 (duplicate dead confirmation logic) | No — target-maintainer code-hygiene note, not a customer-assessable pattern | Application-specific, internal | No | **No** | Excluded from the reusable library; remains documented in `labs/exp-005-openhands-sdk/findings.md` only |

**Merges performed:** finding-001d and finding-001e were merged into one
entry (FL-01) — they were two evidence tiers of the same finding, not
two findings. **That merged entry was itself retired on 2026-09-27**
(see above); the merge record is kept for history.

**Splits performed:** none required — no reviewed finding was found to
conflate multiple distinct issues into one entry.

**Obsolete entries removed:** EXP-001's original finding-001,
EXP-005's FINDING-005-3, and (2026-09-27) the merged finding-001d+e
entry (FL-01), per the table above.

---

## 6. Recommended Changes

1. Exclude EXP-001's original finding-001 from the customer-facing
   Finding Library; retain only as internal methodology history (done —
   see `07-finding-library.md`).
2. Merge finding-001d/e into a single library entry (done; that entry,
   FL-01, was subsequently retired on 2026-09-27 — see §1 update above
   and `labs/experiments/001-langgraph-approval-drift/CLOSURE.md`).
3. Add a "Known-Safe Patterns" appendix to the Finding Library,
   separate from the vulnerability-finding list, so 001b/c's real value
   (evidence of what *good* looks like, useful for customer
   conversations) isn't lost by having nowhere to live (done — see
   `07-finding-library.md`).
4. Split the Threat Model's outcome vocabulary into two explicit axes
   (evidence-coverage vs. target-mitigation-strength) in
   `docs/methodology/02-threat-model.md` — recommended, not yet
   executed as a doc edit; scoped as v1.1 roadmap work
   (`assessment-kit/v1.0/15-roadmap-v1.1.md`).
5. Define a static-review-only variant of Baseline Verification (Stage
   7) for engagements that cannot get live staging access — recommended,
   not yet executed; scoped as v1.1 roadmap work. Until defined, the SOP
   (`02-standard-operating-procedure.md`) instructs assessors to
   explicitly record "Baseline Verification not performed — static
   review only, no substitute defined" rather than silently skipping
   the stage, which is the interim discipline this report requires
   starting immediately, independent of when the formal substitute
   ships.
6. Do not promote H10 or the Pillar 3 delegation-delivery checklist
   question to "core, always-test" status until a second target
   confirms them — keep both as explicitly-labeled proposals in the
   checklists (done — see `03-assessment-checklists.md`).
7. Add two new coverage gaps to the Threat Model's own acknowledgment
   list: multi-tenant isolation and tool supply-chain/dependency
   integrity — neither currently has a home in the seven pillars or
   eight areas. Recommended for v1.1; not retrofitted into
   `docs/methodology/` by this report (see §7 — the existing
   methodology docs are being frozen at their current scope for pilot
   stability, not actively edited by this exercise).
8. Require every engagement's SOW to include an explicit "framework
   version verified against installed package" step before testing
   begins (already implicit in the corpus's practice; made explicit
   and mandatory in `02-standard-operating-procedure.md`, Stage 1).

---

## 7. Methodology Freeze Recommendation

| Component | Classification | Justification |
|---|---|---|
| The 4-outcome vocabulary (PASS/FAIL/INCONCLUSIVE/NOT_APPLICABLE) | **FROZEN** | Code-enforced (`assessment/models.py::Finding.__post_init__`), used consistently across every experiment in the corpus without needing to be redefined, and is the load-bearing contract the report renderer and every template in this kit depend on. Changing it now would break more than it would fix. |
| The eight assessment areas (`AssessmentArea` enum) | **FROZEN** | Code-enforced, mapped 1:1 to the customer-facing SOW pillars, exercised in the one real (dry-run) engagement mechanics test. Adding/removing an area is explicitly gated in `docs/methodology/01-security-property-catalog.md` on "repeated evidence," which has not yet occurred for any candidate 9th area. |
| The seven-factor severity rubric | **FROZEN** | Used consistently, explicitly a "judgment call, not a formula" by design — freezing the *factors*, not attempting to freeze away the residual subjectivity named in §4, which is inherent to any real severity judgment and not a defect this kit should pretend to solve by adding false precision. |
| H1-H6 | **STABLE** | Evidenced across 3-4 targets each, consistent results, but the corpus itself is only 4-5 targets total (§3) — "stable, keep testing these on every new target" rather than "frozen, beyond further evidence-gathering." |
| Pillars 5 (Execution) and 6 (Persistence) | **STABLE** | Same reasoning as H1-H6 — well-evidenced, not yet at the scale that would justify calling agent-security research on this topic "settled." |
| H7, H8, H9 | **EVOLVING** | Defined, motivated by real gaps, never yet run against any target. Keep in the standing hypothesis table (so future work extends rather than reinvents), but do not present them to a customer as tested properties. |
| H10 (Signal Independence), Pillar 3's delegation-delivery checklist question | **EVOLVING** | Single-target evidence (§2). Real and usable in an engagement today (a good question to ask), but not yet promotable to "core, expect this everywhere." |
| The Threat Model's outcome-vocabulary split (two axes) proposed by EXP-005 | **EVOLVING** | Internally consistent, low-risk to adopt, but not yet actually merged into `docs/methodology/02-threat-model.md` — the proposal exists, the doc edit doesn't. |
| Static-review-only Baseline Verification substitute | **RESEARCH** | Identified as a gap, not yet designed, not yet run once. Should not be treated as part of the operational SOP until it exists as more than a named need. |
| Multi-tenant isolation / supply-chain integrity as new pillars/areas | **RESEARCH** | Named in this report as gaps (§4, §6); no property definition, no test method, no evidence. Purely a roadmap item. |
| The customer-facing report template and SOW (`docs/business/ASSESSMENT_PACK.md`) | **STABLE** | Real, usable, but explicitly "not yet field-tested against an external customer" per that document's own status line — this report finds no reason to distrust it, but "stable, ready to use" is a different claim than "frozen, proven in the field," which only a real engagement can establish. |

**Overall Part G conclusion:** the *machinery* of the methodology
(outcome vocabulary, area taxonomy, severity rubric — the parts that
are code-enforced or used identically every single time regardless of
target) is genuinely mature enough to freeze for pilot stability. The
*content* (which hypotheses, which pillars are complete, which
adaptations generalize) is evidence-backed but still growing, and
should stay explicitly STABLE/EVOLVING rather than being prematurely
frozen — freezing content this young would optimize for the appearance
of certainty over the actual, honestly-smaller amount of certainty this
corpus has earned.

---

## 8. Final Verdict

**Operational readiness: READY WITH LIMITATIONS.**

Crypton's assessment capability is mature enough to run a real, paid,
scoped customer engagement using the machinery frozen in §7 — the
outcome vocabulary, the eight areas, the severity rubric, and the
overall 13-stage pipeline are consistent, code-enforced or
process-enforced, and have now been exercised (in whole or in stages)
five separate times with a demonstrated, repeated practice of catching
and disclosing the method's own limitations rather than hiding them.
That combination — a stable evidentiary contract plus a track record of
honest self-correction (EXP-003B's harness-bug catch, EXP-001's own
supersession of its first result, EXP-005's INCONCLUSIVE-not-guessed
discipline) — is real operational maturity, not a claim of certainty
this report is unwilling to make.

It is **not** unqualified READY, for three concrete, evidenced reasons,
none of which are fixed by more documentation:

1. **Corpus breadth (§3).** Five targets, meaningful overlap between two
   of them (LangChain/LangGraph), zero non-Python targets, zero
   enterprise/closed-source targets, zero live-execution confirmation
   for EXP-005's two findings. A first customer engagement outside this
   footprint is real adaptation work, not a drop-in application of
   existing test code, and should be scoped and priced accordingly.
2. **Methodology content maturity (§7).** Only the machinery is frozen;
   H7-H10 and two of the seven pillars are explicitly STABLE, not
   FROZEN — a first engagement should expect to extend, not just apply,
   the hypothesis set, and should budget assessor time for that.
3. **Untested end-to-end stage 1-5 execution against a live, unfamiliar
   customer system with a real intake process** — every prior run
   either entered the pipeline partway (EXP-003A/B/C, stage 6) or ran
   static-only with no customer intake at all (EXP-005). The customer
   discovery kit, questionnaires, and worksheets in this Assessment Kit
   (Deliverables 4-5) are new, never-yet-used-on-a-real-customer
   artifacts — see `11-pilot-readiness-report.md` for the specific
   GO/NO-GO reasoning this drives.

This verdict is deliberately not NOT READY: nothing in this re-review
found a load-bearing claim to be false, a finding to be fabricated, or
the core evidentiary contract to be broken. It is deliberately not
unqualified READY: the corpus is real but narrow, and pretending
otherwise would be exactly the overclaiming this entire exercise exists
to prevent.
