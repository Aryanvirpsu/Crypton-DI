"""
Unit-level tests for the deterministic pre-execution approval pipeline
(approval.verify_and_consume_approval) — one test per check/reason code,
exercised directly rather than through a full AgentRunner.run(). See
test_agent.py for the end-to-end integration tests (approval required,
substitution denied, replay denied, approval-cannot-override-delegation).
"""

import time

from agentlab import authorization
from agentlab.agents import register_agent
from agentlab.approval import (
    APPROVAL_AGENT_MISMATCH,
    APPROVAL_ALREADY_CONSUMED,
    APPROVAL_ARGUMENT_MISMATCH,
    APPROVAL_CANCELLED,
    APPROVAL_DELEGATION_MISMATCH,
    APPROVAL_EXPIRED,
    APPROVAL_INSTANCE_MISMATCH,
    APPROVAL_NOT_FOUND,
    APPROVAL_PENDING,
    APPROVAL_PRINCIPAL_MISMATCH,
    APPROVAL_RESOURCE_MISMATCH,
    APPROVAL_TASK_MISMATCH,
    APPROVAL_TOOL_MISMATCH,
    approve,
    cancel,
    request_approval,
    verify_and_consume_approval,
)
from agentlab.delegation import grant_delegation
from agentlab.webauthn import FakeWebAuthnVerifier

ALL_TOOLS = frozenset({"get_customer", "get_order", "add_case_note"})
VERIFIER = FakeWebAuthnVerifier()


def setup(db_conn, human, *, instance_id="inst_1", task_id="task_1"):
    agent = register_agent(db_conn, human, name="a", version="0.1.0")
    grant = grant_delegation(
        db_conn,
        human,
        agent_id=agent.agent_id,
        allowed_tools=ALL_TOOLS,
        resource_constraints={"customer_ids": {"cust_1", "cust_2"}},
        ttl_seconds=3600,
    )
    appr = request_approval(
        db_conn,
        human,
        agent_id=agent.agent_id,
        delegation_id=grant.delegation_id,
        tool_name="add_case_note",
        resource_type="customer_ids",
        resource_id="cust_1",
        proposed_arguments={"customer_id": "cust_1", "note": "refund approved"},
        instance_id=instance_id,
        task_id=task_id,
        ttl_seconds=300,
    )
    return agent, grant, appr


def approved(db_conn, human, appr):
    return approve(db_conn, human, appr.approval_id, verifier=VERIFIER)


def check(db_conn, *, human, agent, grant, appr, instance_id="inst_1", task_id="task_1", arguments=None):
    return verify_and_consume_approval(
        db_conn,
        approval_id=appr.approval_id,
        principal=human,
        agent_id=agent.agent_id,
        instance_id=instance_id,
        task_id=task_id,
        delegation_id=grant.delegation_id,
        tool_name="add_case_note",
        resource_type="customer_ids",
        resource_id="cust_1",
        validated_arguments=arguments or {"customer_id": "cust_1", "note": "refund approved"},
    )


def test_valid_approval_allows_and_consumes(db_conn, alice):
    agent, grant, appr = setup(db_conn, alice)
    approved(db_conn, alice, appr)
    decision = check(db_conn, human=alice, agent=agent, grant=grant, appr=appr)
    assert decision.allowed
    assert decision.reason_code == authorization.ALLOW
    assert decision.decision_id.startswith("authz_")


def test_argument_substitution_denied(db_conn, alice):
    """approve note=X, execute with a different note -> DENY. The
    canonical TOCTOU / substitution-protection property."""
    agent, grant, appr = setup(db_conn, alice)
    approved(db_conn, alice, appr)
    decision = check(
        db_conn,
        human=alice,
        agent=agent,
        grant=grant,
        appr=appr,
        arguments={"customer_id": "cust_1", "note": "SOMETHING ELSE ENTIRELY"},
    )
    assert not decision.allowed
    assert decision.reason_code == APPROVAL_ARGUMENT_MISMATCH


def test_resource_substitution_denied(db_conn, alice):
    """approved for cust_1, executed against cust_2 -> DENY, even though
    cust_2 is in the delegation's own resource scope."""
    agent, grant, appr = setup(db_conn, alice)
    approved(db_conn, alice, appr)
    decision = verify_and_consume_approval(
        db_conn,
        approval_id=appr.approval_id,
        principal=alice,
        agent_id=agent.agent_id,
        instance_id="inst_1",
        task_id="task_1",
        delegation_id=grant.delegation_id,
        tool_name="add_case_note",
        resource_type="customer_ids",
        resource_id="cust_2",  # not cust_1
        validated_arguments={"customer_id": "cust_2", "note": "refund approved"},
    )
    assert not decision.allowed
    assert decision.reason_code == APPROVAL_RESOURCE_MISMATCH


