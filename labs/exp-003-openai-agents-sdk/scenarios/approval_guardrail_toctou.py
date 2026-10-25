"""
scenarios/approval_guardrail_toctou.py — H5: approval/guardrail TOCTOU.

The documented execution lifecycle says tool input guardrails run
immediately before invocation (in addition to any that are configured to
run before approval too). This constructs a harmless, mutable
policy/state example: a plain module-level dict `POLICY`, checked by a
real `agents.tool_input_guardrail`-decorated function - not a private SDK
internal.

    approval time:  POLICY["limit"] = 1000    (transfer of 100 is well within it)
    before resume:  POLICY["limit"] = 50      (application-level policy change)
    approved action: transfer 100

If the guardrail genuinely re-runs immediately before invocation, it
should see the CHANGED limit and block the now-stale-relative-to-policy
authorization, even though the human already approved it under the old
limit.
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

POLICY = {"limit": 1000}
TRANSFER_ARGS = {"account": "checking", "amount": 100, "destination": "vendor-A"}


@agents.tool_input_guardrail
async def policy_limit_guardrail(data: agents.ToolInputGuardrailData) -> agents.ToolGuardrailFunctionOutput:
    ctx = data.context
    raw_args = ctx.tool_arguments
    args = json.loads(raw_args) if isinstance(raw_args, str) else raw_args
    amount = args.get("amount", 0) if isinstance(args, dict) else 0
    if amount > POLICY["limit"]:
        return agents.ToolGuardrailFunctionOutput.raise_exception(
            output_info={"limit_at_check_time": POLICY["limit"], "attempted_amount": amount}
        )
    return agents.ToolGuardrailFunctionOutput.allow()


@agents.function_tool(needs_approval=True, tool_input_guardrails=[policy_limit_guardrail])
async def transfer_funds_guarded(ctx: agents.tool_context.ToolContext, account: str, amount: int, destination: str) -> str:
    fixtures._record(ctx, "transfer_funds_guarded", {"account": account, "amount": amount, "destination": destination})
    return f"transferred {amount} from {account} to {destination}"


def reset_policy(limit: int = 1000) -> None:
    POLICY["limit"] = limit


async def run_h5_policy_changes_between_approval_and_resume(scenario_id: str = "h5_policy_toctou") -> dict:
    reset_policy(1000)
    sink = AuditSink(scenario_id)
    token = fixtures.bind_sink(sink)
    try:
        agent = build_agent(
            name="root",
            tools=[transfer_funds_guarded],
            steps=[
                [function_call("transfer_funds_guarded", TRANSFER_ARGS, call_id="toctou_1")],
                [assistant_message("done")],
            ],
        )
        result = await agents.Runner.run(agent, "transfer 100")
        state = result.to_state()
        interruptions = state.get_interruptions()
        approved_under_limit = POLICY["limit"]
        for item in interruptions:
            state.approve(item)

        # Application-level policy change AFTER human approval, BEFORE resume.
        POLICY["limit"] = 50

        error: Exception | None = None
        result_after = None
        try:
            result_after = await agents.Runner.run(agent, state)
        except Exception as exc:  # noqa: BLE001 - deliberately broad, we want to classify whatever the SDK raises
            error = exc

        return {
            "sink": sink,
            "interruptions": interruptions,
            "approved_under_limit": approved_under_limit,
            "limit_at_resume": POLICY["limit"],
            "error": error,
            "result_after": result_after,
        }
    finally:
        fixtures.unbind_sink(token)
        reset_policy(1000)


async def run_h5_policy_changes_across_serialize_reload(scenario_id: str = "h5_policy_toctou_serialize_reload") -> dict:
    """Same as run_h5_policy_changes_between_approval_and_resume, but the
    resume path goes through RunState.to_string() -> RunState.from_string()
    first - checking the guardrail re-check isn't specific to the
    in-memory resume path only."""
    reset_policy(1000)
    sink = AuditSink(scenario_id)
    token = fixtures.bind_sink(sink)
    try:
        agent = build_agent(
            name="root",
            tools=[transfer_funds_guarded],
            steps=[
                [function_call("transfer_funds_guarded", TRANSFER_ARGS, call_id="sr_1")],
                [assistant_message("done")],
            ],
        )
        result = await agents.Runner.run(agent, "transfer 100")
        state = result.to_state()
        for item in state.get_interruptions():
            state.approve(item)
        serialized = state.to_string()

        POLICY["limit"] = 50  # policy change AFTER approval, BEFORE reload+resume
        reloaded = await agents.RunState.from_string(agent, serialized)

        error: Exception | None = None
        try:
            await agents.Runner.run(agent, reloaded)
        except Exception as exc:  # noqa: BLE001
            error = exc

        return {"sink": sink, "error": error}
    finally:
        fixtures.unbind_sink(token)
        reset_policy(1000)


async def run_h5_policy_unchanged_control(scenario_id: str = "h5_policy_control") -> dict:
    """Control: same flow, but the policy is NOT changed before resume -
    confirms the guardrail does not spuriously block a still-valid call."""
    reset_policy(1000)
    sink = AuditSink(scenario_id)
    token = fixtures.bind_sink(sink)
    try:
        agent = build_agent(
            name="root",
            tools=[transfer_funds_guarded],
            steps=[
                [function_call("transfer_funds_guarded", TRANSFER_ARGS, call_id="control_1")],
                [assistant_message("done")],
            ],
        )
        result = await agents.Runner.run(agent, "transfer 100")
        state = result.to_state()
        for item in state.get_interruptions():
            state.approve(item)
        result_after = await agents.Runner.run(agent, state)
        return {"sink": sink, "result_after": result_after}
    finally:
        fixtures.unbind_sink(token)
        reset_policy(1000)
