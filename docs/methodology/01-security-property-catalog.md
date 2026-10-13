# Crypton Security Property Catalog v0.1

**Part of EXP-004 — Crypton Agent Authorization Assessment Methodology.**
Status: v0.1, internally derived from real evidence (EXP-001, EXP-003A/B/C,
`docs/ASSESSMENT_METHODOLOGY.md`'s eight areas, `assessment/models.py`'s
`AssessmentArea` enum), not yet field-tested against a customer's system.

**What this is:** the framework-independent vocabulary for what Crypton
checks. Every future experiment, assessment area, and customer finding
should be expressible as "this pillar, this property, this outcome" —
not "this LangGraph behavior" or "this OpenAI SDK flag." Framework
specifics belong in the [Framework Adaptation Guide](04-framework-adaptation-guide.md),
never in this document.

**What this is not:** a replacement for `docs/ASSESSMENT_METHODOLOGY.md`'s
eight areas, which remain the assessor's actual customer-engagement
procedure and are already implemented in code (`assessment/models.py::AssessmentArea`,
enforced by `Finding.__post_init__`). This catalog is a **reorganization
and extension** of that same ground truth into seven universal pillars,
with two pillars (Execution, Persistence) made explicit for the first
time because EXP-003's cross-framework testing (replay, exactly-once
execution, checkpoint/resume binding) surfaced them as properties that
needed their own vocabulary, not a footnote inside an existing area. See
"Reconciliation with existing assessment areas" at the end of this
document for the exact mapping — nothing here contradicts or supersedes
the 8-area procedure; it gives it a shared vocabulary across frameworks.

---

## How to read a pillar

Each pillar has:
- **Definition** — the one-sentence property being asked about.
- **Questions** — the assessor's actual checklist (from the user's EXP-004 spec, retained verbatim as the working checklist).
- **Grounding** — which real experiment(s) already produced evidence bearing on this pillar, and which existing assessment area it corresponds to.
- **Universal hypotheses** — testable claims stated framework-independently (feeds the [Threat Model](02-threat-model.md) and [Framework Adaptation Guide](04-framework-adaptation-guide.md)).

---

## Pillar 1 — Identity

**Definition:** whether an acting agent has a durable, owned, non-impersonable identity distinct from any human, credential, or single execution instance.

**Questions:**
- Who owns this agent?
- Can ownership change?
- Is identity cryptographically bound?
* Can identity be impersonated?
* Does identity survive persistence?

**Grounding:**
- Corresponds to `AssessmentArea.IDENTITY_OWNERSHIP` (area 1 of 8).
- Crypton's own reference implementation (`labs/agent-lab/src/agentlab/identity.py`, WebAuthn/Ed25519-bound `Agent` records) is the PASS exemplar used analogically in `assessment-engagement/reference-agentlab-dryrun/`.
- Not directly exercised by EXP-003A/B/C — those labs test authorization/delegation/execution properties on frameworks that generally treat "identity" as out of scope (the calling application supplies it). This is itself worth noting in the Framework Adaptation Guide: none of OpenAI Agents SDK, Microsoft Agent Framework, or LangGraph provide a cryptographically-bound agent identity primitive — identity is always the integrating application's responsibility.

**Universal hypotheses:**
- U-ID-1: Agent identity is a durable record, not a caller-supplied string.
- U-ID-2: Identity cannot be reassigned or reused across a revoked/decommissioned agent without a new registration event.
- U-ID-3: A component "downstream" of the identity check cannot act under an identity it wasn't actually issued.

---

## Pillar 2 — Authority

**Definition:** whether the permission to perform a specific action is explicitly created, boundedly represented, and cannot silently expand, leak, replay, or outlive the policy that granted it.

**Questions:**
- Where is authority created?
- How is authority represented?
- Can authority expand?
- Can authority leak?
- Can authority be replayed?
- Can authority outlive policy?

