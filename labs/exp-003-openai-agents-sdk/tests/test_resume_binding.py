"""
tests/test_resume_binding.py — H4: RunState resume/replay binding.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
EVIDENCE_DIR = REPO_ROOT / "evidence"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

import pytest

from crypton_exp003.evidence import AuthorizationBinding, build_evidence, write_evidence
from scenarios.resume_state import (
    TRANSFER_ARGS,
    run_h4_multiple_outstanding_partial_resolution,
    run_h4_reconstructed_agent,
    run_h4_repeated_resume_of_completed_state,
)

TARGET = {"package": "openai-agents", "version": "0.22.2"}


@pytest.mark.asyncio
async def test_h4_reconstructed_agent_preserves_call_and_arguments():
    r = await run_h4_reconstructed_agent()
    sink = r["sink"]

    assert [i.call_id for i in r["interruptions"]] == ["recon_1"]
    assert sink.count_for_tool("transfer_funds") == 1
    executed = sink.records[0]
    assert executed.arguments == TRANSFER_ARGS
    assert executed.call_id == "recon_1"
    # The reconstructed agent's OWN script is what's used for the
    # continuation - not a hidden reference back to the original object.
    assert r["result2"].final_output == "reconstructed agent's own script"

    evidence = build_evidence(
        experiment="EXP-003A", scenario="h4_reconstructed_agent", target=TARGET,
        proposal={"tool_name": "transfer_funds", "arguments": TRANSFER_ARGS, "call_id": "recon_1"},
        approval_request={"call_id": "recon_1"},
        approval_decision={"decision": "approve", "call_id": "recon_1", "via": "RunState.from_string(agent2, ...) where agent2 is a DIFFERENT object than the one that produced the serialized state"},
        delegation_path=[],
        execution=[vars(rec) for rec in sink.records],
        authorization_binding=AuthorizationBinding(tool_match=True, argument_match=True, call_id_match=True, namespace_match=True, delegation_match=True),
        outcome="PASS",
        notes=["A freshly-constructed, equivalent Agent object (same name/tools, different Python object and different ScriptedModel) can reload and resume the state correctly, executing with the ORIGINAL call's arguments unchanged."],
    )
    write_evidence(evidence, EVIDENCE_DIR / "h4_reconstructed_agent.json")


@pytest.mark.asyncio
async def test_h4_resuming_with_one_unresolved_approval_does_not_execute_it():
    r = await run_h4_multiple_outstanding_partial_resolution()

    assert {i.call_id for i in r["interruptions_round_1"]} == {"multi_1", "multi_2"}
    # Only the approved item executed.
    executed_call_ids = [rec.call_id for rec in r["records_after_partial_resume"]]
    assert executed_call_ids == ["multi_1"]
    # The unresolved item is neither silently executed nor lost - it
    # re-surfaces as a still-pending interruption.
    assert [i.call_id for i in r["remaining_interruptions"]] == ["multi_2"]

    evidence = build_evidence(
        experiment="EXP-003A", scenario="h4_multiple_outstanding_partial_resolution", target=TARGET,
        proposal={"calls": [{"tool_name": "transfer_funds", "call_id": "multi_1"}, {"tool_name": "delete_record", "call_id": "multi_2"}]},
        approval_request={"pending": ["multi_1", "multi_2"]},
        approval_decision={"multi_1": "approved", "multi_2": "left unresolved, resumed anyway"},
        delegation_path=[],
        execution=[vars(rec) for rec in r["records_after_partial_resume"]],
        authorization_binding=AuthorizationBinding(tool_match=True, argument_match=True, call_id_match=True, namespace_match=True, delegation_match=True),
        outcome="PASS",
        notes=[
            "Resuming a RunState with one of two pending approvals still unresolved does NOT raise "
            "an error and does NOT execute the unresolved call. It executes only the resolved call "
            "and correctly re-presents the unresolved one as a still-pending interruption on the "
            "new result's state - no detachment, no silent authorization, no lost approval request.",
        ],
    )
    write_evidence(evidence, EVIDENCE_DIR / "h4_multiple_outstanding_partial_resolution.json")


@pytest.mark.asyncio
async def test_h4_repeated_resume_of_completed_state_does_not_reexecute():
    r = await run_h4_repeated_resume_of_completed_state()

    assert [rec.call_id for rec in r["records_after_first_resume"]] == ["repeat_1"]
    assert r["completed_state_interruptions"] == []
    # Calling Runner.run again on an already-completed state must not
    # re-execute the original tool call ("APPROVED CALL X turning into
    # EXECUTED CALL Y without another authorization decision" - here,
    # verifying X does not turn into a SECOND execution of X either).
    assert r["sink"].count_for_tool("transfer_funds") == 1
    assert [rec.call_id for rec in r["records_after_second_resume"]] == ["repeat_1"]
    assert r["result3"].final_output == "still done - extra scripted step so a second resume has somewhere to go"

    evidence = build_evidence(
        experiment="EXP-003A", scenario="h4_repeated_resume_of_completed_state", target=TARGET,
        proposal={"tool_name": "transfer_funds", "arguments": TRANSFER_ARGS, "call_id": "repeat_1"},
        approval_request={"call_id": "repeat_1"},
        approval_decision={"decision": "approve", "call_id": "repeat_1"},
        delegation_path=[],
        execution=[vars(rec) for rec in r["records_after_second_resume"]],
        authorization_binding=AuthorizationBinding(tool_match=True, argument_match=True, call_id_match=True, namespace_match=True, delegation_match=True),
        outcome="PASS",
        notes=[
            "Calling Runner.run again on a RunState with zero pending interruptions does not "
            "re-execute the already-completed tool call - exactly-once holds across a repeated, "
            "mistaken resume attempt. Note: this DID advance the conversation to a new model turn "
            "(consuming a further scripted step) rather than raising a 'nothing to resume' error - "
            "worth knowing operationally, but not itself an authorization gap.",
        ],
    )
    write_evidence(evidence, EVIDENCE_DIR / "h4_repeated_resume_of_completed_state.json")
