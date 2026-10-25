"""
crypton_exp003b/fixtures.py — deterministic model + safe side-effect tools.

Schema-generalization note (see README): openai-agents' ToolContext gave
each tool invocation a rich per-call object (call_id, agent, namespace)
INSIDE the function body. agent_framework.FunctionInvocationContext (the
nearest equivalent found) only exposes `add_tools`/`remove_tools` -
no per-call identity fields. So unlike EXP-003A's fixtures, these fake
tools cannot self-report agent/call identity from inside the function;
they only record the raw fact "this tool executed with these arguments"
to a plain, bound list. Scenario code (which already holds the agent,
workflow, checkpoint, and approval-request identity from constructing the
run) is responsible for building the full AuditRecord after the fact by
correlating this raw execution fact with that known metadata. This is an
honest structural difference from EXP-003A, not a workaround.

ScriptedChatClient is a deterministic MODEL substitute (composed with the
framework's own FunctionInvocationLayer mixin, required for automatic
function-invocation/approval handling - see target.json's notes for why).
No framework authorization/workflow behavior is reimplemented here.
"""

from __future__ import annotations

import contextvars
from typing import Any

import agent_framework as af
from agent_framework._tools import FunctionInvocationLayer

_current_sink: contextvars.ContextVar[list[dict[str, Any]] | None] = contextvars.ContextVar(
    "crypton_exp003b_sink", default=None
)


def bind_execution_log() -> tuple[list[dict[str, Any]], contextvars.Token]:
    log: list[dict[str, Any]] = []
    token = _current_sink.set(log)
    return log, token


def unbind_execution_log(token: contextvars.Token) -> None:
    _current_sink.reset(token)


def _record_execution(tool_name: str, arguments: dict[str, Any]) -> None:
    log = _current_sink.get()
    if log is None:
        raise RuntimeError("no execution log bound - call fixtures.bind_execution_log() before running a scenario")
    log.append({"tool_name": tool_name, "arguments": dict(arguments)})


class ScriptedChatClient(FunctionInvocationLayer, af.BaseChatClient):
    """Deterministic chat client: pops one pre-scripted list of Content
    items per call to `_inner_get_response`. Raises (not hangs, not
    silently defaults) when the script runs out, so a scenario/test bug
    (wrong number of scripted turns) fails loudly rather than looking like
    a framework behavior.

    Root-cause note (harness fix, not a framework fix): `_inner_get_response`
    must be a PLAIN function, not `async def`. `BaseChatClient.get_response()`
    (agent_framework/_clients.py, the "no compaction overrides" fast path)
    does `return self._inner_get_response(...)` WITHOUT awaiting it. For
    stream=False that's fine (the un-awaited coroutine IS the promised
    `Awaitable[ChatResponse]`). For stream=True, `get_response()`'s own
    contract promises a `ResponseStream` (an async-iterable) SYNCHRONOUSLY,
    not a coroutine that resolves to one. If `_inner_get_response` were
    declared `async def`, calling it unawaited always yields a bare
    coroutine object regardless of what it internally returns - which is
    exactly what produced `TypeError("'async for' requires an object with
    __aiter__ method, got coroutine")` when Agent.as_tool() invoked a nested
    agent with stream=True in the earlier EXP-003B H2 pass. Declaring this
    method WITHOUT `async def` and returning an async-generator object
    directly (via a nested `async def` generator, called but not awaited)
    for the streaming branch matches the framework's actual calling
    convention. The non-streaming branch still returns a genuine coroutine
    via a small `async def` helper, preserving the `Awaitable[ChatResponse]`
    contract for stream=False. This is a fix to this lab's own test double,
    within documented public extension points (BaseChatClient) - no
    framework code, private object, or private state was touched."""

    def __init__(self, responses: list[list[Any]]):
        super().__init__()
        object.__setattr__(self, "_responses", list(responses))
        object.__setattr__(self, "call_count", 0)

    def _pop_response(self) -> list[Any]:
        self.call_count += 1
        if not self._responses:
            raise RuntimeError(f"ScriptedChatClient exhausted: call #{self.call_count} has no scripted response")
        return self._responses.pop(0)

    def _inner_get_response(self, *, messages, stream, options, **kwargs):
        if stream:

            async def _updates():
                contents = self._pop_response()
                yield af.ChatResponseUpdate(contents=contents, role="assistant")

            # Two different framework call sites disagree on what a
            # streaming _inner_get_response() call should hand back
            # without being awaited first:
            #   - BaseChatClient.get_response()'s "fast path" (_clients.py)
            #     does `return self._inner_get_response(...)` unawaited and
            #     needs the result to already be async-iterable.
            #   - Its "slow path" (with compaction overrides) explicitly
            #     accepts EITHER a ResponseStream directly OR something
            #     awaitable that resolves to one.
            # ResponseStream itself is async-iterable (implements
            # __aiter__), so wrapping the generator in one satisfies the
            # fast path's "must not need awaiting" requirement while also
            # satisfying the slow path's explicit `isinstance(..., ResponseStream)`
            # check - one return value, both call sites. A finalizer is
            # required (matches how the framework's own FunctionInvocationLayer
            # constructs its streams, _tools.py) - without one, aggregating
            # updates into a final ChatResponse (e.g. via get_final_response())
            # fails, since there is no sensible default aggregation.
            return af.ResponseStream(_updates(), finalizer=af.ChatResponse.from_updates)

        async def _response() -> af.ChatResponse:
            contents = self._pop_response()
            return af.ChatResponse(messages=af.Message(role="assistant", contents=contents))

        return _response()  # a coroutine - matches Awaitable[ChatResponse]


@af.tool(approval_mode="always_require")
def transfer_funds(account: str, amount: int, destination: str) -> str:
    _record_execution("transfer_funds", {"account": account, "amount": amount, "destination": destination})
    return f"transferred {amount} from {account} to {destination}"


@af.tool(approval_mode="always_require")
def issue_refund(customer: str, amount: int) -> str:
    _record_execution("issue_refund", {"customer": customer, "amount": amount})
    return f"refunded {amount} to {customer}"


@af.tool(approval_mode="always_require")
def delete_record(record_id: str) -> str:
    _record_execution("delete_record", {"record_id": record_id})
    return f"deleted record {record_id}"


@af.tool(approval_mode="always_require")
def deploy_service(environment: str, version: str) -> str:
    _record_execution("deploy_service", {"environment": environment, "version": version})
    return f"deployed {version} to {environment}"


ALL_TOOLS = (transfer_funds, issue_refund, delete_record, deploy_service)
