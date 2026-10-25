# Crypton Agent Authorization & Delegation Assessment — Reference Dry Run

**Crypton Reference Agent Lab — Methodology Dry Run**

Date: 2026-09-02  
Assessor: Crypton (self-assessment, synthetic engagement)

---

## Engagement Metadata

| Field | Value |
|---|---|
| Assessment ID | ENG-REFERENCE-AGENTLAB-DRYRUN |
| Client identifier | Crypton (self-assessment) |
| System identifier | labs/agent-lab reference implementation |
| Assessment type | agent_authorization_delegation_assessment |
| Tested workflow | reference-agent tool-call workflow (get_customer / get_order / add_case_note) |
| Tested layer(s) | delegation, authorization, approval, chain_verification, tracing |
| Environment | Agent Lab isolated SQLite (disposable, non-production — see DECISIONS.md invariant 10) |
| Target product | labs/agent-lab |
| Target version | v0.6.1 |
| Target commit | ca2f52f0f5e10fc9c809cfd04bf3bd2d9890476d |
| Testing window | *(not recorded)* |
| Configuration | Default Agent Lab config; FakeWebAuthnVerifier (documented simulation, see limitations); deterministic uuid4/time patch scoped to this script only (see module docstring). |
| Excluded scope | Gateway enforcement, real WebAuthn wiring, production key custody, MCP — none exist yet (see DECISIONS.md v0.6 'what v0.6 deliberately did not build'). |

## 1. Decision Sheet

