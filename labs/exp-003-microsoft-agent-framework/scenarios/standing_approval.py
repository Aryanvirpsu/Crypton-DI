"""scenarios/standing_approval.py — H5: standing/always approval scope.

Microsoft Agent Framework has an explicit, first-class TWO-TIER design
(confirmed via source, agent_framework/_harness/_tool_approval.py):
ToolApprovalRule(tool_name, arguments=None) with `arguments=None` scopes
to the TOOL alone (any arguments); with `arguments={...}` scopes to
"tool_with_arguments" (exact argument match only). Two helpers build the
approval-response content directly: create_always_approve_tool_response
(broad) and create_always_approve_tool_with_arguments_response (narrow).
Scope is session-backed (ToolApprovalState lives on the AgentSession).
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

import agent_framework as af

from crypton_exp003b import fixtures

FIRST_ARGS = {"account": "checking", "amount": 100, "destination": "A"}
SAME_ARGS = dict(FIRST_ARGS)
LOW_RISK_DIFFERENT_ARGS = {"account": "checking", "amount": 50, "destination": "A"}
HIGH_IMPACT_ARGS = {"account": "checking", "amount": 999999, "destination": "ZZZ"}


def _pending(response: af.AgentResponse) -> list:
    return [c for msg in response.messages for c in msg.contents if getattr(c, "type", None) == "function_approval_request"]


async def run_h5_broad_always_approve_sequence(scenario_id: str = "h5_broad") -> dict:
    log, token = fixtures.bind_execution_log()
    try:
        client = fixtures.ScriptedChatClient(
            [
                [af.Content.from_function_call(call_id="s1", name="transfer_funds", arguments=FIRST_ARGS)],
                [af.Content.from_function_call(call_id="s2", name="transfer_funds", arguments=SAME_ARGS)],
                [af.Content.from_function_call(call_id="s3", name="transfer_funds", arguments=LOW_RISK_DIFFERENT_ARGS)],
                [af.Content.from_function_call(call_id="s4", name="transfer_funds", arguments=HIGH_IMPACT_ARGS)],
                [af.Content.from_text("all done")],
            ]
        )
        agent = af.Agent(client, name="root", instructions="test", tools=[fixtures.transfer_funds], middleware=[af.ToolApprovalMiddleware()])
        session = agent.create_session()

        response0 = await agent.run("transfer", session=session)
        pending0 = _pending(response0)
        log_before = list(log)
        always_response = af.create_always_approve_tool_response(pending0[0])
        response1 = await agent.run(af.Message(role="user", contents=[always_response]), session=session)
        pending1 = _pending(response1)

        return {"log": log, "log_before": log_before, "pending0": pending0, "pending1": pending1, "response1": response1}
    finally:
        fixtures.unbind_execution_log(token)


async def run_h5_narrow_always_approve_with_arguments(scenario_id: str = "h5_narrow") -> dict:
    log, token = fixtures.bind_execution_log()
    try:
        client = fixtures.ScriptedChatClient(
            [
                [af.Content.from_function_call(call_id="n1", name="transfer_funds", arguments=FIRST_ARGS)],
                [af.Content.from_function_call(call_id="n2", name="transfer_funds", arguments=HIGH_IMPACT_ARGS)],
            ]
        )
        agent = af.Agent(client, name="root", instructions="test", tools=[fixtures.transfer_funds], middleware=[af.ToolApprovalMiddleware()])
        session = agent.create_session()

        response0 = await agent.run("transfer", session=session)
        pending0 = _pending(response0)
        narrow_response = af.create_always_approve_tool_with_arguments_response(pending0[0])
        response1 = await agent.run(af.Message(role="user", contents=[narrow_response]), session=session)
        pending1 = _pending(response1)

        return {"log": log, "pending0": pending0, "pending1": pending1}
    finally:
        fixtures.unbind_execution_log(token)


async def run_h5_separate_session_control(scenario_id: str = "h5_separate_session") -> dict:
    """Control: a genuinely separate session (no shared ToolApprovalState)
    for the same tool must still require its own approval."""
    log, token = fixtures.bind_execution_log()
    try:
        client = fixtures.ScriptedChatClient([[af.Content.from_function_call(call_id="sep_1", name="transfer_funds", arguments=FIRST_ARGS)]])
        agent = af.Agent(client, name="root", instructions="test", tools=[fixtures.transfer_funds], middleware=[af.ToolApprovalMiddleware()])
        fresh_session = agent.create_session()
        response = await agent.run("transfer", session=fresh_session)
        return {"log": log, "pending": _pending(response)}
    finally:
        fixtures.unbind_execution_log(token)
