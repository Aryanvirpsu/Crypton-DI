import pytest

from agentlab.identity import AgentInstance
from agentlab.models import AddCaseNoteArgs, GetCustomerArgs, GetOrderArgs
from agentlab.tools import ToolError, add_case_note, get_customer, get_order


def test_get_customer_returns_own_record(db_conn, alice):
    instance = AgentInstance.new()
    result = get_customer(db_conn, alice, instance, GetCustomerArgs(customer_id="cust_1"))
    assert result["customer_id"] == "cust_1"
    assert result["name"] == "Alice Anders"


def test_get_customer_isolated_from_other_user(db_conn, bob):
    """Requirement: separate users' data remains isolated. cust_1 belongs
    to alice — bob must not be able to read it through the same tool."""
    instance = AgentInstance.new()
    with pytest.raises(ToolError, match="customer_not_found"):
        get_customer(db_conn, bob, instance, GetCustomerArgs(customer_id="cust_1"))


def test_get_order_isolated_from_other_user(db_conn, alice):
    instance = AgentInstance.new()
    with pytest.raises(ToolError, match="order_not_found"):
        get_order(db_conn, alice, instance, GetOrderArgs(order_id="ord_2"))  # owned by bob


def test_get_customer_unknown_id(db_conn, alice):
    instance = AgentInstance.new()
    with pytest.raises(ToolError, match="customer_not_found"):
        get_customer(db_conn, alice, instance, GetCustomerArgs(customer_id="does-not-exist"))


def test_add_case_note_persists_for_owner(db_conn, alice):
    instance = AgentInstance.new()
    result = add_case_note(db_conn, alice, instance, AddCaseNoteArgs(customer_id="cust_1", note="called back"))
    assert result["status"] == "note_added"
    row = db_conn.execute("SELECT note, owner_user_id FROM case_notes WHERE customer_id = 'cust_1'").fetchone()
    assert row["note"] == "called back"
    assert row["owner_user_id"] == "user-alice"


def test_add_case_note_blocked_for_other_users_customer(db_conn, bob):
    instance = AgentInstance.new()
    with pytest.raises(ToolError, match="customer_not_found"):
        add_case_note(db_conn, bob, instance, AddCaseNoteArgs(customer_id="cust_1", note="hi"))
