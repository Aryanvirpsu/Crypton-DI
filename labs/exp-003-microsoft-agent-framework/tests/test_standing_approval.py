"""tests/test_standing_approval.py — H5: standing/always-approve scope."""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
EVIDENCE_DIR = REPO_ROOT / "evidence"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

import pytest

from crypton_exp003b.evidence import AuthorizationBinding, StateBoundary, build_evidence, write_evidence
from scenarios.standing_approval import (
    FIRST_ARGS,
    HIGH_IMPACT_ARGS,
    LOW_RISK_DIFFERENT_ARGS,
    run_h5_broad_always_approve_sequence,
    run_h5_narrow_always_approve_with_arguments,
    run_h5_separate_session_control,
)

TARGET = {"package": "agent-framework", "version": "1.18.0"}


@pytest.mark.asyncio
async def test_h5_broad_always_approve_covers_all_subsequent_calls_including_high_impact():
    r = await run_h5_broad_always_approve_sequence()

    assert r["log_before"] == []
    assert len(r["pending0"]) == 1
    assert r["pending1"] == []  # no further interruption for calls s2/s3/s4
    assert [rec["arguments"] for rec in r["log"]] == [FIRST_ARGS, FIRST_ARGS, LOW_RISK_DIFFERENT_ARGS, HIGH_IMPACT_ARGS]

    evidence = build_evidence(
        experiment="EXP-003B", scenario="h5_broad_always_approve_sequence", target=TARGET,
        proposal={"initial_call": {"tool_name": "transfer_funds", "arguments": FIRST_ARGS, "call_id": "s1"}},
        approval_request={"call_id": "s1"},
        approval_decision={"decision": "create_always_approve_tool_response (broad, tool-scope)"},
        delegation_path=[],
        state_boundary=StateBoundary(workflow=None, session="shared AgentSession across all 4 calls", checkpoint=None),
        execution=r["log"],
        authorization_binding=AuthorizationBinding(
            tool_match=True, argument_match=None, request_match=True, agent_match=True, delegation_match=True, checkpoint_match=None,
            notes=["argument_match null by design: this scenario specifically tests whether DIFFERENT arguments are still authorized."],
        ),
        outcome="PASS",
        notes=[
            "AUTHORITY_EXPANSION: initial approved authority = one transfer_funds call with "
            f"{FIRST_ARGS!r}. Subsequent automatically authorized authority = ALL further "
            f"transfer_funds calls in this session, including a materially higher-impact one "
            f"({HIGH_IMPACT_ARGS!r}), with zero additional human decisions.",
            "Classification: EXPECTED_BY_DESIGN. Confirmed via source "
            "(agent_framework/_harness/_tool_approval.py): ToolApprovalRule(tool_name, "
            "arguments=None) is documented to apply to the tool itself, matching ANY arguments - "
            "this is not a hidden mechanism, it is the framework's own explicit 'tool' scope tier, "
            "distinct from its 'tool_with_arguments' tier (see the narrow-scope test).",
        ],
    )
    write_evidence(evidence, EVIDENCE_DIR / "h5_broad_always_approve_sequence.json")


@pytest.mark.asyncio
async def test_h5_narrow_always_approve_does_not_cover_different_arguments():
    r = await run_h5_narrow_always_approve_with_arguments()

    assert [rec["arguments"] for rec in r["log"]] == [FIRST_ARGS]  # only the matching-args call executed
    assert len(r["pending1"]) == 1  # the different-args call is still pending, not auto-approved

    evidence = build_evidence(
        experiment="EXP-003B", scenario="h5_narrow_always_approve_arguments_scoped", target=TARGET,
        proposal={"initial_call": {"tool_name": "transfer_funds", "arguments": FIRST_ARGS, "call_id": "n1"}},
        approval_request={"call_id": "n1"},
        approval_decision={"decision": "create_always_approve_tool_with_arguments_response (narrow, tool+arguments scope)"},
        delegation_path=[],
        state_boundary=StateBoundary(workflow=None, session="shared AgentSession across both calls", checkpoint=None),
        execution=r["log"],
        authorization_binding=AuthorizationBinding(
            tool_match=True, argument_match=True, request_match=True, agent_match=True, delegation_match=True, checkpoint_match=None,
        ),
        outcome="PASS",
        notes=[
            "The complementary, narrower standing-approval helper behaves exactly as documented: "
            "a materially different (higher-impact) call to the SAME tool, in the SAME session, "
            "under a tool_with_arguments-scoped grant, remained pending and did NOT auto-execute. "
            "This is the clean control confirming the framework's two-tier scope design is real "
            "and both tiers work as named.",
        ],
    )
    write_evidence(evidence, EVIDENCE_DIR / "h5_narrow_always_approve_arguments_scoped.json")


@pytest.mark.asyncio
async def test_h5_separate_session_does_not_inherit_standing_approval():
    r = await run_h5_separate_session_control()
    assert len(r["pending"]) == 1  # fresh session, no shared ToolApprovalState -> still requires approval
    assert r["log"] == []
