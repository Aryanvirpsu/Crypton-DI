"""
tests/test_sticky_approval.py — H3: sticky / always-approve scope.

We are NOT assuming persistent approval is a vulnerability. This
determines exactly what authority `always_approve=True` grants, and
classifies each observation as EXPECTED BY DESIGN / BOUND MORE NARROWLY
THAN EXPECTED / BOUND MORE BROADLY THAN DOCUMENTED / POTENTIAL SECURITY
GAP / INCONCLUSIVE - never "vulnerability" merely because the grant is
broad.
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
from scenarios.sticky_approval import (
    FIRST_CALL_ARGS,
    HIGHER_IMPACT_ARGS,
    LOW_RISK_DIFFERENT_ARGS,
    run_h3_same_tool_sequence,
    run_h3_same_tool_via_nested_agent,
    run_h3_similarly_named_tool,
)

TARGET = {"package": "openai-agents", "version": "0.22.2"}


@pytest.mark.asyncio
async def test_h3_always_approve_covers_same_same_and_higher_impact_calls():
    """AUTHORITY EXPANSION record: initial approved authority (one
    send_message call, low-risk args) versus subsequent automatically
    authorized authority (three further calls, including a materially
    higher-impact one), all under one always_approve=True grant."""
    r = await run_h3_same_tool_sequence()
    rounds = r["rounds"]

    # Only round 0 interrupted; every subsequent call (same args, different
    # low-risk args, materially higher-impact args) executed with NO further
    # interruption once always_approve=True was granted on round 0.
    assert rounds[0]["interruptions"] == [("send_message", "sticky_1")]
    assert rounds[1]["interruptions"] == []
    assert len(rounds) == 2, "expected exactly one further round with no pending interruptions"

    sink = r["sink"]
    assert [rec.call_id for rec in sink.records] == ["sticky_1", "sticky_2", "sticky_3", "sticky_4"]
    by_call_id = {rec.call_id: rec for rec in sink.records}
    assert by_call_id["sticky_1"].arguments == FIRST_CALL_ARGS
    assert by_call_id["sticky_3"].arguments == LOW_RISK_DIFFERENT_ARGS
    assert by_call_id["sticky_4"].arguments == HIGHER_IMPACT_ARGS  # the materially higher-impact call executed too

    classification = "EXPECTED BY DESIGN"
    evidence = build_evidence(
        experiment="EXP-003A",
        scenario="h3_always_approve_same_tool_sequence",
        target=TARGET,
        proposal={"initial_call": {"tool_name": "send_message", "arguments": FIRST_CALL_ARGS, "call_id": "sticky_1"}},
        approval_request={"call_id": "sticky_1", "tool_name": "send_message"},
        approval_decision={"decision": "approve", "always_approve": True, "call_id": "sticky_1"},
        delegation_path=[],
        execution=[vars(rec) for rec in sink.records],
        authorization_binding=AuthorizationBinding(
            tool_match=True, argument_match=None, call_id_match=True, namespace_match=True, delegation_match=True,
            notes=[
                "argument_match is null by design here: this scenario specifically tests whether "
                "DIFFERENT arguments are still authorized under one always_approve grant - the "
                "relevant fact is that they were, not whether they equal the original.",
            ],
        ),
        outcome="PASS",
        notes=[
            "AUTHORITY EXPANSION: initial approved authority = one send_message call with "
            f"{FIRST_CALL_ARGS!r}. Subsequent automatically authorized authority = ALL further "
            "send_message calls in this run, including one with materially different, "
            f"higher-impact arguments ({HIGHER_IMPACT_ARGS!r}), with zero additional human "
            "approval decisions.",
            f"Classification: {classification}. Reason: source inspection "
            "(agents/_tool_invocation.py::tool_invocation_approval_scope) confirms the persisted "
            "'always' grant is keyed by (invocation_type, tool_lookup_key) - i.e. TOOL IDENTITY - "
            "and explicitly does not include argument content in that scope key. The parameter "
            "is literally named always_approve, matching this behavior; this is not a hidden or "
            "undocumented mechanism.",
            "OBSERVATION (not a CANDIDATE FINDING): a human reviewer who clicks 'always approve' "
            "on a low-risk-looking call may not realize they have granted blanket authority over "
            "arbitrary future argument values for that exact tool identity within the continuing "
            "run. Worth flagging explicitly in any real assessment engagement that recommends "
            "always_approve usage.",
        ],
    )
    write_evidence(evidence, EVIDENCE_DIR / "h3_always_approve_same_tool_sequence.json")


@pytest.mark.asyncio
async def test_h3_always_approve_does_not_persist_across_separate_agent_contexts():
    r = await run_h3_same_tool_via_nested_agent()

    assert [rec.call_id for rec in r["records_after_first_agent"]] == ["nested_sticky_1"]
    # A genuinely separate agent/run (no shared context or RunState) still
    # required its OWN approval - the always_approve grant did not leak in.
    assert [(i.tool_name, i.call_id) for i in r["interruptions_2"]] == [("send_message", "nested_sticky_2")]

    evidence = build_evidence(
        experiment="EXP-003A",
        scenario="h3_always_approve_scope_across_separate_contexts",
        target=TARGET,
        proposal={"note": "always_approve granted under agent 'root'; a separate agent 'root_2' with its own fresh context later proposes the same tool"},
        approval_request={"first_agent_call_id": "nested_sticky_1", "second_agent_call_id": "nested_sticky_2"},
        approval_decision={"first_agent": "approve, always_approve=True", "second_agent": "no decision made yet - still pending"},
        delegation_path=[],
        execution=[vars(rec) for rec in r["records_after_first_agent"]],
        authorization_binding=AuthorizationBinding(
            tool_match=True, argument_match=None, call_id_match=True, namespace_match=True, delegation_match=True,
        ),
        outcome="PASS",
        notes=[
            "AUTHORITY EXPANSION: NONE observed across genuinely separate run contexts (no shared "
            "context= or RunState continuation). The second, independent agent run still required "
            "its own fresh approval for the identical tool.",
            "Classification: EXPECTED BY DESIGN. This only shows the grant is scoped to a "
            "continuing context, not that it is impossible to make it broader - a calling "
            "application that deliberately passes the SAME `context=` object across unrelated runs "
            "could plausibly extend this scope further; that configuration was not tested here "
            "(out of scope for this pass; candidate for Phase 2).",
        ],
    )
    write_evidence(evidence, EVIDENCE_DIR / "h3_always_approve_scope_across_separate_contexts.json")


@pytest.mark.asyncio
async def test_h3_always_approve_does_not_cover_a_differently_named_tool():
    r = await run_h3_similarly_named_tool()

    assert [(i.tool_name, i.call_id) for i in r["interruptions_round_1"]] == [("send_message", "similar_1")]
    # send_message_v2 is a DIFFERENT tool (different name/object) - it must
    # still require its own approval despite the always_approve grant on
    # send_message.
    assert [(i.tool_name, i.call_id) for i in r["interruptions_round_2"]] == [("send_message_v2", "similar_2")]
    assert [rec.tool_name for rec in r["sink"].records] == ["send_message"]  # v2 not yet approved/executed

    evidence = build_evidence(
        experiment="EXP-003A",
        scenario="h3_always_approve_does_not_bleed_to_similar_name",
        target=TARGET,
        proposal={"note": "send_message approved with always_approve=True; a distinctly-named tool send_message_v2 is proposed next"},
        approval_request={"round_1": "send_message", "round_2": "send_message_v2"},
        approval_decision={"round_1": "approve, always_approve=True", "round_2": "not yet decided"},
        delegation_path=[],
        execution=[vars(rec) for rec in r["sink"].records],
        authorization_binding=AuthorizationBinding(
            tool_match=True, argument_match=True, call_id_match=True, namespace_match=True, delegation_match=True,
        ),
        outcome="PASS",
        notes=[
            "A true tool-NAME collision within one Agent is not constructible via the public API "
            "(function_tool/Agent enforce unique tool names) - tested the closest reachable "
            "analogue, a distinctly-named-but-visually-similar tool, which correctly required its "
            "own approval.",
            "The 'same bare tool name in another namespace' sub-case from the task's H3 list was "
            "NOT independently constructed in this pass: every scenario in this experiment "
            "observed tool_namespace=None for ordinary FunctionTool/Agent.as_tool configurations, "
            "and no documented public API for assigning an explicit namespace to a second "
            "same-named FunctionTool was found without deeper MCP-specific investigation. Recorded "
            "as INCONCLUSIVE / not reached, not as PASS.",
        ],
    )
    write_evidence(evidence, EVIDENCE_DIR / "h3_always_approve_does_not_bleed_to_similar_name.json")
