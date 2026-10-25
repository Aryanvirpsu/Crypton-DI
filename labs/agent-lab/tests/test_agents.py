import pytest

from agentlab import db
from agentlab.agents import (
    AgentAccessError,
    AgentNotActiveError,
    AgentValidationError,
    DuplicateAgentError,
    bootstrap_support_agent,
    disable_agent,
    enable_agent,
    get_agent,
    list_agents,
    register_agent,
    require_active_agent,
)
from agentlab.lifecycle import AgentStatus


def test_register_agent(db_conn, alice):
    agent = register_agent(db_conn, alice, name="support-agent", version="0.1.0")
    assert agent.agent_id.startswith("agt_")
    assert agent.name == "support-agent"
    assert agent.version == "0.1.0"
    assert agent.owner_user_id == "user-alice"
    assert agent.status == AgentStatus.ACTIVE


def test_registration_persists_across_reconnect(tmp_path, alice):
    db_path = tmp_path / "persist.db"
    conn1 = db.connect(db_path)
    agent = register_agent(conn1, alice, name="support-agent", version="0.1.0")
    conn1.close()

    conn2 = db.connect(db_path)
    reloaded = get_agent(conn2, alice, agent.agent_id)
    assert reloaded.agent_id == agent.agent_id
    assert reloaded.name == "support-agent"
    conn2.close()


def test_generated_agent_id_is_canonical_and_unique(db_conn, alice):
    """register_agent has no `agent_id` parameter at all — the caller
    cannot supply one; two registrations always get different IDs."""
    a = register_agent(db_conn, alice, name="agent-one", version="0.1.0")
    b = register_agent(db_conn, alice, name="agent-two", version="0.1.0")
    assert a.agent_id != b.agent_id
    assert a.agent_id.startswith("agt_")
    assert b.agent_id.startswith("agt_")


def test_registration_rejects_empty_name(db_conn, alice):
    with pytest.raises(AgentValidationError):
        register_agent(db_conn, alice, name="   ", version="0.1.0")


def test_registration_rejects_empty_version(db_conn, alice):
    with pytest.raises(AgentValidationError):
        register_agent(db_conn, alice, name="support-agent", version="")


def test_duplicate_registration_rejected(db_conn, alice):
    """(owner_user_id, name, version) must be unique — see agents.py's
    module docstring for the rule this enforces."""
    register_agent(db_conn, alice, name="support-agent", version="0.1.0")
    with pytest.raises(DuplicateAgentError):
        register_agent(db_conn, alice, name="support-agent", version="0.1.0")


def test_same_name_different_owners_allowed(db_conn, alice, bob):
    """The uniqueness rule is owner-scoped: two different humans may each
    register their own "support-agent" v0.1.0 independently."""
    a = register_agent(db_conn, alice, name="support-agent", version="0.1.0")
    b = register_agent(db_conn, bob, name="support-agent", version="0.1.0")
    assert a.agent_id != b.agent_id
    assert a.owner_user_id != b.owner_user_id


def test_same_owner_different_versions_allowed(db_conn, alice):
    v1 = register_agent(db_conn, alice, name="support-agent", version="0.1.0")
    v2 = register_agent(db_conn, alice, name="support-agent", version="0.2.0")
    assert v1.agent_id != v2.agent_id


def test_get_own_agent(db_conn, alice):
    agent = register_agent(db_conn, alice, name="support-agent", version="0.1.0")
    reloaded = get_agent(db_conn, alice, agent.agent_id)
    assert reloaded == agent


def test_list_own_agents(db_conn, alice):
    a = register_agent(db_conn, alice, name="agent-one", version="0.1.0")
    b = register_agent(db_conn, alice, name="agent-two", version="0.1.0")
    listed_ids = {a.agent_id for a in list_agents(db_conn, alice)}
    assert listed_ids == {a.agent_id, b.agent_id}


def test_list_agents_excludes_other_users(db_conn, alice, bob):
    register_agent(db_conn, alice, name="alices-agent", version="0.1.0")
    register_agent(db_conn, bob, name="bobs-agent", version="0.1.0")
    alice_names = {a.name for a in list_agents(db_conn, alice)}
    assert alice_names == {"alices-agent"}


def test_cross_user_agent_read_rejected(db_conn, alice, bob):
    agent = register_agent(db_conn, alice, name="support-agent", version="0.1.0")
    with pytest.raises(AgentAccessError, match="agent_not_found"):
        get_agent(db_conn, bob, agent.agent_id)


def test_unknown_agent_read_rejected(db_conn, alice):
    with pytest.raises(AgentAccessError, match="agent_not_found"):
        get_agent(db_conn, alice, "agt_does_not_exist")


def test_disable_agent(db_conn, alice):
    agent = register_agent(db_conn, alice, name="support-agent", version="0.1.0")
    disabled = disable_agent(db_conn, alice, agent.agent_id)
    assert disabled.status == AgentStatus.DISABLED
    assert disabled.updated_at >= agent.updated_at


def test_enable_agent_after_disable(db_conn, alice):
    agent = register_agent(db_conn, alice, name="support-agent", version="0.1.0")
    disable_agent(db_conn, alice, agent.agent_id)
    re_enabled = enable_agent(db_conn, alice, agent.agent_id)
    assert re_enabled.status == AgentStatus.ACTIVE


def test_cross_user_disable_rejected(db_conn, alice, bob):
    agent = register_agent(db_conn, alice, name="support-agent", version="0.1.0")
    with pytest.raises(AgentAccessError, match="agent_not_found"):
        disable_agent(db_conn, bob, agent.agent_id)
    # confirm nothing changed
    still_active = get_agent(db_conn, alice, agent.agent_id)
    assert still_active.status == AgentStatus.ACTIVE


def test_require_active_agent_succeeds_when_active(db_conn, alice):
    agent = register_agent(db_conn, alice, name="support-agent", version="0.1.0")
    resolved = require_active_agent(db_conn, alice, agent.agent_id)
    assert resolved.agent_id == agent.agent_id
    assert resolved.status == AgentStatus.ACTIVE


def test_require_active_agent_rejects_disabled(db_conn, alice):
    agent = register_agent(db_conn, alice, name="support-agent", version="0.1.0")
    disable_agent(db_conn, alice, agent.agent_id)
    with pytest.raises(AgentNotActiveError, match="agent_not_active"):
        require_active_agent(db_conn, alice, agent.agent_id)


def test_require_active_agent_rejects_unknown(db_conn, alice):
    with pytest.raises(AgentAccessError, match="agent_not_found"):
        require_active_agent(db_conn, alice, "agt_does_not_exist")


def test_require_active_agent_rejects_wrong_owner(db_conn, alice, bob):
    agent = register_agent(db_conn, alice, name="support-agent", version="0.1.0")
    with pytest.raises(AgentAccessError, match="agent_not_found"):
        require_active_agent(db_conn, bob, agent.agent_id)


def test_bootstrap_support_agent_is_idempotent(db_conn, alice):
    first = bootstrap_support_agent(db_conn, alice)
    second = bootstrap_support_agent(db_conn, alice)
    assert first.agent_id == second.agent_id
    assert len(list_agents(db_conn, alice)) == 1


def test_bootstrap_support_agent_is_owner_scoped(db_conn, alice, bob):
    alices = bootstrap_support_agent(db_conn, alice)
    bobs = bootstrap_support_agent(db_conn, bob)
    assert alices.agent_id != bobs.agent_id
    assert alices.owner_user_id == "user-alice"
    assert bobs.owner_user_id == "user-bob"
