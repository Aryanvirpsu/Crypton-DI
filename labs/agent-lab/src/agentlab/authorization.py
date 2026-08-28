"""
Deterministic pre-execution authorization pipeline (v0.4).

This is the single choke point every tool call goes through immediately
before it executes — see agent.py's `_handle_tool_call`. It replaces
authz.py's static, name-keyed permission map with a check against a real,
persisted Delegation (delegation.py), keyed by canonical `agent_id`.

Every step here is a plain, deterministic comparison against already-
verified, database-backed values (the resolved Agent row, the real
AgentInstance, the real Delegation row) — never anything read out of a
model's tool-call arguments, and never a judgment call. The model has no
vote in any of this; see LEARNING.md's v0.4 addendum, "why the LLM never
makes an authorization decision."

`authorize_tool_call` always returns an AuthorizationDecision — ALLOW or
DENY — and always persists it (`record_decision`) before returning. It
never raises for an ordinary authorization failure: a DENY is the normal,
expected outcome of a call that shouldn't be allowed, not an exceptional
condition. Callers (agent.py) branch on `.decision`, and a DENY must
correspond to zero tool side effects — nothing past this function is ever
reached on DENY.
"""

from __future__ import annotations

import sqlite3
import time
import uuid
from dataclasses import dataclass

from . import agents, delegation as delegation_module
from .identity import AgentInstance, HumanIdentity
from .lifecycle import AgentStatus, DelegationStatus

# Stable, machine-readable reason codes. `ALLOW` is the only non-denial
# value `reason_code` ever takes.
ALLOW = "ALLOW"
AGENT_DISABLED = "AGENT_DISABLED"
DELEGATION_NOT_FOUND = "DELEGATION_NOT_FOUND"
DELEGATION_REVOKED = "DELEGATION_REVOKED"
DELEGATION_EXPIRED = "DELEGATION_EXPIRED"
PRINCIPAL_MISMATCH = "PRINCIPAL_MISMATCH"
AGENT_MISMATCH = "AGENT_MISMATCH"
TASK_MISMATCH = "TASK_MISMATCH"
TOOL_NOT_ALLOWED = "TOOL_NOT_ALLOWED"
RESOURCE_NOT_ALLOWED = "RESOURCE_NOT_ALLOWED"
INVALID_ARGUMENTS = "INVALID_ARGUMENTS"

DECISION_ALLOW = "ALLOW"
DECISION_DENY = "DENY"


@dataclass(frozen=True)
class AuthorizationDecision:
    decision_id: str
    decision: str  # DECISION_ALLOW | DECISION_DENY
    reason_code: str
    principal_user_id: str
    agent_id: str
    instance_id: str
    task_id: str
    delegation_id: str | None
    tool_name: str
    resource: str | None
    timestamp: float

    @property
    def allowed(self) -> bool:
        return self.decision == DECISION_ALLOW


def _new_decision_id() -> str:
    return f"authz_{uuid.uuid4().hex}"


def record_decision(
    conn: sqlite3.Connection,
    *,
    decision: str,
    reason_code: str,
    principal_user_id: str,
    agent_id: str,
    instance_id: str,
    task_id: str,
    delegation_id: str | None,
    tool_name: str,
    resource: str | None,
) -> AuthorizationDecision:
    """Persist one AuthorizationDecision row — append-only evidence, never
    updated or deleted. Called for every attempted consequential tool
    call, ALLOW or DENY, so a denial always leaves a trace even though it
    leaves no other side effect."""
    decision_id = _new_decision_id()
    timestamp = time.time()
    conn.execute(
        "INSERT INTO authorization_decisions "
        "(decision_id, decision, reason_code, principal_user_id, agent_id, instance_id, task_id, "
        "delegation_id, tool_name, resource, timestamp) "
        "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        (
            decision_id,
            decision,
            reason_code,
            principal_user_id,
            agent_id,
            instance_id,
            task_id,
            delegation_id,
            tool_name,
            resource,
            timestamp,
        ),
    )
    conn.commit()
    return AuthorizationDecision(
        decision_id=decision_id,
        decision=decision,
        reason_code=reason_code,
        principal_user_id=principal_user_id,
        agent_id=agent_id,
        instance_id=instance_id,
        task_id=task_id,
        delegation_id=delegation_id,
        tool_name=tool_name,
        resource=resource,
        timestamp=timestamp,
    )


def get_decisions_for_instance(conn: sqlite3.Connection, instance_id: str) -> list[AuthorizationDecision]:
    """Read-back for tests/inspection: every decision recorded for one
    AgentInstance, in the order they were made."""
    rows = conn.execute(
        "SELECT * FROM authorization_decisions WHERE instance_id = ? ORDER BY timestamp", (instance_id,)
    ).fetchall()
    return [
        AuthorizationDecision(
            decision_id=row["decision_id"],
            decision=row["decision"],
            reason_code=row["reason_code"],
            principal_user_id=row["principal_user_id"],
            agent_id=row["agent_id"],
            instance_id=row["instance_id"],
            task_id=row["task_id"],
            delegation_id=row["delegation_id"],
            tool_name=row["tool_name"],
            resource=row["resource"],
            timestamp=row["timestamp"],
        )
        for row in rows
    ]