This reference dry run exercises all eight assessment areas against a known,
controlled workflow (Crypton's own Agent Lab), using real primitive calls
rather than fabricated results, to validate the assessment delivery kit's
Finding/Evidence/Report model before it is used on an external engagement.

Of eight areas assessed: five reached **ENFORCED** with cited evidence and a
fully satisfied seven-gate evidence record (scope attenuation across a signed
multi-hop chain, pre-execution enforcement, bypass-attempt denial, approval
one-shot/replay protection, and evidence traceability itself), one reached
**VIOLATED** (a delegation grant broader than the stated task's need — a
realistic over-provisioning finding, not a defect in enforcement; Gate 7
confirms the over-grant genuinely crosses this workflow's own stated
least-privilege boundary, not merely that something looked excessive), one is
**UNVERIFIABLE** (agent identity provisioning process — no external record
source exists to inspect in this synthetic engagement, which is an evidence
gap, not evidence that the underlying concept is unrepresentable), and one is
**NOT_APPLICABLE** (live revocation-propagation timing — no production
traffic exists to time).

This is not an external customer result and should not be read as one.

| Outcome | Count |
|---|---|
| VIOLATED | 1 |
| UNVERIFIABLE | 1 |
| UNREPRESENTABLE | 0 |
| NOT APPLICABLE | 1 |
| ENFORCED | 5 |

**Highest severity among VIOLATED findings:** MEDIUM (AD-02)

## 2. Authority Map / Authority Diff

Authority Model for AD-02 (Delegation grant exceeds the stated task's need), derived deterministically from this engagement's own findings.json/evidence_manifest.json through the Authority Model presentation projection — see assessment-engagement/authority-model-fixtures/agentlab-ad02/authority-view.json for the underlying structured projection this was rendered from.

```
WORKFLOW            Reference Agent Lab workflow (get_customer / get_order / add_case_note)
PRINCIPAL           UNKNOWN
ACTOR               reference-agent-a
PROPERTY            Delegation & Authority Flow
OUTCOME             VIOLATED


EXPECTED / PERMITTED

Principal UNKNOWN [UNKNOWN]
  |
  [derived]
  v
Grant (no formal grant_id -- see derivation_notes)
    allowed_tools=[add_case_note, get_customer], allowed_resources=[customer_ids:cust_1]


OBSERVED

Grant
  |
  ════
  v
Action authz_00000000000040008000000000000011
    << UNEXPECTED AUTHORITY


FIRST DIVERGENCE
Grant

TYPE
SCOPE_EXPANSION

UNEXPECTED EXERCISE
Action authz_00000000000040008000000000000011


SEMANTIC GAPS

Principal
UNKNOWN -- Not named in AD-02's own cited evidence (E-001/E-002/E-003).


EVIDENCE

| Element | Status / Basis | Evidence |
|---|---|---|
| Action (get_order) | OBSERVED | E-003 |
| Principal | UNKNOWN | E-001 |


ACTION REQUIRED

Scope each delegation's allowed_tools and resource_constraints to the specific task being authorized, not a broader standing grant convenient for multiple future tasks.

OWNER
TBD

RETEST PASSES WHEN
Re-verify after the grant is narrowed to get_customer on cust_1 only (plus the approved add_case_note) — get_order on cust_2 should then DENY.
```

## 3. Action Queue

| Finding | Severity | Recommendation | Retest condition |
|---|---|---|---|
| AD-02 | MEDIUM | Scope each delegation's allowed_tools and resource_constraints to the specific task being authorized, not a broader standing grant convenient for multiple future tasks. | Re-verify after the grant is narrowed to get_customer on cust_1 only (plus the approved add_case_note) — get_order on cust_2 should then DENY. |

## 4. Findings Summary

| Outcome | Count |
|---|---|
| VIOLATED | 1 |
| UNVERIFIABLE | 1 |
| UNREPRESENTABLE | 0 |
| NOT APPLICABLE | 1 |
| ENFORCED | 5 |

**VIOLATED findings by severity:**

| Severity | Count |
|---|---|
| MEDIUM | 1 |

**Coverage by area:**

| Area | Findings |
|---|---|
| Agent Identity & Ownership | 1 |
| Delegation & Authority Flow | 1 |
| Scope Attenuation / Privilege Inheritance | 1 |
| Pre-Execution Authorization Boundary & Bypass | 1 |
| Authorization Bypass Paths | 1 |
| Human-Approval Binding (Replay / TOCTOU / Drift) | 1 |
| Revocation / Expiry Propagation | 1 |
| Evidence / Accountability Reconstruction | 1 |

## 5. Finding Cards

### AD-01 — Agent identity provisioning/offboarding process

- **Area:** Agent Identity & Ownership
- **Outcome:** UNVERIFIABLE
- **Tested layer:** process/documentation

**Observation**

This synthetic engagement has no external identity/HR/IT system of record — agents were registered directly by this dry run's own script, not through a reviewable provisioning process.

**Expected control**

A documented, auditable process governs who may register an agent, on whose authority, and how it is deprovisioned.

**Evidence:** none cited.

**Limitations**

UNVERIFIABLE, not UNREPRESENTABLE: the concept of a provisioning process is entirely representable (Agent Lab's own `agents.register_agent` call is one), this synthetic engagement simply has no external provisioning/offboarding documentation or process interview to inspect — an evidence-access gap, not a semantic one.

### AD-02 — Delegation grant exceeds the stated task's need

- **Area:** Delegation & Authority Flow
- **Outcome:** VIOLATED
- **Severity:** MEDIUM
- **Tested layer:** delegation_grant

**Observation**

The root delegation grants get_customer, get_order, and add_case_note against both cust_1 and cust_2, while the documented workflow task (workflow.md) only requires get_customer (plus approved add_case_note) against cust_1. E-003 shows the excess scope is technically usable (get_order on cust_2 is ALLOWed), confirming this is a real, exploitable over-grant, not a theoretical one.

**Expected control**

Delegated authority should be scoped to the minimum tools and resources the stated task actually requires (least privilege at grant time).

**Invariant**

A delegation's allowed_tools/resource_constraints are a subset of the stated task's actual need.

**Risk / impact**

An agent with more standing authority than its task needs increases the blast radius of a compromised or misbehaving agent instance, and of a manipulated model output that requests an in-scope-but-unintended action — enforcement will correctly ALLOW it, because it was, in fact, granted. Gate 7 (security-claim/trust-boundary triage): this workflow's own stated scope-of-work (workflow.md) is the claimed least-privilege boundary, and E-003 shows it was genuinely crossed — the excess grant is usable, not merely present in configuration.

**Evidence**

- `E-001` — Root delegation dlg_00000000000040008000000000000004 granted to reference-agent-a: allowed_tools=['add_case_note', 'get_customer', 'get_order'], resource_constraints={'customer_ids': ['cust_1', 'cust_2']}
- `E-002` — Stated task need for this workflow: get_customer on cust_1 only (plus approved add_case_note).
- `E-003` — Root grant technically authorizes get_order on cust_2, outside the stated task need — decision_id=authz_00000000000040008000000000000011, decision=ALLOW, reason_code=ALLOW

**Evidence gate status**

- G1 Representability: satisfied
- G2 Observability: satisfied
- G3 Controlled execution: satisfied
- G4 Effect confirmation: satisfied
- G5 Calibration/control: satisfied
- G6 Reproduction: satisfied
- G7 Security-claim triage: satisfied
- Notes: Synthetic self-assessment of Crypton's own Agent Lab reference implementation — see this module's source comment above this constant for the specific, honest basis of each gate in this context; G5/G6 lean on Agent Lab's own 263-test adversarial suite as calibration/reproduction evidence, per docs/ASSESSMENT_METHODOLOGY.md's sanctioned self-assessment exception, not a fresh negative case or repeated run constructed separately per finding.

**Recommendation**

Scope each delegation's allowed_tools and resource_constraints to the specific task being authorized, not a broader standing grant convenient for multiple future tasks.

**Retest condition:** Re-verify after the grant is narrowed to get_customer on cust_1 only (plus the approved add_case_note) — get_order on cust_2 should then DENY.

**Confidence:** High — directly observed in the delegation's own persisted grant.

### AD-03 — Multi-hop delegation narrows correctly and is cryptographically verifiable

- **Area:** Scope Attenuation / Privilege Inheritance
- **Outcome:** ENFORCED
- **Tested layer:** delegation_chain

**Observation**

A second-hop delegation (agent-a -> agent-b) narrows the root grant to a single tool and a single resource id, is Ed25519-signed by agent-a's registered key, and verifies as a 2-hop chain with a full, reconstructable authority path back to the human operator. Gate 7: this directly satisfies the claimed boundary that a child delegation may only narrow, never widen, its parent's grant (chain.verify_attenuation's subset rule).

**Expected control**

Authority delegated across multiple hops must only ever narrow, never widen, and each hop must be independently, cryptographically attributable.

**Invariant**

child.allowed_tools <= parent.allowed_tools and child's resource id sets are subsets of the parent's, for every hop.

**Evidence**

- `E-004` — 2-hop chain verification, delegation dlg_00000000000040008000000000000005: allowed=True, depth=2, authority_path=[('human', 'user-reference'), ('agent', 'agt_00000000000040008000000000000001'), ('agent', 'agt_00000000000040008000000000000002')], chain_hash=597dc7f902b91c260150530f980ee817585198121a78389a8f10c3ed26cec5a2

**Evidence gate status**

- G1 Representability: satisfied
- G2 Observability: satisfied
- G3 Controlled execution: satisfied
- G4 Effect confirmation: satisfied
- G5 Calibration/control: satisfied
- G6 Reproduction: satisfied
- G7 Security-claim triage: satisfied
- Notes: Synthetic self-assessment of Crypton's own Agent Lab reference implementation — see this module's source comment above this constant for the specific, honest basis of each gate in this context; G5/G6 lean on Agent Lab's own 263-test adversarial suite as calibration/reproduction evidence, per docs/ASSESSMENT_METHODOLOGY.md's sanctioned self-assessment exception, not a fresh negative case or repeated run constructed separately per finding.

**Recommendation**

Where a workflow delegates authority across more than one agent, verify — not merely document — that each hop narrows the prior one and is independently attributable.

### AD-04 — In-scope request authorized, then executed exactly once

- **Area:** Pre-Execution Authorization Boundary & Bypass
- **Outcome:** ENFORCED
- **Tested layer:** agent_execution_loop

**Observation**

A request within the delegation's granted scope (get_customer/cust_1) was driven through the real agent execution loop (AgentRunner.run), not verified in isolation — a spy on the underlying tool function confirms it ran exactly once, and only after a persisted ALLOW decision was recorded. Gate 7: the claimed boundary (authorization must gate execution, not follow it) is the exact thing exercised and confirmed here, not an incidental observation.

**Expected control**

Authorization must be evaluated, and must return ALLOW, before a protected action's side effect occurs — never inferred after the fact from a result.

**Control / calibration**

Paired with AD-05's denied, zero-side-effect negative case driven through the same real loop.

**Evidence**

- `E-005` — In-scope request (get_customer/cust_1) driven through the real agent loop: ALLOW, and tools.TOOL_FUNCTIONS['get_customer'] was called exactly 1 time(s) (spy-instrumented for this run) — decision_id=authz_0000000000004000800000000000000b, decision=ALLOW, reason_code=ALLOW

**Evidence gate status**

- G1 Representability: satisfied
- G2 Observability: satisfied
- G3 Controlled execution: satisfied
- G4 Effect confirmation: satisfied
- G5 Calibration/control: satisfied
- G6 Reproduction: satisfied
- G7 Security-claim triage: satisfied
- Notes: Synthetic self-assessment of Crypton's own Agent Lab reference implementation — see this module's source comment above this constant for the specific, honest basis of each gate in this context; G5/G6 lean on Agent Lab's own 263-test adversarial suite as calibration/reproduction evidence, per docs/ASSESSMENT_METHODOLOGY.md's sanctioned self-assessment exception, not a fresh negative case or repeated run constructed separately per finding.

**Recommendation**

Confirm the customer's own enforcement point sits before the tool/action executes, not as a post-hoc audit check.

### AD-05 — Out-of-scope request denied pre-execution, zero side effects

- **Area:** Authorization Bypass Paths
- **Outcome:** ENFORCED
- **Tested layer:** agent_execution_loop

**Observation**

A request for a tool never granted to the acting agent (add_case_note, dropped at the chained hop) was driven through the real agent execution loop and denied with a specific reason code — a spy on the underlying tool function confirms it was never called. Gate 7: this is the negative case for AD-04's claimed boundary — the same boundary, confirmed from the other direction.

**Expected control**

A request outside the effective delegated scope must be denied, not silently narrowed or ignored, and the denial must be evidenced.

**Control / calibration**

Paired with AD-04's allowed, exactly-once positive case driven through the same real loop.

**Evidence**

- `E-006` — Out-of-scope request (add_case_note, never granted to agent-b) driven through the real agent loop: DENY, and tools.TOOL_FUNCTIONS['add_case_note'] was called 0 time(s) (spy-instrumented for this run — zero confirms no side effect occurred) — decision_id=authz_00000000000040008000000000000010, decision=DENY, reason_code=TOOL_NOT_ALLOWED

**Evidence gate status**

- G1 Representability: satisfied
- G2 Observability: satisfied
- G3 Controlled execution: satisfied
- G4 Effect confirmation: satisfied
- G5 Calibration/control: satisfied
- G6 Reproduction: satisfied
- G7 Security-claim triage: satisfied
- Notes: Synthetic self-assessment of Crypton's own Agent Lab reference implementation — see this module's source comment above this constant for the specific, honest basis of each gate in this context; G5/G6 lean on Agent Lab's own 263-test adversarial suite as calibration/reproduction evidence, per docs/ASSESSMENT_METHODOLOGY.md's sanctioned self-assessment exception, not a fresh negative case or repeated run constructed separately per finding.

**Recommendation**

Verify bypass attempts against the customer's actual enforcement point produce a denial with an auditable reason, not just an application-level error the caller could suppress.

### AD-06 — Approval is one-shot and replay-resistant

- **Area:** Human-Approval Binding (Replay / TOCTOU / Drift)
- **Outcome:** ENFORCED
- **Tested layer:** approval_consumption

**Observation**

An approval tied to one exact tool call is consumed successfully once; a second consumption attempt with identical arguments is denied (APPROVAL_ALREADY_CONSUMED), both evidenced. Gate 7: the claimed boundary (an approval authorizes exactly one execution) is exercised directly, with its own within-finding negative case (the replay attempt).

**Expected control**

A human approval must bind to the exact action approved and be usable at most once — replaying it must not authorize a second execution.

**Invariant**

An Approval's (approved -> consumed) transition is atomic and one-way; a second verify_and_consume_approval on the same approval_id denies.

**Control / calibration**

The replay attempt (second consumption, same approval_id) is this finding's own negative case, run immediately after the successful first consumption.

**Evidence**

- `E-007` — Approval apr_00000000000040008000000000000012: first consumption allowed (decision_id=authz_00000000000040008000000000000013), replay attempt denied (decision_id=authz_00000000000040008000000000000014, reason_code=APPROVAL_ALREADY_CONSUMED)

**Evidence gate status**

- G1 Representability: satisfied
- G2 Observability: satisfied
- G3 Controlled execution: satisfied
- G4 Effect confirmation: satisfied
- G5 Calibration/control: satisfied
- G6 Reproduction: satisfied
- G7 Security-claim triage: satisfied
- Notes: Synthetic self-assessment of Crypton's own Agent Lab reference implementation — see this module's source comment above this constant for the specific, honest basis of each gate in this context; G5/G6 lean on Agent Lab's own 263-test adversarial suite as calibration/reproduction evidence, per docs/ASSESSMENT_METHODOLOGY.md's sanctioned self-assessment exception, not a fresh negative case or repeated run constructed separately per finding.

**Recommendation**

Confirm the customer's own approval mechanism binds to exact action arguments (not just tool name) and enforces single-use, not just a UI-level disabled button.

**Limitations**

The underlying WebAuthn ceremony is a documented Agent-Lab-only simulation (FakeWebAuthnVerifier) — this evidences the binding/consumption protocol, not a live authenticator.

### AD-07 — Live revocation-propagation timing

- **Area:** Revocation / Expiry Propagation
- **Outcome:** NOT APPLICABLE
- **Tested layer:** n/a — no live traffic in this engagement

**Observation**

This synthetic engagement has no live/production traffic against which to measure the delay between a revocation action and enforcement taking effect.

**Expected control**

Revoking a delegation or signing key should take effect on the next authorization check, with no meaningful propagation delay.

**Evidence:** none cited.

**Limitations**

Applicable to a live/staging engagement with real traffic; not to a one-shot synthetic dry run.

### AD-08 — Every authorization decision above is independently traceable

- **Area:** Evidence / Accountability Reconstruction
- **Outcome:** ENFORCED
- **Tested layer:** authorization_decisions_ledger

**Observation**

AD-02 through AD-06's evidence all resolve to real rows in one append-only authorization_decisions ledger, addressable by decision_id, without needing to reconstruct anything from partial logs. Gate 7: the claimed boundary (every consequential decision must be reconstructible from durable records alone) is demonstrated by actually doing the reconstruction, not merely asserting the ledger exists.

**Expected control**

Every consequential authorization decision must be reconstructible after the fact — who, what, when, and the outcome — from durable evidence, not inferred.

**Evidence**

- `E-008` — Every decision cited above (E-003, E-005, E-006, E-007) is a row in the same append-only authorization_decisions ledger, queryable by decision_id — this evidence entry demonstrates that traceability itself, not a separate control.

**Evidence gate status**

- G1 Representability: satisfied
- G2 Observability: satisfied
- G3 Controlled execution: satisfied
- G4 Effect confirmation: satisfied
- G5 Calibration/control: satisfied
- G6 Reproduction: satisfied
- G7 Security-claim triage: satisfied
- Notes: Synthetic self-assessment of Crypton's own Agent Lab reference implementation — see this module's source comment above this constant for the specific, honest basis of each gate in this context; G5/G6 lean on Agent Lab's own 263-test adversarial suite as calibration/reproduction evidence, per docs/ASSESSMENT_METHODOLOGY.md's sanctioned self-assessment exception, not a fresh negative case or repeated run constructed separately per finding.

**Recommendation**

Confirm the customer's own system persists an equivalent durable, queryable decision ledger — not only human-readable logs that expire or aren't decision-scoped.

## 6. Verified Controls

| Finding | Title | Evidence |
|---|---|---|
| AD-03 | Multi-hop delegation narrows correctly and is cryptographically verifiable | `E-004` |
| AD-04 | In-scope request authorized, then executed exactly once | `E-005` |
| AD-05 | Out-of-scope request denied pre-execution, zero side effects | `E-006` |
| AD-06 | Approval is one-shot and replay-resistant | `E-007` |
| AD-08 | Every authorization decision above is independently traceable | `E-008` |

## 7. Limitations

- This is a **self-assessment of Crypton's own reference implementation**,
  not an independent assessment of a customer's system. Findings here
  demonstrate the *methodology and reporting mechanics*, not Crypton's
  differentiation versus any competitor or category.
- No external identity/HR/IT provisioning system exists for this synthetic
  engagement — AD-01 is UNVERIFIABLE for exactly that reason (evidence could
  not be obtained within this engagement's scope), not because the
  underlying concept is unrepresentable, and not because the question is
  unimportant.
- No live/production traffic exists for this engagement — AD-07 is
  NOT_APPLICABLE for the same reason.
- Agent Lab's WebAuthn step-up is a documented simulation
  (`webauthn.FakeWebAuthnVerifier`) — AD-06's approval-binding evidence
  demonstrates the *protocol design* (argument-hash binding, one-shot
  consumption), not a live authenticator ceremony.
- All findings trace to real, reproducible primitive calls made by
  `agentlab.assessment.reference_dryrun.build_reference_engagement` —
  re-running it against a fresh database reproduces the same
  ENFORCED/VIOLATED/UNVERIFIABLE/NOT_APPLICABLE pattern *and* the same
  evidence IDs byte-for-byte (delegation_id, decision_id, chain_hash, etc.
  are deterministic within this synthetic fixture — see the module
  docstring's "Determinism" note); a real customer engagement's evidence
  IDs are ordinary random values, not fixed like this reference fixture's.
- This engagement predates the 2026-09-30 schema expansion for most
  structured metadata fields (`target_commit`, `testing_window`, etc.) —
  those are set where genuinely known (this *is* Agent Lab, so its own
  product/version/commit are knowable) and left `None` where they
  describe a concept this synthetic, single-run engagement doesn't have
  (e.g. no multi-day `testing_window` — it is one scripted run).

## 8. Retest Record

| Finding | Outcome | Retest condition |
|---|---|---|
| AD-02 | VIOLATED | Re-verify after the grant is narrowed to get_customer on cust_1 only (plus the approved add_case_note) — get_order on cust_2 should then DENY. |

## 9. Evidence Index

| ID | Type | Source | Description | Reference | Classification |
|---|---|---|---|---|---|
| `E-001` | config_excerpt | labs/agent-lab (delegation.grant_delegation call in this dry run) | Root delegation dlg_00000000000040008000000000000004 granted to reference-agent-a: allowed_tools=['add_case_note', 'get_customer', 'get_order'], resource_constraints={'customer_ids': ['cust_1', 'cust_2']} | delegations.delegation_id = 'dlg_00000000000040008000000000000004' | RELEASE_APPROVED |
| `E-002` | config_excerpt | assessment-engagement/reference-agentlab-dryrun/workflow.md | Stated task need for this workflow: get_customer on cust_1 only (plus approved add_case_note). |  | RELEASE_APPROVED |
| `E-003` | authorization_log | chain.verify_delegation_chain (this dry run) | Root grant technically authorizes get_order on cust_2, outside the stated task need — decision_id=authz_00000000000040008000000000000011, decision=ALLOW, reason_code=ALLOW | authorization_decisions.decision_id = 'authz_00000000000040008000000000000011' | RELEASE_APPROVED |
| `E-004` | execution_trace | chain.verify_delegation_chain (this dry run) | 2-hop chain verification, delegation dlg_00000000000040008000000000000005: allowed=True, depth=2, authority_path=[('human', 'user-reference'), ('agent', 'agt_00000000000040008000000000000001'), ('agent', 'agt_00000000000040008000000000000002')], chain_hash=597dc7f902b91c260150530f980ee817585198121a78389a8f10c3ed26cec5a2 | delegations.delegation_id = 'dlg_00000000000040008000000000000005' | RELEASE_APPROVED |
| `E-005` | authorization_log | AgentRunner.run() (this dry run), instance_id=00000000-0000-4000-8000-00000000000a | In-scope request (get_customer/cust_1) driven through the real agent loop: ALLOW, and tools.TOOL_FUNCTIONS['get_customer'] was called exactly 1 time(s) (spy-instrumented for this run) — decision_id=authz_0000000000004000800000000000000b, decision=ALLOW, reason_code=ALLOW | authorization_decisions.decision_id = 'authz_0000000000004000800000000000000b' | RELEASE_APPROVED |
| `E-006` | authorization_log | AgentRunner.run() (this dry run), instance_id=00000000-0000-4000-8000-00000000000f | Out-of-scope request (add_case_note, never granted to agent-b) driven through the real agent loop: DENY, and tools.TOOL_FUNCTIONS['add_case_note'] was called 0 time(s) (spy-instrumented for this run — zero confirms no side effect occurred) — decision_id=authz_00000000000040008000000000000010, decision=DENY, reason_code=TOOL_NOT_ALLOWED | authorization_decisions.decision_id = 'authz_00000000000040008000000000000010' | RELEASE_APPROVED |
| `E-007` | approval_record | approval.py (this dry run) | Approval apr_00000000000040008000000000000012: first consumption allowed (decision_id=authz_00000000000040008000000000000013), replay attempt denied (decision_id=authz_00000000000040008000000000000014, reason_code=APPROVAL_ALREADY_CONSUMED) | approvals.approval_id = 'apr_00000000000040008000000000000012' | RELEASE_APPROVED |
| `E-008` | execution_trace | authorization_decisions table (this dry run) | Every decision cited above (E-003, E-005, E-006, E-007) is a row in the same append-only authorization_decisions ledger, queryable by decision_id — this evidence entry demonstrates that traceability itself, not a separate control. |  | RELEASE_APPROVED |

## 10. Technical Appendix

### Scope

This is a **synthetic, self-assessed reference engagement**, not an external customer assessment.

The workflow assessed is Crypton's own Agent Lab reference implementation
(`labs/agent-lab`) — a small, real (not simulated) agent loop with three tools
(`get_customer`, `get_order`, `add_case_note`), backed by real Delegation,
Approval, and multi-hop chain-verification primitives.

Purpose: demonstrate that the Agent Authorization & Delegation Assessment
methodology (`docs/ASSESSMENT_METHODOLOGY.md`) and its Finding/Evidence/Report
model can represent a real engagement's full range of outcomes — ENFORCED,
VIOLATED, UNREPRESENTABLE, and NOT_APPLICABLE — using genuine evidence, not
fabricated results.

This dry run does not validate Crypton's differentiation against any specific
competitor or claim external customer readiness. It validates the delivery
kit's reporting mechanics against a known, controlled workflow.

### Workflow Assessed

**Reference workflow:** a single registered agent ("reference-agent"), granted
a Delegation by a human operator, executing tool calls against Agent Lab's
seeded demo data (`cust_1` owned by the reference operator; `cust_2` owned by
a different principal in the seed data and used here only as an out-of-scope
resource id, never actually read).

**Stated task need for this workflow (as would appear in a real engagement's
scope-of-work document):** the agent needs to read one customer's profile
(`get_customer` on `cust_1`) and, with human approval, add a case note for
that same customer. Nothing else.

**Actual grant issued (see AD-02):** the delegation used in this dry run
grants `get_customer`, `get_order`, and `add_case_note` against both `cust_1`
and `cust_2` — broader than the stated task need, on purpose, to produce a
realistic over-provisioning finding.

A second, narrower chained delegation (Agent A -> Agent B) is also issued to
demonstrate multi-hop scope attenuation (AD-03).

### Methodology

Findings above use the fixed five-outcome vocabulary defined in `docs/ASSESSMENT_METHODOLOGY.md`: **ENFORCED** (the claimed security property was evidenced and held), **VIOLATED** (evidenced, every evidence-sufficiency gate satisfied, and Gate 7 security-claim/trust-boundary triage affirmatively confirmed a real claimed boundary was crossed), **UNREPRESENTABLE** (the target has no semantic state to express this property at all), **UNVERIFIABLE** (the property is representable but sufficient evidence could not be obtained), and **NOT_APPLICABLE** (the control does not apply to this workflow). Every ENFORCED and VIOLATED finding above cites at least one entry in §9's evidence index and a fully satisfied seven-gate evidence_gate_status (G1 Representability, G2 Observability, G3 Controlled execution, G4 Effect confirmation, G5 Calibration/control, G6 Reproduction, G7 Security-claim triage) — passing G1-G6 alone is never sufficient for VIOLATED. Severity, where set, follows the contextual rubric in the methodology document — it is not a fixed mapping from finding category to severity level.

### All Recommendations (also itemized per-VIOLATED-finding in §3)

- **AD-02:** Scope each delegation's allowed_tools and resource_constraints to the specific task being authorized, not a broader standing grant convenient for multiple future tasks.
- **AD-03:** Where a workflow delegates authority across more than one agent, verify — not merely document — that each hop narrows the prior one and is independently attributable.
- **AD-04:** Confirm the customer's own enforcement point sits before the tool/action executes, not as a post-hoc audit check.
- **AD-05:** Verify bypass attempts against the customer's actual enforcement point produce a denial with an auditable reason, not just an application-level error the caller could suppress.
- **AD-06:** Confirm the customer's own approval mechanism binds to exact action arguments (not just tool name) and enforces single-use, not just a UI-level disabled button.
- **AD-08:** Confirm the customer's own system persists an equivalent durable, queryable decision ledger — not only human-readable logs that expire or aren't decision-scoped.
