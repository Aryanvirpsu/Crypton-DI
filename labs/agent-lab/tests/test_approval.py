import pytest

from agentlab.agents import AgentAccessError, register_agent
from agentlab.approval import (
    ApprovalAccessError,
    ApprovalValidationError,
    approve,
    cancel,
    compute_argument_hash,
    get_approval,
    request_approval,
)
from agentlab.delegation import grant_delegation
from agentlab.lifecycle import ApprovalStatus, LifecycleError
from agentlab.webauthn import FakeWebAuthnVerifier, WebAuthnVerificationError

ALL_TOOLS = frozenset({"get_customer", "get_order", "add_case_note"})
VERIFIER = FakeWebAuthnVerifier()


def make_agent_and_delegation(db_conn, human):
    agent = register_agent(db_conn, human, name="a", version="0.1.0")
    grant = grant_delegation(
        db_conn,
        human,
        agent_id=agent.agent_id,
        allowed_tools=ALL_TOOLS,
        resource_constraints={"customer_ids": {"cust_1"}},
        ttl_seconds=3600,
    )
    return agent, grant


def test_request_approval(db_conn, alice):
    agent, grant = make_agent_and_delegation(db_conn, alice)
    appr = request_approval(
        db_conn,
        alice,
        agent_id=agent.agent_id,
        delegation_id=grant.delegation_id,
        tool_name="add_case_note",
        resource_type="customer_ids",
        resource_id="cust_1",
        proposed_arguments={"customer_id": "cust_1", "note": "refund approved"},
        ttl_seconds=300,
    )
    assert appr.approval_id.startswith("apr_")
    assert appr.principal_user_id == "user-alice"
    assert appr.agent_id == agent.agent_id
    assert appr.delegation_id == grant.delegation_id
    assert appr.tool_name == "add_case_note"
    assert appr.resource == "customer_ids:cust_1"
    assert appr.status == ApprovalStatus.PENDING
    assert appr.approved_at is None
    assert appr.consumed_at is None
    assert appr.webauthn_reference is None
    assert appr.expires_at > appr.issued_at
    # the hash matches what execution-time recomputation would produce
    expected = compute_argument_hash(
        "add_case_note", "customer_ids:cust_1", {"customer_id": "cust_1", "note": "refund approved"}
    )
    assert appr.argument_hash == expected


def test_argument_hash_is_deterministic_regardless_of_key_order(db_conn, alice):
    """Canonicalization must be deterministic: the same logical arguments,
    constructed in a different key order, hash identically."""
    a = compute_argument_hash("add_case_note", "customer_ids:cust_1", {"customer_id": "cust_1", "note": "x"})
    b = compute_argument_hash("add_case_note", "customer_ids:cust_1", {"note": "x", "customer_id": "cust_1"})
    assert a == b


def test_argument_hash_changes_with_argument_value(db_conn, alice):
    a = compute_argument_hash("add_case_note", "customer_ids:cust_1", {"customer_id": "cust_1", "note": "x"})
    b = compute_argument_hash("add_case_note", "customer_ids:cust_1", {"customer_id": "cust_1", "note": "y"})
    assert a != b


def test_argument_hash_changes_with_resource(db_conn, alice):
    a = compute_argument_hash("add_case_note", "customer_ids:cust_1", {"customer_id": "cust_1", "note": "x"})
    b = compute_argument_hash("add_case_note", "customer_ids:cust_2", {"customer_id": "cust_1", "note": "x"})
    assert a != b


def test_argument_hash_changes_with_tool(db_conn, alice):
    a = compute_argument_hash("add_case_note", None, {"customer_id": "cust_1"})
    b = compute_argument_hash("get_customer", None, {"customer_id": "cust_1"})
    assert a != b


def test_request_approval_requires_owning_the_agent(db_conn, alice, bob):
    agent, grant = make_agent_and_delegation(db_conn, alice)
    with pytest.raises(AgentAccessError):
        request_approval(
            db_conn,
            bob,
            agent_id=agent.agent_id,
            delegation_id=grant.delegation_id,
            tool_name="add_case_note",
            resource_type="customer_ids",
            resource_id="cust_1",
            proposed_arguments={"customer_id": "cust_1", "note": "x"},
            ttl_seconds=300,
        )


def test_request_approval_rejects_delegation_for_a_different_agent(db_conn, alice):
    agent_a, grant_a = make_agent_and_delegation(db_conn, alice)
    agent_b = register_agent(db_conn, alice, name="b", version="0.1.0")
    with pytest.raises(ApprovalValidationError):
        request_approval(
            db_conn,
            alice,
            agent_id=agent_b.agent_id,
            delegation_id=grant_a.delegation_id,  # granted to agent_a, not agent_b
            tool_name="add_case_note",
            resource_type="customer_ids",
            resource_id="cust_1",
            proposed_arguments={"customer_id": "cust_1", "note": "x"},
            ttl_seconds=300,
        )


def test_request_approval_rejects_unknown_tool(db_conn, alice):
    agent, grant = make_agent_and_delegation(db_conn, alice)
    with pytest.raises(ApprovalValidationError):
        request_approval(
            db_conn,
            alice,
            agent_id=agent.agent_id,
            delegation_id=grant.delegation_id,
            tool_name="delete_everything",
            resource_type=None,
            resource_id=None,
            proposed_arguments={},
            ttl_seconds=300,
        )


