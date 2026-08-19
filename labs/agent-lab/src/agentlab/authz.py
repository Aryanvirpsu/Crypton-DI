"""
Tool allowlist — the authorization boundary between "the model asked for
a tool" and "the tool actually runs" (requirement 4: explicit authority).

Deliberately just a Python dict for v0.1, not a policy engine — do not
generalize this ahead of when it's actually needed (see labs/README.md's
"out of scope" list; a real policy engine is future work, not this lab).
"""

from __future__ import annotations

from dataclasses import dataclass


class AuthorizationError(Exception):
    """Raised when a tool call is not permitted for the calling agent."""


@dataclass(frozen=True)
class AgentPermissions:
    agent_id: str
    allowed_tools: frozenset[str]


# Registered agent applications and exactly which tools each may call.
# This is the "tool permissions in code/config" example from the spec:
#
#   support-agent
#     get_customer:   allowed
#     get_order:      allowed
#     add_case_note:  allowed
#
# `read-only-agent` exists purely to make the "unauthorized tool
# rejected" boundary exercisable in tests with a *registered* agent that
# still isn't allowed to write — as opposed to an agent id that doesn't
# exist at all (also tested, but a different failure mode).
REGISTRY: dict[str, AgentPermissions] = {
    "support-agent": AgentPermissions(
        agent_id="support-agent",
        allowed_tools=frozenset({"get_customer", "get_order", "add_case_note"}),
    ),
    "read-only-agent": AgentPermissions(
        agent_id="read-only-agent",
        allowed_tools=frozenset({"get_customer", "get_order"}),
    ),
}


def check_authorized(agent_id: str, tool_name: str) -> None:
    """Raise AuthorizationError unless `agent_id` is registered AND
    `tool_name` is in its allowlist. Called on every tool call, before
    argument validation or execution — the model's tool name is never
    trusted on its own, and an unregistered agent id is rejected the same
    way an over-reaching registered one is."""
    perms = REGISTRY.get(agent_id)
    if perms is None:
        raise AuthorizationError(f"unregistered_agent: {agent_id}")
    if tool_name not in perms.allowed_tools:
        raise AuthorizationError(f"tool_not_allowed: {agent_id} -> {tool_name}")
