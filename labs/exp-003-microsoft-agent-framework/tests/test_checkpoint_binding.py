"""tests/test_checkpoint_binding.py — H3 (deeper) + replay/occurrence safety.

Contains the most significant result of EXP-003B Phase 1: replaying a
stale approval response against an already-completed checkpoint, on the
SAME live Workflow object, produced a SECOND, duplicate execution with no
error and no warning specific to the replay. See the CANDIDATE_FINDING
notes on the second test below for the full, hedged characterization.
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
from scenarios.checkpoint_approval import ARGS, run_h3_restore_same_checkpoint_twice, run_replay_stale_response_after_completion

TARGET = {"package": "agent-framework", "version": "1.18.0"}


@pytest.mark.asyncio
async def test_h3_restoring_same_checkpoint_twice_is_independent_branching_not_a_bypass():
    r = await run_h3_restore_same_checkpoint_twice()

    assert len(r["log_after_branch_1"]) == 1
    assert len(r["log"]) == 2  # both independently-restored branches executed

    evidence = build_evidence(
        experiment="EXP-003B", scenario="h3_restore_same_checkpoint_twice", target=TARGET,
        proposal={"tool_name": "transfer_funds", "arguments": ARGS, "call_id": "branch_1"},
        approval_request={"note": "one pending request, restored into two independent Agent/Workflow object graphs from the same checkpoint_id"},
        approval_decision={"branch_1": "approve (own to_function_approval_response call)", "branch_2": "approve (own, separate to_function_approval_response call)"},
        delegation_path=[],
        state_boundary=StateBoundary(workflow="h3_workflow", session=None, checkpoint="one checkpoint_id, restored twice independently"),
        execution=r["log"],
        authorization_binding=AuthorizationBinding(
            tool_match=True, argument_match=True, request_match=True, agent_match=True, delegation_match=True, checkpoint_match=True,
        ),
        outcome="PASS",
        notes=[
            "NOT a vulnerability, per the task's explicit instruction not to label normal "
            "checkpoint branching as one: each of the two independently-reconstructed branches "
            "received its OWN separate to_function_approval_response() call in this test - i.e. "
            "two genuine approval submissions were made, one per branch, and each branch executed "
            "exactly once as a result. This confirms checkpoint restoration creates a genuinely "
            "independent execution timeline per restore, each still gated by requiring its own "
            "approval delivery - it does not, by itself, duplicate execution without a matching "
            "number of approval decisions.",
        ],
    )
    write_evidence(evidence, EVIDENCE_DIR / "h3_restore_same_checkpoint_twice.json")


@pytest.mark.asyncio
async def test_replay_stale_response_after_completion_causes_duplicate_execution():
    r = await run_replay_stale_response_after_completion()

    assert len(r["log_after_first_completion"]) == 1
    # CANDIDATE_FINDING: re-delivering the SAME response, against the SAME
    # (already-consumed) checkpoint_id, on the SAME live Workflow object,
    # after the cycle already completed, produced a SECOND execution -
    # with no exception raised.
    duplicate_execution_occurred = len(r["log"]) == 2
    assert duplicate_execution_occurred
    assert r["error"] is None

    evidence = build_evidence(
        experiment="EXP-003B", scenario="replay_stale_response_after_completion", target=TARGET,
        proposal={"tool_name": "transfer_funds", "arguments": ARGS, "call_id": "stale_1"},
        approval_request={"note": "one pending request, approved once"},
        approval_decision={
            "first_delivery": "approve - completed the cycle, executed once",
            "second_delivery": "the IDENTICAL response object, re-submitted against the SAME checkpoint_id, on the SAME live Workflow object, after the first cycle had already completed",
        },
        delegation_path=[],
        state_boundary=StateBoundary(workflow="replay_workflow", session=None, checkpoint="same checkpoint_id delivered twice to the same live Workflow object"),
        execution=r["log"],
        authorization_binding=AuthorizationBinding(
            tool_match=True, argument_match=True, request_match=True, agent_match=True, delegation_match=True, checkpoint_match=False,
            notes=["checkpoint_match=False: the second delivery targeted a checkpoint_id that no longer represented live pending state (the cycle had already completed), yet was accepted anyway."],
        ),
        outcome="CANDIDATE_FINDING",
        notes=[
            "ONE human approval decision resulted in TWO executions of transfer_funds with "
            "identical arguments, on a minimal, realistic reproducer: (1) propose+pause, "
            "(2) approve+resume to completion, (3) re-submit the SAME approval response against "
            "the SAME checkpoint_id on the SAME still-live Workflow object. No exception, no "
            "warning specific to the replay was observed - the framework's own log only showed a "
            "generic 'pending requests will be overwritten' warning during the FIRST resume, not "
            "the second.",
            "Meets most of the task's CANDIDATE FINDING bar: reproducible (deterministic), clean "
            "baseline (B1-B3 all pass), realistic/public lifecycle (Runner-level API only, no "
            "private internals touched), a stated authorization invariant this violates ('one "
            "approval occurrence should not silently authorize another distinct occurrence' - "
            "task section 11), a real authority mismatch (1 decision -> 2 executions), meaningful "
            "side-effect potential (duplicate financial transaction pattern), affected version "
            "(agent-framework==1.18.0), and a minimal reproducer (this test).",
            "RULED OUT one hedge: this is NOT specific to reusing the same live Workflow object. "
            "Re-tested with a SECOND, FRESHLY RECONSTRUCTED Agent/Workflow object graph (the exact "
            "same reconstruction pattern used throughout this lab's passing B3/H1/H2/H6 tests) "
            "restoring the same checkpoint_id and re-delivering the same already-consumed "
            "response: duplicate execution occurred again, with no exception. This means the gap "
            "(such as it is) lives in the checkpoint/response bookkeeping itself, not in "
            "object-lifetime misuse - strengthening this as a CANDIDATE_FINDING rather than a "
            "test-harness artifact.",
            "STILL NOT CONFIRMED (why this stays CANDIDATE_FINDING, not a rated vulnerability): "
            "whether upstream documentation states that a checkpoint_id/response pair is expected "
            "to be single-use, and whether InMemoryCheckpointStorage specifically (vs. "
            "FileCheckpointStorage or a real deployment's storage backend) is representative here. "
            "Also not established: real-world exploitability (an attacker would need the ability "
            "to redeliver a response against a stale checkpoint_id, which is itself an application-"
            "level access-control question outside this SDK's scope). See README and the final "
            "report's Phase-2 recommendation.",
        ],
    )
    write_evidence(evidence, EVIDENCE_DIR / "replay_stale_response_after_completion.json")
