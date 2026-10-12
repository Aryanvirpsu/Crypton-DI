"""
scenarios/resume_state.py — H4: RunState resume/replay binding.

Question: can approval become detached from the original call/arguments/
tool identity/namespace/delegation path? Verified via three realistic
lifecycle transitions (see README's H4 notes for why these three, and
which sub-cases from the task's list were NOT independently exercised):

  A. reconstructed equivalent agent - RunState.from_string() given a
     freshly-built Agent object (same name/tools, different Python
     object and different ScriptedModel) instead of the literal original.
  B. multiple outstanding approvals, resolved + unresolved - approve only
     one of two pending items and resume anyway.
  C. repeated attempt to resume an already-completed RunState.
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
from crypton_exp003.tools import build_agent

TRANSFER_ARGS = {"account": "checking", "amount": 100, "destination": "vendor-A"}
DELETE_ARGS = {"record_id": "rec-9"}


async def run_h4_reconstructed_agent(scenario_id: str = "h4_reconstructed_agent") -> dict:
    sink = AuditSink(scenario_id)
    token = fixtures.bind_sink(sink)
    try:
        agent1 = build_agent(
            name="root",
            tools=[fixtures.transfer_funds],
            steps=[[function_call("transfer_funds", TRANSFER_ARGS, call_id="recon_1")], [assistant_message("original script - should not be reached")]],
        )
        result = await agents.Runner.run(agent1, "go")
        state = result.to_state()
        serialized = state.to_string()

        # A DIFFERENT Agent object: same name/tools, its OWN distinct
        # ScriptedModel - simulates a process restart that rebuilds the
        # agent from source rather than keeping the original in memory.
        agent2 = build_agent(
            name="root",
            tools=[fixtures.transfer_funds],
            steps=[[assistant_message("reconstructed agent's own script")]],
        )
        reloaded = await agents.RunState.from_string(agent2, serialized)
        interruptions = reloaded.get_interruptions()
        for item in interruptions:
            reloaded.approve(item)
        result2 = await agents.Runner.run(agent2, reloaded)

        return {"sink": sink, "interruptions": interruptions, "result2": result2}
    finally:
        fixtures.unbind_sink(token)


async def run_h4_multiple_outstanding_partial_resolution(scenario_id: str = "h4_multi_partial") -> dict:
    sink = AuditSink(scenario_id)
    token = fixtures.bind_sink(sink)
    try:
        agent = build_agent(
            name="root",
            tools=[fixtures.transfer_funds, fixtures.delete_record],
            steps=[
                [
                    function_call("transfer_funds", TRANSFER_ARGS, call_id="multi_1"),
                    function_call("delete_record", DELETE_ARGS, call_id="multi_2"),
                ],
                [assistant_message("both requested")],
            ],
        )
        result = await agents.Runner.run(agent, "do both")
        state = result.to_state()
        interruptions_round_1 = state.get_interruptions()

        for item in interruptions_round_1:
            if item.tool_name == "transfer_funds":
                state.approve(item)
        # Deliberately resume with delete_record still unresolved - the
        # realistic "caller forgot to check every approval was handled" case.
        result2 = await agents.Runner.run(agent, state)
        records_after_partial_resume = list(sink.records)
        remaining_interruptions = result2.to_state().get_interruptions()

        return {
            "sink": sink,
            "interruptions_round_1": interruptions_round_1,
            "records_after_partial_resume": records_after_partial_resume,
            "remaining_interruptions": remaining_interruptions,
        }
    finally:
        fixtures.unbind_sink(token)


async def run_h4_repeated_resume_of_completed_state(scenario_id: str = "h4_repeat_resume") -> dict:
    sink = AuditSink(scenario_id)
    token = fixtures.bind_sink(sink)
    try:
        agent = build_agent(
            name="root",
            tools=[fixtures.transfer_funds],
            steps=[
                [function_call("transfer_funds", TRANSFER_ARGS, call_id="repeat_1")],
                [assistant_message("done")],
                [assistant_message("still done - extra scripted step so a second resume has somewhere to go")],
            ],
        )
        result = await agents.Runner.run(agent, "go")
        state = result.to_state()
        for item in state.get_interruptions():
            state.approve(item)
        result2 = await agents.Runner.run(agent, state)
        records_after_first_resume = list(sink.records)

        completed_state = result2.to_state()
        completed_state_interruptions = completed_state.get_interruptions()
        result3 = await agents.Runner.run(agent, completed_state)
        records_after_second_resume = list(sink.records)

        return {
            "sink": sink,
            "records_after_first_resume": records_after_first_resume,
            "completed_state_interruptions": completed_state_interruptions,
            "result3": result3,
            "records_after_second_resume": records_after_second_resume,
        }
    finally:
        fixtures.unbind_sink(token)
