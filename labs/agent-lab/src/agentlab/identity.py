"""
Identity boundary for Agent Lab.

Reuses Crypton's existing identity foundation instead of inventing a new
one:

  - Human identity            — the same JWT crypton-id issues (HS256,
    secret from JWT_SECRET, claims: sub, username, cred_id, iat, exp).
    See services/crypton-id/src/jwt.rs for the source of truth this
    mirrors. Verification here is local (same trust boundary crypton-id's
    own `AuthUser` extractor uses), not a network call to crypton-id.
  - Agent application identity — which *registered* agent app is running
    (e.g. "support-agent"). Fixed at deploy time, analogous to an OAuth
    client_id — not something that changes per request.
  - Agent instance identity    — one unique ID per agent *run*. New every
    time `AgentRunner.run()` is called, even for the same human and the
    same agent app.

These are kept as three distinct types on purpose — see LEARNING.md
section 7 for why collapsing them into one ID would hide the exact
question agent-security testing needs to ask later: "which human, via
which agent, in which specific run, did this?"
"""

from __future__ import annotations

import os
import time
import uuid
from dataclasses import dataclass

import jwt as pyjwt


class IdentityError(Exception):
    """Raised when a human identity token fails verification."""


@dataclass(frozen=True)
class HumanIdentity:
    """The authenticated human this agent run acts on behalf of.
    Field names mirror crypton-id's JWT claims exactly — this is not a
    parallel identity system, it *is* that token, decoded.
    """

    user_id: str  # JWT `sub`
    username: str
    cred_id: str  # device/credential id that signed in


@dataclass(frozen=True)
class AgentIdentity:
    """The registered agent *application* — the identity of the agent
    software itself, independent of who runs it or how many times.

    v0.3: `agent_id` is now always a canonical ID minted by
    `agents.register_agent` (e.g. `agt_3f9a...`), never a hand-picked
    string like `"support-agent"` — that string now lives in `name`,
    which is descriptive metadata, not identity (see agents.py, authz.py,
    and LEARNING.md's "why names aren't secure identities"). This
    dataclass is still just a lightweight, DB-free carrier — the actual
    source of truth is the persisted `agents.Agent` row; application code
    should only ever construct an `AgentIdentity` from one (see
    `agent.py`'s `run()`), never freehand with an arbitrary agent_id.
    """

    agent_id: str
    version: str = "0.1.0"
    name: str = ""
    owner_user_id: str = ""


@dataclass(frozen=True)
class AgentInstance:
    """One unique run of the agent loop — the unit tracing hangs off.
    Analogous to crypton-gateway's x-request-id, but scoped to a whole
    agent execution (many model calls, many tool calls) rather than a
    single HTTP hop.

    Extended in v0.2 to reference the Task and Session it executes
    within, so one specific run can always be traced back to "which
    task, in which session" without grepping the trace log — see
    tasks.py, sessions.py, and LEARNING.md's Session/Task/AgentInstance
    section. `agent_id`/`human_user_id`/`session_id`/`task_id` default to
    "" only so `.new()` stays a convenient no-arg constructor for tests
    and tools that only care about `instance_id` (see tools.py, which
    only ever reads `instance.instance_id`); a real run (agent.py)
    always constructs AgentInstance with every field populated.
    """

    instance_id: str
    started_at: float
    agent_id: str = ""
    human_user_id: str = ""
    session_id: str = ""
    task_id: str = ""
    ended_at: float | None = None
    status: str = "running"

    @classmethod
    def new(cls) -> "AgentInstance":
        return cls(instance_id=str(uuid.uuid4()), started_at=time.time())


def verify_human_identity(token: str, *, secret: str | None = None) -> HumanIdentity:
    """Verify a crypton-id JWT and extract the human identity.

    Same algorithm, claims, and secret crypton-id itself uses
    (services/crypton-id/src/jwt.rs: HS256, JWT_SECRET, claims
    {sub, username, cred_id, iat, exp}). A tampered or expired token, or
    one signed with a different secret, raises IdentityError — the agent
    never runs without a verified human behind it.
    """
    secret = secret or os.environ.get("JWT_SECRET")
    if not secret:
        raise IdentityError("JWT_SECRET is not configured")

    try:
        claims = pyjwt.decode(token, secret, algorithms=["HS256"])
    except pyjwt.PyJWTError as e:
        raise IdentityError(f"invalid_token: {e}") from e

    for field_name in ("sub", "username", "cred_id"):
        if field_name not in claims:
            raise IdentityError(f"malformed_token: missing {field_name}")

    return HumanIdentity(
        user_id=str(claims["sub"]),
        username=str(claims["username"]),
        cred_id=str(claims["cred_id"]),
    )


def issue_dev_token(
    user_id: str,
    username: str,
    cred_id: str,
    *,
    secret: str | None = None,
    ttl_seconds: int = 3600,
) -> str:
    """Local-dev helper: mints a token in the exact shape crypton-id
    issues, so `verify_human_identity` runs the identical code path
    whether the token came from a real login or this shortcut.

    NOT a second auth system — just a way to exercise the lab without a
    live crypton-id + WebAuthn ceremony. Never used against production.
    """
    secret = secret or os.environ.get("JWT_SECRET")
    if not secret:
        raise IdentityError("JWT_SECRET is not configured")
    now = int(time.time())
    claims = {
        "sub": user_id,
        "username": username,
        "cred_id": cred_id,
        "iat": now,
        "exp": now + ttl_seconds,
    }
    return pyjwt.encode(claims, secret, algorithm="HS256")