**Grounding:**
- Spans `AssessmentArea.DELEGATION_AUTHORITY_FLOW`, `PRE_EXECUTION_BOUNDARY`, `AUTHORIZATION_BYPASS`, and `REVOCATION_EXPIRY` (areas 2, 4, 5, 7 of 8) — "Authority" is deliberately the broadest pillar; the existing 8-area methodology already splits it into four operationally distinct check-points because a real engagement needs that granularity, but all four answer sub-questions of this one pillar.
- **"Can authority be replayed?"** is the pillar question most directly tested by EXP-003A/B/C's H1 (argument binding) and the pre-existing `finding-001` (approval bound to proposed args, not re-verified against executed args). EXP-003C's H1c (`update_state()`-mediated drift, `CANDIDATE_FINDING_APPLICATION_LEVEL`) is fresh, real evidence for this exact question.
- **"Can authority outlive policy?"** maps to H5 (policy/guardrail TOCTOU) across all three EXP-003 targets — PASS on A/B (framework re-runs guardrails at resume), PASS on C only because the application built the re-check itself (no framework primitive existed).

**Universal hypotheses:** (these are the canonical H1-H6 set, restated in pillar language — see [Threat Model](02-threat-model.md) for the attacker-objective framing of the same properties)
- U-AUTH-1 (= H1, Argument Binding): the arguments actually executed match the arguments that were authorized, verified at execution time, not assumed from approval time.
- U-AUTH-2 (= H5, Policy/Guardrail Drift): a policy change between authorization and execution is reflected before the action runs (no TOCTOU window).
- U-AUTH-3 (= H4, Persistence/Resume Binding): authority does not get re-granted or re-executed merely because state was checkpointed and resumed.
- U-AUTH-4 (= revocation area, not yet exercised by EXP-003A/B/C — see "Coverage gaps" in the Threat Model): a revoked or expired grant is denied on the very next check, not after a propagation delay.

---

## Pillar 3 — Delegation

**Definition:** whether authority passed from one principal (human or agent) to another is explicit, chainable, attenuating, and revocable — never ambient or self-granted.

**Questions:**
- Can authority be delegated?
- Can delegation be chained?
- Is attenuation enforced?
- Can child authority exceed parent authority?
- Can delegation be revoked?

**Grounding:**
- Corresponds to `AssessmentArea.DELEGATION_AUTHORITY_FLOW` + `SCOPE_ATTENUATION` (areas 2, 3 of 8).
- **"Can child authority exceed parent authority?"** is EXP-003's H2 (Delegation Boundary), tested identically across all three targets:
  - EXP-003A (OpenAI Agents SDK): PASS — `Agent.as_tool()` delegation approval never confers the nested tool's own approval.
  - EXP-003B (Microsoft Agent Framework): PASS, after harness repair — same independence confirmed via `Agent.as_tool()` with real streaming.
  - EXP-003C (LangGraph): PASS — a parent's `interrupt()` and a child subgraph's own `interrupt()` are fully independent (two distinct interrupt ids, two required decisions).
- This is Crypton's strongest cross-framework result to date: three structurally different frameworks, same universal hypothesis, same clean PASS, evidenced independently each time. It is the leading example in this catalog of what "framework-independent property, framework-specific test" actually looks like in practice.
- Crypton's own reference implementation (`labs/agent-lab/src/agentlab/chain.py::verify_attenuation`) is the PASS exemplar for "is attenuation enforced" — exact-set subset semantics for tools/resources, `child <= parent` for expiry.

**Universal hypotheses:**
- U-DEL-1 (= H2, Delegation Boundary): approving a delegation does not implicitly approve the delegated agent's own subsequent tool calls.
- U-DEL-2: at every hop, granted scope is a subset of (or equal to) the granting hop's scope — never a superset.
- U-DEL-3: revoking a parent's delegation is observable/enforceable against already-delegated children (not yet directly tested by EXP-003A/B/C — coverage gap, see Threat Model).

---

## Pillar 4 — Human Approval

**Definition:** whether a human confirmation binds to the exact proposed action, is consumed at most once, matches what actually executes, and has a defined scope, expiry, and revocability.

**Questions:**
- What exactly is approved?
- Who approved it?
- What was executed?
- Can approved arguments drift?
- Can approvals be replayed?
- Can approvals expire?
- Can approvals be revoked?

