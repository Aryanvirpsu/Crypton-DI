CRYPTON AGENT AUTHORIZATION ASSESSMENT

System assessed:   Crypton Agent Lab reference implementation (labs/agent-lab). **Self-assessment of Crypton's own lab. Not a customer system.**
Scope:             1 workflow: Human → Agent A → Agent B → Agent C delegated tool use (get_customer / get_order / add_case_note), isolated SQLite, in-process.
Assessment date:   2026-10-12
Crypton version:   code freeze 1b93abe887c9e28f3217cb67e0e707a5a9f618f6, branch verification/big-idea-stabilization (see evidence/repo-version.txt)

EXECUTIVE RESULT

1 authorization/delegation workflow evaluated. 5 boundary attempts plus 1 control action. All boundary attempts were blocked before any tool executed. 0 violations observed.

PROPERTY
Authority attenuation across a multi-hop delegation chain (finding ID CRYPTON-DEMO-001): authority delegated from a principal through agents may only narrow at each hop, and revoking any ancestor invalidates every descendant.

EXPECTED SECURITY BOUNDARY
Agent C, holding a delegation derived from alice via A and B, may call only `get_customer` on `cust_1`. It must not regain tools its ancestors dropped, reach resources narrowed out upstream, mint broader child grants (applies to B), or keep acting after an ancestor is revoked.

OBSERVED BEHAVIOR
- Permitted action (`get_customer`/`cust_1`): allowed; authority path alice→A→B→C recorded with chain hash.
- `add_case_note` and `get_order` by C: denied (`TOOL_NOT_ALLOWED`).
- `get_customer` on `cust_2` by C: denied (`RESOURCE_NOT_ALLOWED`).
- B issuing a child grant broader than its own: refused at issuance (`attenuation_violation`).
- Ancestor D1 revoked: C denied (`CHAIN_ANCESTOR_REVOKED`), failed hop attributed to Agent B.

ASSESSMENT
ENFORCED

EVIDENCE
- Reproduce: `labs/agent-lab/.venv/Scripts/python.exe crypton-bigidea-2026/demo/run_demo.py` → exit 0. Output: `demo/raw-output.txt`.
- Automated: `labs/agent-lab/tests/test_delegation_chain.py` (RAT-6; passes within 453/453 in `evidence/regression/agent-lab.txt`), including real-loop tests that spy on the tool function to show zero execution on denial.
- Source: `agentlab/chain.py` `verify_delegation_chain`; `agentlab/delegation.py` `grant_chained_delegation` (attenuation check raises `DelegationAttenuationError`).
- Companion check: EXP-007A shows the controller-substitution test distinguishes a deliberately weak policy (FAIL on 3 of 4 legs) from the secure one (PASS).

SECURITY IMPACT
If this control were absent, a sub-agent could exercise authority its delegator never intended (privilege amplification / confused deputy), and a revoked parent's descendants could continue to act. For this workflow, those paths are closed at issuance and again at execution.

REMEDIATION
No remediation required for the tested properties. Ongoing control: keep issuance-time attenuation and execution-time full-chain verification as separate checks, and keep the revocation and tamper regression tests in CI.

LIMITATIONS
This assessment covers the specified workflow only and is not a comprehensive penetration test or certification of the system. Additional limits for this specific run: the target is Crypton's own code and tests; human approval uses a fake WebAuthn verifier; private signing keys are handled in-process with no key-custody model; there is no network, multi-process, or production-traffic coverage; revocation propagation timing was not measured. It is not an external customer result and should not be read as one.
