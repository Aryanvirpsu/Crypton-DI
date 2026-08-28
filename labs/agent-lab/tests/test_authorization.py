"""
Unit-level tests for the deterministic pre-execution authorization
pipeline (authorization.py) — one test per pipeline step/reason code,
exercised directly against `authorize_tool_call` rather than through a
full `AgentRunner.run()`. See test_agent.py for the integration-level
regression test ("same name, different version") and the end-to-end
DENY/ALLOW behavior wired into the agent loop.
"""

import time

from agentlab import agents, authorization, delegation
from agentlab.agents import register_agent
from agentlab.identity import AgentInstance

RESOURCES = {"customer_ids": {"cust_1"}, "order_ids": {"ord_1"}}
ALL_TOOLS = frozenset({"get_customer", "get_order", "add_case_note"})


def make_instance(*, agent_id, human_user_id, task_id="task_1", instance_id="inst_1"):
    return AgentInstance(
        instance_id=instance_id,
        started_at=time.time(),
        agent_id=agent_id,
        human_user_id=human_user_id,
        session_id="sess_1",
        task_id=task_id,
    )


def test_valid_delegation_allows(db_conn, alice):
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    grant = delegation.grant_delegation(
        db_conn, alice, agent_id=agent.agent_id, allowed_tools=ALL_TOOLS, resource_constraints=RESOURCES, ttl_seconds=3600
    )
    instance = make_instance(agent_id=agent.agent_id, human_user_id=alice.user_id)
    decision = authorization.authorize_tool_call(
        db_conn,
        principal=alice,
        agent_id=agent.agent_id,
        instance=instance,
        task_id=instance.task_id,
        delegation_id=grant.delegation_id,
        tool_name="get_customer",
        resource_type="customer_ids",
        resource_id="cust_1",
    )
    assert decision.allowed
    assert decision.reason_code == authorization.ALLOW
    assert decision.decision_id.startswith("authz_")


def test_disabled_agent_denied(db_conn, alice):
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    grant = delegation.grant_delegation(
        db_conn, alice, agent_id=agent.agent_id, allowed_tools=ALL_TOOLS, resource_constraints=RESOURCES, ttl_seconds=3600
    )
    agents.disable_agent(db_conn, alice, agent.agent_id)
    instance = make_instance(agent_id=agent.agent_id, human_user_id=alice.user_id)
    decision = authorization.authorize_tool_call(
        db_conn,
        principal=alice,
        agent_id=agent.agent_id,
        instance=instance,
        task_id=instance.task_id,
        delegation_id=grant.delegation_id,
        tool_name="get_customer",
        resource_type="customer_ids",
        resource_id="cust_1",
    )
    assert not decision.allowed
    assert decision.reason_code == authorization.AGENT_DISABLED


def test_unknown_agent_denied(db_conn, alice):
    instance = make_instance(agent_id="agt_does_not_exist", human_user_id=alice.user_id)
    decision = authorization.authorize_tool_call(
        db_conn,
        principal=alice,
        agent_id="agt_does_not_exist",
        instance=instance,
        task_id=instance.task_id,
        delegation_id="dlg_does_not_exist",
        tool_name="get_customer",
        resource_type="customer_ids",
        resource_id="cust_1",
    )
    assert not decision.allowed
    assert decision.reason_code == authorization.AGENT_MISMATCH


def test_instance_agent_mismatch_denied(db_conn, alice):
    """The AgentInstance references a different agent_id than the one
    this call claims to be running as."""
    agent_a = register_agent(db_conn, alice, name="a", version="0.1.0")
    agent_b = register_agent(db_conn, alice, name="b", version="0.1.0")
    grant = delegation.grant_delegation(
        db_conn, alice, agent_id=agent_a.agent_id, allowed_tools=ALL_TOOLS, resource_constraints=RESOURCES, ttl_seconds=3600
    )
    instance = make_instance(agent_id=agent_b.agent_id, human_user_id=alice.user_id)  # belongs to agent_b
    decision = authorization.authorize_tool_call(
        db_conn,
        principal=alice,
        agent_id=agent_a.agent_id,  # claiming agent_a
        instance=instance,
        task_id=instance.task_id,
        delegation_id=grant.delegation_id,
        tool_name="get_customer",
        resource_type="customer_ids",
        resource_id="cust_1",
    )
    assert not decision.allowed
    assert decision.reason_code == authorization.AGENT_MISMATCH


