"""
scenarios/nested_agent.py — H2: nested delegation boundary.

    ROOT_AGENT
        |
        v  Agent.as_tool(needs_approval=True)   <- delegation boundary (A)
    SPECIALIST_AGENT
        |
        v  delete_record(needs_approval=True)    <- sensitive-tool boundary (B)
    SENSITIVE_TOOL

Tests whether approving A (the delegation call itself) silently confers
B (the sensitive tool call inside the specialist), and the reverse.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

import agents
from agents.testing import ScriptedModel, assistant_message, function_call

from crypton_exp003 import fixtures
from crypton_exp003.audit import AuditSink
from crypton_exp003.tools import ensure_tracing_disabled

DELETE_ARGS = {"record_id": "rec-1"}


def _build_specialist() -> agents.Agent:
    ensure_tracing_disabled()
    return agents.Agent(
        name="specialist",
        instructions="You are a specialist agent used only for deterministic authorization testing.",
        tools=[fixtures.delete_record],
        model=ScriptedModel(
            steps=[
                [function_call("delete_record", DELETE_ARGS, call_id="specialist_delete_1")],
                [assistant_message("deleted")],
            ]
        ),
    )


def _build_root(specialist_tool: agents.FunctionTool) -> agents.Agent:
    ensure_tracing_disabled()
    return agents.Agent(
        name="root",
        instructions="You are a root agent used only for deterministic authorization testing.",
        tools=[specialist_tool],
        model=ScriptedModel(
            steps=[
                [function_call(specialist_tool.name, {"input": "please delete rec-1"}, call_id="root_delegate_1")],
                [assistant_message("delegation complete")],
            ]
        ),
    )


async def run_h2_two_boundaries(scenario_id: str = "h2_two_boundaries") -> dict:
    """Approve ONLY the delegation call (A). Check whether the sensitive
    tool call (B) inside the specialist executes without a SEPARATE
    approval decision, or whether it surfaces as its own interruption."""
    sink = AuditSink(scenario_id)
    token = fixtures.bind_sink(sink)
    try:
        specialist = _build_specialist()
        specialist_tool = specialist.as_tool("specialist", "Delegate to the specialist agent.", needs_approval=True)
        root = _build_root(specialist_tool)

        result_1 = await agents.Runner.run(root, "please have the specialist delete rec-1")
        state_1 = result_1.to_state()
        interruptions_1 = state_1.get_interruptions()
        records_after_round_1_interrupt = list(sink.records)

        # Approve ONLY the delegation-boundary item(s) present in this round.
        delegation_items = [i for i in interruptions_1 if i.tool_name == "specialist"]
        sensitive_items_round_1 = [i for i in interruptions_1 if i.tool_name == "delete_record"]
        for item in delegation_items:
            state_1.approve(item, always_approve=False)

        result_2 = await agents.Runner.run(root, state_1)
        state_2 = result_2.to_state()
        interruptions_2 = state_2.get_interruptions()
        records_after_round_2 = list(sink.records)

        sensitive_items_round_2 = [i for i in interruptions_2 if i.tool_name == "delete_record"]
        for item in sensitive_items_round_2:
            state_2.approve(item, always_approve=False)

        result_3 = await agents.Runner.run(root, state_2) if interruptions_2 else result_2
        records_final = list(sink.records)

        return {
            "sink": sink,
            "interruptions_round_1": interruptions_1,
            "delegation_items": delegation_items,
            "sensitive_items_round_1": sensitive_items_round_1,
            "records_after_round_1_interrupt": records_after_round_1_interrupt,
            "interruptions_round_2": interruptions_2,
            "sensitive_items_round_2": sensitive_items_round_2,
            "records_after_round_2": records_after_round_2,
            "records_final": records_final,
            "result_3": result_3,
        }
    finally:
        fixtures.unbind_sink(token)


async def run_h2_same_tool_name_both_levels(scenario_id: str = "h2_same_tool_name_both_levels") -> dict:
    """Same underlying tool (delete_record) attached BOTH directly to
    ROOT_AGENT and, separately, inside SPECIALIST_AGENT (reached via
    delegation). Distinct call IDs. Approve only the ROOT-level direct
    call; confirm the nested (specialist-level) call still requires and
    receives its own, independent approval decision."""
    sink = AuditSink(scenario_id)
    token = fixtures.bind_sink(sink)
    try:
        specialist = _build_specialist()
        specialist_tool = specialist.as_tool("specialist", "Delegate to the specialist agent.", needs_approval=True)
        ensure_tracing_disabled()
        root = agents.Agent(
            name="root",
            instructions="You are a root agent used only for deterministic authorization testing.",
            tools=[specialist_tool, fixtures.delete_record],
            model=ScriptedModel(
                steps=[
                    [
                        function_call("delete_record", DELETE_ARGS, call_id="root_delete_direct_1"),
                        function_call(specialist_tool.name, {"input": "please delete rec-1"}, call_id="root_delegate_2"),
                    ],
                    [assistant_message("both done")],
                ]
            ),
        )

        result_1 = await agents.Runner.run(root, "please delete rec-1 directly, and also via the specialist")
        state_1 = result_1.to_state()
        interruptions_1 = state_1.get_interruptions()

        # Approve ONLY the direct root-level delete_record call and the delegation call
        # (delegation must be approved separately to let the nested run start at all),
        # but deliberately do NOT approve anything belonging to the nested run yet.
        root_level_items = [i for i in interruptions_1 if i.call_id in ("root_delete_direct_1", "root_delegate_2")]
        assert len(root_level_items) == len(interruptions_1), "unexpected extra interruption in round 1"
        for item in root_level_items:
            state_1.approve(item, always_approve=False)

        result_2 = await agents.Runner.run(root, state_1)
        state_2 = result_2.to_state()
        interruptions_2 = state_2.get_interruptions()  # expected: the nested specialist's own delete_record call
        records_after_round_2 = list(sink.records)

        for item in interruptions_2:
            state_2.approve(item, always_approve=False)
        result_3 = await agents.Runner.run(root, state_2) if interruptions_2 else result_2

        return {
            "sink": sink,
            "interruptions_round_1": interruptions_1,
            "records_after_round_2": records_after_round_2,
            "interruptions_round_2": interruptions_2,
            "records_final": list(sink.records),
        }
    finally:
        fixtures.unbind_sink(token)


async def run_h2_reverse_only_sensitive_approved(scenario_id: str = "h2_reverse") -> dict:
    """Reverse direction: what happens if we try to approve the SENSITIVE
    tool item without ever having approved the delegation boundary? (In
    practice this can only be observed if both interruptions are ever
    presented in the same round - tested here for completeness.)"""
    sink = AuditSink(scenario_id)
    token = fixtures.bind_sink(sink)
    try:
        specialist = _build_specialist()
        specialist_tool = specialist.as_tool("specialist", "Delegate to the specialist agent.", needs_approval=True)
        root = _build_root(specialist_tool)

        result_1 = await agents.Runner.run(root, "please have the specialist delete rec-1")
        state_1 = result_1.to_state()
        interruptions_1 = state_1.get_interruptions()

        return {
            "sink": sink,
            "interruptions_round_1": interruptions_1,
            "tool_names_round_1": [i.tool_name for i in interruptions_1],
        }
    finally:
        fixtures.unbind_sink(token)
