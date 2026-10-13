import pytest

from agentlab.agents import AgentAccessError, register_agent
from agentlab.delegation import (
    DelegationAccessError,
    DelegationValidationError,
    bootstrap_delegation,
    get_delegation,
    get_delegation_unchecked,
    grant_delegation,
    revoke_delegation,
)
from agentlab.lifecycle import DelegationStatus, LifecycleError

RESOURCES = {"customer_ids": {"cust_1"}}
ALL_TOOLS = frozenset({"get_customer", "get_order", "add_case_note"})


def test_grant_delegation(db_conn, alice):
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    grant = grant_delegation(
        db_conn, alice, agent_id=agent.agent_id, allowed_tools=ALL_TOOLS, resource_constraints=RESOURCES, ttl_seconds=3600
    )
    assert grant.delegation_id.startswith("dlg_")
    assert grant.principal_user_id == "user-alice"
    assert grant.agent_id == agent.agent_id
    assert grant.task_id is None
    assert grant.allowed_tools == ALL_TOOLS
    assert grant.resource_constraints == {"customer_ids": frozenset({"cust_1"})}
    assert grant.status == DelegationStatus.ACTIVE
    assert grant.revoked_at is None
    assert grant.expires_at > grant.issued_at


def test_grant_delegation_is_task_bound_when_task_id_given(db_conn, alice):
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    grant = grant_delegation(
        db_conn,
        alice,
        agent_id=agent.agent_id,
        allowed_tools=ALL_TOOLS,
        resource_constraints=RESOURCES,
        task_id="task_123",
        ttl_seconds=3600,
    )
    assert grant.task_id == "task_123"


def test_grant_delegation_requires_owning_the_agent(db_conn, alice, bob):
    """A principal can only delegate authority over an agent they own —
    never an arbitrary agent_id, even a real, active one."""
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    with pytest.raises(AgentAccessError):
        grant_delegation(db_conn, bob, agent_id=agent.agent_id, allowed_tools=ALL_TOOLS, ttl_seconds=3600)


def test_grant_delegation_rejects_empty_tools(db_conn, alice):
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    with pytest.raises(DelegationValidationError):
        grant_delegation(db_conn, alice, agent_id=agent.agent_id, allowed_tools=[], ttl_seconds=3600)


@pytest.mark.parametrize("ttl", [0, -1, -3600])
def test_grant_delegation_rejects_nonpositive_ttl(db_conn, alice, ttl):
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    with pytest.raises(DelegationValidationError):
        grant_delegation(db_conn, alice, agent_id=agent.agent_id, allowed_tools=ALL_TOOLS, ttl_seconds=ttl)


def test_grant_delegation_rejects_excessive_ttl(db_conn, alice):
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    with pytest.raises(DelegationValidationError):
        grant_delegation(
            db_conn, alice, agent_id=agent.agent_id, allowed_tools=ALL_TOOLS, ttl_seconds=60 * 60 * 24 * 365
        )


def test_get_own_delegation(db_conn, alice):
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    grant = grant_delegation(db_conn, alice, agent_id=agent.agent_id, allowed_tools=ALL_TOOLS, ttl_seconds=3600)
    reloaded = get_delegation(db_conn, alice, grant.delegation_id)
    assert reloaded == grant


def test_get_delegation_ownership_enforced(db_conn, alice, bob):
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    grant = grant_delegation(db_conn, alice, agent_id=agent.agent_id, allowed_tools=ALL_TOOLS, ttl_seconds=3600)
    with pytest.raises(DelegationAccessError):
        get_delegation(db_conn, bob, grant.delegation_id)


def test_get_delegation_unknown_rejected(db_conn, alice):
    with pytest.raises(DelegationAccessError):
        get_delegation(db_conn, alice, "dlg_does_not_exist")


def test_get_delegation_unchecked_has_no_ownership_gate(db_conn, alice, bob):
    """Used only by the authorization pipeline, which needs to see a
    delegation exists (and inspect its principal) even when the caller
    isn't its principal — that's precisely how PRINCIPAL_MISMATCH gets
    detected instead of a plain not-found."""
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    grant = grant_delegation(db_conn, alice, agent_id=agent.agent_id, allowed_tools=ALL_TOOLS, ttl_seconds=3600)
    found = get_delegation_unchecked(db_conn, grant.delegation_id)
    assert found is not None
    assert found.delegation_id == grant.delegation_id
    assert get_delegation_unchecked(db_conn, "dlg_does_not_exist") is None


def test_revoke_delegation(db_conn, alice):
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    grant = grant_delegation(db_conn, alice, agent_id=agent.agent_id, allowed_tools=ALL_TOOLS, ttl_seconds=3600)
    revoked = revoke_delegation(db_conn, alice, grant.delegation_id)
    assert revoked.status == DelegationStatus.REVOKED
    assert revoked.revoked_at is not None
    assert revoked.revoked_at >= grant.issued_at

    # the record survives — it is retained, not deleted
    reloaded = get_delegation(db_conn, alice, grant.delegation_id)
    assert reloaded.status == DelegationStatus.REVOKED


def test_revoke_delegation_wrong_principal_rejected(db_conn, alice, bob):
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    grant = grant_delegation(db_conn, alice, agent_id=agent.agent_id, allowed_tools=ALL_TOOLS, ttl_seconds=3600)
    with pytest.raises(DelegationAccessError):
        revoke_delegation(db_conn, bob, grant.delegation_id)
    # confirm nothing changed
    still_active = get_delegation(db_conn, alice, grant.delegation_id)
    assert still_active.status == DelegationStatus.ACTIVE


def test_double_revoke_rejected(db_conn, alice):
    """Revocation is one-way — re-revoking an already-revoked delegation
    is not a real transition, not a silent no-op."""
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    grant = grant_delegation(db_conn, alice, agent_id=agent.agent_id, allowed_tools=ALL_TOOLS, ttl_seconds=3600)
    revoke_delegation(db_conn, alice, grant.delegation_id)
    with pytest.raises(LifecycleError):
        revoke_delegation(db_conn, alice, grant.delegation_id)


def test_bootstrap_delegation_mints_a_fresh_grant_each_time(db_conn, alice):
    """Unlike bootstrap_support_agent, this is NOT idempotent — a
    delegation is a time-bounded grant, not a stable registration."""
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    first = bootstrap_delegation(db_conn, alice, agent.agent_id, allowed_tools=ALL_TOOLS)
    second = bootstrap_delegation(db_conn, alice, agent.agent_id, allowed_tools=ALL_TOOLS)
    assert first.delegation_id != second.delegation_id


def test_delegation_persists_across_reconnect(tmp_path, alice):
    from agentlab import db

    db_path = tmp_path / "persist.db"
    conn1 = db.connect(db_path)
    agent = register_agent(conn1, alice, name="a", version="0.1.0")
    grant = grant_delegation(conn1, alice, agent_id=agent.agent_id, allowed_tools=ALL_TOOLS, ttl_seconds=3600)
    conn1.close()

    conn2 = db.connect(db_path)
    reloaded = get_delegation(conn2, alice, grant.delegation_id)
    assert reloaded.delegation_id == grant.delegation_id
    assert reloaded.allowed_tools == ALL_TOOLS
    conn2.close()