def test_wrong_principal_denied(db_conn, alice, bob):
    agent, grant, appr = setup(db_conn, alice)
    approved(db_conn, alice, appr)
    decision = verify_and_consume_approval(
        db_conn,
        approval_id=appr.approval_id,
        principal=bob,
        agent_id=agent.agent_id,
        instance_id="inst_1",
        task_id="task_1",
        delegation_id=grant.delegation_id,
        tool_name="add_case_note",
        resource_type="customer_ids",
        resource_id="cust_1",
        validated_arguments={"customer_id": "cust_1", "note": "refund approved"},
    )
    assert not decision.allowed
    assert decision.reason_code == APPROVAL_PRINCIPAL_MISMATCH


def test_wrong_agent_denied(db_conn, alice):
    agent, grant, appr = setup(db_conn, alice)
    approved(db_conn, alice, appr)
    other_agent = register_agent(db_conn, alice, name="other", version="0.1.0")
    decision = verify_and_consume_approval(
        db_conn,
        approval_id=appr.approval_id,
        principal=alice,
        agent_id=other_agent.agent_id,  # not the agent this approval names
        instance_id="inst_1",
        task_id="task_1",
        delegation_id=grant.delegation_id,
        tool_name="add_case_note",
        resource_type="customer_ids",
        resource_id="cust_1",
        validated_arguments={"customer_id": "cust_1", "note": "refund approved"},
    )
    assert not decision.allowed
    assert decision.reason_code == APPROVAL_AGENT_MISMATCH


def test_wrong_instance_denied(db_conn, alice):
    agent, grant, appr = setup(db_conn, alice, instance_id="inst_A")
    approved(db_conn, alice, appr)
    decision = check(db_conn, human=alice, agent=agent, grant=grant, appr=appr, instance_id="inst_B")
    assert not decision.allowed
    assert decision.reason_code == APPROVAL_INSTANCE_MISMATCH


def test_wrong_task_denied(db_conn, alice):
    agent, grant, appr = setup(db_conn, alice, task_id="task_alpha")
    approved(db_conn, alice, appr)
    decision = check(db_conn, human=alice, agent=agent, grant=grant, appr=appr, task_id="task_beta")
    assert not decision.allowed
    assert decision.reason_code == APPROVAL_TASK_MISMATCH


def test_unbound_instance_and_task_match_anything(db_conn, alice):
    """instance_id/task_id are optional bindings — an approval requested
    without them is checked against any instance/task, like delegation's
    optional task_id."""
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    grant = grant_delegation(
        db_conn,
        alice,
        agent_id=agent.agent_id,
        allowed_tools=ALL_TOOLS,
        resource_constraints={"customer_ids": {"cust_1"}},
        ttl_seconds=3600,
    )
    appr = request_approval(
        db_conn,
        alice,
        agent_id=agent.agent_id,
        delegation_id=grant.delegation_id,
        tool_name="add_case_note",
        resource_type="customer_ids",
        resource_id="cust_1",
        proposed_arguments={"customer_id": "cust_1", "note": "x"},
        ttl_seconds=300,
    )
    approved(db_conn, alice, appr)
    decision = check(
        db_conn,
        human=alice,
        agent=agent,
        grant=grant,
        appr=appr,
        instance_id="whatever",
        task_id="whatever_else",
        arguments={"customer_id": "cust_1", "note": "x"},
    )
    assert decision.allowed


def test_wrong_delegation_denied(db_conn, alice):
    agent, grant, appr = setup(db_conn, alice)
    approved(db_conn, alice, appr)
    other_grant = grant_delegation(
        db_conn,
        alice,
        agent_id=agent.agent_id,
        allowed_tools=ALL_TOOLS,
        resource_constraints={"customer_ids": {"cust_1"}},
        ttl_seconds=3600,
    )
    decision = verify_and_consume_approval(
        db_conn,
        approval_id=appr.approval_id,
        principal=alice,
        agent_id=agent.agent_id,
        instance_id="inst_1",
        task_id="task_1",
        delegation_id=other_grant.delegation_id,  # not the delegation this approval names
        tool_name="add_case_note",
        resource_type="customer_ids",
        resource_id="cust_1",
        validated_arguments={"customer_id": "cust_1", "note": "refund approved"},
    )
    assert not decision.allowed
    assert decision.reason_code == APPROVAL_DELEGATION_MISMATCH