def test_instance_principal_mismatch_denied(db_conn, alice, bob):
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    grant = delegation.grant_delegation(
        db_conn, alice, agent_id=agent.agent_id, allowed_tools=ALL_TOOLS, resource_constraints=RESOURCES, ttl_seconds=3600
    )
    instance = make_instance(agent_id=agent.agent_id, human_user_id=bob.user_id)  # instance belongs to bob
    decision = authorization.authorize_tool_call(
        db_conn,
        principal=alice,  # but alice is calling
        agent_id=agent.agent_id,
        instance=instance,
        task_id=instance.task_id,
        delegation_id=grant.delegation_id,
        tool_name="get_customer",
        resource_type="customer_ids",
        resource_id="cust_1",
    )
    assert not decision.allowed
    assert decision.reason_code == authorization.PRINCIPAL_MISMATCH


def test_delegation_not_found_denied(db_conn, alice):
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    instance = make_instance(agent_id=agent.agent_id, human_user_id=alice.user_id)
    decision = authorization.authorize_tool_call(
        db_conn,
        principal=alice,
        agent_id=agent.agent_id,
        instance=instance,
        task_id=instance.task_id,
        delegation_id="dlg_00000000000000000000000000000000",
        tool_name="get_customer",
        resource_type="customer_ids",
        resource_id="cust_1",
    )
    assert not decision.allowed
    assert decision.reason_code == authorization.DELEGATION_NOT_FOUND


def test_delegation_wrong_principal_denied(db_conn, alice, bob):
    """bob owns his own active agent, but presents a delegation_id that
    was granted by (and belongs to) alice, for alice's agent — not his.
    Reachable even though bob's own agent/instance/principal are all
    internally consistent, because a delegation_id is a bare handle that
    can reference any delegation row at all."""
    alices_agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    bobs_agent = register_agent(db_conn, bob, name="b", version="0.1.0")
    alices_grant = delegation.grant_delegation(
        db_conn,
        alice,
        agent_id=alices_agent.agent_id,
        allowed_tools=ALL_TOOLS,
        resource_constraints=RESOURCES,
        ttl_seconds=3600,
    )
    instance = make_instance(agent_id=bobs_agent.agent_id, human_user_id=bob.user_id)
    decision = authorization.authorize_tool_call(
        db_conn,
        principal=bob,
        agent_id=bobs_agent.agent_id,
        instance=instance,
        task_id=instance.task_id,
        delegation_id=alices_grant.delegation_id,  # not bob's
        tool_name="get_customer",
        resource_type="customer_ids",
        resource_id="cust_1",
    )
    assert not decision.allowed
    assert decision.reason_code == authorization.PRINCIPAL_MISMATCH


def test_delegation_wrong_agent_denied(db_conn, alice):
    """A delegation granted to one agent_id must never authorize a
    different agent_id — the core v0.3-bug regression, at the pipeline
    level (see test_agent.py for the end-to-end version)."""
    agent_a = register_agent(db_conn, alice, name="same-name", version="0.1.0")
    agent_b = register_agent(db_conn, alice, name="same-name", version="0.2.0")
    grant = delegation.grant_delegation(
        db_conn, alice, agent_id=agent_a.agent_id, allowed_tools=ALL_TOOLS, resource_constraints=RESOURCES, ttl_seconds=3600
    )
    instance = make_instance(agent_id=agent_b.agent_id, human_user_id=alice.user_id)
    decision = authorization.authorize_tool_call(
        db_conn,
        principal=alice,
        agent_id=agent_b.agent_id,
        instance=instance,
        task_id=instance.task_id,
        delegation_id=grant.delegation_id,  # granted to agent_a, not agent_b
        tool_name="get_customer",
        resource_type="customer_ids",
        resource_id="cust_1",
    )
    assert not decision.allowed
    assert decision.reason_code == authorization.AGENT_MISMATCH


def test_delegation_wrong_task_denied(db_conn, alice):
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    grant = delegation.grant_delegation(
        db_conn,
        alice,
        agent_id=agent.agent_id,
        allowed_tools=ALL_TOOLS,
        resource_constraints=RESOURCES,
        task_id="task_alpha",
        ttl_seconds=3600,
    )
    instance = make_instance(agent_id=agent.agent_id, human_user_id=alice.user_id, task_id="task_beta")
    decision = authorization.authorize_tool_call(
        db_conn,
        principal=alice,
        agent_id=agent.agent_id,
        instance=instance,
        task_id="task_beta",  # not task_alpha
        delegation_id=grant.delegation_id,
        tool_name="get_customer",
        resource_type="customer_ids",
        resource_id="cust_1",
    )
    assert not decision.allowed
    assert decision.reason_code == authorization.TASK_MISMATCH


