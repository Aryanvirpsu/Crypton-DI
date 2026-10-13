"""
Human Approval bound to the exact consequential action (v0.5).

v0.4 answers "does this agent possess delegated authority?" (delegation.py
/ authorization.py). This module answers a different question that
Delegation cannot: "for this specific tool call, did the correct human
approve *this exact action*?" Both must say yes before a tool that
requires approval executes — see agent.py's `_handle_tool_call`, where
this check runs strictly after `authorization.authorize_tool_call`
returns ALLOW, never as a substitute for it and never able to override a
DENY from it.

Kept distinct from Delegation and the Tool Registry, per LEARNING.md:

    Tool Registry = what tools exist            (models.py)
    Delegation    = what authority this agent has (delegation.py)
    Approval      = a human's confirmation of ONE exact action (this module)

The central security property (see `compute_argument_hash`): an Approval
is worthless if it only binds to a tool *name*. It binds to the tool
name, the resource, AND a canonical hash of the validated arguments — so
approving "add_case_note(customer_id=cust_1, note='refund approved')" can
never be replayed against a mutated amount, a different customer, or a
different tool. The hash is computed over already-Pydantic-validated
arguments, never raw model JSON (see models.TOOL_ARG_MODELS) — this is
the same "never hash/trust arbitrary model text" discipline
authorization.py's resource-scope check already follows.

WebAuthn boundary: see webauthn.py. `approve()` performs (or, in tests,
simulates via FakeWebAuthnVerifier) the step-up ceremony that turns a
pending Approval into an approved one; the challenge it verifies against
is derived from `approval_id` + `argument_hash`, not caller-supplied, so
the ceremony itself is bound to the exact action, not just to "some
approval or other."
"""

from __future__ import annotations

import hashlib
import json
import sqlite3
import time
import uuid
from dataclasses import dataclass, replace
from typing import Any

from . import agents, authorization, delegation as delegation_module, models
from .identity import HumanIdentity
from .lifecycle import ApprovalStatus, parse_approval_status, validate_approval_transition
from .webauthn import WebAuthnVerificationError, WebAuthnVerifier

MAX_APPROVAL_TTL_SECONDS = 15 * 60  # finite AND short — an approval is not a delegation

# Approval-specific reason codes. ALLOW and the DECISION_* constants are
# reused from authorization.py — there is exactly one evidence ledger
# (authorization_decisions), and "approved and consumed" is still, at
# root, an authorization decision; see authorization.py's module
# docstring for why the LLM never participates in any of these.
APPROVAL_REQUIRED = "APPROVAL_REQUIRED"
APPROVAL_NOT_FOUND = "APPROVAL_NOT_FOUND"
APPROVAL_PENDING = "APPROVAL_PENDING"
APPROVAL_EXPIRED = "APPROVAL_EXPIRED"
APPROVAL_ALREADY_CONSUMED = "APPROVAL_ALREADY_CONSUMED"
APPROVAL_CANCELLED = "APPROVAL_CANCELLED"
APPROVAL_PRINCIPAL_MISMATCH = "APPROVAL_PRINCIPAL_MISMATCH"
APPROVAL_AGENT_MISMATCH = "APPROVAL_AGENT_MISMATCH"
APPROVAL_INSTANCE_MISMATCH = "APPROVAL_INSTANCE_MISMATCH"
APPROVAL_TASK_MISMATCH = "APPROVAL_TASK_MISMATCH"
APPROVAL_DELEGATION_MISMATCH = "APPROVAL_DELEGATION_MISMATCH"
APPROVAL_TOOL_MISMATCH = "APPROVAL_TOOL_MISMATCH"
APPROVAL_RESOURCE_MISMATCH = "APPROVAL_RESOURCE_MISMATCH"
APPROVAL_ARGUMENT_MISMATCH = "APPROVAL_ARGUMENT_MISMATCH"
WEBAUTHN_VERIFICATION_FAILED = "WEBAUTHN_VERIFICATION_FAILED"


class ApprovalAccessError(Exception):
    """Raised when an approval doesn't exist, or exists but isn't owned
    (as principal) by the caller. Same "look identical" reasoning as
    AgentAccessError/DelegationAccessError."""


class ApprovalValidationError(Exception):
    """Raised when an approval request's parameters are invalid: unknown
    tool, a resource_type that doesn't match the tool, a non-positive or
    excessive TTL, or arguments that don't validate against the tool's
    own schema."""


class ApprovalLifecycleError(Exception):
    """Raised for an invalid Approval status transition (e.g. approving
    an already-consumed approval, or cancelling one already cancelled).
    See lifecycle.validate_approval_transition."""