**Grounding:**
- Corresponds to `AssessmentArea.APPROVAL_BINDING` (area 6 of 8) — the single most evidence-dense pillar in Crypton's history: `finding-001` (a real, confirmed finding), EXP-002's binding-drift demonstration (semantic-rulebook repo), and EXP-003A/B/C's H1 and H3 all bear directly on this pillar.
- **"What exactly is approved?" / "Can approved arguments drift?"**: `finding-001` (LangChain `HumanInTheLoopMiddleware` composed with a benign second middleware reorders approval vs. execution); EXP-003C's H1c (`update_state()` drift on the safe pattern). Both are real, evidenced, and distinct mechanisms producing the same property failure — direct evidence that this is a recurring failure *class*, not a one-off bug in one framework.
- **"Can approvals be replayed?"**: tested as H4 across A/B/C — PASS in all three (exactly-once execution held, including across checkpoint persistence).
- **"Can approvals expire? Are they scoped narrowly or broadly ('always approve')?"**: this is H3, and it is where the three frameworks diverge most sharply:
  - EXP-003A: `always_approve` flag — supported, but **argument-blind by design** (broad only).
  - EXP-003B: `ToolApprovalRule` — explicit two-tier design (broad tool-only vs. narrow tool+arguments).
  - EXP-003C: **no primitive at all**, at either the raw `interrupt()` layer or `HumanInTheLoopMiddleware`/`InterruptOnConfig` — NOT_APPLICABLE, source-verified.

**Universal hypotheses:**
- U-APP-1 (= H1 applied to approval specifically): the approval record binds to the full argument set actually executed, not merely a tool/action label.
- U-APP-2 (= H4 applied to approval specifically): a single human decision authorizes at most one execution.
- U-APP-3 (= H3): if a "standing"/broad approval exists, its scope (tool-only vs. tool+arguments) is explicit and documented, not implicit or undiscoverable.

---

## Pillar 5 — Execution

**Definition:** whether what actually runs is deterministic, bound to what was authorized, substitution-resistant, and executes exactly once per authorization.

**Questions:**
- What actually executes?
- Is execution deterministic?
- Can execution be substituted?
- Is execution exactly once?
- Can execution resume safely?

**Grounding:**
- Corresponds most closely to `AssessmentArea.PRE_EXECUTION_BOUNDARY` (area 4 of 8), but the existing 8-area methodology frames that area around *ordering* (does the check gate the action, or does the action run and get logged after?) — it does not have a dedicated sub-area for exactly-once/substitution/resumability semantics specifically. EXP-003's H1/H2/H4 results are the concrete evidence that this deserves to be its own pillar going forward, not folded silently into "pre-execution boundary."
- "Is execution exactly once?" and "Can execution resume safely?" are H4, tested with genuine cross-process-style persistence in all three labs (EXP-003A: `RunState.from_string`/`from_json` reload; EXP-003B: `FileCheckpointStorage`; EXP-003C: real-file `SqliteSaver`, fresh connection + fresh compiled graph). All three PASS.
- "Can execution be substituted?" (tool substitution, CLAUDE.md §7 failure class 3) has **no dedicated EXP-003 test yet** — see Threat Model coverage gaps.

**Universal hypotheses:**
- U-EXEC-1 (= H4): resuming a checkpointed, already-completed unit of work does not re-execute its side effect.
- U-EXEC-2 (= H1, execution-arguments framing): the executing component sources its arguments from a binding that cannot be overridden by a channel outside the original authorization (the resume-payload-sourced vs. state-sourced distinction in EXP-003C is the concrete example of this hypothesis being violated vs. held by construction).
- U-EXEC-3: an action authorized against one tool/interface cannot be satisfied by routing through a different, less-constrained interface to the same effect (tool substitution — not yet tested, coverage gap).

---

## Pillar 6 — Persistence

**Definition:** whether checkpointed/serialized state can be used to gain, restore, or replay authority beyond what a fresh, live evaluation would grant.

**Questions:**
- What survives checkpoints?
- What survives crashes?
- Can restored state change authority?
- Can persistence replay authority?