def authorize_tool_call(
    conn: sqlite3.Connection,
    *,
    principal: HumanIdentity,
    agent_id: str,
    instance: AgentInstance,
    task_id: str,
    delegation_id: str,
    tool_name: str,
    resource_type: str | None,
    resource_id: str | None,
) -> AuthorizationDecision:
    """Run every pre-execution check, in order, and persist the result.

    `agent_id` is always the canonical id resolved server-side before this
    call (see agent.py) — never a name, never anything the model supplied.
    `resource_type`/`resource_id` describe the specific resource this tool
    call touches (e.g. `("customer_ids", "cust_1")`), or `(None, None)` for
    a tool with no resource scope to check.
    """

    def deny(reason_code: str) -> AuthorizationDecision:
        return record_decision(
            conn,
            decision=DECISION_DENY,
            reason_code=reason_code,
            principal_user_id=principal.user_id,
            agent_id=agent_id,
            instance_id=instance.instance_id,
            task_id=task_id,
            delegation_id=delegation_id,
            tool_name=tool_name,
            resource=f"{resource_type}:{resource_id}" if resource_type else None,
        )

    # 1 + 2: the agent exists, is owned by this principal, and is active.
    # get_agent's AgentAccessError already covers "doesn't exist" and
    # "exists but isn't yours" identically — never reveal which.
    try:
        agent_record = agents.get_agent(conn, principal, agent_id)
    except agents.AgentAccessError:
        return deny(AGENT_MISMATCH)
    if agent_record.status != AgentStatus.ACTIVE:
        return deny(AGENT_DISABLED)

    # 3: the AgentInstance this call is happening inside really belongs to
    # this canonical agent — never a different agent's instance.
    if instance.agent_id != agent_id:
        return deny(AGENT_MISMATCH)

    # 4: the principal making this call is the same human the instance was
    # created for.
    if instance.human_user_id != principal.user_id:
        return deny(PRINCIPAL_MISMATCH)

    # 5: a delegation with this id actually exists.
    delegation = delegation_module.get_delegation_unchecked(conn, delegation_id)
    if delegation is None:
        return deny(DELEGATION_NOT_FOUND)

    # 6: the delegation's principal is this principal — never someone
    # else's grant of authority.
    if delegation.principal_user_id != principal.user_id:
        return deny(PRINCIPAL_MISMATCH)

    # 7: the delegation's agent_id is this canonical agent_id — this is
    # the fix for the v0.3 bug: a delegation granted to one agent_id can
    # never authorize a different agent_id, even one registered under the
    # exact same name/owner. Names are never compared here at all.
    if delegation.agent_id != agent_id:
        return deny(AGENT_MISMATCH)

    # 8: when the delegation is task-bound, this call's task must be the
    # exact task it was bound to.
    if delegation.task_id is not None and delegation.task_id != task_id:
        return deny(TASK_MISMATCH)

    # 9-11: the delegation itself is still usable — not revoked, not
    # expired. (There is no third "inactive" state distinct from these
    # two; DelegationStatus is binary — see lifecycle.py.)
    if delegation.status == DelegationStatus.REVOKED:
        return deny(DELEGATION_REVOKED)
    if time.time() > delegation.expires_at:
        return deny(DELEGATION_EXPIRED)

    # 12: the specific tool being called is in this delegation's allowlist.
    if tool_name not in delegation.allowed_tools:
        return deny(TOOL_NOT_ALLOWED)

    # 13: the specific resource being touched is in scope. A resource type
    # absent from resource_constraints means nothing of that type is
    # authorized — safe default deny, never "unconstrained". A tool call
    # that couldn't even produce a resource_id (a missing/malformed
    # argument) is left to step 14 (argument validation) to reject as
    # INVALID_ARGUMENTS instead — that is the more accurate reason, and
    # tools.py's own per-user DB ownership check remains a second layer
    # of defense either way.
    if resource_type is not None and resource_id is not None:
        allowed_ids = delegation.resource_constraints.get(resource_type)
        if not allowed_ids or resource_id not in allowed_ids:
            return deny(RESOURCE_NOT_ALLOWED)

    # Steps 1-13 all passed. (Step 14, tool-argument schema validation,
    # happens in agent.py immediately after this returns ALLOW — see its
    # docstring for why argument shape is checked separately from
    # authority.)
    return record_decision(
        conn,
        decision=DECISION_ALLOW,
        reason_code=ALLOW,
        principal_user_id=principal.user_id,
        agent_id=agent_id,
        instance_id=instance.instance_id,
        task_id=task_id,
        delegation_id=delegation_id,
        tool_name=tool_name,
        resource=f"{resource_type}:{resource_id}" if resource_type else None,
    )


# Single source of truth: which argument field a tool call's resource
# scope is checked against, and under which resource_type key in
# Delegation.resource_constraints. A tool with no entry here has no
# resource scope to check (step 13 is skipped for it).
TOOL_RESOURCE_FIELDS: dict[str, tuple[str, str]] = {
    "get_customer": ("customer_ids", "customer_id"),
    "get_order": ("order_ids", "order_id"),
    "add_case_note": ("customer_ids", "customer_id"),
}


def resource_for_tool_call(tool_name: str, raw_arguments: dict) -> tuple[str | None, str | None]:
    """Extract `(resource_type, resource_id)` from a tool call's raw,
    untrusted arguments for the resource-scope check (step 13) — read
    defensively with `.get()` before the arguments are schema-validated,
    since authorization happens before that (step 14). Only ever reads a
    fixed, known field name for a fixed, known tool; never reflects
    arbitrary model input back into a query."""
    entry = TOOL_RESOURCE_FIELDS.get(tool_name)
    if entry is None:
        return None, None
    resource_type, field_name = entry
    resource_id = raw_arguments.get(field_name) if isinstance(raw_arguments, dict) else None
    if resource_id is not None and not isinstance(resource_id, str):
        resource_id = None  # wrong type is never trusted as a resource id; step 13 will deny on the missing id
    return resource_type, resource_id
