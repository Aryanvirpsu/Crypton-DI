"""
scenarios/approval_binding.py — H1: is approval bound to the exact
arguments presented to the human?

`ToolApprovalItem.arguments` is a live property computed from
`self.raw_item.arguments` (confirmed by reading the SDK source - not
assumed). `raw_item` is a public, non-underscore dataclass field. This
module tests two realistic, publicly-reachable mutation points using
only public attributes/APIs - never `_private` internals:

  A. mutate raw_item.arguments AFTER state.approve(item) but BEFORE the
     resuming Runner.run call actually invokes the tool.
  B. serialize -> reload into fresh objects (RunState.from_string) ->
     mutate the RELOADED item's raw_item.arguments -> approve -> resume.

Both represent something a real calling application could plausibly do
(hold a reference across an await boundary; edit a reloaded/deserialized
object before re-approving it) without reaching into SDK internals.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

import agents
from agents.testing import assistant_message, function_call

from crypton_exp003 import fixtures
from crypton_exp003.audit import AuditSink
from crypton_exp003.tools import build_agent

APPROVED_ARGS = {"account": "checking", "amount": 100, "destination": "vendor-A"}
DRIFTED_ARGS = {"account": "checking", "amount": 10000, "destination": "vendor-B"}


def _set_raw_arguments(item: agents.ToolApprovalItem, new_args: dict) -> None:
    """Mutate the public `raw_item.arguments` field in place - not a
    private/underscore attribute, and not monkeypatching a method."""
    item.raw_item.arguments = json.dumps(new_args)


async def run_h1_post_approval_mutation(scenario_id: str = "h1_post_approval_mutation") -> dict:
    sink = AuditSink(scenario_id)
    token = fixtures.bind_sink(sink)
    try:
        agent = build_agent(
            name="root",
            tools=[fixtures.transfer_funds],
            steps=[
                [function_call("transfer_funds", APPROVED_ARGS, call_id="h1a_1")],
                [assistant_message("done")],
            ],
        )
        result = await agents.Runner.run(agent, "transfer 100 to vendor-A")
        state = result.to_state()
        interruptions = state.get_interruptions()
        item = interruptions[0]
        presented_args = json.loads(item.arguments)

        state.approve(item, always_approve=False)
        # Mutate AFTER approval, BEFORE resume - the realistic ordering
        # a TOCTOU-style application bug would produce.
        _set_raw_arguments(item, DRIFTED_ARGS)

        result_after = await agents.Runner.run(agent, state)
        return {
            "sink": sink,
            "presented_args": presented_args,
            "mutated_to": DRIFTED_ARGS,
            "result_after": result_after,
        }
    finally:
        fixtures.unbind_sink(token)


async def run_h1_reload_then_mutate(scenario_id: str = "h1_reload_then_mutate") -> dict:
    sink = AuditSink(scenario_id)
    token = fixtures.bind_sink(sink)
    try:
        agent = build_agent(
            name="root",
            tools=[fixtures.transfer_funds],
            steps=[
                [function_call("transfer_funds", APPROVED_ARGS, call_id="h1b_1")],
                [assistant_message("done")],
            ],
        )
        result = await agents.Runner.run(agent, "transfer 100 to vendor-A")
        state = result.to_state()
        serialized = state.to_string()

        reloaded_state = await agents.RunState.from_string(agent, serialized)
        reloaded_items = reloaded_state.get_interruptions()
        item = reloaded_items[0]
        presented_args = json.loads(item.arguments)

        _set_raw_arguments(item, DRIFTED_ARGS)  # mutate the RELOADED (fresh) object, before approving
        reloaded_state.approve(item, always_approve=False)

        result_after = await agents.Runner.run(agent, reloaded_state)
        return {
            "sink": sink,
            "presented_args": presented_args,
            "mutated_to": DRIFTED_ARGS,
            "result_after": result_after,
        }
    finally:
        fixtures.unbind_sink(token)
