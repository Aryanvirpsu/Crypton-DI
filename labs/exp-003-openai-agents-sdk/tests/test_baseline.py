"""
tests/test_baseline.py — Baseline A/B/C (README.md section 5).

Must pass before any H1-H5 test is trusted (EXP-003A Phase 1 gate).
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
EVIDENCE_DIR = REPO_ROOT / "evidence"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

import pytest

from crypton_exp003.evidence import AuthorizationBinding, approval_item_to_dict, build_evidence, write_evidence
from scenarios.baseline_approval import PROPOSED_ARGS, run_baseline_a, run_baseline_b, run_baseline_c

TARGET = {"package": "openai-agents", "version": "0.22.2"}


@pytest.mark.asyncio
async def test_baseline_a_approve():
    r = await run_baseline_a()
    sink = r["sink"]

    # 1. no execution occurs before approval
    assert r["records_before_approval"] == []
    # 2. interruption contains the expected action
    assert len(r["interruptions"]) == 1
    interrupted = r["interruptions"][0]
    assert interrupted.tool_name == "transfer_funds"
    # 3. approval resumes the run (no exception, run completed)
    assert r["result_after"].final_output == "done"
    # 4. exactly one action executes
    assert sink.count_for_tool("transfer_funds") == 1
    # 5. executed parameters equal approved parameters
    executed = sink.records[0]
    assert executed.arguments == PROPOSED_ARGS
    # 6. call identity is preserved where the SDK exposes it
    assert executed.call_id == "call_a1"
    assert interrupted.call_id == "call_a1"

    evidence = build_evidence(
        experiment="EXP-003A",
        scenario="baseline_a_approve",
        target=TARGET,
        proposal={"tool_name": "transfer_funds", "arguments": PROPOSED_ARGS, "call_id": "call_a1"},
        approval_request=approval_item_to_dict(interrupted),
        approval_decision={"decision": "approve", "always_approve": False, "call_id": interrupted.call_id},
        delegation_path=[],
        execution=[vars(rec) for rec in sink.records],
        authorization_binding=AuthorizationBinding(
            tool_match=True, argument_match=True, call_id_match=True, namespace_match=True, delegation_match=True
        ),
        outcome="PASS",
        notes=["Baseline: normal documented approve-then-execute flow."],
    )
    write_evidence(evidence, EVIDENCE_DIR / "baseline_a_approve.json")


@pytest.mark.asyncio
async def test_baseline_b_reject():
    r = await run_baseline_b()
    sink = r["sink"]

    assert len(r["interruptions"]) == 1
    assert sink.as_list() == [], "no side effect must occur before or after rejection"
    assert sink.count_for_tool("transfer_funds") == 0

    evidence = build_evidence(
        experiment="EXP-003A",
        scenario="baseline_b_reject",
        target=TARGET,
        proposal={"tool_name": "transfer_funds", "arguments": PROPOSED_ARGS, "call_id": "call_b1"},
        approval_request=approval_item_to_dict(r["interruptions"][0]),
        approval_decision={"decision": "reject", "always_reject": False, "call_id": "call_b1"},
        delegation_path=[],
        execution=[],
        authorization_binding=AuthorizationBinding(
            tool_match=True, argument_match=None, call_id_match=True, namespace_match=True, delegation_match=True,
            notes=["argument_match is null: nothing executed, so there is no executed-argument set to compare against."],
        ),
        outcome="PASS",
        notes=["Baseline: rejection produces zero side effects."],
    )
    write_evidence(evidence, EVIDENCE_DIR / "baseline_b_reject.json")


@pytest.mark.asyncio
async def test_baseline_c_serialize_resume():
    r = await run_baseline_c()
    sink = r["sink"]

    assert len(r["interruptions"]) == 1
    assert r["records_before_reload"] == []
    assert isinstance(r["serialized"], str) and len(r["serialized"]) > 0
    assert r["result_after"].final_output == "done"
    assert sink.count_for_tool("transfer_funds") == 1
    executed = sink.records[0]
    assert executed.arguments == PROPOSED_ARGS
    assert executed.call_id == "call_c1"

    evidence = build_evidence(
        experiment="EXP-003A",
        scenario="baseline_c_serialize_resume",
        target=TARGET,
        proposal={"tool_name": "transfer_funds", "arguments": PROPOSED_ARGS, "call_id": "call_c1"},
        approval_request=approval_item_to_dict(r["interruptions"][0]),
        approval_decision={"decision": "approve", "always_approve": False, "call_id": "call_c1", "via": "RunState.from_string reload"},
        delegation_path=[],
        execution=[vars(rec) for rec in sink.records],
        authorization_binding=AuthorizationBinding(
            tool_match=True, argument_match=True, call_id_match=True, namespace_match=True, delegation_match=True
        ),
        outcome="PASS",
        notes=["Baseline: RunState.to_string() -> RunState.from_string() round trip preserves the pending approval and executes exactly once."],
    )
    write_evidence(evidence, EVIDENCE_DIR / "baseline_c_serialize_resume.json")
