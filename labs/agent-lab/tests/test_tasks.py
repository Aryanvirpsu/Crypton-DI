import pytest
from pydantic import ValidationError

from agentlab.lifecycle import LifecycleError, TaskStatus
from agentlab.sessions import create_session
from agentlab.tasks import (
    TaskAccessError,
    cancel_task,
    create_task,
    get_task,
    update_task_state,
    update_task_status,
)


def test_create_task(db_conn, alice):
    session = create_session(db_conn, alice)
    task = create_task(db_conn, session=session, human=alice, agent_id="support-agent", goal="look up cust_1")
    assert task.status == TaskStatus.CREATED
    assert task.session_id == session.session_id
    assert task.human_user_id == "user-alice"
    assert task.goal == "look up cust_1"
    assert task.state == {"last_tool": None, "last_customer_id": None, "last_order_id": None, "last_action": None}
    assert task.completed_at is None


def test_task_ownership_get_succeeds_for_owner(db_conn, alice):
    session = create_session(db_conn, alice)
    task = create_task(db_conn, session=session, human=alice, agent_id="support-agent", goal="x")
    reloaded = get_task(db_conn, alice, task.task_id)
    assert reloaded == task


def test_multiple_tasks_in_one_session(db_conn, alice):
    session = create_session(db_conn, alice)
    task_a = create_task(db_conn, session=session, human=alice, agent_id="support-agent", goal="task A")
    task_b = create_task(db_conn, session=session, human=alice, agent_id="support-agent", goal="task B")
    assert task_a.task_id != task_b.task_id
    assert task_a.session_id == task_b.session_id == session.session_id


def test_cross_user_task_access_rejected(db_conn, alice, bob):
    session = create_session(db_conn, alice)
    task = create_task(db_conn, session=session, human=alice, agent_id="support-agent", goal="x")
    with pytest.raises(TaskAccessError, match="task_not_found"):
        get_task(db_conn, bob, task.task_id)


def test_cross_user_task_modification_rejected(db_conn, alice, bob):
    session = create_session(db_conn, alice)
    task = create_task(db_conn, session=session, human=alice, agent_id="support-agent", goal="x")

    with pytest.raises(TaskAccessError, match="task_not_found"):
        update_task_status(db_conn, bob, task.task_id, TaskStatus.RUNNING)

    with pytest.raises(TaskAccessError, match="task_not_found"):
        update_task_state(db_conn, bob, task.task_id, {"last_tool": "get_customer"})

    # confirm nothing changed
    still_alices = get_task(db_conn, alice, task.task_id)
    assert still_alices.status == TaskStatus.CREATED


def test_task_state_persists(db_conn, alice):
    session = create_session(db_conn, alice)
    task = create_task(db_conn, session=session, human=alice, agent_id="support-agent", goal="x")
    update_task_status(db_conn, alice, task.task_id, TaskStatus.RUNNING)
    updated = update_task_state(db_conn, alice, task.task_id, {"last_tool": "get_customer", "last_customer_id": "cust_1"})
    assert updated.state["last_tool"] == "get_customer"
    assert updated.state["last_customer_id"] == "cust_1"

    reloaded = get_task(db_conn, alice, task.task_id)
    assert reloaded.state["last_tool"] == "get_customer"
    assert reloaded.state["last_customer_id"] == "cust_1"


def test_update_task_state_rejects_unknown_field(db_conn, alice):
    session = create_session(db_conn, alice)
    task = create_task(db_conn, session=session, human=alice, agent_id="support-agent", goal="x")
    update_task_status(db_conn, alice, task.task_id, TaskStatus.RUNNING)
    with pytest.raises(ValidationError):
        update_task_state(db_conn, alice, task.task_id, {"shell_command": "rm -rf /"})


def test_update_task_state_rejects_wrong_type(db_conn, alice):
    session = create_session(db_conn, alice)
    task = create_task(db_conn, session=session, human=alice, agent_id="support-agent", goal="x")
    update_task_status(db_conn, alice, task.task_id, TaskStatus.RUNNING)
    with pytest.raises(ValidationError):
        update_task_state(db_conn, alice, task.task_id, {"last_customer_id": 12345})


def test_running_to_completed(db_conn, alice):
    session = create_session(db_conn, alice)
    task = create_task(db_conn, session=session, human=alice, agent_id="support-agent", goal="x")
    task = update_task_status(db_conn, alice, task.task_id, TaskStatus.RUNNING)
    task = update_task_status(db_conn, alice, task.task_id, TaskStatus.COMPLETED)
    assert task.status == TaskStatus.COMPLETED
    assert task.completed_at is not None


def test_invalid_transition_via_update_task_status_rejected(db_conn, alice):
    session = create_session(db_conn, alice)
    task = create_task(db_conn, session=session, human=alice, agent_id="support-agent", goal="x")
    with pytest.raises(LifecycleError, match="invalid_transition"):
        update_task_status(db_conn, alice, task.task_id, TaskStatus.COMPLETED)  # skips running


def test_completed_task_cannot_silently_resume(db_conn, alice):
    session = create_session(db_conn, alice)
    task = create_task(db_conn, session=session, human=alice, agent_id="support-agent", goal="x")
    update_task_status(db_conn, alice, task.task_id, TaskStatus.RUNNING)
    update_task_status(db_conn, alice, task.task_id, TaskStatus.COMPLETED)

    with pytest.raises(LifecycleError, match="invalid_transition"):
        update_task_status(db_conn, alice, task.task_id, TaskStatus.RUNNING)

    # state updates are also refused once terminal — the "normal active
    # task flow" cannot be used to modify a completed task.
    with pytest.raises(LifecycleError, match="task_not_active"):
        update_task_state(db_conn, alice, task.task_id, {"last_tool": "get_customer"})


def test_unknown_lifecycle_status_rejected_at_db_boundary(db_conn, alice):
    """A status string that never went through TaskStatus can't sneak
    into a Task via a raw DB write and be read back successfully."""
    session = create_session(db_conn, alice)
    task = create_task(db_conn, session=session, human=alice, agent_id="support-agent", goal="x")
    db_conn.execute("UPDATE tasks SET status = 'orbiting' WHERE task_id = ?", (task.task_id,))
    db_conn.commit()
    with pytest.raises(LifecycleError, match="unknown_status"):
        get_task(db_conn, alice, task.task_id)


def test_cancel_task(db_conn, alice):
    session = create_session(db_conn, alice)
    task = create_task(db_conn, session=session, human=alice, agent_id="support-agent", goal="x")
    cancelled = cancel_task(db_conn, alice, task.task_id)
    assert cancelled.status == TaskStatus.CANCELLED
    assert cancelled.completed_at is not None


def test_cross_user_cancel_rejected(db_conn, alice, bob):
    session = create_session(db_conn, alice)
    task = create_task(db_conn, session=session, human=alice, agent_id="support-agent", goal="x")
    with pytest.raises(TaskAccessError, match="task_not_found"):
        cancel_task(db_conn, bob, task.task_id)
