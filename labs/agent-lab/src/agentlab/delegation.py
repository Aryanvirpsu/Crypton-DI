"""
Delegation (v0.4) — explicit authority a human principal grants to a
canonical agent identity.

Replaces the v0.1/v0.3 static, name-keyed permission map (authz.py,
removed) with a persisted, owned, expiring grant of authority. The bug
that motivated this: authz.py's REGISTRY was keyed by `AgentIdentity.name`
(a descriptive label two different registrations can share — e.g. the
same human's "support-agent" v0.1.0 and v0.2.0), not by the canonical
`agent_id` that actually identifies *which* registration this is. Two
agents with the same name but different agent_ids must never inherit each
other's authority just because they share a label — see
authorization.py's `authorize_tool_call` and LEARNING.md's v0.4 addendum.

A Delegation is keyed by `agent_id`, always. It is never resolved by name,
and granting one requires the principal to already own the agent it names
(agents.get_agent enforces this) — the same "you can only delegate
authority over an agent you actually own" rule that governs every other
write in this lab.

Kept deliberately NOT a policy engine:
  - `allowed_tools` is a flat set of tool names, checked with an `in`.
  - `resource_constraints` is a flat dict of resource_type -> explicit id
    set (e.g. {"customer_ids": {"cust_1"}}). Absence of a resource_type
    key means "nothing of that type is authorized" (safe default deny),
    not "unconstrained" — see authorization.py.
No wildcard, no expression language, no resource hierarchy. If this ever
needs to grow past that, it should become a deliberate, versioned
decision (see DECISIONS.md), not an incremental one here.
"""

from __future__ import annotations

import json
import sqlite3
import time
import uuid
from dataclasses import dataclass, replace
from typing import Iterable

from . import agents
from .identity import HumanIdentity
from .lifecycle import DelegationStatus, parse_delegation_status, validate_delegation_transition
from .tracing import Tracer

MAX_TTL_SECONDS = 60 * 60 * 24 * 30  # 30 days — a delegation must always be finite, and bounded


class DelegationAccessError(Exception):
    """Raised when a delegation doesn't exist, or exists but isn't owned
    (as principal) by the caller. Same "look identical" reasoning as
    AgentAccessError/TaskAccessError — never reveal that another
    principal's delegation exists."""


class DelegationValidationError(Exception):
    """Raised when a delegation grant's parameters are invalid: no tools,
    an unknown tool name, a non-positive or excessive TTL, or resource
    constraints in the wrong shape."""


@dataclass(frozen=True)
class Delegation:
    delegation_id: str
    principal_user_id: str
    agent_id: str
    task_id: str | None
    allowed_tools: frozenset[str]
    resource_constraints: dict[str, frozenset[str]]
    issued_at: float
    expires_at: float
    status: DelegationStatus
    revoked_at: float | None


def _new_delegation_id() -> str:
    """The only place a delegation_id is minted — never caller-supplied,
    exactly like agent_id/session_id/task_id/instance_id elsewhere."""
    return f"dlg_{uuid.uuid4().hex}"


def _encode_resource_constraints(constraints: dict[str, Iterable[str]]) -> str:
    return json.dumps({k: sorted(set(v)) for k, v in constraints.items()})


def _decode_resource_constraints(raw: str) -> dict[str, frozenset[str]]:
    return {k: frozenset(v) for k, v in json.loads(raw).items()}


def _delegation_from_row(row: sqlite3.Row) -> Delegation:
    return Delegation(
        delegation_id=row["delegation_id"],
        principal_user_id=row["principal_user_id"],
        agent_id=row["agent_id"],
        task_id=row["task_id"],
        allowed_tools=frozenset(json.loads(row["allowed_tools"])),
        resource_constraints=_decode_resource_constraints(row["resource_constraints"]),
        issued_at=row["issued_at"],
        expires_at=row["expires_at"],
        status=parse_delegation_status(row["status"]),
        revoked_at=row["revoked_at"],
    )


