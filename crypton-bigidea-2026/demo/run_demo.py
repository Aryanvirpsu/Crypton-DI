"""Verification driver for CRYPTON-DEMO-001 (no new product code).

Rebuilds the exact Human -> Agent A -> Agent B -> Agent C scenario from
labs/agent-lab/tests/test_delegation_chain.py::three_hop_chain using only
existing agentlab APIs, then asks the existing verifier
(agentlab.chain.verify_delegation_chain) about several actions and prints
expected vs. observed. Exit code 1 if any observed != expected.

Run:  labs/agent-lab/.venv/Scripts/python.exe demo/run_demo.py
"""
from __future__ import annotations

import sys
import tempfile
from pathlib import Path

from agentlab import agents, authorization, chain, delegation, signing_keys
from agentlab.db import connect, seed
from agentlab.identity import AgentInstance, HumanIdentity

alice = HumanIdentity(user_id="user-alice", username="alice", cred_id="dev-cred-alice")

with tempfile.TemporaryDirectory() as tmp:
    db = connect(Path(tmp) / "demo.db")
    seed(db)

    a = agents.register_agent(db, alice, name="agent-a", version="0.1.0")
    b = agents.register_agent(db, alice, name="agent-b", version="0.1.0")
    c = agents.register_agent(db, alice, name="agent-c", version="0.1.0")
    key_a, priv_a = signing_keys.create_signing_key(db, alice, a.agent_id)
    key_b, priv_b = signing_keys.create_signing_key(db, alice, b.agent_id)

    d0 = delegation.grant_delegation(
        db, alice, agent_id=a.agent_id,
        allowed_tools=frozenset({"get_customer", "get_order", "add_case_note"}),
        resource_constraints={"customer_ids": {"cust_1", "cust_2"}}, ttl_seconds=3 * 3600)
    d1 = delegation.grant_chained_delegation(
        db, alice, issuer_agent_id=a.agent_id, issuer_private_key_pem=priv_a,
        issuer_signing_key_id=key_a.key_id, parent_delegation_id=d0.delegation_id,
        subject_agent_id=b.agent_id, allowed_tools=frozenset({"get_customer", "get_order"}),
        resource_constraints={"customer_ids": {"cust_1", "cust_2"}}, ttl_seconds=2 * 3600)
    d2 = delegation.grant_chained_delegation(
        db, alice, issuer_agent_id=b.agent_id, issuer_private_key_pem=priv_b,
        issuer_signing_key_id=key_b.key_id, parent_delegation_id=d1.delegation_id,
        subject_agent_id=c.agent_id, allowed_tools=frozenset({"get_customer"}),
        resource_constraints={"customer_ids": {"cust_1"}}, ttl_seconds=1 * 3600)

    print("DELEGATION CHAIN (as issued)")
    print("  D0 alice -> A : tools={get_customer,get_order,add_case_note} customers={cust_1,cust_2} T+3h")
    print("  D1 A -> B     : tools={get_customer,get_order}               customers={cust_1,cust_2} T+2h")
    print("  D2 B -> C     : tools={get_customer}                         customers={cust_1}        T+1h")
    print()

    def verify(tool, rid, delegation_id=None):
        inst = AgentInstance.new()
        inst = AgentInstance(instance_id=inst.instance_id, started_at=inst.started_at,
                             agent_id=c.agent_id, human_user_id=alice.user_id, session_id="", task_id="")
        return chain.verify_delegation_chain(
            db, principal=alice, agent_id=c.agent_id, instance=inst, task_id="",
            delegation_id=delegation_id or d2.delegation_id, tool_name=tool,
            resource_type="customer_ids", resource_id=rid)

    failures = 0

    def check(label, res, expect_allowed, expect_reason=None):
        global failures
        ok = (res.allowed == expect_allowed) and (expect_reason is None or res.denial_reason == expect_reason)
        failures += 0 if ok else 1
        print(f"[{'OK ' if ok else 'BAD'}] {label}")
        print(f"      expected: {'ALLOW' if expect_allowed else 'DENY'}" + (f" ({expect_reason})" if expect_reason else ""))
        print(f"      observed: {'ALLOW' if res.allowed else 'DENY'}" + (f" ({res.denial_reason})" if res.denial_reason else ""))
        if res.allowed:
            print(f"      authority_path: {res.authority_path}")
            print(f"      depth={res.delegation_depth} chain_hash={res.chain_hash}")

    check("C get_customer cust_1 (inside D2)", verify("get_customer", "cust_1"), True)
    check("C add_case_note cust_1 (dropped at D0->D1; amplification attempt)",
          verify("add_case_note", "cust_1"), False, authorization.TOOL_NOT_ALLOWED)
    check("C get_order cust_1 (dropped at D1->D2; amplification attempt)",
          verify("get_order", "cust_1"), False, authorization.TOOL_NOT_ALLOWED)
    check("C get_customer cust_2 (in D0/D1 scope, narrowed out at D2; resource escape)",
          verify("get_customer", "cust_2"), False, authorization.RESOURCE_NOT_ALLOWED)

    # Issuance-time attenuation: B tries to mint a child broader than D1 allows.
    try:
        delegation.grant_chained_delegation(
            db, alice, issuer_agent_id=b.agent_id, issuer_private_key_pem=priv_b,
            issuer_signing_key_id=key_b.key_id, parent_delegation_id=d1.delegation_id,
            subject_agent_id=c.agent_id, allowed_tools=frozenset({"get_customer", "add_case_note"}),
            resource_constraints={"customer_ids": {"cust_1"}}, ttl_seconds=3600)
        print("[BAD] B minted a child delegation broader than its own authority")
        failures += 1
    except Exception as e:  # noqa: BLE001 - we want whatever the library raises
        print(f"[OK ] issuance of broadened child D2' refused: {type(e).__name__}: {e}")

    # Revocation of an ancestor kills the descendant.
    delegation.revoke_delegation(db, alice, d1.delegation_id)
    r = verify("get_customer", "cust_1")
    check("after revoking D1 (ancestor), C get_customer cust_1", r, False, chain.CHAIN_ANCESTOR_REVOKED)
    print(f"      failed_hop: {r.failed_hop} (agent-b = {b.agent_id})")

    print()
    print("RESULT:", "ALL OBSERVATIONS MATCH EXPECTED" if failures == 0 else f"{failures} MISMATCH(ES)")
    db.close()
    sys.exit(1 if failures else 0)
