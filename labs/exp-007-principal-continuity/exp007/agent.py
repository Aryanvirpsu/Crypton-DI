"""Agent A executes only what the policy engine authorizes."""

from __future__ import annotations

from typing import Callable

from .authorization import Decision, PolicyContext, Verdict
from .control_channel import ControlMessage
from .resource import MockResource
from .tracing import Recorder

NOT_EXECUTED = "NOT_EXECUTED"


class Agent:
    def __init__(
        self, ctx: PolicyContext, policy: Callable[[ControlMessage, PolicyContext], Decision],
        policy_name: str, resource: MockResource, recorder: Recorder,
    ) -> None:
        self._ctx = ctx
        self._policy = policy
        self._policy_name = policy_name
        self._resource = resource
        self._recorder = recorder

    def handle(self, msg: ControlMessage, leg: str) -> tuple[Decision, int]:
        decision = self._policy(msg, self._ctx)
        result = NOT_EXECUTED
        if decision.verdict is Verdict.ALLOW and msg.action == "SEND_MESSAGE":
            result = self._resource.send_message(msg.params)
        step = self._recorder.record(
            leg=leg, policy=self._policy_name, principal=self._ctx.session.principal_pid,
            agent=self._ctx.agent_id, grant=self._ctx.grant.as_dict(), authority_root=self._ctx.grant.owner_pid,
            params=msg.params, decision=decision, result=result,
        )
        return decision, step
