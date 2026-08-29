"""
Task lifecycle — validated status values and the only place status
transitions are decided ("use validated lifecycle values ... do not rely
on arbitrary status strings").

Intentionally tiny: an enum and a transition table, not a state-machine
framework. See LEARNING.md for why max-step termination maps to `failed`
rather than inventing a sixth status.
"""

from __future__ import annotations

from enum import Enum


class TaskStatus(str, Enum):
    CREATED = "created"
    RUNNING = "running"
    COMPLETED = "completed"
    FAILED = "failed"
    CANCELLED = "cancelled"


# No outgoing transitions from a terminal status — this is what makes
# "completed/failed/cancelled never silently resumes" true by
# construction (see _VALID_TRANSITIONS below), not a special case bolted
# on top.
TERMINAL_STATUSES = frozenset({TaskStatus.COMPLETED, TaskStatus.FAILED, TaskStatus.CANCELLED})

_VALID_TRANSITIONS: dict[TaskStatus, frozenset[TaskStatus]] = {
    TaskStatus.CREATED: frozenset({TaskStatus.RUNNING, TaskStatus.CANCELLED}),
    TaskStatus.RUNNING: frozenset({TaskStatus.COMPLETED, TaskStatus.FAILED, TaskStatus.CANCELLED}),
    TaskStatus.COMPLETED: frozenset(),
    TaskStatus.FAILED: frozenset(),
    TaskStatus.CANCELLED: frozenset(),
}


class LifecycleError(Exception):
    """Raised for an unknown status value, or a transition not present in
    _VALID_TRANSITIONS."""


def parse_status(value: str) -> TaskStatus:
    """Parse a raw string into a TaskStatus, rejecting anything outside
    the fixed vocabulary. Never trust an arbitrary status string —
    including one read back out of the database, which is why every
    Task load goes through this, not a bare cast."""
    try:
        return TaskStatus(value)
    except ValueError:
        raise LifecycleError(f"unknown_status: {value}") from None


def validate_transition(current: TaskStatus, new: TaskStatus) -> None:
    """Raise LifecycleError unless `current -> new` is an allowed
    transition. This is the single choke point every status change in
    tasks.py goes through — see tasks.update_task_status."""
    if new not in _VALID_TRANSITIONS.get(current, frozenset()):
        raise LifecycleError(f"invalid_transition: {current.value} -> {new.value}")


# ── Agent status (v0.3) ──────────────────────────────────────────────────
# A separate, smaller lifecycle from TaskStatus — an Agent registration is
# either usable or not, and (unlike a Task) can cleanly move back and
# forth, so both directions are valid transitions here.


class AgentStatus(str, Enum):
    ACTIVE = "active"
    DISABLED = "disabled"


_AGENT_VALID_TRANSITIONS: dict[AgentStatus, frozenset[AgentStatus]] = {
    AgentStatus.ACTIVE: frozenset({AgentStatus.DISABLED}),
    AgentStatus.DISABLED: frozenset({AgentStatus.ACTIVE}),
}


def parse_agent_status(value: str) -> AgentStatus:
    try:
        return AgentStatus(value)
    except ValueError:
        raise LifecycleError(f"unknown_status: {value}") from None


def validate_agent_transition(current: AgentStatus, new: AgentStatus) -> None:
    if new not in _AGENT_VALID_TRANSITIONS.get(current, frozenset()):
        raise LifecycleError(f"invalid_transition: {current.value} -> {new.value}")


# ── Delegation status (v0.4) ─────────────────────────────────────────────
# Delegation is one-way: once revoked, a delegation is never reactivated —
# granting fresh authority means issuing a new delegation, not resurrecting
# an old one. This is what "revocation changes state; do not delete
# record" means in practice: the row stays forever, but it can only ever
# move active -> revoked, never back.


class DelegationStatus(str, Enum):
    ACTIVE = "active"
    REVOKED = "revoked"


_DELEGATION_VALID_TRANSITIONS: dict[DelegationStatus, frozenset[DelegationStatus]] = {
    DelegationStatus.ACTIVE: frozenset({DelegationStatus.REVOKED}),
    DelegationStatus.REVOKED: frozenset(),
}


def parse_delegation_status(value: str) -> DelegationStatus:
    try:
        return DelegationStatus(value)
    except ValueError:
        raise LifecycleError(f"unknown_status: {value}") from None


def validate_delegation_transition(current: DelegationStatus, new: DelegationStatus) -> None:
    if new not in _DELEGATION_VALID_TRANSITIONS.get(current, frozenset()):
        raise LifecycleError(f"invalid_transition: {current.value} -> {new.value}")


# ── Approval status (v0.5) ───────────────────────────────────────────────
# pending: requested, not yet human-confirmed.
# approved: human completed WebAuthn step-up; not yet used.
# consumed: successfully backed one (and only one) tool execution — a
#   terminal state reached only through the pre-execution pipeline's
#   atomic consume, never a direct status write.
# cancelled: withdrawn before use — by the principal, or because the
#   underlying request was abandoned.
# expired: TTL passed before the approval was ever consumed. Set lazily
#   (see approval.py's `_load_and_maybe_expire`) — checked live against
#   `expires_at` every time an approval is read, the same discipline
#   DelegationStatus/expiry uses, never a background sweep.


class ApprovalStatus(str, Enum):
    PENDING = "pending"
    APPROVED = "approved"
    CONSUMED = "consumed"
    CANCELLED = "cancelled"
    EXPIRED = "expired"


_APPROVAL_VALID_TRANSITIONS: dict[ApprovalStatus, frozenset[ApprovalStatus]] = {
    ApprovalStatus.PENDING: frozenset({ApprovalStatus.APPROVED, ApprovalStatus.CANCELLED, ApprovalStatus.EXPIRED}),
    ApprovalStatus.APPROVED: frozenset({ApprovalStatus.CONSUMED, ApprovalStatus.CANCELLED, ApprovalStatus.EXPIRED}),
    ApprovalStatus.CONSUMED: frozenset(),
    ApprovalStatus.CANCELLED: frozenset(),
    ApprovalStatus.EXPIRED: frozenset(),
}


def parse_approval_status(value: str) -> ApprovalStatus:
    try:
        return ApprovalStatus(value)
    except ValueError:
        raise LifecycleError(f"unknown_status: {value}") from None


def validate_approval_transition(current: ApprovalStatus, new: ApprovalStatus) -> None:
    if new not in _APPROVAL_VALID_TRANSITIONS.get(current, frozenset()):
        raise LifecycleError(f"invalid_transition: {current.value} -> {new.value}")
