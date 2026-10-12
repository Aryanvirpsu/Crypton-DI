"""tests/test_handoff.py — H2: handoff/delegation authority boundary.

Honest partial result: the delegation-boundary isolation (round 1) is
confirmed. The full cycle (approve delegation -> confirm the nested
sensitive tool ALSO requires its own approval) could not be completed in
this pass due to a streaming-invocation incompatibility between this
lab's ScriptedChatClient test double and Agent.as_tool()'s internal
nested-agent invocation path (Agent.as_tool() invokes the nested agent
with stream=True; the nested call raised
`TypeError("'async for' requires an object with __aiter__ method, got
coroutine")` even after adding streaming support to ScriptedChatClient).
This is recorded as INCONCLUSIVE, not PASS or FAIL - it is a test-harness
gap, not an observed framework behavior, and is the top Phase-2 priority
for this hypothesis. See README's "H2 - what was and wasn't resolved".
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
EVIDENCE_DIR = REPO_ROOT / "evidence"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

import pytest

from crypton_exp003b.evidence import AuthorizationBinding, StateBoundary, build_evidence, write_evidence
from scenarios.handoff_delegation import run_h2_as_tool_delegation_boundary

TARGET = {"package": "agent-framework", "version": "1.18.0"}


@pytest.mark.asyncio
async def test_h2_round1_delegation_boundary_isolated_from_nested_tool():
    """Confirmed: after the delegation call is proposed, only the
    delegation itself is pending - the nested sensitive tool has not been
    reached, and nothing executed."""
    r = await run_h2_as_tool_delegation_boundary()

    assert r["log_after_round1"] == []
    assert len(r["round1_pending"]) == 1
    assert r["round1_pending"][0].data.function_call.name == "refund_agent"

    evidence = build_evidence(
        experiment="EXP-003B", scenario="h2_as_tool_delegation_boundary", target=TARGET,
        proposal={"tool_name": "refund_agent (delegation)", "call_id": "delegate_1"},
        approval_request={"round_1_pending": [(p.data.function_call.name, p.data.function_call.call_id) for p in r["round1_pending"]]},
        approval_decision={"round_1": "not yet decided in this test - isolation confirmed before any decision"},
        delegation_path=[{"agent": "root_agent", "calls": "refund_agent", "call_id": "delegate_1"}],
        state_boundary=StateBoundary(workflow="h2_workflow", session=None, checkpoint="checkpoint_1"),
        execution=r["log"],
        authorization_binding=AuthorizationBinding(
            tool_match=True, argument_match=None, request_match=True, agent_match=None, delegation_match=None, checkpoint_match=True,
            notes=[
                "agent_match/delegation_match null: the full cycle (confirming the nested "
                "issue_refund call also independently requires approval after the delegation is "
                "approved) could not be completed in this pass - see module docstring and "
                "README's H2 notes. Only the pre-decision isolation fact is asserted here.",
            ],
        ),
        outcome="INCONCLUSIVE",
        notes=[
            "PARTIAL RESULT: round 1 correctly shows only the delegation call pending, not the "
            "nested sensitive tool - so at minimum, proposing a delegation does not itself expose "
            "or pre-authorize the nested tool call.",
            "NOT CONFIRMED: whether approving the delegation subsequently requires a SEPARATE "
            "approval for the nested issue_refund call, because completing round 2 hit a "
            "streaming-invocation incompatibility in this lab's ScriptedChatClient test double "
            "when Agent.as_tool() invokes the nested agent (TypeError: \"'async for' requires an "
            "object with __aiter__ method, got coroutine\"). This is a test-harness gap, not an "
            "observed framework authorization behavior - recorded as INCONCLUSIVE, not PASS.",
            "Top Phase-2 priority for EXP-003B: fix ScriptedChatClient's streaming path for "
            "Agent.as_tool()-invoked nested agents (and/or investigate whether "
            "agent_framework.orchestrations.HandoffBuilder cleanly surfaces a downstream agent's "
            "function_approval_request as a workflow-level request_info event - also not resolved "
            "in this pass; see README).",
        ],
    )
    write_evidence(evidence, EVIDENCE_DIR / "h2_as_tool_delegation_boundary.json")
