"""
tests/test_argument_binding.py — H1: approved argument binding.

Question: is approval bound to the exact arguments presented to the
human, or can materially different arguments reach execution under the
same authorization event?

Two realistic, publicly-reachable mutation vectors are tested (see
scenarios/approval_binding.py's docstring for why these were chosen over
private-internal corruption):

  A. mutate the approval item's public `raw_item.arguments` AFTER
     `state.approve(item)` but BEFORE the resuming run executes it.
  B. serialize -> RunState.from_string() reload -> mutate the RELOADED
     item's arguments -> approve -> resume.
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
from scenarios.approval_binding import (
    APPROVED_ARGS,
    run_h1_post_approval_mutation,
    run_h1_reload_then_mutate,
)

TARGET = {"package": "openai-agents", "version": "0.22.2"}


@pytest.mark.asyncio
async def test_h1a_post_approval_mutation_does_not_execute_drifted_args():
    """PASS condition: after approve() is called on the original item,
    mutating that same object's raw_item.arguments before resume must
    NOT cause the drifted arguments to execute."""
    r = await run_h1_post_approval_mutation()
    sink = r["sink"]

    assert sink.count_for_tool("transfer_funds") == 1
    executed = sink.records[0]

    argument_match = executed.arguments == r["presented_args"]
    drift_leaked = executed.arguments == r["mutated_to"]

    assert argument_match, (
        f"APPROVAL_BINDING FAIL: executed args {executed.arguments} != presented/approved args {r['presented_args']}"
    )
    assert not drift_leaked

    evidence = build_evidence(
        experiment="EXP-003A",
        scenario="h1a_post_approval_mutation",
        target=TARGET,
        proposal={"tool_name": "transfer_funds", "arguments": APPROVED_ARGS, "call_id": "h1a_1"},
        approval_request={"tool_name": "transfer_funds", "arguments": r["presented_args"], "call_id": "h1a_1"},
        approval_decision={"decision": "approve", "call_id": "h1a_1", "note": "approved BEFORE the raw_item.arguments mutation"},
        delegation_path=[],
        execution=[vars(rec) for rec in sink.records],
        authorization_binding=AuthorizationBinding(
            tool_match=True,
            argument_match=argument_match,
            call_id_match=executed.call_id == "h1a_1",
            namespace_match=True,
            delegation_match=True,
            notes=[
                "Mutated the approval item's public raw_item.arguments field AFTER state.approve() "
                "but before resume. Executed arguments matched the ORIGINALLY PRESENTED/APPROVED "
                "values, not the post-approval mutation - the SDK does not re-read raw_item live "
                "at execution time for a call_id it has already registered an invocation record for."
            ],
        ),
        outcome="PASS",
        notes=[
            "APPROVAL_BINDING = PASS for this transition: materially changed arguments after "
            "approval did not reach execution.",
        ],
    )
    write_evidence(evidence, EVIDENCE_DIR / "h1a_post_approval_mutation.json")


@pytest.mark.asyncio
async def test_h1b_reload_then_mutate_before_approval_is_rejected_by_sdk():
    """PASS condition (stronger than A): the SDK's own
    tool-invocation-fingerprint check detects that the reloaded item's
    arguments no longer match what was registered for this call_id when
    the state was reconstructed, and raises ModelBehaviorError rather
    than silently approving/executing the drifted arguments."""
    with pytest.raises(agents.exceptions.ModelBehaviorError, match="reused a tool call ID"):
        await run_h1_reload_then_mutate()

    evidence = build_evidence(
        experiment="EXP-003A",
        scenario="h1b_reload_then_mutate_before_approval",
        target=TARGET,
        proposal={"tool_name": "transfer_funds", "arguments": APPROVED_ARGS, "call_id": "h1b_1"},
        approval_request={"tool_name": "transfer_funds", "arguments": APPROVED_ARGS, "call_id": "h1b_1"},
        approval_decision={
            "decision": "attempted_approve_of_mutated_item",
            "call_id": "h1b_1",
            "note": "reloaded item's raw_item.arguments was mutated to a materially different value before calling reloaded_state.approve()",
        },
        delegation_path=[],
        execution=None,
        authorization_binding=AuthorizationBinding(
            tool_match=True,
            argument_match=False,
            call_id_match=True,
            namespace_match=True,
            delegation_match=True,
            notes=[
                "SDK raised agents.exceptions.ModelBehaviorError('Model reused a tool call ID for a "
                "different invocation.') from RunContextWrapper._tool_invocation_status, confirmed "
                "by reading agents/run_context.py and agents/_tool_invocation.py source: a "
                "per-call-id fingerprint over {approval_scope, arguments, ...semantic fields} is "
                "registered at first sight and re-checked on every subsequent registration attempt "
                "for that call_id; a mismatch is treated as a hard error, not a silent pass-through.",
            ],
        ),
        outcome="PASS",
        notes=[
            "APPROVAL_BINDING = PASS: the SDK actively detects and blocks (via a hard exception) an "
            "attempt to approve a reloaded item whose arguments were altered after "
            "serialization/reload, rather than executing the drifted content.",
        ],
    )
    write_evidence(evidence, EVIDENCE_DIR / "h1b_reload_then_mutate_before_approval.json")
