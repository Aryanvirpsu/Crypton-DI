"""
crypton_exp003/tools.py — small SDK-usage helpers shared by scenarios.

Deliberately thin: agent construction and approval-resolution boilerplate
that every scenario would otherwise repeat. Not a framework — each
scenario still drives its own Runner.run/RunState calls explicitly, since
the whole point of this experiment is to observe those calls directly,
not hide them behind an abstraction.
"""

from __future__ import annotations

from typing import Any

import agents
from agents.testing import ScriptedModel

_TRACING_DISABLED = False


def ensure_tracing_disabled() -> None:
    """Idempotent. Every scenario/test calls this before constructing any
    agent - Phase 1 must not require network access or an API key."""
    global _TRACING_DISABLED
    if not _TRACING_DISABLED:
        agents.set_tracing_disabled(True)
        _TRACING_DISABLED = True


def build_agent(
    *,
    name: str,
    tools: list[Any],
    steps: list[Any],
    instructions: str = "You are a test agent used only for deterministic authorization testing.",
    handoffs: list[Any] | None = None,
) -> agents.Agent:
    ensure_tracing_disabled()
    return agents.Agent(
        name=name,
        instructions=instructions,
        tools=list(tools),
        handoffs=list(handoffs or []),
        model=ScriptedModel(steps=steps),
    )


def approve_all(state: agents.RunState, *, always_approve: bool = False) -> list[agents.ToolApprovalItem]:
    """Approve every currently pending interruption. Returns the items
    approved, in the order RunState reported them - so a scenario can
    assert on exactly which calls were pending, not just that some were."""
    items = list(state.get_interruptions())
    for item in items:
        state.approve(item, always_approve=always_approve)
    return items


def reject_all(
    state: agents.RunState, *, always_reject: bool = False, rejection_message: str | None = None
) -> list[agents.ToolApprovalItem]:
    items = list(state.get_interruptions())
    for item in items:
        state.reject(item, always_reject=always_reject, rejection_message=rejection_message)
    return items
