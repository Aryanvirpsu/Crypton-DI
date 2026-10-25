"""
tests/test_approval_guardrail_lifecycle.py — H5: approval/guardrail TOCTOU.

    approval time:  POLICY["limit"] = 1000  (transfer of 100 is within it)
    before resume:  POLICY["limit"] = 50    (application-level policy change)
    approved action: transfer 100

If the tool input guardrail genuinely re-runs immediately before
invocation (as documented), it should see the CHANGED limit and block the
now-stale authorization even though a human already approved it under the
old policy.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
EVIDENCE_DIR = REPO_ROOT / "evidence"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

import agents
import pytest

from crypton_exp003.evidence import AuthorizationBinding, build_evidence, write_evidence
from scenarios.approval_guardrail_toctou import (
    TRANSFER_ARGS,
    run_h5_policy_changes_across_serialize_reload,
    run_h5_policy_changes_between_approval_and_resume,
    run_h5_policy_unchanged_control,
)

TARGET = {"package": "openai-agents", "version": "0.22.2"}


@pytest.mark.asyncio
async def test_h5_control_unchanged_policy_executes_normally():
    """Control: confirms the guardrail does not spuriously block a still-valid call."""
    r = await run_h5_policy_unchanged_control()
    assert r["result_after"].final_output == "done"
    assert r["sink"].count_for_tool("transfer_funds_guarded") == 1


@pytest.mark.asyncio
async def test_h5_policy_change_between_approval_and_resume_blocks_execution():
    r = await run_h5_policy_changes_between_approval_and_resume()

    assert r["approved_under_limit"] == 1000
    assert r["limit_at_resume"] == 50
    assert isinstance(r["error"], agents.exceptions.ToolInputGuardrailTripwireTriggered)
    assert r["sink"].as_list() == [], "the approved-but-now-policy-stale transfer must not execute"

    evidence = build_evidence(
        experiment="EXP-003A",
        scenario="h5_policy_toctou_in_memory_resume",
        target=TARGET,
        proposal={"tool_name": "transfer_funds_guarded", "arguments": TRANSFER_ARGS, "call_id": "toctou_1"},
        approval_request={"call_id": "toctou_1", "policy_limit_at_approval": 1000},
        approval_decision={"decision": "approve", "call_id": "toctou_1", "note": "approved while limit=1000; limit changed to 50 before resume"},
        delegation_path=[],
        execution=None,
        authorization_binding=AuthorizationBinding(
            tool_match=True, argument_match=None, call_id_match=True, namespace_match=True, delegation_match=True,
            notes=["argument_match is null: nothing executed, so there is no executed-argument set to compare."],
        ),
        outcome="PASS",
        notes=[
            "The documented lifecycle (tool input guardrails run again immediately before "
            "invocation) held: the guardrail re-evaluated the CURRENT policy state at resume "
            "time (limit=50) against the approved call's amount (100), and raised "
            "ToolInputGuardrailTripwireTriggered - blocking execution despite prior human "
            "approval under a since-changed policy. The authorization decision became stale and "
            "the SDK's own guardrail re-check caught it, without needing a fresh human decision.",
        ],
    )
    write_evidence(evidence, EVIDENCE_DIR / "h5_policy_toctou_in_memory_resume.json")


@pytest.mark.asyncio
async def test_h5_policy_change_across_serialize_reload_also_blocks_execution():
    r = await run_h5_policy_changes_across_serialize_reload()
    assert isinstance(r["error"], agents.exceptions.ToolInputGuardrailTripwireTriggered)
    assert r["sink"].as_list() == []

    evidence = build_evidence(
        experiment="EXP-003A",
        scenario="h5_policy_toctou_serialize_reload_resume",
        target=TARGET,
        proposal={"tool_name": "transfer_funds_guarded", "arguments": TRANSFER_ARGS, "call_id": "sr_1"},
        approval_request={"call_id": "sr_1", "policy_limit_at_approval": 1000},
        approval_decision={"decision": "approve", "call_id": "sr_1", "note": "approved, then RunState.to_string()/from_string() round trip, then limit changed to 50 before resume"},
        delegation_path=[],
        execution=None,
        authorization_binding=AuthorizationBinding(
            tool_match=True, argument_match=None, call_id_match=True, namespace_match=True, delegation_match=True,
        ),
        outcome="PASS",
        notes=[
            "Same protection holds across a RunState serialize/reload resume path, not only the "
            "plain in-memory resume path - the guardrail re-check is not specific to one resume "
            "mechanism.",
            "Inverse question (can something change that guardrails would NOT catch, leaving a "
            "stale-but-guardrail-passing authorization) was considered but not independently "
            "demonstrated: this guardrail only inspects `amount`. Any state dimension the "
            "application does not wire into a guardrail (e.g. a changed 'is this destination "
            "still valid' business check) would, trivially, not be caught - this is a "
            "configuration/coverage question for whoever authors guardrails, not a demonstrated "
            "SDK defect. Recorded as an OBSERVATION, not a finding.",
        ],
    )
    write_evidence(evidence, EVIDENCE_DIR / "h5_policy_toctou_serialize_reload_resume.json")
