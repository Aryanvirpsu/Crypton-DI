"""
v0.6 — scope attenuation engine (chain.verify_attenuation, §13-16).

Unit-level: exercises `verify_attenuation` directly against constructed
Delegation objects (not a persisted chain — that's test_delegation_chain.py's
job). Every dimension it covers — tools, resources, expiry — plus the
required boundary cases from the v0.6 test matrix (§37): equal authority,
narrows, broadens.
"""

from dataclasses import replace

import pytest

from agentlab.chain import verify_attenuation
from agentlab.delegation import Delegation
from agentlab.lifecycle import DelegationStatus


def _parent(**overrides) -> Delegation:
    base = dict(
        delegation_id="dlg_parent",
        principal_user_id="user-alice",
        agent_id="agt_parent_subject",
        task_id=None,
        allowed_tools=frozenset({"get_customer", "get_order", "add_case_note"}),
        resource_constraints={"customer_ids": frozenset({"cust_1", "cust_2", "cust_3"})},
        issued_at=1000.0,
        expires_at=2000.0,
        status=DelegationStatus.ACTIVE,
        revoked_at=None,
    )
    base.update(overrides)
    return Delegation(**base)


def _child(**overrides) -> Delegation:
    base = dict(
        delegation_id="dlg_child",
        principal_user_id="user-alice",
        agent_id="agt_child_subject",
        task_id=None,
        allowed_tools=frozenset({"get_customer"}),
        resource_constraints={"customer_ids": frozenset({"cust_1"})},
        issued_at=1000.0,
        expires_at=1500.0,
        status=DelegationStatus.ACTIVE,
        revoked_at=None,
    )
    base.update(overrides)
    return Delegation(**base)


# ── exact equality is always valid ────────────────────────────────────────


def test_child_equal_authority_is_valid():
    parent = _parent()
    child = _child(allowed_tools=parent.allowed_tools, resource_constraints=parent.resource_constraints, expires_at=parent.expires_at)
    ok, reason = verify_attenuation(parent, child)
    assert ok, reason


# ── tools/actions (§14) ────────────────────────────────────────────────────


def test_child_removing_a_tool_is_valid():
    parent = _parent()
    child = _child(allowed_tools=frozenset({"get_customer", "add_case_note"}))  # dropped get_order
    ok, reason = verify_attenuation(parent, child)
    assert ok, reason


def test_child_adding_a_tool_is_invalid():
    parent = _parent(allowed_tools=frozenset({"get_customer", "get_order"}))
    child = _child(allowed_tools=frozenset({"get_customer", "add_case_note"}))  # add_case_note never in parent
    ok, reason = verify_attenuation(parent, child)
    assert not ok
    assert "allowed_tools" in reason


def test_empty_child_tool_set_is_a_valid_but_useless_narrowing():
    """The model permits a child with zero tools (the empty set is a
    subset of anything) — documented as valid-but-pointless, not an
    error: such a delegation could never authorize any tool call at all,
    so it isn't a security hole, just a degenerate grant."""
    parent = _parent()
    child = _child(allowed_tools=frozenset())
    ok, reason = verify_attenuation(parent, child)
    assert ok, reason


# ── resources (§15) ─────────────────────────────────────────────────────────


def test_child_narrowing_resource_ids_is_valid():
    parent = _parent(resource_constraints={"customer_ids": frozenset({"cust_1", "cust_2", "cust_3"})})
    child = _child(resource_constraints={"customer_ids": frozenset({"cust_1"})})
    ok, reason = verify_attenuation(parent, child)
    assert ok, reason


def test_child_broadening_resource_ids_is_invalid():
    parent = _parent(resource_constraints={"customer_ids": frozenset({"cust_1"})})
    child = _child(resource_constraints={"customer_ids": frozenset({"cust_1", "cust_2"})})
    ok, reason = verify_attenuation(parent, child)
    assert not ok
    assert "customer_ids" in reason


def test_child_resource_id_not_prefix_matched():
    """Explicitly proves the naive-and-dangerous "customer/1" prefixes
    "customer/123" reading is NOT how this engine works — the existing
    model is exact id sets, and "cust_1" is simply not a member of
    {"cust_123"}, full stop, regardless of any textual relationship
    between the two strings."""
    parent = _parent(resource_constraints={"customer_ids": frozenset({"cust_123"})})
    child = _child(resource_constraints={"customer_ids": frozenset({"cust_1"})})
    ok, reason = verify_attenuation(parent, child)
    assert not ok
    assert "customer_ids" in reason


def test_child_resource_type_parent_never_scoped_is_invalid():
    """A resource_type absent from the parent's constraints is not
    "unconstrained" — it's "nothing of that type is authorized," per the
    existing authorization.py semantics this engine deliberately mirrors.
    A child cannot legitimately narrow into a type its parent never
    granted any authority over at all."""
    parent = _parent(resource_constraints={"customer_ids": frozenset({"cust_1"})})
    child = _child(resource_constraints={"order_ids": frozenset({"ord_1"})})
    ok, reason = verify_attenuation(parent, child)
    assert not ok
    assert "order_ids" in reason


def test_child_may_narrow_to_a_subset_of_resource_types():
    """A child may scope fewer resource_types than the parent — dropping
    a whole dimension of authority is a valid narrowing, symmetric with
    dropping a tool."""
    parent = _parent(
        resource_constraints={"customer_ids": frozenset({"cust_1"}), "order_ids": frozenset({"ord_1"})}
    )
    child = _child(resource_constraints={"customer_ids": frozenset({"cust_1"})})
    ok, reason = verify_attenuation(parent, child)
    assert ok, reason


# ── expiry (§16) ─────────────────────────────────────────────────────────────


def test_child_shorter_expiry_is_valid():
    parent = _parent(expires_at=2000.0)
    child = _child(expires_at=1000.0)
    ok, reason = verify_attenuation(parent, child)
    assert ok, reason


def test_child_equal_expiry_is_valid():
    parent = _parent(expires_at=2000.0)
    child = _child(expires_at=2000.0)
    ok, reason = verify_attenuation(parent, child)
    assert ok, reason


def test_child_longer_expiry_is_invalid():
    parent = _parent(expires_at=2000.0)
    child = _child(expires_at=2001.0)
    ok, reason = verify_attenuation(parent, child)
    assert not ok
    assert "expires_at" in reason
