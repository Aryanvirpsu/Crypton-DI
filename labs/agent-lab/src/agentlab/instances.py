"""
Persistence for AgentInstance rows — "every instance references
instance_id, agent_id, human_user_id, session_id, task_id, start/end
timestamps, status."

`identity.AgentInstance` is the *identity* type (who/what this run is);
this module is its persistence, kept separate the same way
sessions.py/tasks.py separate the Session/Task dataclasses from their
SQLite operations. An AgentInstance is always subordinate to a Task, so
protecting Task ownership (tasks.get_task) already protects it
transitively — this module doesn't duplicate an ownership boundary, it
just records and reads instance rows.
"""

from __future__ import annotations

import sqlite3
import time

from .identity import AgentInstance, HumanIdentity

INSTANCE_RUNNING = "running"
INSTANCE_COMPLETED = "completed"
INSTANCE_FAILED = "failed"
INSTANCE_CANCELLED = "cancelled"

_VALID_INSTANCE_STATUSES = frozenset({INSTANCE_RUNNING, INSTANCE_COMPLETED, INSTANCE_FAILED, INSTANCE_CANCELLED})


def record_instance_started(conn: sqlite3.Connection, instance: AgentInstance) -> None:
    """Persist the AgentInstance row at the moment execution begins —
    "create AgentInstance" in the lifecycle flow, called after the Task
    it belongs to already exists."""
    conn.execute(
        "INSERT INTO agent_instances "
        "(instance_id, agent_id, human_user_id, session_id, task_id, started_at, ended_at, status) "
        "VALUES (?, ?, ?, ?, ?, ?, NULL, ?)",
        (
            instance.instance_id,
            instance.agent_id,
            instance.human_user_id,
            instance.session_id,
            instance.task_id,
            instance.started_at,
            INSTANCE_RUNNING,
        ),
    )
    conn.commit()


def record_instance_finished(conn: sqlite3.Connection, instance_id: str, *, status: str, ended_at: float | None = None) -> None:
    if status not in _VALID_INSTANCE_STATUSES:
        raise ValueError(f"unknown_instance_status: {status}")
    ended_at = ended_at if ended_at is not None else time.time()
    conn.execute(
        "UPDATE agent_instances SET status = ?, ended_at = ? WHERE instance_id = ?",
        (status, ended_at, instance_id),
    )
    conn.commit()


def get_instance(conn: sqlite3.Connection, human: HumanIdentity, instance_id: str) -> AgentInstance | None:
    """Read-back for tests/inspection: returns None if the instance
    doesn't exist or isn't the caller's — mirrors the ownership pattern
    elsewhere, but this is not itself a security boundary (nothing routes
    tool calls through instance_id lookups; Task ownership is what
    actually gates execution)."""
    row = conn.execute("SELECT * FROM agent_instances WHERE instance_id = ?", (instance_id,)).fetchone()
    if row is None or row["human_user_id"] != human.user_id:
        return None
    return AgentInstance(
        instance_id=row["instance_id"],
        started_at=row["started_at"],
        agent_id=row["agent_id"],
        human_user_id=row["human_user_id"],
        session_id=row["session_id"],
        task_id=row["task_id"],
        ended_at=row["ended_at"],
        status=row["status"],
    )