@dataclass(frozen=True)
class Approval:
    approval_id: str
    principal_user_id: str
    agent_id: str
    instance_id: str | None
    task_id: str | None
    delegation_id: str
    tool_name: str
    resource: str | None
    argument_hash: str
    issued_at: float
    expires_at: float
    status: ApprovalStatus
    approved_at: float | None
    consumed_at: float | None
    webauthn_reference: str | None


def _new_approval_id() -> str:
    return f"apr_{uuid.uuid4().hex}"


def canonicalize_arguments(validated_arguments: dict[str, Any]) -> str:
    """Deterministic JSON encoding of already-validated arguments: sorted
    keys, no whitespace, stable across dict-construction order. Never
    called on raw model output — only on a Pydantic model's `.model_dump()`
    (see models.TOOL_ARG_MODELS), which has already rejected unknown
    fields and coerced types."""
    return json.dumps(validated_arguments, sort_keys=True, separators=(",", ":"), default=str)


def compute_argument_hash(tool_name: str, resource: str | None, validated_arguments: dict[str, Any]) -> str:
    """The action digest: tool_name + resource identity + canonical
    validated arguments, hashed. This is what an Approval actually binds
    to — not the tool name alone. Changing any one input (a different
    amount, a different customer_id, even reusing the exact same
    arguments against a different tool) produces a different digest, and
    a different digest is an automatic APPROVAL_ARGUMENT_MISMATCH."""
    canonical = f"{tool_name}|{resource or ''}|{canonicalize_arguments(validated_arguments)}"
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()


def _approval_from_row(row: sqlite3.Row) -> Approval:
    return Approval(
        approval_id=row["approval_id"],
        principal_user_id=row["principal_user_id"],
        agent_id=row["agent_id"],
        instance_id=row["instance_id"],
        task_id=row["task_id"],
        delegation_id=row["delegation_id"],
        tool_name=row["tool_name"],
        resource=row["resource"],
        argument_hash=row["argument_hash"],
        issued_at=row["issued_at"],
        expires_at=row["expires_at"],
        status=parse_approval_status(row["status"]),
        approved_at=row["approved_at"],
        consumed_at=row["consumed_at"],
        webauthn_reference=row["webauthn_reference"],
    )


def request_approval(
    conn: sqlite3.Connection,
    principal: HumanIdentity,
    *,
    agent_id: str,
    delegation_id: str,
    tool_name: str,
    resource_type: str | None,
    resource_id: str | None,
    proposed_arguments: dict[str, Any],
    instance_id: str | None = None,
    task_id: str | None = None,
    ttl_seconds: int,
) -> Approval:
    """Request approval for one exact, specific tool call. Re-validates
    `proposed_arguments` against the tool's own Pydantic model — the same
    schema `agent.py` enforces at execution time — so the hash stored here
    and the hash recomputed at execution time are canonicalized from the
    same shape, not just "whatever the caller happened to type."

    Requires the principal to already own `agent_id` (agents.get_agent)
    and the referenced delegation (delegation.get_delegation), and that
    the delegation actually names this same agent_id — an approval never
    exists in a vacuum, only in the context of a real, owned delegation.
    None of this checking is a substitute for the pre-execution pipeline's
    own re-verification at consume time; it only rejects an obviously
    malformed request early.
    """
    agents.get_agent(conn, principal, agent_id)  # ownership-enforced; raises AgentAccessError
    grant = delegation_module.get_delegation(conn, principal, delegation_id)  # ownership-enforced
    if grant.agent_id != agent_id:
        raise ApprovalValidationError(f"delegation_agent_mismatch: {delegation_id} is not for {agent_id}")

    arg_model = models.TOOL_ARG_MODELS.get(tool_name)
    if arg_model is None:
        raise ApprovalValidationError(f"unknown_tool: {tool_name}")

    expected_resource_field = authorization.TOOL_RESOURCE_FIELDS.get(tool_name)
    if expected_resource_field is not None and expected_resource_field[0] != resource_type:
        raise ApprovalValidationError(
            f"resource_type_mismatch: {tool_name} expects resource_type={expected_resource_field[0]!r}, got {resource_type!r}"
        )
    if expected_resource_field is None and resource_type is not None:
        raise ApprovalValidationError(f"unscoped_tool_given_resource_type: {tool_name}")

    try:
        validated = arg_model.model_validate(proposed_arguments)
    except Exception as e:  # pydantic.ValidationError — re-raised as our own error type
        raise ApprovalValidationError(f"invalid_arguments: {e}") from e

    if ttl_seconds <= 0:
        raise ApprovalValidationError("ttl_seconds must be positive — an approval must have a finite expiration")
    if ttl_seconds > MAX_APPROVAL_TTL_SECONDS:
        raise ApprovalValidationError(
            f"ttl_seconds must be at most {MAX_APPROVAL_TTL_SECONDS} — approvals are short-lived by design"
        )

    resource = f"{resource_type}:{resource_id}" if resource_type else None
    argument_hash = compute_argument_hash(tool_name, resource, validated.model_dump())

    approval_id = _new_approval_id()
    now = time.time()
    expires_at = now + ttl_seconds
    conn.execute(
        "INSERT INTO approvals "
        "(approval_id, principal_user_id, agent_id, instance_id, task_id, delegation_id, tool_name, resource, "
        "argument_hash, issued_at, expires_at, status, approved_at, consumed_at, webauthn_reference) "
        "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, NULL, NULL)",
        (
            approval_id,
            principal.user_id,
            agent_id,
            instance_id,
            task_id,
            delegation_id,
            tool_name,
            resource,
            argument_hash,
            now,
            expires_at,
            ApprovalStatus.PENDING.value,
        ),
    )
    conn.commit()

    return Approval(
        approval_id=approval_id,
        principal_user_id=principal.user_id,
        agent_id=agent_id,
        instance_id=instance_id,
        task_id=task_id,
        delegation_id=delegation_id,
        tool_name=tool_name,
        resource=resource,
        argument_hash=argument_hash,
        issued_at=now,
        expires_at=expires_at,
        status=ApprovalStatus.PENDING,
        approved_at=None,
        consumed_at=None,
        webauthn_reference=None,
    )