**Grounding:**
- Partially corresponds to `AssessmentArea.REVOCATION_EXPIRY` (area 7 of 8), but that area is framed around live/production timing (does revocation propagate on the next check?) — it predates EXP-003's checkpoint/resume-specific findings and doesn't have language for "does reconstructing an agent from serialized state change what it's authorized to do." This pillar is the other genuinely new addition this catalog makes over the existing 8 areas.
- Directly evidenced by EXP-003A (RunState reload — H4), EXP-003B (FileCheckpointStorage — H4, plus the resolved H2 delegation-after-reconstruction case), and EXP-003C (SqliteSaver real-file persistence — Baseline C, H4). All PASS: reconstructing an agent from a fresh connection/process did not change or re-grant authority beyond what was already pending.
- **"Can restored state change authority?"** is the exact question EXP-003C's H1c answers YES to via `update_state()` — a legitimate, documented public API can mutate checkpointed state (and therefore the authority a resumed action will exercise) between proposal and resume. This is this pillar's most concrete cross-cutting finding: persistence mechanisms designed for durability, not security, can become a drift vector when their write APIs are reachable by more than the original proposer.

**Universal hypotheses:**
- U-PERSIST-1 (= H4): reconstructing an agent/workflow from a checkpoint does not itself constitute a fresh authorization grant.
- U-PERSIST-2 (= EXP-003C H1c): the set of principals who can legitimately mutate checkpointed/pending state is a defined, reviewed trust boundary — not an accident of which APIs happen to be public.

---

## Pillar 7 — Auditability

**Definition:** whether every authorization decision and its corresponding execution can be reconstructed after the fact, from durable records, with attribution preserved, without trusting the agent's own self-report.

**Questions:**
- Can every action be reconstructed?
- Is attribution preserved?
- Is evidence immutable?
- Can an investigator reproduce the authorization chain?

**Grounding:**
- Corresponds to `AssessmentArea.EVIDENCE_ACCOUNTABILITY` (area 8 of 8) — the least-changed pillar in this catalog, since the existing area's definition already matches this pillar's questions closely.
- Every EXP-003 evidence JSON file (`crypton_exp003*/evidence.py`'s `build_evidence`/`write_evidence` pattern, consistent in spirit across all three labs) is itself a working demonstration of this pillar applied to Crypton's *own* research process — each scenario's proposal, approval request/decision, delegation path, state boundary, and execution are recorded as durable, reconstructable JSON, independent of re-running the test. This is the direct precedent for the [Evidence & Reporting Standard](05-evidence-reporting-standard.md).

**Universal hypotheses:**
- U-AUDIT-1: a durable record exists that, by itself (no insider knowledge, no trusting the agent), answers who/what/under-what-authority/outcome for a given action.
- U-AUDIT-2: the record is written at decision time, not reconstructed later from unrelated application logs.

---

## Reconciliation with existing assessment areas

| This catalog's pillar | `AssessmentArea` enum value(s) | Relationship |
|---|---|---|
| 1. Identity | `IDENTITY_OWNERSHIP` | 1:1, unchanged |
| 2. Authority | `DELEGATION_AUTHORITY_FLOW`, `PRE_EXECUTION_BOUNDARY`, `AUTHORIZATION_BYPASS`, `REVOCATION_EXPIRY` | Pillar is broader; existing areas remain the operational check-points |
| 3. Delegation | `DELEGATION_AUTHORITY_FLOW`, `SCOPE_ATTENUATION` | 1:1 in substance |
| 4. Human Approval | `APPROVAL_BINDING` | 1:1, unchanged |
| 5. Execution | `PRE_EXECUTION_BOUNDARY` (partial) | **New explicit surface** — exactly-once/substitution/determinism were implicit, now named |
| 6. Persistence | `REVOCATION_EXPIRY` (partial) | **New explicit surface** — checkpoint/resume authority questions were not previously named at all |
| 7. Auditability | `EVIDENCE_ACCOUNTABILITY` | 1:1, unchanged |

**No code or enum change is proposed by this document.** `assessment/models.py::AssessmentArea` remains the eight areas an actual customer engagement is scoped and reported against — this catalog is the shared vocabulary that explains *why* those eight areas are the right eight, and gives Crypton's research labs (EXP-00X) a framework-independent target to test against before any customer exists. If a future engagement's findings repeatedly need Execution or Persistence as their own reportable area (rather than a sub-point under Pre-Execution Boundary or Revocation/Expiry), that is the evidence-driven trigger to propose a 9th/10th `AssessmentArea` value — not this document, unilaterally.
