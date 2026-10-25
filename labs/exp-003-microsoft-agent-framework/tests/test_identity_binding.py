"""tests/test_identity_binding.py — H6: identity/origin binding."""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
EVIDENCE_DIR = REPO_ROOT / "evidence"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

import pytest

from crypton_exp003b.evidence import AuthorizationBinding, StateBoundary, build_evidence, write_evidence
from scenarios.identity_binding import run_h6_cross_agent_response_injection

TARGET = {"package": "agent-framework", "version": "1.18.0"}


@pytest.mark.asyncio
async def test_h6_approval_from_one_agent_workflow_cannot_authorize_another():
    r = await run_h6_cross_agent_response_injection()

    assert r["pending_a_request_id"] != r["pending_b_request_id"]
    assert isinstance(r["error"], ValueError)
    assert "unknown request ID" in str(r["error"])
    assert r["log"] == []  # zero execution in either workflow

    evidence = build_evidence(
        experiment="EXP-003B", scenario="h6_cross_agent_response_injection", target=TARGET,
        proposal={"agent_a_call": {"call_id": "a_1", "request_id": r["pending_a_request_id"]}, "agent_b_call": {"call_id": "b_1", "request_id": r["pending_b_request_id"]}},
        approval_request={"agent_a_request_id": r["pending_a_request_id"], "agent_b_request_id": r["pending_b_request_id"]},
        approval_decision={"attempted": "submit agent_A's approval response into agent_B's workflow, keyed by agent_A's request_id"},
        delegation_path=[],
        state_boundary=StateBoundary(workflow="h6_workflow_a / h6_workflow_b (two independent workflows)", session=None, checkpoint="checkpoint_b"),
        execution=r["log"],
        authorization_binding=AuthorizationBinding(
            tool_match=True, argument_match=None, request_match=False, agent_match=False, delegation_match=None, checkpoint_match=False,
        ),
        outcome="PASS",
        notes=[
            "An approval response's request_id is validated against the RECEIVING workflow's own "
            "registered pending requests. Submitting agent_A's approval into agent_B's workflow "
            "raised an explicit, hard ValueError('Response provided for unknown request ID') "
            "rather than being silently accepted, ignored, or misapplied. Zero execution occurred "
            "in either workflow as a result of the cross-agent injection attempt.",
            "This confirms origin/identity binding is enforced at the request_id level across "
            "independent workflow instances, even when both agents expose the identical tool with "
            "identical arguments (ARGS_A == ARGS_B by construction, to rule out argument-content "
            "as the discriminator).",
        ],
    )
    write_evidence(evidence, EVIDENCE_DIR / "h6_cross_agent_response_injection.json")