def get_approval(conn: sqlite3.Connection, principal: HumanIdentity, approval_id: str) -> Approval:
    """Ownership-enforced load — raises ApprovalAccessError unless
    `approval_id` exists AND its principal is `principal`."""
    row = conn.execute("SELECT * FROM approvals WHERE approval_id = ?", (approval_id,)).fetchone()
    if row is None or row["principal_user_id"] != principal.user_id:
        raise ApprovalAccessError(f"approval_not_found: {approval_id}")
    return _approval_from_row(row)


def _load_and_maybe_expire(conn: sqlite3.Connection, approval_id: str) -> Approval | None:
    """Raw lookup (no ownership check — used only by the pre-execution
    pipeline, exactly like delegation.get_delegation_unchecked) that also
    lazily transitions pending/approved -> expired if the TTL has passed.
    Expiry is checked live every time, never cached and never swept by a
    background job — the same discipline delegation.py's expiry check
    uses."""
    row = conn.execute("SELECT * FROM approvals WHERE approval_id = ?", (approval_id,)).fetchone()
    if row is None:
        return None
    approval = _approval_from_row(row)
    if approval.status in (ApprovalStatus.PENDING, ApprovalStatus.APPROVED) and time.time() > approval.expires_at:
        conn.execute(
            "UPDATE approvals SET status = ? WHERE approval_id = ? AND status = ?",
            (ApprovalStatus.EXPIRED.value, approval_id, approval.status.value),
        )
        conn.commit()
        approval = replace(approval, status=ApprovalStatus.EXPIRED)
    return approval


def approve(
    conn: sqlite3.Connection,
    principal: HumanIdentity,
    approval_id: str,
    *,
    verifier: WebAuthnVerifier,
) -> Approval:
    """The human confirms this exact action via WebAuthn step-up. The
    challenge verified is derived from `approval_id` + `argument_hash` —
    never a caller-supplied string — so the ceremony is cryptographically
    tied to *this* action, not interchangeable with approving any other
    pending request. Raises WebAuthnVerificationError (and leaves the
    approval untouched, still pending) if the ceremony fails.

    Raises ApprovalLifecycleError if the approval is not currently
    pending (already approved/consumed/cancelled/expired) — approving is
    a one-way, one-time transition."""
    current = get_approval(conn, principal, approval_id)
    validate_approval_transition(current.status, ApprovalStatus.APPROVED)

    challenge = f"{current.approval_id}:{current.argument_hash}"
    evidence = verifier.verify_step_up(principal=principal, challenge=challenge)  # raises on failure

    now = time.time()
    conn.execute(
        "UPDATE approvals SET status = ?, approved_at = ?, webauthn_reference = ? WHERE approval_id = ?",
        (ApprovalStatus.APPROVED.value, now, evidence.reference, approval_id),
    )
    conn.commit()
    return replace(current, status=ApprovalStatus.APPROVED, approved_at=now, webauthn_reference=evidence.reference)


def cancel(conn: sqlite3.Connection, principal: HumanIdentity, approval_id: str) -> Approval:
    """Principal-initiated withdrawal — pending or approved -> cancelled.
    Never deletes the row; a cancelled approval is retained as history,
    exactly like a revoked delegation."""
    current = get_approval(conn, principal, approval_id)
    validate_approval_transition(current.status, ApprovalStatus.CANCELLED)
    conn.execute("UPDATE approvals SET status = ? WHERE approval_id = ?", (ApprovalStatus.CANCELLED.value, approval_id))
    conn.commit()
    return replace(current, status=ApprovalStatus.CANCELLED)


