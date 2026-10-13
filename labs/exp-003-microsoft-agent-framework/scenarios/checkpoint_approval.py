"""scenarios/checkpoint_approval.py — H3 (deeper) + replay/occurrence safety.

checkpoint A contains approval request X. Restore A independently TWICE
(two separate reconstructed agent/workflow object graphs, same
checkpoint_id, same storage). Approve X in EACH restored branch. Does
each branch execute X independently (expected branching semantics), or
does approving one corrupt/duplicate into the other?

Also: resume an already-completed workflow a second time (occurrence
safety - a stale/duplicate approval response delivered after the
original cycle already completed).
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

ARGS = {"account": "checking", "amount": 100, "destination": "A"}


async def run_h3_restore_same_checkpoint_twice(scenario_id: str = "h3_double_restore") -> dict:
    log, token = fixtures.bind_execution_log()
    try:
        client = fixtures.ScriptedChatClient([[af.Content.from_function_call(call_id="branch_1", name="transfer_funds", arguments=ARGS)]])
        agent = af.Agent(client, name="root", instructions="test", tools=[fixtures.transfer_funds])
        storage = af.InMemoryCheckpointStorage()
        workflow = build_single_agent_workflow(agent=agent, workflow_name="h3_workflow", checkpoint_storage=storage)
        result = await workflow.run("transfer", checkpoint_storage=storage)
        pending = get_pending_approval_requests(result)
        checkpoint_id = await latest_checkpoint_id(storage, workflow_name="h3_workflow")

        # Branch 1: independent reconstructed agent/workflow, restore, approve.
        client_1 = fixtures.ScriptedChatClient([[af.Content.from_text("done")], [af.Content.from_text("done2")]])
        agent_1 = af.Agent(client_1, name="root", instructions="test", tools=[fixtures.transfer_funds])
        workflow_1 = build_single_agent_workflow(agent=agent_1, workflow_name="h3_workflow", checkpoint_storage=storage)
        response_1 = pending[0].data.to_function_approval_response(approved=True)
        result_1 = await workflow_1.run(checkpoint_id=checkpoint_id, responses={pending[0].request_id: response_1}, checkpoint_storage=storage)
        log_after_branch_1 = list(log)

        # Branch 2: a SECOND, independent reconstruction restoring the SAME checkpoint_id again.
        client_2 = fixtures.ScriptedChatClient([[af.Content.from_text("done")], [af.Content.from_text("done2")]])
        agent_2 = af.Agent(client_2, name="root", instructions="test", tools=[fixtures.transfer_funds])
        workflow_2 = build_single_agent_workflow(agent=agent_2, workflow_name="h3_workflow", checkpoint_storage=storage)
        response_2 = pending[0].data.to_function_approval_response(approved=True)
        result_2 = await workflow_2.run(checkpoint_id=checkpoint_id, responses={pending[0].request_id: response_2}, checkpoint_storage=storage)
        log_after_branch_2 = list(log)

        return {"log": log, "log_after_branch_1": log_after_branch_1, "log_after_branch_2": log_after_branch_2}
    finally:
        fixtures.unbind_execution_log(token)


async def run_replay_stale_response_after_completion(scenario_id: str = "replay_stale") -> dict:
    log, token = fixtures.bind_execution_log()
    try:
        client = fixtures.ScriptedChatClient(
            [
                [af.Content.from_function_call(call_id="stale_1", name="transfer_funds", arguments=ARGS)],
                [af.Content.from_text("done")],
                [af.Content.from_text("done2")],
            ]
        )
        agent = af.Agent(client, name="root", instructions="test", tools=[fixtures.transfer_funds])
        storage = af.InMemoryCheckpointStorage()
        workflow = build_single_agent_workflow(agent=agent, workflow_name="replay_workflow", checkpoint_storage=storage)
        result = await workflow.run("transfer", checkpoint_storage=storage)
        pending = get_pending_approval_requests(result)
        checkpoint_id = await latest_checkpoint_id(storage, workflow_name="replay_workflow")

        response = pending[0].data.to_function_approval_response(approved=True)
        await workflow.run(checkpoint_id=checkpoint_id, responses={pending[0].request_id: response}, checkpoint_storage=storage)
        log_after_first_completion = list(log)

        # Deliver the SAME response a second time, against the (now stale) checkpoint_id.
        error: Exception | None = None
        try:
            await workflow.run(checkpoint_id=checkpoint_id, responses={pending[0].request_id: response}, checkpoint_storage=storage)
        except Exception as exc:  # noqa: BLE001
            error = exc

        return {"log": log, "log_after_first_completion": log_after_first_completion, "error": error}
    finally:
        fixtures.unbind_execution_log(token)
