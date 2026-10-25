"""scenarios/approval_binding.py — H1: approval <-> argument binding.

Realistic, public-API-only mutation: after the approval decision is
generated (to_function_approval_response), mutate the underlying
function_call Content's public `.arguments`/`.parse_arguments` state
before resuming, and see whether the drifted arguments reach execution.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

import json

import agent_framework as af

from crypton_exp003b import fixtures
from crypton_exp003b.authority import build_single_agent_workflow, get_pending_approval_requests, latest_checkpoint_id

APPROVED_ARGS = {"account": "checking", "amount": 100, "destination": "vendor-A"}
DRIFTED_ARGS = {"account": "checking", "amount": 10000, "destination": "vendor-B"}


async def run_h1_mutate_after_approval_decision(scenario_id: str = "h1_mutate_after_decision") -> dict:
    log, token = fixtures.bind_execution_log()
    try:
        client = fixtures.ScriptedChatClient(
            [
                [af.Content.from_function_call(call_id="h1_1", name="transfer_funds", arguments=APPROVED_ARGS)],
                [af.Content.from_text("done")],
            ]
        )
        agent = af.Agent(client, name="root", instructions="test", tools=[fixtures.transfer_funds])
        storage = af.InMemoryCheckpointStorage()
        workflow = build_single_agent_workflow(agent=agent, workflow_name="h1_workflow", checkpoint_storage=storage)

        result = await workflow.run("transfer 100", checkpoint_storage=storage)
        pending = get_pending_approval_requests(result)
        req = pending[0]
        presented_args = json.loads(req.data.function_call.arguments) if isinstance(req.data.function_call.arguments, str) else req.data.function_call.arguments

        response_content = req.data.to_function_approval_response(approved=True)
        # Mutate the public function_call content's arguments AFTER the
        # approval response object was built, BEFORE resume.
        req.data.function_call.arguments = json.dumps(DRIFTED_ARGS)

        checkpoint_id = await latest_checkpoint_id(storage, workflow_name="h1_workflow")
        error: Exception | None = None
        try:
            await workflow.run(checkpoint_id=checkpoint_id, responses={req.request_id: response_content}, checkpoint_storage=storage)
        except Exception as exc:  # noqa: BLE001
            error = exc

        return {"log": log, "presented_args": presented_args, "error": error}
    finally:
        fixtures.unbind_execution_log(token)