def test_task_bound_delegation_allows_matching_task(db_conn, alice):
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    grant = delegation.grant_delegation(
        db_conn,
        alice,
        agent_id=agent.agent_id,
        allowed_tools=ALL_TOOLS,
        resource_constraints=RESOURCES,
        task_id="task_alpha",
        ttl_seconds=3600,
    )
    instance = make_instance(agent_id=agent.agent_id, human_user_id=alice.user_id, task_id="task_alpha")
    decision = authorization.authorize_tool_call(
        db_conn,
        principal=alice,
        agent_id=agent.agent_id,
        instance=instance,
        task_id="task_alpha",
        delegation_id=grant.delegation_id,
        tool_name="get_customer",
        resource_type="customer_ids",
        resource_id="cust_1",
    )
    assert decision.allowed


def test_revoked_delegation_denied(db_conn, alice):
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    grant = delegation.grant_delegation(
        db_conn, alice, agent_id=agent.agent_id, allowed_tools=ALL_TOOLS, resource_constraints=RESOURCES, ttl_seconds=3600
    )
    delegation.revoke_delegation(db_conn, alice, grant.delegation_id)
    instance = make_instance(agent_id=agent.agent_id, human_user_id=alice.user_id)
    decision = authorization.authorize_tool_call(
        db_conn,
        principal=alice,
        agent_id=agent.agent_id,
        instance=instance,
        task_id=instance.task_id,
        delegation_id=grant.delegation_id,
        tool_name="get_customer",
        resource_type="customer_ids",
        resource_id="cust_1",
    )
    assert not decision.allowed
    assert decision.reason_code == authorization.DELEGATION_REVOKED


def test_expired_delegation_denied(db_conn, alice):
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    grant = delegation.grant_delegation(
        db_conn, alice, agent_id=agent.agent_id, allowed_tools=ALL_TOOLS, resource_constraints=RESOURCES, ttl_seconds=3600
    )
    # Force it into the past — no need to actually sleep out a TTL.
    db_conn.execute("UPDATE delegations SET expires_at = ? WHERE delegation_id = ?", (time.time() - 1, grant.delegation_id))
    db_conn.commit()
    instance = make_instance(agent_id=agent.agent_id, human_user_id=alice.user_id)
    decision = authorization.authorize_tool_call(
        db_conn,
        principal=alice,
        agent_id=agent.agent_id,
        instance=instance,
        task_id=instance.task_id,
        delegation_id=grant.delegation_id,
        tool_name="get_customer",
        resource_type="customer_ids",
        resource_id="cust_1",
    )
    assert not decision.allowed
    assert decision.reason_code == authorization.DELEGATION_EXPIRED


def test_tool_not_allowed_denied(db_conn, alice):
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    grant = delegation.grant_delegation(
        db_conn,
        alice,
        agent_id=agent.agent_id,
        allowed_tools=frozenset({"get_customer"}),
        resource_constraints=RESOURCES,
        ttl_seconds=3600,
    )
    instance = make_instance(agent_id=agent.agent_id, human_user_id=alice.user_id)
    decision = authorization.authorize_tool_call(
        db_conn,
        principal=alice,
        agent_id=agent.agent_id,
        instance=instance,
        task_id=instance.task_id,
        delegation_id=grant.delegation_id,
        tool_name="add_case_note",
        resource_type="customer_ids",
        resource_id="cust_1",
    )
    assert not decision.allowed
    assert decision.reason_code == authorization.TOOL_NOT_ALLOWED


def test_resource_in_scope_succeeds(db_conn, alice):
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    grant = delegation.grant_delegation(
        db_conn,
        alice,
        agent_id=agent.agent_id,
        allowed_tools=ALL_TOOLS,
        resource_constraints={"customer_ids": {"cust_1", "cust_2"}},
        ttl_seconds=3600,
    )
    instance = make_instance(agent_id=agent.agent_id, human_user_id=alice.user_id)
    decision = authorization.authorize_tool_call(
        db_conn,
        principal=alice,
        agent_id=agent.agent_id,
        instance=instance,
        task_id=instance.task_id,
        delegation_id=grant.delegation_id,
        tool_name="get_customer",
        resource_type="customer_ids",
        resource_id="cust_2",
    )
    assert decision.allowed


