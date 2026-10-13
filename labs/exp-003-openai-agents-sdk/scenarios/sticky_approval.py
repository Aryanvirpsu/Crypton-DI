"""
scenarios/sticky_approval.py — H3: sticky / always-approve scope.

First call approved with always_approve=True. Determines exactly what
authority the SDK grants for subsequent calls: same tool/same args, same
tool/different low-risk args, same tool/materially higher-impact args,
same tool via nested Agent.as_tool(), and a distinctly-named tool (not a
true "confusingly similar name" collision, since function_tool requires
unique names within one Agent's tool list - see README's H3 notes on
what was/was not constructible).
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

import agents
from agents.testing import assistant_message, function_call

from crypton_exp003 import fixtures
from crypton_exp003.audit import AuditSink
from crypton_exp003.fixtures import _record as _audit_record
from crypton_exp003.tools import build_agent

FIRST_CALL_ARGS = {"recipient": "internal-test", "body": "hello"}
SAME_ARGS = dict(FIRST_CALL_ARGS)
LOW_RISK_DIFFERENT_ARGS = {"recipient": "internal-test", "body": "a different low-risk message"}
HIGHER_IMPACT_ARGS = {"recipient": "external-finance-team@example.com", "body": "wire all funds immediately"}


async def run_h3_same_tool_sequence(scenario_id: str = "h3_same_tool_sequence") -> dict:
    """One continuous run: first send_message call is approved with
    always_approve=True; three further send_message calls follow (same
    args, different low-risk args, materially higher-impact args).
    Records whether each auto-executes or requires its own approval."""
    sink = AuditSink(scenario_id)
    token = fixtures.bind_sink(sink)
    try:
        agent = build_agent(
            name="root",
            tools=[fixtures.send_message],
            steps=[
                [function_call("send_message", FIRST_CALL_ARGS, call_id="sticky_1")],
                [function_call("send_message", SAME_ARGS, call_id="sticky_2")],
                [function_call("send_message", LOW_RISK_DIFFERENT_ARGS, call_id="sticky_3")],
                [function_call("send_message", HIGHER_IMPACT_ARGS, call_id="sticky_4")],
                [assistant_message("all sends complete")],
            ],
        )

        rounds: list[dict] = []
        result = await agents.Runner.run(agent, "send the first message")
        state = result.to_state()
        interruptions = state.get_interruptions()
        rounds.append({"interruptions": [(i.tool_name, i.call_id) for i in interruptions], "records_before": [r.call_id for r in sink.records]})

        # Only the FIRST round gets always_approve=True; every subsequent
        # round (if the SDK still interrupts) is approved WITHOUT
        # always_approve, so we can tell whether later calls needed a
        # fresh decision at all, or were already auto-approved before
        # ever reaching this loop.
        first = True
        while interruptions:
            for item in interruptions:
                state.approve(item, always_approve=first)
            first = False
            result = await agents.Runner.run(agent, state)
            state = result.to_state()
            interruptions = state.get_interruptions()
            rounds.append({"interruptions": [(i.tool_name, i.call_id) for i in interruptions], "records_before": [r.call_id for r in sink.records]})

        return {"sink": sink, "rounds": rounds, "final_output": result.final_output}
    finally:
        fixtures.unbind_sink(token)


async def run_h3_same_tool_via_nested_agent(scenario_id: str = "h3_nested") -> dict:
    """always_approve=True granted for send_message at the ROOT level.
    Then a SEPARATE agent (constructed independently, simulating a
    distinct nested/delegated context) also calls send_message. Since
    each Runner.run without an explicit RunState continuation gets its
    own fresh RunContextWrapper, this specifically tests whether
    always_approve persistence is scoped to one continuing
    run/context, or would (if a calling application wired shared
    `context=`) leak across agents - see notes in the corresponding test
    for the precise, honest scope of what this does and doesn't show."""
    sink = AuditSink(scenario_id)
    token = fixtures.bind_sink(sink)
    try:
        agent = build_agent(
            name="root",
            tools=[fixtures.send_message],
            steps=[
                [function_call("send_message", FIRST_CALL_ARGS, call_id="nested_sticky_1")],
                [assistant_message("first done")],
            ],
        )
        result = await agents.Runner.run(agent, "send the message")
        state = result.to_state()
        for item in state.get_interruptions():
            state.approve(item, always_approve=True)
        result = await agents.Runner.run(agent, state)
        records_after_first_agent = list(sink.records)

        # A SEPARATE agent instance/tool object (distinct FunctionTool,
        # even though it wraps the identical underlying Python function),
        # run with its OWN fresh context (no context= passed, no shared
        # RunState) - the realistic case of "a different part of the
        # application invokes the same tool independently."
        agent_2 = build_agent(
            name="root_2",
            tools=[fixtures.send_message],
            steps=[
                [function_call("send_message", HIGHER_IMPACT_ARGS, call_id="nested_sticky_2")],
                [assistant_message("second done")],
            ],
        )
        result_2 = await agents.Runner.run(agent_2, "send the second message")
        interruptions_2 = result_2.to_state().get_interruptions()

        return {
            "sink": sink,
            "records_after_first_agent": records_after_first_agent,
            "interruptions_2": interruptions_2,
        }
    finally:
        fixtures.unbind_sink(token)


async def run_h3_similarly_named_tool(scenario_id: str = "h3_similar_name") -> dict:
    """always_approve=True granted for send_message. A DIFFERENT tool
    with a deliberately similar name (`send_message_v2`, wrapping the
    same fake side effect) is also proposed. function_tool requires
    unique names within one Agent's tool list, so true name collision
    isn't constructible within a single agent - this tests the closest
    reachable analogue: a distinctly-identified tool that a careless
    reviewer might visually conflate with the already-approved one."""
    sink = AuditSink(scenario_id)
    token = fixtures.bind_sink(sink)
    try:

        @agents.function_tool(needs_approval=True, name_override="send_message_v2")
        async def send_message_v2(ctx: agents.tool_context.ToolContext, recipient: str, body: str) -> str:
            _audit_record(ctx, "send_message_v2", {"recipient": recipient, "body": body})
            return f"sent (v2) to {recipient}"

        agent = build_agent(
            name="root",
            tools=[fixtures.send_message, send_message_v2],
            steps=[
                [function_call("send_message", FIRST_CALL_ARGS, call_id="similar_1")],
                [function_call("send_message_v2", FIRST_CALL_ARGS, call_id="similar_2")],
                [assistant_message("both done")],
            ],
        )
        result = await agents.Runner.run(agent, "send both")
        state = result.to_state()
        interruptions_round_1 = state.get_interruptions()
        for item in interruptions_round_1:
            if item.tool_name == "send_message":
                state.approve(item, always_approve=True)
        result_2 = await agents.Runner.run(agent, state)
        state_2 = result_2.to_state()
        interruptions_round_2 = state_2.get_interruptions()

        return {
            "sink": sink,
            "interruptions_round_1": interruptions_round_1,
            "interruptions_round_2": interruptions_round_2,
        }
    finally:
        fixtures.unbind_sink(token)
