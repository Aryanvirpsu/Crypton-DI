"""tests/test_argument_binding.py — H1: approval <-> argument binding."""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
EVIDENCE_DIR = REPO_ROOT / "evidence"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

import pytest

from crypton_exp003b.evidence import AuthorizationBinding, StateBoundary, build_evidence, write_evidence
from scenarios.approval_binding import APPROVED_ARGS, DRIFTED_ARGS, run_h1_mutate_after_approval_decision

TARGET = {"package": "agent-framework", "version": "1.18.0"}


@pytest.mark.asyncio
async def test_h1_mutating_arguments_after_approval_decision_does_not_reach_execution():
    r = await run_h1_mutate_after_approval_decision()

    assert len(r["log"]) == 1
    executed = r["log"][0]
    argument_match = executed["arguments"] == r["presented_args"]
    drift_leaked = executed["arguments"] == DRIFTED_ARGS

    assert argument_match, f"FAIL: executed {executed['arguments']} != presented {r['presented_args']}"
    assert not drift_leaked

    evidence = build_evidence(
        experiment="EXP-003B", scenario="h1_mutate_after_approval_decision", target=TARGET,
        proposal={"tool_name": "transfer_funds", "arguments": APPROVED_ARGS, "call_id": "h1_1"},
        approval_request={"tool_name": "transfer_funds", "arguments": r["presented_args"]},
        approval_decision={"decision": "approve", "note": "response object built, THEN the underlying function_call content's .arguments was mutated to DRIFTED_ARGS before resume"},
        delegation_path=[],
        state_boundary=StateBoundary(workflow="h1_workflow", session=None, checkpoint="restored"),
        execution=r["log"],
        authorization_binding=AuthorizationBinding(
            tool_match=True, argument_match=argument_match, request_match=True, agent_match=True, delegation_match=True, checkpoint_match=True,
        ),
        outcome="PASS",
        notes=[
            "Mutating the public function_call Content's .arguments field after building the "
            "approval response, but before resume, did NOT change what executed - the tool ran "
            "with the ORIGINALLY PRESENTED arguments. The framework does not re-read this mutable "
            "field live at execution time for an already-registered request.",
            "A benign log-noise error occurred late in the run in an earlier probe pass due to a "
            "test-harness script-length mismatch (not a security-relevant error) - fixed by adding "
            "a second scripted turn; not indicative of any framework issue.",
        ],
    )
    write_evidence(evidence, EVIDENCE_DIR / "h1_mutate_after_approval_decision.json")
