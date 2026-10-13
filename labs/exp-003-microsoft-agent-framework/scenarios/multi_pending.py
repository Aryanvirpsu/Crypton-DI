"""scenarios/multi_pending.py — H4: multiple pending approvals.

X = refund $50, Y = refund $5000, proposed in the same turn. Approve only
X; confirm Y cannot execute. Then checkpoint, restore, reject Y, and look
for any response/request correlation confusion (X's decision being
accepted for Y, or correlation based on ordering rather than request
identity).
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

X_ARGS = {"customer": "alice", "amount": 50}
Y_ARGS = {"customer": "bob", "amount": 5000}


async def run_h4_approve_x_reject_y(scenario_id: str = "h4_multi") -> dict:
    log, token = fixtures.bind_execution_log()
    try:
        client = fixtures.ScriptedChatClient(
            [
                [
                    af.Content.from_function_call(call_id="x_1", name="issue_refund", arguments=X_ARGS),
                    af.Content.from_function_call(call_id="y_1", name="issue_refund", arguments=Y_ARGS),
                ],
                [af.Content.from_text("done")],
            ]
        )
        agent = af.Agent(client, name="root", instructions="test", tools=[fixtures.issue_refund])
        storage = af.InMemoryCheckpointStorage()
        workflow = build_single_agent_workflow(agent=agent, workflow_name="h4_workflow", checkpoint_storage=storage)

        result = await workflow.run("refund both", checkpoint_storage=storage)
        pending = get_pending_approval_requests(result)
        log_after_proposal = list(log)

        x_req = next(r for r in pending if r.data.function_call.call_id == "x_1")
        y_req = next(r for r in pending if r.data.function_call.call_id == "y_1")

        checkpoint_before_any_decision = await latest_checkpoint_id(storage, workflow_name="h4_workflow")

        # Approve ONLY X, reject Y, in the SAME resume call - the realistic
        # "reviewer decided on both at once" case.
        x_response = x_req.data.to_function_approval_response(approved=True)
        y_response = y_req.data.to_function_approval_response(approved=False)
        result2 = await workflow.run(
            checkpoint_id=checkpoint_before_any_decision,
            responses={x_req.request_id: x_response, y_req.request_id: y_response},
            checkpoint_storage=storage,
        )
        log_after_decisions = list(log)

        return {
            "log": log,
            "log_after_proposal": log_after_proposal,
            "log_after_decisions": log_after_decisions,
            "pending": pending,
            "x_req": x_req,
            "y_req": y_req,
        }
    finally:
        fixtures.unbind_execution_log(token)
