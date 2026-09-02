"""
The agent loop — the core of Agent Lab.

Explicitly implements, in order, every step the spec calls out:

  1. authenticate human            (identity.py — done by the caller)
  2. resolve + verify the registered agent (agents.require_active_agent)
  3. create/resume session         (sessions.py)
  4. create task                   (tasks.py)
  5. create AgentInstance          (identity.py + instances.py)
  6. mark task running             (tasks.update_task_status)
  7. build a model request         (system prompt + tool specs + conversation)
  8. send it to the model          (ModelProvider.complete)
  9. parse a structured tool call out of the response, if any
 10. reject unknown tool names     (never call a function by a name the
                                     model invented that isn't registered)
 11. run the deterministic pre-execution authorization pipeline against
     this agent's explicit Delegation (authorization.py / delegation.py) —
     never a name-keyed permission map, never the model's judgment
 12. validate the tool call's arguments (models.py / Pydantic)
 12a. for a tool that requires it, verify an approved, unexpired, unused
     Approval whose exact argument digest matches what's about to
     execute (approval.py) — never a substitute for step 11, and never
     overridable by step 11 either; see LEARNING.md's v0.5 addendum
 13. execute the tool              (tools.py)
 14. derive + persist a task-state update from the (trusted) tool result
 15. feed the result back to the model, and repeat — bounded by max_steps
 16. mark task completed/failed, end the AgentInstance, return

Read this file top to bottom to see exactly what "an agent" does here —
see LEARNING.md sections 1-5 and the v0.2/v0.3/v0.4/v0.5 addenda.
"""

from __future__ import annotations

import time
import uuid
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from pydantic import ValidationError

from . import agents, approval as approval_module, authorization, chain as chain_module, instances, sessions, tasks
from .agents import AgentAccessError, AgentNotActiveError
from .identity import AgentIdentity, AgentInstance, HumanIdentity
from .lifecycle import TaskStatus
from .model_provider import ModelProvider, ToolCallRequest
from .models import TOOL_ARG_MODELS, requires_human_approval, tool_specs_for_model
from .tools import TOOL_FUNCTIONS, ToolError
from .tracing import DEFAULT_TRACE_DIR, Tracer

MAX_STEPS_DEFAULT = 6

SYSTEM_PROMPT = (
    "You are a support agent. You can look up customers and orders and "
    "add case notes using the tools provided. Only use the tools given "
    "to you — never invent a tool name or argument. If you don't have "
    "enough information to call a tool, ask in your final text response "
    "instead of guessing an argument value."
)


@dataclass
class AgentResult:
    final_text: str | None
    steps_used: int
    stopped_reason: str  # "final_response" | "max_steps" | "error"
    request_id: str
    instance_id: str
    session_id: str
    task_id: str
    task_status: str  # the Task's final lifecycle status, e.g. "completed" / "failed"
    agent_name: str  # descriptive metadata only, resolved from the registry — see agents.py
    agent_version: str


def derive_state_update(tool_name: str, result: dict[str, Any]) -> dict[str, Any]:
    """The one place a tool's result is allowed to become task state —
    and only through this fixed, per-tool mapping, never by copying a
    tool's full result or the model's raw arguments wholesale.

    `result` here is always the trusted dict `tools.py` returned (already
    scoped to the calling human by a DB query), never `call.arguments`
    (model-controlled). Even so, this only pulls specific known-safe
    fields out — it does not just do `return result`. See tasks.TaskState
    for the schema this is validated against on the way in, and
    LEARNING.md's "why persistent state creates a new security boundary"
    for why that distinction matters."""
    if tool_name == "get_customer":
        return {"last_tool": tool_name, "last_customer_id": result.get("customer_id")}
    if tool_name == "get_order":
        return {
            "last_tool": tool_name,
            "last_order_id": result.get("order_id"),
            "last_customer_id": result.get("customer_id"),
        }
    if tool_name == "add_case_note":
        return {"last_tool": tool_name, "last_customer_id": result.get("customer_id"), "last_action": "note_added"}
    return {"last_tool": tool_name}


