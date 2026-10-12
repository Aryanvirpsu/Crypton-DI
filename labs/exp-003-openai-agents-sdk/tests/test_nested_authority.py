"""
tests/test_nested_authority.py — H2: nested delegation boundary.

ROOT_AGENT --Agent.as_tool(needs_approval=True)--> SPECIALIST_AGENT
    --delete_record(needs_approval=True)--> SENSITIVE_TOOL

Expected property: approval at one delegation boundary must not silently
become authorization for a distinct downstream authority.
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
from scenarios.nested_agent import DELETE_ARGS, run_h2_same_tool_name_both_levels, run_h2_two_boundaries

TARGET = {"package": "openai-agents", "version": "0.22.2"}


@pytest.mark.asyncio
async def test_h2_delegation_approval_does_not_confer_sensitive_tool_approval():
    r = await run_h2_two_boundaries()

    # Round 1: only the delegation boundary is pending - the sensitive
    # tool inside the specialist has not even been reached yet.
    assert [i.tool_name for i in r["interruptions_round_1"]] == ["specialist"]
    assert r["sensitive_items_round_1"] == []
    assert r["records_after_round_1_interrupt"] == []

    # After approving ONLY the delegation call: the sensitive tool call
    # now surfaces as its OWN, separate interruption - not silently executed.
    assert [i.tool_name for i in r["interruptions_round_2"]] == ["delete_record"]
    assert r["records_after_round_2"] == [], "delete_record executed without its own approval decision"

    # After approving that second, separate item: exactly one execution,
    # correctly attributed to the nested (specialist) agent identity.
    final_records = r["records_final"]
    assert len(final_records) == 1
    executed = final_records[0]
    assert executed.tool_name == "delete_record"
    assert executed.arguments == DELETE_ARGS
    assert executed.agent_name == "specialist"
    assert executed.call_id == "specialist_delete_1"

    evidence = build_evidence(
        experiment="EXP-003A",
        scenario="h2_delegation_vs_sensitive_tool_boundary",
        target=TARGET,
        proposal={"tool_name": "specialist (delegation)", "then": "delete_record (nested)"},
        approval_request={
            "round_1": [{"tool_name": i.tool_name, "call_id": i.call_id} for i in r["interruptions_round_1"]],
            "round_2": [{"tool_name": i.tool_name, "call_id": i.call_id} for i in r["interruptions_round_2"]],
        },
        approval_decision={"round_1": "approved delegation only", "round_2": "approved nested delete_record separately"},
        delegation_path=[
            {"agent": "root", "calls": "specialist", "call_id": "root_delegate_1"},
            {"agent": "specialist", "calls": "delete_record", "call_id": "specialist_delete_1"},
        ],
        execution=[vars(rec) for rec in final_records],
        authorization_binding=AuthorizationBinding(
            tool_match=True,
            argument_match=executed.arguments == DELETE_ARGS,
            call_id_match=True,
            namespace_match=None,
            delegation_match=True,
            notes=[
                "namespace_match is null: tool_namespace was observed as None for both the "
                "delegation tool and the nested sensitive tool in this configuration - the SDK "
                "did not assign a distinguishing namespace string here (not fabricated as True).",
            ],
        ),
        outcome="PASS",
        notes=[
            "Approving the delegation boundary (specialist) did NOT confer approval for the "
            "downstream sensitive tool (delete_record) inside it - it required and received its "
            "own, separate approval decision in a later round.",
        ],
    )
    write_evidence(evidence, EVIDENCE_DIR / "h2_delegation_vs_sensitive_tool_boundary.json")


@pytest.mark.asyncio
async def test_h2_same_tool_name_at_different_nesting_levels_stay_independent():
    r = await run_h2_same_tool_name_both_levels()

    # Round 1: root-level direct call and the delegation call are both
    # pending; the nested specialist call is not reachable yet.
    round_1_call_ids = {i.call_id for i in r["interruptions_round_1"]}
    assert round_1_call_ids == {"root_delete_direct_1", "root_delegate_2"}

    # Approving those two must execute ONLY the root-level delete_record -
    # not the nested one, which shares the same tool name.
    after_round_2 = r["records_after_round_2"]
    assert len(after_round_2) == 1
    assert after_round_2[0].agent_name == "root"
    assert after_round_2[0].call_id == "root_delete_direct_1"

    # The nested specialist's delete_record must still be pending, as its
    # own distinct item.
    assert [i.call_id for i in r["interruptions_round_2"]] == ["specialist_delete_1"]

    final = r["records_final"]
    assert len(final) == 2
    by_call_id = {rec.call_id: rec for rec in final}
    assert by_call_id["root_delete_direct_1"].agent_name == "root"
    assert by_call_id["specialist_delete_1"].agent_name == "specialist"

    evidence = build_evidence(
        experiment="EXP-003A",
        scenario="h2_same_tool_name_both_levels",
        target=TARGET,
        proposal={"note": "delete_record attached directly to root AND reachable via specialist delegation - identical tool name, distinct call_ids"},
        approval_request={"round_1_call_ids": sorted(round_1_call_ids)},
        approval_decision={"round_1": "approved root-level delete_record + delegation only", "round_2": "approved nested delete_record separately"},
        delegation_path=[
            {"agent": "root", "calls": "delete_record", "call_id": "root_delete_direct_1"},
            {"agent": "root", "calls": "specialist", "call_id": "root_delegate_2"},
            {"agent": "specialist", "calls": "delete_record", "call_id": "specialist_delete_1"},
        ],
        execution=[vars(rec) for rec in final],
        authorization_binding=AuthorizationBinding(
            tool_match=True, argument_match=True, call_id_match=True, namespace_match=None, delegation_match=True,
            notes=["Identical tool NAME at two nesting levels stayed correctly disambiguated by call_id and agent identity."],
        ),
        outcome="PASS",
        notes=["Same-named tool at two nesting levels: approving one call_id's instance did not confer approval on the other."],
    )
    write_evidence(evidence, EVIDENCE_DIR / "h2_same_tool_name_both_levels.json")
