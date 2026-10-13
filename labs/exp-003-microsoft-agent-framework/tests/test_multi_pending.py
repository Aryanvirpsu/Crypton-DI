"""tests/test_multi_pending.py — H4: multiple pending approvals."""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
EVIDENCE_DIR = REPO_ROOT / "evidence"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

import pytest

from crypton_exp003b.evidence import AuthorizationBinding, StateBoundary, build_evidence, write_evidence
from scenarios.multi_pending import X_ARGS, Y_ARGS, run_h4_approve_x_reject_y

TARGET = {"package": "agent-framework", "version": "1.18.0"}


@pytest.mark.asyncio
async def test_h4_approve_x_reject_y_no_cross_correlation():
    r = await run_h4_approve_x_reject_y()

    assert len(r["pending"]) == 2
    assert r["log_after_proposal"] == []  # no execution before either decision

    # Exactly X executed; Y (the rejected, higher-impact one) did not -
    # the decision for X was never mistakenly applied to Y or vice versa,
    # since the API keys decisions by request_id (a dict), not by
    # proposal order.
    assert len(r["log_after_decisions"]) == 1
    assert r["log_after_decisions"][0]["arguments"] == X_ARGS
    assert r["log_after_decisions"][0]["arguments"] != Y_ARGS

    evidence = build_evidence(
        experiment="EXP-003B", scenario="h4_multiple_pending_approve_x_reject_y", target=TARGET,
        proposal={"x": {"tool_name": "issue_refund", "arguments": X_ARGS, "call_id": "x_1"}, "y": {"tool_name": "issue_refund", "arguments": Y_ARGS, "call_id": "y_1"}},
        approval_request={"x_request_id": r["x_req"].request_id, "y_request_id": r["y_req"].request_id},
        approval_decision={"x": "approve", "y": "reject", "submitted_together_in_one_resume_call": True},
        delegation_path=[],
        state_boundary=StateBoundary(workflow="h4_workflow", session=None, checkpoint="single checkpoint held both pending requests"),
        execution=r["log"],
        authorization_binding=AuthorizationBinding(
            tool_match=True, argument_match=True, request_match=True, agent_match=True, delegation_match=True, checkpoint_match=True,
        ),
        outcome="PASS",
        notes=[
            "No correlation confusion observed: X's approval executed X's exact arguments; Y's "
            "rejection produced zero execution for Y. Decisions are submitted as a dict keyed by "
            "request_id (responses={request_id: response}), which is identity-based by API "
            "construction, not order-based - this structurally rules out the 'response for X "
            "accepted for Y based on ordering' failure mode the hypothesis worried about, rather "
            "than merely failing to trigger it by chance.",
        ],
    )
    write_evidence(evidence, EVIDENCE_DIR / "h4_multiple_pending_approve_x_reject_y.json")