class AgentRunner:
    """One instance = one *claimed* registered agent_id (e.g.
    `"agt_3f9a..."`, never a hand-picked string — see agents.py), acting
    under one *claimed* Delegation (e.g. `"dlg_7c1b..."`, see
    delegation.py). Both ids are only verified inside `run()`
    (agent_id/status) and inside `_handle_tool_call` (the delegation,
    immediately before every tool execution), not at construction time —
    ownership/status/expiry can all change between two `run()` calls on
    the same long-lived runner, so every call re-resolves and re-checks
    them fresh, the same discipline `sessions.get_session`/`tasks.get_task`
    already use for their IDs. Each `run()` call always creates a fresh
    Task and a fresh AgentInstance for that (re-verified) agent, resuming
    at most the Session.

    v0.4: `delegation_id` is required and explicit — this runner never
    falls back to an implicit or ambient grant of authority. See
    LEARNING.md's v0.4 addendum for why authentication (verifying the
    human, the agent) and authorization (what this specific run is
    allowed to do) are kept as two entirely separate questions.

    v0.5: `approval_id` is optional and, when given, is the one
    pre-approved consequential action this run may consume — at most
    once (see approval.py). A run with no `approval_id` can still
    execute every tool that doesn't require approval; it will only ever
    hit APPROVAL_REQUIRED on the ones that do. This mirrors
    `delegation_id`'s role but for a narrower, human-confirmed, one-shot
    grant rather than a standing one."""

    def __init__(
        self,
        *,
        agent_id: str,
        delegation_id: str,
        model: ModelProvider,
        db_conn: Any,
        approval_id: str | None = None,
        max_steps: int = MAX_STEPS_DEFAULT,
        trace_dir: Path | str = DEFAULT_TRACE_DIR,
    ):
        self.agent_id = agent_id
        self.delegation_id = delegation_id
        self.approval_id = approval_id
        self.model = model
        self.db_conn = db_conn
        self.max_steps = max_steps
        self.trace_dir = trace_dir

    async def run(
        self,
        *,
        human: HumanIdentity,
        goal: str,
        session_id: str | None = None,
        request_id: str | None = None,
    ) -> AgentResult:
        request_id = request_id or str(uuid.uuid4())
        # `agent_id` on the tracer is the *claimed* id from construction —
        # correct even if verification below fails, so a rejected launch
        # attempt is still fully traceable (which id was tried, by whom).
        tracer = Tracer(
            request_id=request_id,
            agent_id=self.agent_id,
            human_user_id=human.user_id,
            trace_dir=self.trace_dir,
        )
        tracer.record("run_started", goal=goal)

        # 1. Resolve + verify the registered agent BEFORE creating any
        # session/task/instance — an unknown, foreign, or disabled
        # agent_id means this run never really begins. Nothing is
        # created for a run that can't execute. This is "before creating
        # an AgentInstance verify: exists, owned, active" — pulled to the
        # very top since a Session/Task for an agent that can't run would
        # just be dead weight.
        try:
            agent_record = agents.require_active_agent(self.db_conn, human, self.agent_id)
        except (AgentAccessError, AgentNotActiveError) as e:
            tracer.record("error", kind="agent_invalid", detail=str(e))
            raise

        # The runtime identity for the rest of this run — built entirely
        # from the just-verified, database-backed `agent_record`, never
        # from anything a caller or model supplied directly.
        identity = AgentIdentity(
            agent_id=agent_record.agent_id,
            version=agent_record.version,
            name=agent_record.name,
            owner_user_id=agent_record.owner_user_id,
        )

        # 2. create or resume session. `sessions.get_session` re-checks
        # ownership even if `session_id` came from an untrusted source —
        # a caller can never resume someone else's session by supplying
        # their ID (see sessions.SessionAccessError).
        if session_id is not None:
            session = sessions.get_session(self.db_conn, human, session_id)
            tracer.session_id = session.session_id
            tracer.record("session_resumed")
        else:
            session = sessions.create_session(self.db_conn, human)
            tracer.session_id = session.session_id
            tracer.record("session_created")

        # 3. create the task — a specific requested objective within the
        # session, tagged with the canonical agent_id. Always new; v0.2
        # does not resume an existing task.
        task = tasks.create_task(self.db_conn, session=session, human=human, agent_id=identity.agent_id, goal=goal)
        tracer.task_id = task.task_id
        tracer.record("task_created", goal=goal)

        # 4. create the AgentInstance — one concrete execution of this
        # task, by this registered agent. `human`/`session`/`task`/
        # `identity` here are all real, authenticated, ownership-checked
        # values — never anything read from model output.
        instance = AgentInstance(
            instance_id=str(uuid.uuid4()),
            agent_id=identity.agent_id,
            human_user_id=human.user_id,
            session_id=session.session_id,
            task_id=task.task_id,
            started_at=time.time(),
        )
        tracer.instance_id = instance.instance_id
        instances.record_instance_started(self.db_conn, instance)
        # agent_name/agent_version here are non-authoritative metadata on
        # the trace event, for readability — the actual identity is
        # `instance.agent_id`, already set above.
        tracer.record(
            "agent_instance_created", agent_id=identity.agent_id, agent_name=identity.name, agent_version=identity.version
        )

        # 5. mark task running.
        task = tasks.update_task_status(self.db_conn, human, task.task_id, TaskStatus.RUNNING, tracer=tracer)

        messages: list[dict[str, Any]] = [{"role": "user", "content": goal}]
        tool_specs = tool_specs_for_model()

        for step in range(1, self.max_steps + 1):
            tracer.record("model_call", step=step, message_count=len(messages))
            try:
                response = await self.model.complete(system=SYSTEM_PROMPT, messages=messages, tools=tool_specs)
            except Exception as e:
                # Malformed/failed model output never crashes the loop —
                # it's an explicit, traced stop. running -> failed.
                tracer.record("error", step=step, kind="model_call_failed", detail=str(e))
                return self._finish(
                    tracer=tracer,
                    human=human,
                    task=task,
                    instance=instance,
                    identity=identity,
                    status=TaskStatus.FAILED,
                    final_text=None,
                    steps_used=step,
                    stopped_reason="error",
                    reason="model_call_failed",
                )

            if response.tool_call is None:
                tracer.record("final_response", step=step, text=response.text)
                return self._finish(
                    tracer=tracer,
                    human=human,
                    task=task,
                    instance=instance,
                    identity=identity,
                    status=TaskStatus.COMPLETED,
                    final_text=response.text,
                    steps_used=step,
                    stopped_reason="final_response",
                )

            call = response.tool_call
            tracer.record("tool_call_requested", step=step, tool=call.name, arguments=call.arguments)
            messages.append({"role": "assistant", "content": None, "tool_call": call})

            result_payload, is_error = self._handle_tool_call(call, human, instance, identity, task.task_id, tracer, step)
            messages.append(
                {
                    "role": "tool_result",
                    "tool_use_id": call.id,
                    "name": call.name,
                    "content": result_payload,
                    "is_error": is_error,
                }
            )

            if not is_error:
                state_update = derive_state_update(call.name, result_payload)
                if state_update:
                    try:
                        task = tasks.update_task_state(self.db_conn, human, task.task_id, state_update, tracer=tracer)
                    except Exception as e:
                        # A bug here is our own code's fault (state_update
                        # is derived, not model-controlled) — but the loop
                        # still fails cleanly rather than crashing the caller.
                        tracer.record("error", step=step, kind="task_state_update_failed", detail=str(e))
                        return self._finish(
                            tracer=tracer,
                            human=human,
                            task=task,
                            instance=instance,
                            identity=identity,
                            status=TaskStatus.FAILED,
                            final_text=None,
                            steps_used=step,
                            stopped_reason="error",
                            reason="task_state_update_failed",
                        )

        # Max steps reached. This is an explicit terminal lifecycle state,
        # not a new sixth status: it maps to `failed` (the task did not
        # reach a completed goal), with the specific reason captured in
        # the trace/task history rather than invented as a new status
        # value — see LEARNING.md for why.
        tracer.record("max_steps_reached", max_steps=self.max_steps)
        return self._finish(
            tracer=tracer,
            human=human,
            task=task,
            instance=instance,
            identity=identity,
            status=TaskStatus.FAILED,
            final_text=None,
            steps_used=self.max_steps,
            stopped_reason="max_steps",
            reason="max_steps_exceeded",
        )

    def _finish(
        self,
        *,
        tracer: Tracer,
        human: HumanIdentity,
        task: tasks.Task,
        instance: AgentInstance,
        identity: AgentIdentity,
        status: TaskStatus,
        final_text: str | None,
        steps_used: int,
        stopped_reason: str,
        **event_data: Any,
    ) -> AgentResult:
        """Single exit path for every way `run()` can end: marks the task
        completed/failed, ends the AgentInstance, and builds the result.
        Centralizing this is what guarantees a task never ends up stuck
        `running` — every return statement in the loop goes through here."""
        task = tasks.update_task_status(self.db_conn, human, task.task_id, status, tracer=tracer, **event_data)
        instances.record_instance_finished(
            self.db_conn,
            instance.instance_id,
            status=instances.INSTANCE_COMPLETED if status == TaskStatus.COMPLETED else instances.INSTANCE_FAILED,
        )
        return AgentResult(
            final_text=final_text,
            steps_used=steps_used,
            stopped_reason=stopped_reason,
            request_id=tracer.request_id,
            instance_id=instance.instance_id,
            session_id=task.session_id,
            task_id=task.task_id,
            task_status=task.status.value,
            agent_name=identity.name,
            agent_version=identity.version,
        )

    def _handle_tool_call(
        self,
        call: ToolCallRequest,
        human: HumanIdentity,
        instance: AgentInstance,
        identity: AgentIdentity,
        task_id: str,
        tracer: Tracer,
        step: int,
    ) -> tuple[dict, bool]:
        """Runs every security boundary in order and returns
        (result_payload, is_error) to feed back to the model. The model
        sees *that* something failed and a short reason — never a Python
        stack trace, and never enough detail to distinguish "doesn't
        exist" from "exists but isn't yours" (see tools.py). Nothing here
        or in the tools it calls ever reads an ownership/identity field
        out of `call.arguments` — `human`, `instance`, and `identity` are
        always the real, already-authenticated/verified values from
        `run()`, never anything the model supplied."""

        # Boundary 1 — unknown tool: never look up a function by a name
        # the model invented that isn't in our schema map at all. Not an
        # authorization question (there's no delegation scope for a tool
        # that doesn't exist), so no AuthorizationDecision is recorded for
        # this one — see authorization.py's module docstring.
        if call.name not in TOOL_ARG_MODELS:
            tracer.record("tool_rejected", step=step, tool=call.name, reason="unknown_tool")
            return {"error": "unknown_tool", "tool": call.name}, True

        # Boundary 2 — authorization (v0.4, strengthened to full
        # delegation-chain verification in v0.6): chain.verify_delegation_chain
        # calls authorization.authorize_tool_call as its first, mandatory
        # step (agent identity, instance/principal match, delegation
        # existence/ownership/agent-match/task-match/expiry/revocation,
        # tool allowlist, resource scope — completely unchanged from v0.4),
        # then, only if the leaf delegation has a parent, walks every
        # ancestor: signature validity, signing-key liveness, parent-hash
        # continuity, issuer/subject continuity, and scope attenuation at
        # each hop, up to and including the root. A root-only (single-hop)
        # delegation has no ancestors to walk, so this is exactly v0.4's
        # behavior for every pre-v0.6 delegation — see chain.py's module
        # docstring and DECISIONS.md's v0.6 section. Every one of these
        # checks is against the canonical `identity.agent_id` and real,
        # persisted rows — never `identity.name`, never a caller-supplied
        # chain. The model has no say in any of this. Every attempt —
        # allowed or not, at any hop — is persisted as an
        # AuthorizationDecision before this function returns, so a denial
        # always leaves evidence even though it leaves no other side
        # effect.
        resource_type, resource_id = authorization.resource_for_tool_call(call.name, call.arguments)
        chain_result = chain_module.verify_delegation_chain(
            self.db_conn,
            principal=human,
            agent_id=identity.agent_id,
            instance=instance,
            task_id=task_id,
            delegation_id=self.delegation_id,
            tool_name=call.name,
            resource_type=resource_type,
            resource_id=resource_id,
        )
        tracer.record(
            "chain_verified",
            step=step,
            tool=call.name,
            allowed=chain_result.allowed,
            root_principal=chain_result.root_principal,
            acting_principal=chain_result.acting_principal,
            delegation_depth=chain_result.delegation_depth,
            authority_path=[f"{kind}:{pid}" for kind, pid in chain_result.authority_path],
            chain_hash=chain_result.chain_hash,
            failed_hop=chain_result.failed_hop,
            denial_reason=chain_result.denial_reason,
        )
        decision = chain_result.decision
        if not decision.allowed:
            tracer.record(
                "tool_rejected",
                step=step,
                tool=call.name,
                reason=decision.reason_code.lower(),
                decision_id=decision.decision_id,
            )
            return {"error": "unauthorized", "reason": decision.reason_code}, True

        # Boundary 3 — argument validation (pipeline step 14): the
        # model's JSON is untrusted input and is never passed straight
        # into Python. `extra="forbid"` on every args model also means an
        # attempt to smuggle an ownership-looking field (e.g.
        # "human_user_id", "agent_id") is rejected here. This runs *after*
        # authorization, not before — authority to call a tool at all is a
        # separate question from whether this particular call's shape is
        # well-formed — but a rejection here is still persisted as its own
        # AuthorizationDecision (DENY, INVALID_ARGUMENTS): every attempted
        # consequential tool call gets evidence, not just the ones that
        # fail the delegation check.
        arg_model = TOOL_ARG_MODELS[call.name]
        try:
            validated_args = arg_model.model_validate(call.arguments)
        except ValidationError as e:
            authorization.record_decision(
                self.db_conn,
                decision=authorization.DECISION_DENY,
                reason_code=authorization.INVALID_ARGUMENTS,
                principal_user_id=human.user_id,
                agent_id=identity.agent_id,
                instance_id=instance.instance_id,
                task_id=task_id,
                delegation_id=self.delegation_id,
                tool_name=call.name,
                resource=f"{resource_type}:{resource_id}" if resource_type else None,
            )
            tracer.record("tool_rejected", step=step, tool=call.name, reason="invalid_arguments", detail=str(e))
            return {"error": "invalid_arguments", "detail": str(e)}, True

        tracer.record(
            "tool_arguments_validated", step=step, tool=call.name, arguments=validated_args.model_dump()
        )

        # Boundary 4 — human approval (v0.5): a tool flagged
        # `requires_human_approval` (models.py — Tool Registry metadata,
        # not authority) needs a human's exact-argument-bound
        # confirmation on top of a valid Delegation, checked only now
        # that both authorization *and* argument validation have already
        # passed. A valid Delegation never bypasses this; conversely, a
        # valid Approval could never have overridden the Delegation check
        # above, since that already returned before this line runs.
        # `self.approval_id` is a runner-level constant — like
        # `delegation_id`, never something the model supplies per call.
        if requires_human_approval(call.name):
            if self.approval_id is None:
                decision = authorization.record_decision(
                    self.db_conn,
                    decision=authorization.DECISION_DENY,
                    reason_code=approval_module.APPROVAL_REQUIRED,
                    principal_user_id=human.user_id,
                    agent_id=identity.agent_id,
                    instance_id=instance.instance_id,
                    task_id=task_id,
                    delegation_id=self.delegation_id,
                    tool_name=call.name,
                    resource=f"{resource_type}:{resource_id}" if resource_type else None,
                )
            else:
                decision = approval_module.verify_and_consume_approval(
                    self.db_conn,
                    approval_id=self.approval_id,
                    principal=human,
                    agent_id=identity.agent_id,
                    instance_id=instance.instance_id,
                    task_id=task_id,
                    delegation_id=self.delegation_id,
                    tool_name=call.name,
                    resource_type=resource_type,
                    resource_id=resource_id,
                    validated_arguments=validated_args.model_dump(),
                )
            if not decision.allowed:
                tracer.record(
                    "tool_rejected",
                    step=step,
                    tool=call.name,
                    reason=decision.reason_code.lower(),
                    decision_id=decision.decision_id,
                )
                return {"error": "unauthorized", "reason": decision.reason_code}, True

        # Boundary 5 — execute. Tool functions only ever see the
        # validated Pydantic object, never raw model JSON, and never
        # touch the shell/filesystem/network (see tools.py).
        tool_fn = TOOL_FUNCTIONS[call.name]
        try:
            result = tool_fn(self.db_conn, human, instance, validated_args)
        except ToolError as e:
            tracer.record("tool_error", step=step, tool=call.name, detail=str(e))
            return {"error": "tool_error", "detail": str(e)}, True
        except Exception as e:
            # Last-resort guard: a bug in a tool must never escape as a
            # raw exception to the caller or leak internals to the model.
            tracer.record("tool_error", step=step, tool=call.name, detail=f"unexpected: {e}")
            return {"error": "internal_error"}, True

        tracer.record("tool_result", step=step, tool=call.name, result=result)
        return result, False
