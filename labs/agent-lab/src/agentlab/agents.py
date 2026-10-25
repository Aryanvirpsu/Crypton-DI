"""
Agent Registry (v0.3) — persisted AgentIdentity records.

Replaces ad-hoc, caller-constructed `AgentIdentity(agent_id="support-agent")`
values with real, owned, database-backed registrations. An `agent_id` is
never chosen by a caller or a model — `register_agent` is the only
function that mints one (`_new_agent_id`), and every subsequent use of
an `agent_id` goes through `require_active_agent`, which re-verifies
existence, ownership, and status on every call — the same "never trust
a cached/handed-back ID" discipline `sessions.py`/`tasks.py` already use
for `session_id`/`task_id`.

Uniqueness rule: `(owner_user_id, name, version)` must be unique. The
same human can't register the same name+version twice (that would just
be the same agent registered again under a new ID); different humans may
each register their own "support-agent" v0.1.0 independently, and one
human may register multiple *versions* of a same-named agent. Enforced
by a UNIQUE constraint in db.py (caught here as sqlite3.IntegrityError),
not just a pre-check `SELECT`, to avoid a check-then-insert race.
"""

from __future__ import annotations

import sqlite3
import time
import uuid
from dataclasses import dataclass, replace

from .identity import HumanIdentity
from .lifecycle import AgentStatus, parse_agent_status, validate_agent_transition
from .tracing import Tracer

MAX_NAME_LENGTH = 64
MAX_VERSION_LENGTH = 32

DEFAULT_BOOTSTRAP_NAME = "support-agent"
DEFAULT_BOOTSTRAP_VERSION = "0.1.0"


class AgentAccessError(Exception):
    """Raised when an agent doesn't exist, or exists but isn't owned by
    the calling human. Both cases look identical to the caller — same
    reasoning as sessions.SessionAccessError / tasks.TaskAccessError:
    never reveal that another human's agent exists."""


class AgentNotActiveError(Exception):
    """Raised when an agent exists and is owned by the caller, but its
    status isn't `active` (e.g. disabled). Kept distinct from
    AgentAccessError so "not yours / doesn't exist" and "yours, but
    currently disabled" are two different, individually-testable
    outcomes."""


class DuplicateAgentError(Exception):
    """Raised when a registration would violate the
    (owner_user_id, name, version) uniqueness rule."""


class AgentValidationError(Exception):
    """Raised when a registration's name/version fail basic validation."""


@dataclass(frozen=True)
class Agent:
    agent_id: str
    name: str
    version: str
    owner_user_id: str
    status: AgentStatus
    created_at: float
    updated_at: float


def _new_agent_id() -> str:
    """The only place an agent_id is minted. Never derived from, or
    influenced by, caller/model input — a fresh random ID every call,
    exactly like session_id/task_id/instance_id elsewhere in this lab."""
    return f"agt_{uuid.uuid4().hex}"


def _validate_name_version(name: str, version: str) -> None:
    if not name or not name.strip():
        raise AgentValidationError("name must not be empty")
    if len(name) > MAX_NAME_LENGTH:
        raise AgentValidationError(f"name must be at most {MAX_NAME_LENGTH} characters")
    if not version or not version.strip():
        raise AgentValidationError("version must not be empty")
    if len(version) > MAX_VERSION_LENGTH:
        raise AgentValidationError(f"version must be at most {MAX_VERSION_LENGTH} characters")


def _agent_from_row(row: sqlite3.Row) -> Agent:
    return Agent(
        agent_id=row["agent_id"],
        name=row["name"],
        version=row["version"],
        owner_user_id=row["owner_user_id"],
        status=parse_agent_status(row["status"]),
        created_at=row["created_at"],
        updated_at=row["updated_at"],
    )


def register_agent(
    conn: sqlite3.Connection,
    owner: HumanIdentity,
    *,
    name: str,
    version: str,
    tracer: Tracer | None = None,
) -> Agent:
    """Register a new agent owned by `owner`. Generates the canonical
    `agent_id` — the caller supplies only `name`/`version`, never an ID.
    Raises AgentValidationError for a bad name/version, or
    DuplicateAgentError if `owner` already has an agent registered under
    this exact (name, version)."""
    _validate_name_version(name, version)
    agent_id = _new_agent_id()
    now = time.time()
    try:
        conn.execute(
            "INSERT INTO agents (agent_id, name, version, owner_user_id, status, created_at, updated_at) "
            "VALUES (?, ?, ?, ?, ?, ?, ?)",
            (agent_id, name, version, owner.user_id, AgentStatus.ACTIVE.value, now, now),
        )
        conn.commit()
    except sqlite3.IntegrityError as e:
        raise DuplicateAgentError(f"agent_already_registered: {owner.user_id}/{name}/{version}") from e

    if tracer is not None:
        tracer.record("agent_registered", agent_id=agent_id, agent_name=name, agent_version=version)

    return Agent(
        agent_id=agent_id,
        name=name,
        version=version,
        owner_user_id=owner.user_id,
        status=AgentStatus.ACTIVE,
        created_at=now,
        updated_at=now,
    )


