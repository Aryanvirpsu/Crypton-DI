import pytest

from agentlab import agents
from agentlab.db import connect, seed
from agentlab.identity import HumanIdentity


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
    """Same idea, registered under the "read-only-agent" name — which
    authz.REGISTRY only grants get_customer/get_order, not add_case_note."""
    return agents.register_agent(db_conn, alice, name="read-only-agent", version="0.1.0")