def test_request_approval_rejects_wrong_resource_type_for_tool(db_conn, alice):
    agent, grant = make_agent_and_delegation(db_conn, alice)
    with pytest.raises(ApprovalValidationError):
        request_approval(
            db_conn,
            alice,
            agent_id=agent.agent_id,
            delegation_id=grant.delegation_id,
            tool_name="add_case_note",
            resource_type="order_ids",  # add_case_note is scoped by customer_ids, not order_ids
            resource_id="ord_1",
            proposed_arguments={"customer_id": "cust_1", "note": "x"},
            ttl_seconds=300,
        )


def test_request_approval_rejects_malformed_arguments(db_conn, alice):
    agent, grant = make_agent_and_delegation(db_conn, alice)
    with pytest.raises(ApprovalValidationError):
        request_approval(
            db_conn,
            alice,
            agent_id=agent.agent_id,
            delegation_id=grant.delegation_id,
            tool_name="add_case_note",
            resource_type="customer_ids",
            resource_id="cust_1",
            proposed_arguments={"customer_id": "cust_1"},  # missing required "note"
            ttl_seconds=300,
        )


@pytest.mark.parametrize("ttl", [0, -1, -300])
def test_request_approval_rejects_nonpositive_ttl(db_conn, alice, ttl):
    agent, grant = make_agent_and_delegation(db_conn, alice)
    with pytest.raises(ApprovalValidationError):
        request_approval(
            db_conn,
            alice,
            agent_id=agent.agent_id,
            delegation_id=grant.delegation_id,
            tool_name="add_case_note",
            resource_type="customer_ids",
            resource_id="cust_1",
            proposed_arguments={"customer_id": "cust_1", "note": "x"},
            ttl_seconds=ttl,
        )


def test_request_approval_rejects_long_ttl(db_conn, alice):
    """Approvals are short-lived by design — unlike delegations, there is
    no "just make it a long grant" escape hatch."""
    agent, grant = make_agent_and_delegation(db_conn, alice)
    with pytest.raises(ApprovalValidationError):
        request_approval(
            db_conn,
            alice,
            agent_id=agent.agent_id,
            delegation_id=grant.delegation_id,
            tool_name="add_case_note",
            resource_type="customer_ids",
            resource_id="cust_1",
            proposed_arguments={"customer_id": "cust_1", "note": "x"},
            ttl_seconds=3600,  # over MAX_APPROVAL_TTL_SECONDS (15 min)
        )


def test_approve_transitions_pending_to_approved(db_conn, alice):
    agent, grant = make_agent_and_delegation(db_conn, alice)
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
    approved = approve(db_conn, alice, appr.approval_id, verifier=VERIFIER)
    assert approved.status == ApprovalStatus.APPROVED
    assert approved.approved_at is not None
    assert approved.webauthn_reference is not None
    assert approved.webauthn_reference.startswith("fake:")


def test_approve_wrong_principal_rejected(db_conn, alice, bob):
    agent, grant = make_agent_and_delegation(db_conn, alice)
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
    with pytest.raises(ApprovalAccessError):
        approve(db_conn, bob, appr.approval_id, verifier=VERIFIER)


def test_approve_fails_webauthn_leaves_approval_pending(db_conn, alice):
    """A failed step-up ceremony never silently approves — the approval
    is left exactly as it was."""
    agent, grant = make_agent_and_delegation(db_conn, alice)
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

    class AlwaysFailVerifier(FakeWebAuthnVerifier):
        def verify_step_up(self, *, principal, challenge):
            return super().verify_step_up(principal=principal, challenge=self.FAIL_CHALLENGE)

    with pytest.raises(WebAuthnVerificationError):
        approve(db_conn, alice, appr.approval_id, verifier=AlwaysFailVerifier())

    still_pending = get_approval(db_conn, alice, appr.approval_id)
    assert still_pending.status == ApprovalStatus.PENDING
    assert still_pending.webauthn_reference is None


def test_double_approve_rejected(db_conn, alice):
    agent, grant = make_agent_and_delegation(db_conn, alice)
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
    approve(db_conn, alice, appr.approval_id, verifier=VERIFIER)
    with pytest.raises(LifecycleError):
        approve(db_conn, alice, appr.approval_id, verifier=VERIFIER)


def test_cancel_pending_approval(db_conn, alice):
    agent, grant = make_agent_and_delegation(db_conn, alice)
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
    cancelled = cancel(db_conn, alice, appr.approval_id)
    assert cancelled.status == ApprovalStatus.CANCELLED
    # retained, not deleted
    reloaded = get_approval(db_conn, alice, appr.approval_id)
    assert reloaded.status == ApprovalStatus.CANCELLED


def test_cancel_approved_approval(db_conn, alice):
    agent, grant = make_agent_and_delegation(db_conn, alice)
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
    approve(db_conn, alice, appr.approval_id, verifier=VERIFIER)
    cancelled = cancel(db_conn, alice, appr.approval_id)
    assert cancelled.status == ApprovalStatus.CANCELLED


def test_double_cancel_rejected(db_conn, alice):
    agent, grant = make_agent_and_delegation(db_conn, alice)
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
    cancel(db_conn, alice, appr.approval_id)
    with pytest.raises(LifecycleError):
        cancel(db_conn, alice, appr.approval_id)


def test_get_approval_ownership_enforced(db_conn, alice, bob):
    agent, grant = make_agent_and_delegation(db_conn, alice)
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
    with pytest.raises(ApprovalAccessError):
        get_approval(db_conn, bob, appr.approval_id)


def test_get_approval_unknown_rejected(db_conn, alice):
    with pytest.raises(ApprovalAccessError):
        get_approval(db_conn, alice, "apr_does_not_exist")