def test_resource_outside_scope_denied(db_conn, alice):
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    grant = delegation.grant_delegation(
        db_conn,
        alice,
        agent_id=agent.agent_id,
        allowed_tools=ALL_TOOLS,
        resource_constraints={"customer_ids": {"cust_1"}},
        ttl_seconds=3600,
    )
    instance = make_instance(agent_id=agent.agent_id, human_user_id=alice.user_id)
    decision = authorization.authorize_tool_call(
        db_conn,
        principal=alice,
        agent_id=agent.agent_id,
        instance=instance,
        task_id=instance.task_id,
        delegation_id=grant.delegation_id,
        tool_name="get_customer",
        resource_type="customer_ids",
        resource_id="cust_99",  # not in scope
    )
    assert not decision.allowed
    assert decision.reason_code == authorization.RESOURCE_NOT_ALLOWED


def test_resource_type_absent_from_constraints_is_safe_default_deny(db_conn, alice):
    """A resource_type never mentioned in resource_constraints means
    nothing of that type is authorized — not "unconstrained"."""
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    grant = delegation.grant_delegation(
        db_conn,
        alice,
        agent_id=agent.agent_id,
        allowed_tools=ALL_TOOLS,
        resource_constraints={"customer_ids": {"cust_1"}},  # no "order_ids" key at all
        ttl_seconds=3600,
    )
    instance = make_instance(agent_id=agent.agent_id, human_user_id=alice.user_id)
    decision = authorization.authorize_tool_call(
        db_conn,
        principal=alice,
        agent_id=agent.agent_id,
        instance=instance,
        task_id=instance.task_id,
        delegation_id=grant.delegation_id,
        tool_name="get_order",
        resource_type="order_ids",
        resource_id="ord_1",
    )
    assert not decision.allowed
    assert decision.reason_code == authorization.RESOURCE_NOT_ALLOWED


def test_tool_with_no_resource_scope_skips_resource_check(db_conn, alice):
    """resource_type=None (an unscoped tool) never triggers RESOURCE_NOT_ALLOWED."""
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    grant = delegation.grant_delegation(
        db_conn,
        alice,
        agent_id=agent.agent_id,
        allowed_tools=frozenset({"some_unscoped_tool"}),
        resource_constraints={},
        ttl_seconds=3600,
    )
    instance = make_instance(agent_id=agent.agent_id, human_user_id=alice.user_id)
    decision = authorization.authorize_tool_call(
        db_conn,
        principal=alice,
        agent_id=agent.agent_id,
        instance=instance,
        task_id=instance.task_id,
        delegation_id=grant.delegation_id,
        tool_name="some_unscoped_tool",
        resource_type=None,
        resource_id=None,
    )
    assert decision.allowed


def test_every_denial_is_persisted_as_evidence(db_conn, alice):
    agent = register_agent(db_conn, alice, name="a", version="0.1.0")
    instance = make_instance(agent_id=agent.agent_id, human_user_id=alice.user_id)
    decision = authorization.authorize_tool_call(
        db_conn,
        principal=alice,
        agent_id=agent.agent_id,
        instance=instance,
        task_id=instance.task_id,
        delegation_id="dlg_00000000000000000000000000000000",
        tool_name="get_customer",
        resource_type="customer_ids",
        resource_id="cust_1",
    )
    assert not decision.allowed
    persisted = authorization.get_decisions_for_instance(db_conn, instance.instance_id)
    assert len(persisted) == 1
    assert persisted[0].decision_id == decision.decision_id
    assert persisted[0].decision == authorization.DECISION_DENY
    assert persisted[0].reason_code == authorization.DELEGATION_NOT_FOUND
    # full evidence set: principal, agent, instance, task, delegation, tool, resource, reason, timestamp
    assert persisted[0].principal_user_id == alice.user_id
    assert persisted[0].agent_id == agent.agent_id
    assert persisted[0].instance_id == instance.instance_id
    assert persisted[0].task_id == instance.task_id
    assert persisted[0].delegation_id == "dlg_00000000000000000000000000000000"
    assert persisted[0].tool_name == "get_customer"
    assert persisted[0].resource == "customer_ids:cust_1"
    assert persisted[0].timestamp > 0


def test_resource_for_tool_call_ignores_wrong_typed_argument(db_conn, alice):
    """A model handing back a non-string customer_id is never trusted as
    a resource id — it's treated as missing, which the resource-scope
    check then safely denies."""
    resource_type, resource_id = authorization.resource_for_tool_call("get_customer", {"customer_id": 12345})
    assert resource_type == "customer_ids"
    assert resource_id is None


def test_resource_for_tool_call_unscoped_tool(db_conn, alice):
    resource_type, resource_id = authorization.resource_for_tool_call("unknown_tool", {"anything": "x"})
    assert resource_type is None
    assert resource_id is None