def test_wrong_tool_denied(db_conn, alice):
    agent, grant, appr = setup(db_conn, alice)
    approved(db_conn, alice, appr)
    decision = verify_and_consume_approval(
        db_conn,
        approval_id=appr.approval_id,
        principal=alice,
        agent_id=agent.agent_id,
        instance_id="inst_1",
        task_id="task_1",
        delegation_id=grant.delegation_id,
        tool_name="get_customer",  # approval names add_case_note
        resource_type="customer_ids",
        resource_id="cust_1",
        validated_arguments={"customer_id": "cust_1"},
    )
    assert not decision.allowed
    assert decision.reason_code == APPROVAL_TOOL_MISMATCH


def test_not_found_denied(db_conn, alice):
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    grant = grant_delegation(
        db_conn,
        alice,
        agent_id=agent.agent_id,
        allowed_tools=ALL_TOOLS,
        resource_constraints={"customer_ids": {"cust_1"}},
        ttl_seconds=3600,
    )
    decision = verify_and_consume_approval(
        db_conn,
        approval_id="apr_does_not_exist",
        principal=alice,
        agent_id=agent.agent_id,
        instance_id="inst_1",
        task_id="task_1",
        delegation_id=grant.delegation_id,
        tool_name="add_case_note",
        resource_type="customer_ids",
        resource_id="cust_1",
        validated_arguments={"customer_id": "cust_1", "note": "x"},
    )
    assert not decision.allowed
    assert decision.reason_code == APPROVAL_NOT_FOUND


def test_pending_approval_denied(db_conn, alice):
    """Never approved — an approval must be explicitly human-confirmed
    before it can back an execution."""
    agent, grant, appr = setup(db_conn, alice)
    decision = check(db_conn, human=alice, agent=agent, grant=grant, appr=appr)
    assert not decision.allowed
    assert decision.reason_code == APPROVAL_PENDING


def test_expired_approval_denied(db_conn, alice):
    agent, grant, appr = setup(db_conn, alice)
    approved(db_conn, alice, appr)
    db_conn.execute("UPDATE approvals SET expires_at = ? WHERE approval_id = ?", (time.time() - 1, appr.approval_id))
    db_conn.commit()
    decision = check(db_conn, human=alice, agent=agent, grant=grant, appr=appr)
    assert not decision.allowed
    assert decision.reason_code == APPROVAL_EXPIRED


def test_cancelled_approval_denied(db_conn, alice):
    agent, grant, appr = setup(db_conn, alice)
    approved(db_conn, alice, appr)
    cancel(db_conn, alice, appr.approval_id)
    decision = check(db_conn, human=alice, agent=agent, grant=grant, appr=appr)
    assert not decision.allowed
    assert decision.reason_code == APPROVAL_CANCELLED


def test_replay_after_consumption_denied(db_conn, alice):
    """The central one-shot property: first execution allowed, the exact
    same approval used a second time is denied."""
    agent, grant, appr = setup(db_conn, alice)
    approved(db_conn, alice, appr)
    first = check(db_conn, human=alice, agent=agent, grant=grant, appr=appr)
    assert first.allowed

    second = check(db_conn, human=alice, agent=agent, grant=grant, appr=appr)
    assert not second.allowed
    assert second.reason_code == APPROVAL_ALREADY_CONSUMED


def test_every_denial_persists_evidence(db_conn, alice):
    agent, grant, appr = setup(db_conn, alice)
    decision = check(db_conn, human=alice, agent=agent, grant=grant, appr=appr)  # still pending -> deny
    assert not decision.allowed
    persisted = authorization.get_decisions_for_instance(db_conn, "inst_1")
    assert len(persisted) == 1
    assert persisted[0].decision_id == decision.decision_id
    assert persisted[0].reason_code == APPROVAL_PENDING
    assert persisted[0].principal_user_id == alice.user_id
    assert persisted[0].agent_id == agent.agent_id
    assert persisted[0].task_id == "task_1"
    assert persisted[0].delegation_id == grant.delegation_id
    assert persisted[0].tool_name == "add_case_note"
    assert persisted[0].resource == "customer_ids:cust_1"
