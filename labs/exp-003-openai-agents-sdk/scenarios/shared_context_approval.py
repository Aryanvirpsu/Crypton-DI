"""
scenarios/shared_context_approval.py — EXP-003A.2 surgical follow-up.

Question: can a documented shared `context=` object cause an
always_approve decision from one logically separate run to authorize
calls in another run?

Source check first (agents/run_context.py, via inspect.signature/
dataclasses.fields on RunContextWrapper): `_approvals` and
`_tool_invocations` - the fields that back always_approve/always_reject -
are declared as `default_factory=dict` fields ON THE WRAPPER itself, not
on the user-supplied `context` payload. `Runner.run(agent, input,
context=...)` constructs a NEW RunContextWrapper per call, wrapping
whatever `context` object was passed. So even the literal same `context`
object, passed to two independent Runner.run() calls, should not carry
approval bookkeeping between them - confirmed empirically below.
"""

from __future__ import annotations

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

RUN_A_ARGS = {"account": "checking", "amount": 100, "destination": "A"}
RUN_B_ARGS = {"account": "checking", "amount": 10000, "destination": "B"}


class SharedAppContext:
    """A plausible, documented-shape application context object - the
    kind a real caller would pass via Runner.run(..., context=...) to
    thread request-scoped state through tool calls."""


async def run_shared_context_probe(scenario_id: str = "a2_shared_context") -> dict:
    sink = AuditSink(scenario_id)
    token = fixtures.bind_sink(sink)
    try:
        shared_ctx = SharedAppContext()

        # RUN A - approved with always_approve=True, using shared_ctx.
        agent_a = build_agent(
            name="root_A",
            tools=[fixtures.transfer_funds],
            steps=[[function_call("transfer_funds", RUN_A_ARGS, call_id="rA_1")], [assistant_message("A done")]],
        )
        result_a = await agents.Runner.run(agent_a, "transfer 100 to A", context=shared_ctx)
        state_a = result_a.to_state()
        for item in state_a.get_interruptions():
            state_a.approve(item, always_approve=True)
        result_a2 = await agents.Runner.run(agent_a, state_a)

        # RUN B - a genuinely distinct run (fresh agent, fresh input, NOT
        # a RunState continuation of A) that deliberately passes the SAME
        # shared_ctx object, with a materially different, higher-impact
        # call to the SAME tool identity.
        agent_b = build_agent(
            name="root_B",
            tools=[fixtures.transfer_funds],
            steps=[[function_call("transfer_funds", RUN_B_ARGS, call_id="rB_1")], [assistant_message("B done")]],
        )
        result_b = await agents.Runner.run(agent_b, "transfer 10000 to B", context=shared_ctx)
        interruptions_b = result_b.to_state().get_interruptions()
        same_context_object = result_b.context_wrapper.context is shared_ctx

        # RUN C - control: fresh, unshared context, same tool identity.
        agent_c = build_agent(
            name="root_C",
            tools=[fixtures.transfer_funds],
            steps=[[function_call("transfer_funds", RUN_A_ARGS, call_id="rC_1")], [assistant_message("C done")]],
        )
        result_c = await agents.Runner.run(agent_c, "transfer 100 to A")
        interruptions_c = result_c.to_state().get_interruptions()

        return {
            "sink": sink,
            "result_a2": result_a2,
            "same_context_object_confirmed": same_context_object,
            "interruptions_b": interruptions_b,
            "interruptions_c": interruptions_c,
        }
    finally:
        fixtures.unbind_sink(token)
