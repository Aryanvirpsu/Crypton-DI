"""
Task persistence + lifecycle (state boundaries 2 and 3; see LEARNING.md).

A Task is one specific requested objective within a Session. `Task.state`
is the small, schema-validated slice of structured application state
needed to perform it — never a dumping ground for raw model output. The
only place a tool result is allowed to become a state update is
`agent.derive_state_update`, and even that goes through
`TaskState.model_validate` on the way in here.

Every function that touches a task re-loads it through `get_task` first,
which re-checks ownership against the caller's HumanIdentity — a Task
object a caller already has lying around, or a task_id supplied by
anyone (including a model), is never trusted on its own.
"""

from __future__ import annotations

import json
import sqlite3
import time
import uuid
from dataclasses import dataclass, replace
from typing import Any

from pydantic import BaseModel, ConfigDict, Field

from .identity import HumanIdentity
from .lifecycle import TERMINAL_STATUSES, LifecycleError, TaskStatus, parse_status, validate_transition
from .sessions import Session, touch_session
from .tracing import Tracer


class TaskAccessError(Exception):
    """Raised when a task doesn't exist, or exists but isn't owned by the
    calling human. See sessions.SessionAccessError — same reasoning."""


class TaskState(BaseModel):
    """The full, fixed shape of a task's structured state.

    `extra="forbid"` means an update introducing an unrecognized field is
    rejected, not silently merged — this is what "malformed state update
    rejected" means in practice. Deliberately narrow: only what this
    lab's three tools ever need to remember, never a free-form blob.
    """

    model_config = ConfigDict(extra="forbid")

    last_tool: str | None = Field(default=None, max_length=64)
    last_customer_id: str | None = Field(default=None, max_length=64)
    last_order_id: str | None = Field(default=None, max_length=64)
    last_action: str | None = Field(default=None, max_length=64)


@dataclass(frozen=True)
class Task:
    task_id: str
    session_id: str
    human_user_id: str
    agent_id: str
    goal: str
    status: TaskStatus
    state: dict[str, Any]
    created_at: float
    updated_at: float
    completed_at: float | None


# Which trace event a status transition corresponds to — centralizes the
# lifecycle-event names (task_started, task_completed, task_failed,
# task_cancelled) in one place so agent.py never hand-picks an event name
# that could drift from the actual status being set.
_TRANSITION_EVENT: dict[TaskStatus, str] = {
    TaskStatus.RUNNING: "task_started",
    TaskStatus.COMPLETED: "task_completed",
    TaskStatus.FAILED: "task_failed",
    TaskStatus.CANCELLED: "task_cancelled",
}


def _task_from_row(row: sqlite3.Row) -> Task:
    return Task(
        task_id=row["task_id"],
        session_id=row["session_id"],
        human_user_id=row["human_user_id"],
        agent_id=row["agent_id"],
        goal=row["goal"],
        status=parse_status(row["status"]),
        state=json.loads(row["state"]),
        created_at=row["created_at"],
        updated_at=row["updated_at"],
        completed_at=row["completed_at"],
    )


def create_task(conn: sqlite3.Connection, *, session: Session, human: HumanIdentity, agent_id: str, goal: str) -> Task:
    if not goal or not goal.strip():
        raise ValueError("goal must not be empty")

    task_id = str(uuid.uuid4())
    now = time.time()
    initial_state = TaskState()
    conn.execute(
        "INSERT INTO tasks "
        "(task_id, session_id, human_user_id, agent_id, goal, status, state, created_at, updated_at, completed_at) "
        "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NULL)",
        (
            task_id,
            session.session_id,
            human.user_id,
            agent_id,
            goal,
            TaskStatus.CREATED.value,
            initial_state.model_dump_json(),
            now,
            now,
        ),
    )
    conn.commit()
    touch_session(conn, session)

    return Task(
        task_id=task_id,
        session_id=session.session_id,
        human_user_id=human.user_id,
        agent_id=agent_id,
        goal=goal,
        status=TaskStatus.CREATED,
        state=initial_state.model_dump(),
        created_at=now,
        updated_at=now,
        completed_at=None,
    )


def get_task(conn: sqlite3.Connection, human: HumanIdentity, task_id: str) -> Task:
    """Ownership-enforced load — the only way code in this package may
    read a task."""
    row = conn.execute("SELECT * FROM tasks WHERE task_id = ?", (task_id,)).fetchone()
    if row is None or row["human_user_id"] != human.user_id:
        raise TaskAccessError(f"task_not_found: {task_id}")
    return _task_from_row(row)


def update_task_status(
    conn: sqlite3.Connection,
    human: HumanIdentity,
    task_id: str,
    new_status: TaskStatus,
    *,
    tracer: Tracer | None = None,
    **event_data: Any,
) -> Task:
    """The only function allowed to change `tasks.status`.

    Always re-loads and re-checks ownership first, then requires the
    transition to be in lifecycle.py's allowed table — terminal statuses
    have no outgoing transitions there, so this raises rather than
    letting a completed/failed/cancelled task silently return to
    `running`.
    """
    task = get_task(conn, human, task_id)
    validate_transition(task.status, new_status)

    now = time.time()
    completed_at = now if new_status in TERMINAL_STATUSES else task.completed_at
    conn.execute(
        "UPDATE tasks SET status = ?, updated_at = ?, completed_at = ? WHERE task_id = ?",
        (new_status.value, now, completed_at, task_id),
    )
    conn.commit()

    if tracer is not None:
        event = _TRANSITION_EVENT.get(new_status, "task_status_changed")
        tracer.record(
            event,
            task_id=task_id,
            from_status=task.status.value,
            to_status=new_status.value,
            **event_data,
        )

    return replace(task, status=new_status, updated_at=now, completed_at=completed_at)


def update_task_state(
    conn: sqlite3.Connection,
    human: HumanIdentity,
    task_id: str,
    partial_update: dict[str, Any],
    *,
    tracer: Tracer | None = None,
) -> Task:
    """The only function allowed to change `tasks.state`.

    Merges `partial_update` onto the task's current state and validates
    the *result* against TaskState — an unknown field or wrong type is
    rejected (raises `pydantic.ValidationError`), never silently
    written. Refuses to update a task that has already reached a
    terminal status (raises LifecycleError) — this is what "completed
    tasks can't be modified via the normal active-task flow" means.
    """
    task = get_task(conn, human, task_id)
    if task.status in TERMINAL_STATUSES:
        raise LifecycleError(f"task_not_active: {task_id} is {task.status.value}")

    merged = {**task.state, **partial_update}
    validated = TaskState.model_validate(merged)  # raises pydantic.ValidationError on unknown/malformed fields

    now = time.time()
    conn.execute(
        "UPDATE tasks SET state = ?, updated_at = ? WHERE task_id = ?",
        (validated.model_dump_json(), now, task_id),
    )
    conn.commit()

    if tracer is not None:
        tracer.record("task_state_updated", task_id=task_id, update=partial_update)

    return replace(task, state=validated.model_dump(), updated_at=now)


def cancel_task(conn: sqlite3.Connection, human: HumanIdentity, task_id: str, *, tracer: Tracer | None = None) -> Task:
    """Human-initiated cancellation — independent of any agent run (there
    may not be one in progress). Goes through the same ownership check
    and transition table as every other status change."""
    return update_task_status(conn, human, task_id, TaskStatus.CANCELLED, tracer=tracer)