def get_agent(conn: sqlite3.Connection, human: HumanIdentity, agent_id: str) -> Agent:
    """Ownership-enforced load — the only way code in this package may
    read an agent. Raises AgentAccessError unless `agent_id` exists AND
    belongs to `human`."""
    row = conn.execute("SELECT * FROM agents WHERE agent_id = ?", (agent_id,)).fetchone()
    if row is None or row["owner_user_id"] != human.user_id:
        raise AgentAccessError(f"agent_not_found: {agent_id}")
    return _agent_from_row(row)


def list_agents(conn: sqlite3.Connection, human: HumanIdentity) -> list[Agent]:
    """Every agent owned by `human`, active or disabled — never another
    human's, regardless of status."""
    rows = conn.execute(
        "SELECT * FROM agents WHERE owner_user_id = ? ORDER BY created_at", (human.user_id,)
    ).fetchall()
    return [_agent_from_row(row) for row in rows]


def _set_status(
    conn: sqlite3.Connection,
    human: HumanIdentity,
    agent_id: str,
    new_status: AgentStatus,
    *,
    tracer: Tracer | None,
    event: str,
) -> Agent:
    agent = get_agent(conn, human, agent_id)  # ownership re-checked, never trust a cached Agent
    validate_agent_transition(agent.status, new_status)
    now = time.time()
    conn.execute("UPDATE agents SET status = ?, updated_at = ? WHERE agent_id = ?", (new_status.value, now, agent_id))
    conn.commit()
    if tracer is not None:
        tracer.record(event, agent_id=agent_id, from_status=agent.status.value, to_status=new_status.value)
    return replace(agent, status=new_status, updated_at=now)


def disable_agent(conn: sqlite3.Connection, human: HumanIdentity, agent_id: str, *, tracer: Tracer | None = None) -> Agent:
    return _set_status(conn, human, agent_id, AgentStatus.DISABLED, tracer=tracer, event="agent_disabled")


def enable_agent(conn: sqlite3.Connection, human: HumanIdentity, agent_id: str, *, tracer: Tracer | None = None) -> Agent:
    """Re-enabling is cleanly supported — same transition table, just the
    other direction (see lifecycle.py)."""
    return _set_status(conn, human, agent_id, AgentStatus.ACTIVE, tracer=tracer, event="agent_enabled")


def require_active_agent(conn: sqlite3.Connection, human: HumanIdentity, agent_id: str) -> Agent:
    """The single function `AgentRunner.run()` calls before creating an
    AgentInstance: exists + owned (via get_agent) + status == active, in
    that order. Raises AgentAccessError for an unknown or foreign
    agent_id, AgentNotActiveError for a real, owned, but disabled one.
    Never silently substitutes a different agent — it either returns
    exactly the agent asked for, or raises."""
    agent = get_agent(conn, human, agent_id)
    if agent.status != AgentStatus.ACTIVE:
        raise AgentNotActiveError(f"agent_not_active: {agent_id} is {agent.status.value}")
    return agent


def bootstrap_support_agent(conn: sqlite3.Connection, human: HumanIdentity) -> Agent:
    """Local-dev convenience only — must be explicitly requested (see
    cli.py's `--bootstrap-support-agent` flag; never invoked implicitly
    during a normal run). Creates, or reuses if it already exists,
    `human`'s own "support-agent" v0.1.0 registration, so a fresh lab
    install has something to run against without a separate manual
    `register` step first. Idempotent per owner thanks to the
    uniqueness rule — safe to call on every dev run."""
    for agent in list_agents(conn, human):
        if agent.name == DEFAULT_BOOTSTRAP_NAME and agent.version == DEFAULT_BOOTSTRAP_VERSION:
            return agent
    return register_agent(conn, human, name=DEFAULT_BOOTSTRAP_NAME, version=DEFAULT_BOOTSTRAP_VERSION)
