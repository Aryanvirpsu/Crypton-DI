"""The policy engines under test.

vulnerable_policy:        "Agent A holds a valid capability" -> allow.
secure_policy:            valid capability AND authenticated controller AND controller
                          is inside the authority chain rooted at the grant's owner.
authenticated_only_policy: DELIBERATELY BROKEN control. Authenticates the controller and
                          checks the capability but ignores authority-chain continuity.
                          The 007-A gate must reject it as a secure policy.

Reason codes keep three failure families separate:
  NO_CAPABILITY            ordinary authorization failure (capability absent/expired/revoked)
  CONTINUITY_UNVERIFIED    controller could not be authenticated / bound to the session
  CONTROLLER_NOT_AUTHORIZED controller authenticated fine, but holds no delegation from
                            the authority root (the pure principal-continuity failure)
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

from .control_channel import ControlMessage
from .identity import KeyDirectory, Session, verify


class Verdict(str, Enum):
    ALLOW = "ALLOW"
    STEP_UP = "STEP_UP"
    DENY = "DENY"


CAPABILITY_HELD = "CAPABILITY_HELD"
CONTINUITY_VERIFIED = "CONTINUITY_VERIFIED"
NO_CAPABILITY = "NO_CAPABILITY"
CONTINUITY_UNVERIFIED = "CONTINUITY_UNVERIFIED"
CONTROLLER_NOT_AUTHORIZED = "CONTROLLER_NOT_AUTHORIZED"
AUTHENTICATION_FAILED = "AUTHENTICATION_FAILED"  # used only by the broken control policy

NOW = 1_700_000_000  # fixed evaluation time: no wall-clock dependence


@dataclass(frozen=True)
class Grant:
    """U -> A capability grant. `authorized_controllers` is the set of principals allowed
    to drive A under this grant: U plus any explicit U -> X delegations."""

    agent_id: str
    owner_pid: str
    capabilities: frozenset
    session_id: str
    authorized_controllers: frozenset = frozenset()
    revoked: bool = False
    expires_at: int | None = None

    def as_dict(self) -> dict:
        return {
            "agent_id": self.agent_id,
            "owner_pid": self.owner_pid,
            "capabilities": sorted(self.capabilities),
            "session_id": self.session_id,
            "authorized_controllers": sorted(self.authorized_controllers),
            "revoked": self.revoked,
            "expires_at": self.expires_at,
        }


@dataclass(frozen=True)
class PolicyContext:
    agent_id: str
    grant: Grant
    session: Session
    keys: KeyDirectory
    now: int = NOW


@dataclass(frozen=True)
class Decision:
    verdict: Verdict
    reason: str
    detail: str
    controller_attribution: str  # what the system will record as the controller
    controller_changed: bool  # what the system itself observed
    agent_authenticated: bool = True
    capability_valid: bool = True
    controller_authenticated: bool | None = None  # None = not evaluated by this policy
    controller_authorized: bool | None = None  # None = not evaluated by this policy

    def render(self) -> str:
        return f"{self.verdict.value}:{self.reason}:{self.detail}"


def agent_authenticated(ctx: PolicyContext) -> bool:
    return ctx.grant.agent_id == ctx.agent_id


def capability_valid(msg: ControlMessage, ctx: PolicyContext) -> bool:
    g = ctx.grant
    return (
        agent_authenticated(ctx)
        and msg.action in g.capabilities
        and not g.revoked
        and (g.expires_at is None or ctx.now < g.expires_at)
    )


def controller_authenticated(msg: ControlMessage, ctx: PolicyContext) -> str | None:
    """None if the controller authenticates; otherwise the failure detail."""
    if not msg.signature:
        return "UNSIGNED"
    key = ctx.keys.key_for(msg.controller_pid)
    if key is None:
        return "UNREGISTERED_CONTROLLER"
    if not verify(key, msg.signed_payload(), msg.signature):
        return "BAD_SIGNATURE"
    return None


def controller_authorized(msg: ControlMessage, ctx: PolicyContext) -> bool:
    """Is the controller inside the authority chain rooted at the grant owner?"""
    return msg.controller_pid in ctx.grant.authorized_controllers


def _session_bound(msg: ControlMessage, ctx: PolicyContext) -> bool:
    s = ctx.session
    return (msg.session_id, msg.task_id, msg.nonce) == (s.session_id, s.task_id, s.nonce)


def _no_capability(msg: ControlMessage, ctx: PolicyContext, **kw) -> Decision:
    return Decision(
        Verdict.DENY, NO_CAPABILITY, "capability absent, expired or revoked",
        msg.controller_pid, False, agent_authenticated=agent_authenticated(ctx),
        capability_valid=False, **kw,
    )


def vulnerable_policy(msg: ControlMessage, ctx: PolicyContext) -> Decision:
    # Trusts whoever reaches the agent; never asks who is controlling it. The controller
    # is recorded as claimed in the header, unverified.
    if capability_valid(msg, ctx):
        return Decision(
            Verdict.ALLOW, CAPABILITY_HELD,
            f"executed under {ctx.grant.owner_pid}'s grant to {ctx.agent_id}; controller not checked",
            msg.controller_pid, False,
        )
    return _no_capability(msg, ctx)


def secure_policy(msg: ControlMessage, ctx: PolicyContext) -> Decision:
    if not capability_valid(msg, ctx):
        return _no_capability(msg, ctx)
    auth_failure = controller_authenticated(msg, ctx)
    if auth_failure is not None:
        return Decision(
            Verdict.STEP_UP, CONTINUITY_UNVERIFIED, auth_failure,
            f"UNVERIFIED(claimed={msg.controller_pid})", True,
            controller_authenticated=False, controller_authorized=False,
        )
    if not controller_authorized(msg, ctx):
        # Authenticated honestly as itself, but no delegation from the authority root.
        return Decision(
            Verdict.STEP_UP, CONTROLLER_NOT_AUTHORIZED,
            f"{msg.controller_pid} authenticated but not in authority chain of {ctx.grant.owner_pid}",
            msg.controller_pid, True,
            controller_authenticated=True, controller_authorized=False,
        )
    if not _session_bound(msg, ctx):
        return Decision(
            Verdict.STEP_UP, CONTINUITY_UNVERIFIED, "SESSION_MISMATCH",
            msg.controller_pid, True,
            controller_authenticated=True, controller_authorized=True,
        )
    return Decision(
        Verdict.ALLOW, CONTINUITY_VERIFIED, "authenticated controller in authority chain, session bound",
        msg.controller_pid, False,
        controller_authenticated=True, controller_authorized=True,
    )


def authenticated_only_policy(msg: ControlMessage, ctx: PolicyContext) -> Decision:
    """Broken control: authentication + capability, no authority-chain continuity."""
    if not capability_valid(msg, ctx):
        return _no_capability(msg, ctx)
    auth_failure = controller_authenticated(msg, ctx)
    if auth_failure is not None:
        return Decision(
            Verdict.DENY, AUTHENTICATION_FAILED, auth_failure, msg.controller_pid, False,
            controller_authenticated=False,
        )
    return Decision(
        Verdict.ALLOW, CAPABILITY_HELD, "authenticated controller; authority chain not checked",
        msg.controller_pid, False, controller_authenticated=True,
    )
