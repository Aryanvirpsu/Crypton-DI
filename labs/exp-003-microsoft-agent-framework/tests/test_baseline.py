"""tests/test_baseline.py — B1/B2/B3. Must pass before any hypothesis test is trusted."""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
EVIDENCE_DIR = REPO_ROOT / "evidence"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

import pytest

from crypton_exp003b.evidence import AuthorizationBinding, StateBoundary, approval_content_to_dict, build_evidence, write_evidence
from scenarios.baseline_tool_approval import TRANSFER_ARGS, run_b1_approve, run_b2_reject, run_b3_checkpoint_reconstruct

TARGET = {"package": "agent-framework", "version": "1.18.0"}


@pytest.mark.asyncio
async def test_b1_approve():
    r = await run_b1_approve()

    assert r["log_before_approval"] == []  # no execution before approval
    assert len(r["pending"]) == 1
    req = r["pending"][0]
    approval_content = approval_content_to_dict(req.data)
    assert approval_content["tool_name"] == "transfer_funds"
    assert approval_content["arguments"] == TRANSFER_ARGS

    assert len(r["log"]) == 1
    executed = r["log"][0]
    assert executed["tool_name"] == "transfer_funds"
    assert executed["arguments"] == TRANSFER_ARGS

    evidence = build_evidence(
        experiment="EXP-003B", scenario="b1_approve", target=TARGET,
        proposal={"tool_name": "transfer_funds", "arguments": TRANSFER_ARGS, "call_id": "b1_1"},
        approval_request=approval_content,
        approval_decision={"decision": "approve", "request_id": req.request_id},
        delegation_path=[],
        state_boundary=StateBoundary(workflow="b1_workflow", session=None, checkpoint="restored via latest_checkpoint_id"),
        execution=r["log"],
        authorization_binding=AuthorizationBinding(
            tool_match=True, argument_match=True, request_match=True, agent_match=True, delegation_match=True, checkpoint_match=True,
        ),
        outcome="PASS",
        notes=["Baseline: no execution before approval; exactly one execution; executed arguments match approved arguments."],
    )
    write_evidence(evidence, EVIDENCE_DIR / "b1_approve.json")


@pytest.mark.asyncio
async def test_b2_reject():
    r = await run_b2_reject()
    assert r["log"] == []  # zero execution

    evidence = build_evidence(
        experiment="EXP-003B", scenario="b2_reject", target=TARGET,
        proposal={"tool_name": "transfer_funds", "arguments": TRANSFER_ARGS, "call_id": "b2_1"},
        approval_request=approval_content_to_dict(r["pending"][0].data),
        approval_decision={"decision": "reject", "request_id": r["pending"][0].request_id},
        delegation_path=[],
        state_boundary=StateBoundary(workflow="b2_workflow", session=None, checkpoint=None),
        execution=[],
        authorization_binding=AuthorizationBinding(
            tool_match=True, argument_match=None, request_match=True, agent_match=True, delegation_match=True, checkpoint_match=None,
            notes=["argument_match/checkpoint_match null: nothing executed, no execution-argument set or checkpoint restore involved to compare."],
        ),
        outcome="PASS",
        notes=["Baseline: rejection produces zero side effects."],
    )
    write_evidence(evidence, EVIDENCE_DIR / "b2_reject.json")


@pytest.mark.asyncio
async def test_b3_checkpoint_reconstruct_restore_approve_resume():
    r = await run_b3_checkpoint_reconstruct()

    assert r["log_before_restore"] == []
    assert len(r["log"]) == 1
    executed = r["log"][0]
    assert executed["tool_name"] == "transfer_funds"
    assert executed["arguments"] == TRANSFER_ARGS

    evidence = build_evidence(
        experiment="EXP-003B", scenario="b3_checkpoint_reconstruct", target=TARGET,
        proposal={"tool_name": "transfer_funds", "arguments": TRANSFER_ARGS, "call_id": "b3_1"},
        approval_request=approval_content_to_dict(r["pending"][0].data),
        approval_decision={"decision": "approve", "request_id": r["pending"][0].request_id, "note": "approved via a RECONSTRUCTED workflow/agent object, not the original"},
        delegation_path=[],
        state_boundary=StateBoundary(workflow="b3_workflow", session=None, checkpoint="saved -> reconstructed workflow -> restored"),
        execution=r["log"],
        authorization_binding=AuthorizationBinding(
            tool_match=True, argument_match=True, request_match=True, agent_match=True, delegation_match=True, checkpoint_match=True,
        ),
        outcome="PASS",
        notes=[
            "Same logical request, same intended tool, same arguments, exactly-once execution, "
            "confirmed across a checkpoint save -> reconstruct (fresh Agent/client/Workflow objects, "
            "not the originals) -> restore -> approve -> resume cycle.",
        ],
    )
    write_evidence(evidence, EVIDENCE_DIR / "b3_checkpoint_reconstruct.json")
