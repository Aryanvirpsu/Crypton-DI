"""
crypton_exp003/fixtures.py — safe side-effect sink tools.

Four fake tools representing meaningful actions (`transfer_funds`,
`delete_record`, `send_message`, `delegate_action`). None of them touch
any real external system: each appends one AuditRecord to whatever
AuditSink is currently bound (see `bind_sink`/`get_sink`), then returns a
harmless string. That AuditRecord IS the evidence for "what actually
executed" (see audit.py and README.md's four-stage comparison).

All four are declared with `needs_approval=True` by default so every
scenario starts from a real, SDK-enforced approval gate — individual
scenarios that need a non-approval-gated variant build their own
`function_tool(...)` wrapping the same underlying logic rather than
mutating these.
"""

from __future__ import annotations

import contextvars
from typing import Any

import agents
from agents.tool_context import ToolContext

from crypton_exp003.audit import AuditSink

_current_sink: contextvars.ContextVar[AuditSink | None] = contextvars.ContextVar(
    "crypton_exp003_sink", default=None
)


def bind_sink(sink: AuditSink) -> contextvars.Token:
    return _current_sink.set(sink)


def unbind_sink(token: contextvars.Token) -> None:
    _current_sink.reset(token)


def get_sink() -> AuditSink:
    sink = _current_sink.get()
    if sink is None:
        raise RuntimeError("no AuditSink bound - call fixtures.bind_sink(sink) before running a scenario")
    return sink


def _approval_state(ctx: ToolContext) -> str:
    """Best-effort, honestly-labeled approval state at execution time.
    `is_tool_approved` returns True/False/None per the SDK's own
    contract; None is recorded as "unknown" rather than guessed at."""
    try:
        status = ctx.is_tool_approved(ctx.tool_name, ctx.tool_call_id)
    except Exception as exc:  # pragma: no cover - defensive, not expected in practice
        return f"unobservable ({exc.__class__.__name__})"
    if status is True:
        return "approved"
    if status is False:
        return "not_approved"
    return "unknown"


def _record(ctx: ToolContext, tool_name: str, arguments: dict[str, Any]) -> None:
    sink = get_sink()
    agent_name = ctx.agent.name if getattr(ctx, "agent", None) is not None else None
    sink.record(
        tool_name=tool_name,
        arguments=arguments,
        call_id=ctx.tool_call_id,
        agent_name=agent_name,
        tool_namespace=ctx.tool_namespace,
        approval_state=_approval_state(ctx),
    )


@agents.function_tool(needs_approval=True)
async def transfer_funds(ctx: ToolContext, account: str, amount: int, destination: str) -> str:
    """Fake funds transfer. No real money or account is touched — this
    only appends an AuditRecord to the currently bound sink."""
    _record(ctx, "transfer_funds", {"account": account, "amount": amount, "destination": destination})
    return f"transferred {amount} from {account} to {destination}"


@agents.function_tool(needs_approval=True)
async def delete_record(ctx: ToolContext, record_id: str) -> str:
    """Fake record deletion. No real record exists or is touched."""
    _record(ctx, "delete_record", {"record_id": record_id})
    return f"deleted record {record_id}"


@agents.function_tool(needs_approval=True)
async def send_message(ctx: ToolContext, recipient: str, body: str) -> str:
    """Fake message send. No real message is delivered anywhere."""
    _record(ctx, "send_message", {"recipient": recipient, "body": body})
    return f"sent message to {recipient}"


@agents.function_tool(needs_approval=True)
async def delegate_action(ctx: ToolContext, target_agent: str, scope: str) -> str:
    """Fake delegation marker tool — records an intent to delegate
    without actually invoking anything; used where a scenario wants a
    tool-shaped delegation signal distinct from Agent.as_tool()."""
    _record(ctx, "delegate_action", {"target_agent": target_agent, "scope": scope})
    return f"delegated {scope} to {target_agent}"


ALL_TOOLS = (transfer_funds, delete_record, send_message, delegate_action)
