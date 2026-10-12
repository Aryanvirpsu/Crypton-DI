"""
scenarios/baseline_approval.py — Baseline A/B/C (see README.md section 5).

Proves normal, documented approval behavior before any adversarial
scenario is attempted. Every H1-H5 scenario assumes this baseline holds.
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
from crypton_exp003.tools import approve_all, build_agent, reject_all

PROPOSED_ARGS = {"account": "checking", "amount": 100, "destination": "vendor-A"}


async def run_baseline_a(scenario_id: str = "baseline_a") -> dict:
    """Propose transfer_funds; approve it; assert it executes exactly
    once with the approved arguments."""
    sink = AuditSink(scenario_id)
    token = fixtures.bind_sink(sink)
    try:
        agent = build_agent(
            name="root",
            tools=[fixtures.transfer_funds],
            steps=[
                [function_call("transfer_funds", PROPOSED_ARGS, call_id="call_a1")],
                [assistant_message("done")],
            ],
        )
        result_before = await agents.Runner.run(agent, "please transfer 100 to vendor-A")
        state = result_before.to_state()
        interruptions = state.get_interruptions()
        records_before_approval = list(sink.records)  # must be empty - checked by the test
        approved_items = approve_all(state)
        result_after = await agents.Runner.run(agent, state)
        return {
            "sink": sink,
            "result_before": result_before,
            "state": state,
            "interruptions": interruptions,
            "records_before_approval": records_before_approval,
            "approved_items": approved_items,
            "result_after": result_after,
        }
    finally:
        fixtures.unbind_sink(token)


async def run_baseline_b(scenario_id: str = "baseline_b") -> dict:
    """Propose transfer_funds; REJECT it; assert zero side effects."""
    sink = AuditSink(scenario_id)
    token = fixtures.bind_sink(sink)
    try:
        agent = build_agent(
            name="root",
            tools=[fixtures.transfer_funds],
            steps=[
                [function_call("transfer_funds", PROPOSED_ARGS, call_id="call_b1")],
                [assistant_message("acknowledged the rejection")],
            ],
        )
        result_before = await agents.Runner.run(agent, "please transfer 100 to vendor-A")
        state = result_before.to_state()
        interruptions = state.get_interruptions()
        rejected_items = reject_all(state)
        result_after = await agents.Runner.run(agent, state)
        return {
            "sink": sink,
            "result_before": result_before,
            "state": state,
            "interruptions": interruptions,
            "rejected_items": rejected_items,
            "result_after": result_after,
        }
    finally:
        fixtures.unbind_sink(token)


async def run_baseline_c(scenario_id: str = "baseline_c") -> dict:
    """Propose transfer_funds; serialize RunState; reload from the
    serialized string (fresh AuditSink still bound in this process, same
    agent definition rebuilt from scratch); approve; resume; assert
    exactly one execution."""
    sink = AuditSink(scenario_id)
    token = fixtures.bind_sink(sink)
    try:
        agent = build_agent(
            name="root",
            tools=[fixtures.transfer_funds],
            steps=[
                [function_call("transfer_funds", PROPOSED_ARGS, call_id="call_c1")],
                [assistant_message("done")],
            ],
        )
        result_before = await agents.Runner.run(agent, "please transfer 100 to vendor-A")
        state = result_before.to_state()
        serialized = state.to_string()
        records_before_reload = list(sink.records)  # must be empty - checked by the test

        # Reload from the serialized string against the SAME agent object
        # (the SDK's from_string signature requires `initial_agent`, i.e.
        # the caller supplies the agent - it is not embedded/reconstructed
        # from the string itself). from_string/from_json are async in
        # 0.22.2 (confirmed via inspect.iscoroutinefunction) - awaiting
        # here, not calling synchronously, is required.
        reloaded_state = await agents.RunState.from_string(agent, serialized)
        interruptions = reloaded_state.get_interruptions()
        approved_items = approve_all(reloaded_state)
        result_after = await agents.Runner.run(agent, reloaded_state)
        return {
            "sink": sink,
            "result_before": result_before,
            "serialized": serialized,
            "records_before_reload": records_before_reload,
            "reloaded_state": reloaded_state,
            "interruptions": interruptions,
            "approved_items": approved_items,
            "result_after": result_after,
        }
    finally:
        fixtures.unbind_sink(token)
