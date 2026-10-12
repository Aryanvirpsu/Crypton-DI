"""scenarios/identity_binding.py — H6: identity/origin binding.

Two independent agents (agent_A, agent_B), each capable of invoking the
same sensitive tool, in two independent workflows. Question: can an
approval request/response originating in agent_A's workflow ever
authorize execution in agent_B's workflow?
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

import agent_framework as af

from crypton_exp003b import fixtures
from crypton_exp003b.authority import build_single_agent_workflow, get_pending_approval_requests, latest_checkpoint_id

ARGS_A = {"account": "checking", "amount": 100, "destination": "A"}
ARGS_B = {"account": "checking", "amount": 100, "destination": "A"}  # identical args, different agent/workflow


async def run_h6_cross_agent_response_injection(scenario_id: str = "h6_cross_agent") -> dict:
    log, token = fixtures.bind_execution_log()
    try:
        client_a = fixtures.ScriptedChatClient([[af.Content.from_function_call(call_id="a_1", name="transfer_funds", arguments=ARGS_A)]])
        agent_a = af.Agent(client_a, name="agent_A", instructions="A", tools=[fixtures.transfer_funds])
        storage_a = af.InMemoryCheckpointStorage()
        workflow_a = build_single_agent_workflow(agent=agent_a, workflow_name="h6_workflow_a", checkpoint_storage=storage_a)
        result_a = await workflow_a.run("transfer", checkpoint_storage=storage_a)
        pending_a = get_pending_approval_requests(result_a)
        checkpoint_a = await latest_checkpoint_id(storage_a, workflow_name="h6_workflow_a")

        client_b = fixtures.ScriptedChatClient([[af.Content.from_function_call(call_id="b_1", name="transfer_funds", arguments=ARGS_B)]])
        agent_b = af.Agent(client_b, name="agent_B", instructions="B", tools=[fixtures.transfer_funds])
        storage_b = af.InMemoryCheckpointStorage()
        workflow_b = build_single_agent_workflow(agent=agent_b, workflow_name="h6_workflow_b", checkpoint_storage=storage_b)
        result_b = await workflow_b.run("transfer", checkpoint_storage=storage_b)
        pending_b = get_pending_approval_requests(result_b)
        checkpoint_b = await latest_checkpoint_id(storage_b, workflow_name="h6_workflow_b")

        # Take agent_A's approval response and try to inject it into
        # agent_B's workflow, keyed by agent_A's request_id.
        response_from_a = pending_a[0].data.to_function_approval_response(approved=True)
        error: Exception | None = None
        try:
            cross_result = await workflow_b.run(
                checkpoint_id=checkpoint_b, responses={pending_a[0].request_id: response_from_a}, checkpoint_storage=storage_b
            )
            cross_pending = get_pending_approval_requests(cross_result)
        except Exception as exc:  # noqa: BLE001
            error = exc
            cross_pending = None

        return {
            "log": log,
            "checkpoint_a": checkpoint_a,
            "checkpoint_b": checkpoint_b,
            "error": error,
            "cross_pending": cross_pending,
            "pending_b_request_id": pending_b[0].request_id,
            "pending_a_request_id": pending_a[0].request_id,
        }
    finally:
        fixtures.unbind_execution_log(token)
