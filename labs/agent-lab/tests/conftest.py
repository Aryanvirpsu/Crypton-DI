import pytest

from agentlab import agents, delegation
from agentlab.db import connect, seed
from agentlab.identity import HumanIdentity

# Resource ids seeded by db.seed() — see db.py. Delegation fixtures below
# scope resource_constraints to exactly these so tools that read seeded
# data succeed under a real delegation, not because it was left wide open.
ALICE_RESOURCES = {"customer_ids": {"cust_1"}, "order_ids": {"ord_1"}}
ALL_TOOLS = frozenset({"get_customer", "get_order", "add_case_note"})


@pytest.fixture
def db_conn(tmp_path):
    conn = connect(tmp_path / "test.db")
    seed(conn)
    yield conn
    conn.close()


@pytest.fixture
def trace_dir(tmp_path):
    d = tmp_path / "traces"
    d.mkdir()
    return d


@pytest.fixture
def alice() -> HumanIdentity:
    return HumanIdentity(user_id="user-alice", username="alice", cred_id="dev-cred-alice")


@pytest.fixture
def bob() -> HumanIdentity:
    return HumanIdentity(user_id="user-bob", username="bob", cred_id="dev-cred-bob")


@pytest.fixture
def support_agent(db_conn, alice) -> agents.Agent:
    """A real, persisted Agent registration owned by alice — the v0.3
    replacement for the old bare `AgentIdentity(agent_id="support-agent")`.
    Returns the full `agents.Agent` record; use `.agent_id` to run it."""
    return agents.register_agent(db_conn, alice, name="support-agent", version="0.1.0")


@pytest.fixture
def read_only_agent(db_conn, alice) -> agents.Agent:
    """Same idea, registered under the "read-only-agent" name — paired
    with `read_only_delegation` below, which grants get_customer/get_order
    but not add_case_note."""
    return agents.register_agent(db_conn, alice, name="read-only-agent", version="0.1.0")


@pytest.fixture
def support_agent_delegation(db_conn, alice, support_agent) -> delegation.Delegation:
    """A real, persisted Delegation (v0.4) granting `support_agent` full
    tool authority scoped to alice's seeded demo data — the replacement
    for the old authz.REGISTRY entry of the same name. Use `.delegation_id`
    to run it."""
    return delegation.grant_delegation(
        db_conn,
        alice,
        agent_id=support_agent.agent_id,
        allowed_tools=ALL_TOOLS,
        resource_constraints=ALICE_RESOURCES,
        ttl_seconds=3600,
    )


@pytest.fixture
def read_only_delegation(db_conn, alice, read_only_agent) -> delegation.Delegation:
    """Paired with `read_only_agent`: authorizes get_customer/get_order
    but deliberately excludes add_case_note, exercising the
    TOOL_NOT_ALLOWED boundary."""
    return delegation.grant_delegation(
        db_conn,
        alice,
        agent_id=read_only_agent.agent_id,
        allowed_tools=frozenset({"get_customer", "get_order"}),
        resource_constraints=ALICE_RESOURCES,
        ttl_seconds=3600,
    )