def grant_delegation(
    conn: sqlite3.Connection,
    principal: HumanIdentity,
    *,
    agent_id: str,
    allowed_tools: Iterable[str],
    resource_constraints: dict[str, Iterable[str]] | None = None,
    task_id: str | None = None,
    ttl_seconds: int,
    tracer: Tracer | None = None,
) -> Delegation:
    """Grant explicit, finite-lived authority over `agent_id` to itself,
    on `principal`'s behalf. Raises AgentAccessError (via agents.get_agent)
    unless `principal` already owns `agent_id` — a principal can only
    delegate authority over an agent they own, never an arbitrary one.

    `task_id`, when given, task-binds this delegation: it will only ever
    authorize calls made within that exact task (see authorization.py,
    step "delegation task matches when task-bound"). Left `None` for a
    delegation that authorizes any task run by this agent on this
    principal's behalf — task-bound is preferred where the caller has a
    task already in hand, but is not mandatory.
    """
    # Ownership check first: never let a caller grant authority over an
    # agent_id they don't own, and never leak whether it exists otherwise.
    agents.get_agent(conn, principal, agent_id)

    allowed_tools = frozenset(allowed_tools)
    if not allowed_tools:
        raise DelegationValidationError("allowed_tools must not be empty")

    resource_constraints = {k: frozenset(v) for k, v in (resource_constraints or {}).items()}

    if ttl_seconds <= 0:
        raise DelegationValidationError("ttl_seconds must be positive — a delegation must have a finite expiration")
    if ttl_seconds > MAX_TTL_SECONDS:
        raise DelegationValidationError(f"ttl_seconds must be at most {MAX_TTL_SECONDS}")

    delegation_id = _new_delegation_id()
    now = time.time()
    expires_at = now + ttl_seconds
    conn.execute(
        "INSERT INTO delegations "
        "(delegation_id, principal_user_id, agent_id, task_id, allowed_tools, resource_constraints, "
        "issued_at, expires_at, status, revoked_at) "
        "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NULL)",
        (
            delegation_id,
            principal.user_id,
            agent_id,
            task_id,
            json.dumps(sorted(allowed_tools)),
            _encode_resource_constraints(resource_constraints),
            now,
            expires_at,
            DelegationStatus.ACTIVE.value,
        ),
    )
    conn.commit()

    if tracer is not None:
        tracer.record(
            "delegation_granted",
            delegation_id=delegation_id,
            agent_id=agent_id,
            task_id=task_id,
            allowed_tools=sorted(allowed_tools),
        )

    return Delegation(
        delegation_id=delegation_id,
        principal_user_id=principal.user_id,
        agent_id=agent_id,
        task_id=task_id,
        allowed_tools=allowed_tools,
        resource_constraints=resource_constraints,
        issued_at=now,
        expires_at=expires_at,
        status=DelegationStatus.ACTIVE,
        revoked_at=None,
    )


def get_delegation(conn: sqlite3.Connection, principal: HumanIdentity, delegation_id: str) -> Delegation:
    """Ownership-enforced load — the only way application code (CLI,
    tests) may read a delegation back. Raises DelegationAccessError unless
    `delegation_id` exists AND its principal is `principal`."""
    row = conn.execute("SELECT * FROM delegations WHERE delegation_id = ?", (delegation_id,)).fetchone()
    if row is None or row["principal_user_id"] != principal.user_id:
        raise DelegationAccessError(f"delegation_not_found: {delegation_id}")
    return _delegation_from_row(row)


def get_delegation_unchecked(conn: sqlite3.Connection, delegation_id: str) -> Delegation | None:
    """Raw lookup with no ownership check — used only by the pre-execution
    authorization pipeline (authorization.py), which needs to distinguish
    "no such delegation" from "exists, but not this principal's" as two
    separate, individually-evidenced reason codes (DELEGATION_NOT_FOUND vs
    PRINCIPAL_MISMATCH). Never used to grant access to a delegation's
    contents outside that pipeline."""
    row = conn.execute("SELECT * FROM delegations WHERE delegation_id = ?", (delegation_id,)).fetchone()
    return None if row is None else _delegation_from_row(row)


def revoke_delegation(
    conn: sqlite3.Connection, principal: HumanIdentity, delegation_id: str, *, tracer: Tracer | None = None
) -> Delegation:
    """The only function allowed to change `delegations.status`. Revoking
    an already-revoked delegation raises LifecycleError (status
    transitions are one-way and re-revoking isn't a real transition) —
    callers that want idempotent revoke should catch that explicitly.
    Never deletes the row: a revoked delegation is retained forever as
    evidence/history (see module docstring)."""
    delegation = get_delegation(conn, principal, delegation_id)
    validate_delegation_transition(delegation.status, DelegationStatus.REVOKED)

    now = time.time()
    conn.execute(
        "UPDATE delegations SET status = ?, revoked_at = ? WHERE delegation_id = ?",
        (DelegationStatus.REVOKED.value, now, delegation_id),
    )
    conn.commit()

    if tracer is not None:
        tracer.record("delegation_revoked", delegation_id=delegation_id, agent_id=delegation.agent_id)

    return replace(delegation, status=DelegationStatus.REVOKED, revoked_at=now)


def bootstrap_delegation(
    conn: sqlite3.Connection,
    principal: HumanIdentity,
    agent_id: str,
    *,
    allowed_tools: Iterable[str],
    resource_constraints: dict[str, Iterable[str]] | None = None,
    ttl_seconds: int = 3600,
    tracer: Tracer | None = None,
) -> Delegation:
    """Local-dev convenience only — must be explicitly requested (see
    cli.py's `--bootstrap-delegation` flag; never invoked implicitly
    during a normal run). Unlike `agents.bootstrap_support_agent`, this is
    NOT idempotent: a delegation is a time-bounded grant, not a stable
    registration, so every call mints a fresh one. Exists purely so the
    lab's `--fake` CLI flow has something to authorize against without a
    separate manual `grant_delegation` step first."""
    return grant_delegation(
        conn,
        principal,
        agent_id=agent_id,
        allowed_tools=allowed_tools,
        resource_constraints=resource_constraints,
        ttl_seconds=ttl_seconds,
        tracer=tracer,
    )
