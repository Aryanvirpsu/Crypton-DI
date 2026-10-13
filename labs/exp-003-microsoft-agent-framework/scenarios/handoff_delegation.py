"""scenarios/handoff_delegation.py — H2: handoff/delegation authority boundary.

Primary, well-understood mechanism: Agent.as_tool(approval_mode=...) -
ROOT_AGENT delegates to REFUND_AGENT as a tool; REFUND_AGENT's own
issue_refund tool is separately approval-gated. Tests whether approving
the delegation call confers approval for the nested sensitive tool.

Secondary mechanism (agent_framework.orchestrations.HandoffBuilder, true
multi-agent handoff): attempted but NOT confirmed to cleanly surface a
nested tool's function_approval_request as a workflow-level request_info
event in this pass - see README's "H2 - what was and wasn't resolved".
No unauthorized execution was observed there either (EXECUTED stayed
empty in every attempt), but the approval mechanism's precise behavior in
that path is recorded as an OBSERVATION requiring further investigation,
not a PASS or a FAIL.
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

REFUND_ARGS = {"customer": "alice", "amount": 50}


def _pending(response) -> list:
    return [c for msg in response.messages for c in msg.contents if getattr(c, "type", None) == "function_approval_request"]


async def run_h2_as_tool_delegation_boundary(scenario_id: str = "h2_as_tool") -> dict:
    log, token = fixtures.bind_execution_log()
    try:
        refund_client = fixtures.ScriptedChatClient(
            [
                [af.Content.from_function_call(call_id="refund_1", name="issue_refund", arguments=REFUND_ARGS)],
                [af.Content.from_text("refund processed")],
            ]
        )
        refund_agent = af.Agent(refund_client, name="refund_agent", instructions="refund", tools=[fixtures.issue_refund])
        refund_tool = refund_agent.as_tool(name="refund_agent", description="delegate to refund agent", approval_mode="always_require")

        root_client = fixtures.ScriptedChatClient(
            [
                [af.Content.from_function_call(call_id="delegate_1", name="refund_agent", arguments={"task": "please refund alice 50"})],
                [af.Content.from_text("delegation complete")],
            ]
        )
        root_agent = af.Agent(root_client, name="root_agent", instructions="root", tools=[refund_tool])
        storage = af.InMemoryCheckpointStorage()
        workflow = build_single_agent_workflow(agent=root_agent, workflow_name="h2_workflow", checkpoint_storage=storage)

        result = await workflow.run("please have the refund agent refund alice 50", checkpoint_storage=storage)
        round1_pending = get_pending_approval_requests(result)
        log_after_round1 = list(log)

        # Round 1: only the DELEGATION call should be pending - the nested
        # issue_refund call has not been reached yet.
        assert all(p.data.function_call.name == "refund_agent" for p in round1_pending), round1_pending

        checkpoint_1 = await latest_checkpoint_id(storage, workflow_name="h2_workflow")
        delegate_req = round1_pending[0]
        delegate_response = delegate_req.data.to_function_approval_response(approved=True)
        result2 = await workflow.run(
            checkpoint_id=checkpoint_1, responses={delegate_req.request_id: delegate_response}, checkpoint_storage=storage
        )
        round2_pending = get_pending_approval_requests(result2)
        log_after_round2 = list(log)

        return {
            "log": log,
            "log_after_round1": log_after_round1,
            "round1_pending": round1_pending,
            "log_after_round2": log_after_round2,
            "round2_pending": round2_pending,
        }
    finally:
        fixtures.unbind_execution_log(token)