def verify_and_consume_approval(
    conn: sqlite3.Connection,
    *,
    approval_id: str,
    principal: HumanIdentity,
    agent_id: str,
    instance_id: str,
    task_id: str,
    delegation_id: str,
    tool_name: str,
    resource_type: str | None,
    resource_id: str | None,
    validated_arguments: dict[str, Any],
) -> authorization.AuthorizationDecision:
    """The pre-execution approval pipeline — run immediately before tool
    execution, strictly after `authorization.authorize_tool_call` has
    already returned ALLOW (see agent.py). Every check here is against
    already-verified, database-backed values; the model never supplies
    `approval_id`, `instance_id`, `task_id`, or `delegation_id` — those
    always come from the run's own already-authenticated state, the same
    discipline authorize_tool_call already follows.

    Always returns an AuthorizationDecision (persisted before returning),
    ALLOW or DENY — never raises for an ordinary denial. On ALLOW, the
    approval has already been atomically consumed (approved -> consumed);
    a second call with the same approval_id is guaranteed to see
    APPROVAL_ALREADY_CONSUMED, not a fresh ALLOW — this is what makes an
    approval one-shot.
    """
    resource = f"{resource_type}:{resource_id}" if resource_type else None

    def deny(reason_code: str) -> authorization.AuthorizationDecision:
        return authorization.record_decision(
            conn,
            decision=authorization.DECISION_DENY,
            reason_code=reason_code,
            principal_user_id=principal.user_id,
            agent_id=agent_id,
            instance_id=instance_id,
            task_id=task_id,
            delegation_id=delegation_id,
            tool_name=tool_name,
            resource=resource,
        )

    appr = _load_and_maybe_expire(conn, approval_id)
    if appr is None:
        return deny(APPROVAL_NOT_FOUND)
    if appr.principal_user_id != principal.user_id:
        return deny(APPROVAL_PRINCIPAL_MISMATCH)
    if appr.agent_id != agent_id:
        return deny(APPROVAL_AGENT_MISMATCH)
    if appr.instance_id is not None and appr.instance_id != instance_id:
        return deny(APPROVAL_INSTANCE_MISMATCH)
    if appr.task_id is not None and appr.task_id != task_id:
        return deny(APPROVAL_TASK_MISMATCH)
    if appr.delegation_id != delegation_id:
        return deny(APPROVAL_DELEGATION_MISMATCH)
    if appr.tool_name != tool_name:
        return deny(APPROVAL_TOOL_MISMATCH)
    if appr.resource != resource:
        return deny(APPROVAL_RESOURCE_MISMATCH)

    # Lifecycle: only APPROVED may proceed. Each other terminal/pending
    # state gets its own reason code rather than a generic "not usable".
    if appr.status == ApprovalStatus.PENDING:
        return deny(APPROVAL_PENDING)
    if appr.status == ApprovalStatus.CANCELLED:
        return deny(APPROVAL_CANCELLED)
    if appr.status == ApprovalStatus.CONSUMED:
        return deny(APPROVAL_ALREADY_CONSUMED)
    if appr.status == ApprovalStatus.EXPIRED:
        return deny(APPROVAL_EXPIRED)

    # status == APPROVED. Recompute the digest from the arguments about to
    # actually execute — this is the exact-argument-binding check (TOCTOU
    # / substitution protection): approving one set of arguments and then
    # executing a different one, or a different resource, or a different
    # tool, never matches.
    recomputed = compute_argument_hash(tool_name, resource, validated_arguments)
    if recomputed != appr.argument_hash:
        return deny(APPROVAL_ARGUMENT_MISMATCH)

    # All checks passed. Consume atomically: the UPDATE's WHERE clause
    # re-asserts status='approved', so even if something else consumed
    # this approval between the load above and here, at most one caller
    # ever sees rowcount == 1 — the loser falls back to a normal
    # "already consumed" denial rather than a double ALLOW.
    cursor = conn.execute(
        "UPDATE approvals SET status = ?, consumed_at = ? WHERE approval_id = ? AND status = ?",
        (ApprovalStatus.CONSUMED.value, time.time(), approval_id, ApprovalStatus.APPROVED.value),
    )
    conn.commit()
    if cursor.rowcount != 1:
        return deny(APPROVAL_ALREADY_CONSUMED)

    return authorization.record_decision(
        conn,
        decision=authorization.DECISION_ALLOW,
        reason_code=authorization.ALLOW,
        principal_user_id=principal.user_id,
        agent_id=agent_id,
        instance_id=instance_id,
        task_id=task_id,
        delegation_id=delegation_id,
        tool_name=tool_name,
        resource=resource,
    )
