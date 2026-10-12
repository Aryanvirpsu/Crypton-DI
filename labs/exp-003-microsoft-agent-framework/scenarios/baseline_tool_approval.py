"""
scenarios/baseline_tool_approval.py — B1/B2/B3 (see README section "Baseline").

B1 - approved tool. B2 - rejected tool. B3 - checkpointed pending
approval: checkpoint, reconstruct (fresh Agent/client/workflow objects),
restore, approve, resume.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

import agent_framework as af

from crypton_exp003b import fixtures
from crypton_exp003b.authority import approve_request, build_single_agent_workflow, get_pending_approval_requests, latest_checkpoint_id

TRANSFER_ARGS = {"account": "checking", "amount": 100, "destination": "vendor-A"}


def _build_agent(*, name: str, responses: list[list]) -> af.Agent:
    client = fixtures.ScriptedChatClient(responses)
    return af.Agent(client, name=name, instructions="test agent", tools=[fixtures.transfer_funds])


async def run_b1_approve(scenario_id: str = "b1_approve") -> dict:
    log, token = fixtures.bind_execution_log()
    try:
        agent = _build_agent(
            name="root",
            responses=[
                [af.Content.from_function_call(call_id="b1_1", name="transfer_funds", arguments=TRANSFER_ARGS)],
                [af.Content.from_text("done")],
            ],
        )
        storage = af.InMemoryCheckpointStorage()
        workflow = build_single_agent_workflow(agent=agent, workflow_name="b1_workflow", checkpoint_storage=storage)

        result = await workflow.run("transfer 100", checkpoint_storage=storage)
        pending = get_pending_approval_requests(result)
        log_before_approval = list(log)

        assert len(pending) == 1
        checkpoint_id = await latest_checkpoint_id(storage, workflow_name="b1_workflow")
        result2 = await approve_request(
            workflow, request_event=pending[0], approved=True, checkpoint_id=checkpoint_id, checkpoint_storage=storage
        )
        return {"log": log, "log_before_approval": log_before_approval, "pending": pending, "result2": result2}
    finally:
        fixtures.unbind_execution_log(token)


async def run_b2_reject(scenario_id: str = "b2_reject") -> dict:
    log, token = fixtures.bind_execution_log()
    try:
        agent = _build_agent(
            name="root",
            responses=[
                [af.Content.from_function_call(call_id="b2_1", name="transfer_funds", arguments=TRANSFER_ARGS)],
                [af.Content.from_text("acknowledged rejection")],
            ],
        )
        storage = af.InMemoryCheckpointStorage()
        workflow = build_single_agent_workflow(agent=agent, workflow_name="b2_workflow", checkpoint_storage=storage)

        result = await workflow.run("transfer 100", checkpoint_storage=storage)
        pending = get_pending_approval_requests(result)
        assert len(pending) == 1
        checkpoint_id = await latest_checkpoint_id(storage, workflow_name="b2_workflow")
        result2 = await approve_request(
            workflow, request_event=pending[0], approved=False, checkpoint_id=checkpoint_id, checkpoint_storage=storage
        )
        return {"log": log, "pending": pending, "result2": result2}
    finally:
        fixtures.unbind_execution_log(token)


async def run_b3_checkpoint_reconstruct(scenario_id: str = "b3_checkpoint") -> dict:
    log, token = fixtures.bind_execution_log()
    try:
        agent1 = _build_agent(
            name="root",
            responses=[[af.Content.from_function_call(call_id="b3_1", name="transfer_funds", arguments=TRANSFER_ARGS)]],
        )
        storage = af.InMemoryCheckpointStorage()
        workflow1 = build_single_agent_workflow(agent=agent1, workflow_name="b3_workflow", checkpoint_storage=storage)

        result = await workflow1.run("transfer 100", checkpoint_storage=storage)
        pending = get_pending_approval_requests(result)
        assert len(pending) == 1
        log_before_restore = list(log)
        checkpoint_id = await latest_checkpoint_id(storage, workflow_name="b3_workflow")

        # Reconstruct: brand-new Agent/client/Workflow objects, same tool,
        # same workflow_name/checkpoint storage - simulating a process
        # restart that rebuilds everything from source before restoring.
        agent2 = _build_agent(name="root", responses=[[af.Content.from_text("reconstructed - should not be needed")]])
        workflow2 = build_single_agent_workflow(agent=agent2, workflow_name="b3_workflow", checkpoint_storage=storage)

        result2 = await approve_request(
            workflow2, request_event=pending[0], approved=True, checkpoint_id=checkpoint_id, checkpoint_storage=storage
        )
        return {"log": log, "log_before_restore": log_before_restore, "pending": pending, "result2": result2}
    finally:
        fixtures.unbind_execution_log(token)
