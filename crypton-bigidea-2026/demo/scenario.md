# Scenario — Delegated authority must only narrow (CRYPTON-DEMO-001)

Reuses Crypton's existing deterministic RAT-6 scenario (`labs/agent-lab/tests/test_delegation_chain.py::three_hop_chain`). `demo/run_demo.py` rebuilds it with the same library calls and prints expected vs. observed. Nothing new was added to the product.

Reproduce (from repo root, Windows):
```
labs/agent-lab/.venv/Scripts/python.exe crypton-bigidea-2026/demo/run_demo.py
labs/agent-lab/.venv/Scripts/python.exe -m pytest labs/agent-lab/tests/test_delegation_chain.py -v   # run from labs/agent-lab
```
Output: `demo/raw-output.txt` (part 1). Exit code 0 means every observation matched expectation.

## Actors
- **Principal/User:** `user-alice` (human identity, Ed25519-signed token model).
- **Agent A**, **Agent B**, **Agent C:** three registered agents owned by alice, each with an identity (`agt_…`); A and B each hold an Ed25519 signing key.
- **Protected resources/actions:** tools `get_customer`, `get_order`, `add_case_note` on customers `cust_1` (alice's) and `cust_2` (bob's, in seeded data).

## Intended policy
| Hop | Issuer → subject | Tools | Customers | TTL |
|---|---|---|---|---|
| D0 | alice → A | get_customer, get_order, add_case_note | cust_1, cust_2 | 3h |
| D1 | A → B (signed by A) | get_customer, get_order | cust_1, cust_2 | 2h |
| D2 | B → C (signed by B) | get_customer | cust_1 | 1h |

- A may use all three tools on both customers. B may read customers and orders. C may only read customer `cust_1`.
- Rule: each hop may only **narrow** the parent's tools, resources and expiry; revoking any ancestor invalidates all descendants.

## Delegation
Alice grants A broad authority (D0). A re-delegates a strict subset to B (D1); B re-delegates a strict subset to C (D2). Each chained grant is signed by its issuer's Ed25519 key and carries the parent's hash.

## Boundary (actions that must NOT be permitted)
1. C calls `add_case_note` (A had it, A dropped it at D1).
2. C calls `get_order` (B had it, B dropped it at D2).
3. C calls `get_customer` on `cust_2` (A and B had it, B narrowed it out at D2).
4. B tries to **issue** a child grant containing `add_case_note` (broader than B's own D1).
5. A revoked ancestor (D1) and C keeps acting.

## Test
Call `chain.verify_delegation_chain` as C for the allowed action and for each boundary action (1–3, 5); attempt the broadened issuance (4) via `delegation.grant_chained_delegation`. The same scenario is also driven through the real `AgentRunner` loop in `test_rat6_allowed_action_executes_exactly_once_through_the_real_loop` and `test_rat6_permission_amplification_denied_through_real_loop` (spy on the tool function proves zero side effects on denial).

## Result
| # | Action | Expected | Observed | Reason code |
|---|---|---|---|---|
| 0 | C `get_customer` cust_1 | ALLOW | ALLOW (depth 3; path alice→A→B→C; chain hash recorded) | — |
| 1 | C `add_case_note` cust_1 | DENY | DENY | `TOOL_NOT_ALLOWED` |
| 2 | C `get_order` cust_1 | DENY | DENY | `TOOL_NOT_ALLOWED` |
| 3 | C `get_customer` cust_2 | DENY | DENY | `RESOURCE_NOT_ALLOWED` |
| 4 | B issues broadened child | refused | refused: `DelegationAttenuationError` ("child allowed_tools is not a subset of parent allowed_tools") | — |
| 5 | After revoking D1, C `get_customer` cust_1 | DENY | DENY, failed hop = Agent B | `CHAIN_ANCESTOR_REVOKED` |

- **Crypton classification:** **ENFORCED** for every boundary tested.
- **Relevant trace:** `demo/raw-output.txt`; authority path and `chain_hash` in row 0.
- **Remediation implication:** none for this scenario. The control that prevents the failure is the one tested: attenuation checked at issuance **and** re-verified over the whole chain at execution, so a stale or forged leaf can't outlive its ancestors.

## Scope of this result (read before quoting)
Crypton's own reference implementation passed Crypton's own tests. That shows the primitive behaves as designed in a controlled in-process SQLite setting. It does **not** show Crypton found a defect in anyone else's system, nor that the same properties hold in a deployed multi-service system.

---

# Companion scenario — Controller substitution (CRYPTON-DEMO-002, shows the test can fail)

EXP-007A (`labs/exp-007-principal-continuity`, FROZEN) models: user U grants Agent A a capability; party B, who is **not** in U's authority chain, becomes the effective controller. Run: `python -m exp007.scenario_007a` (output: `demo/raw-output.txt`, part 2).

| Leg | Controller | `vulnerable_policy` | `secure_policy` |
|---|---|---|---|
| a | U, signed | ALLOW | ALLOW (`CONTINUITY_VERIFIED`) |
| b | B unsigned, spoofing U | **ALLOW → FAIL** | STEP_UP (`CONTINUITY_UNVERIFIED`) |
| c | B forged signature | **ALLOW → FAIL** | STEP_UP (`CONTINUITY_UNVERIFIED`) |
| d | B authenticated as B, no U→B delegation | **ALLOW → FAIL** | STEP_UP (`CONTROLLER_NOT_AUTHORIZED`) |
| cal | U, agent lacks capability | DENY | DENY (`NO_CAPABILITY`) |

Verdicts: vulnerable policy FAIL, secure policy PASS. A deliberately weaker `authenticated_only_policy` is rejected by the gate (`test_authenticated_only_mutant_fails_specifically_on_authenticated_substitute`). **The "vulnerable" policy is a strawman Crypton wrote to show the test discriminates, not a real product.** Its legitimate value is demonstrating that leg d (authenticated but unauthorized controller) is a distinct failure from bad authentication and from missing capability.
