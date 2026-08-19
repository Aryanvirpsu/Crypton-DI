"""
Agent-loop tests, all driven by FakeModelProvider so nothing here makes a
network call.
"""

import json

import pytest

from agentlab import agents, instances, tasks
from agentlab.agent import AgentRunner
from agentlab.agents import AgentAccessError, AgentNotActiveError
from agentlab.model_provider import FakeModelProvider, ModelResponse, ToolCallRequest


def register_agent_for(db_conn, owner, name="support-agent", version="0.1.0"):
    """Test helper: register a real agent for whichever human is about
    to run it (fixtures `support_agent`/`read_only_agent` are always
    alice's; some scenarios need bob to own his own copy)."""
    return agents.register_agent(db_conn, owner, name=name, version=version)


def make_runner(owner, agent_name, db_conn, trace_dir, script, max_steps=6):
    """Registers `agent_name` for `owner` and builds a runner for it —
    the v0.3 replacement for constructing a bare AgentIdentity by hand."""
    agent = register_agent_for(db_conn, owner, name=agent_name)
    model = FakeModelProvider(script=script)
    runner = AgentRunner(agent_id=agent.agent_id, model=model, db_conn=db_conn, max_steps=max_steps, trace_dir=trace_dir)
    return runner, model, agent


async def test_valid_tool_call_flow(db_conn, trace_dir, alice):
    runner, _, agent = make_runner(
        alice,
        "support-agent",
        db_conn,
        trace_dir,
        script=[
            ModelResponse(tool_call=ToolCallRequest(id="c1", name="get_customer", arguments={"customer_id": "cust_1"})),
            ModelResponse(text="Alice Anders is gold tier."),
        ],
    )
    result = await runner.run(human=alice, goal="who is cust_1?")
    assert result.stopped_reason == "final_response"
    assert result.final_text == "Alice Anders is gold tier."
    assert result.steps_used == 2
    assert result.task_status == "completed"
    assert result.agent_name == "support-agent"
    assert result.agent_version == "0.1.0"


async def test_unknown_tool_rejected(db_conn, trace_dir, alice):
    runner, _, _ = make_runner(
        alice,
        "support-agent",
        db_conn,
        trace_dir,
        script=[
            ModelResponse(tool_call=ToolCallRequest(id="c1", name="delete_customer", arguments={"customer_id": "cust_1"})),
            ModelResponse(text="I can't do that."),
        ],
    )
    result = await runner.run(human=alice, goal="delete cust_1")
    events = [e.event for e in _trace_events(trace_dir, result.request_id)]
    reasons = [e.data.get("reason") for e in _trace_events(trace_dir, result.request_id) if e.event == "tool_rejected"]
    assert "tool_rejected" in events
    assert "unknown_tool" in reasons
    # the tool never ran — no crash, model gets told and recovers
    assert result.stopped_reason == "final_response"
    assert result.task_status == "completed"


async def test_malformed_arguments_rejected(db_conn, trace_dir, alice):
    runner, _, _ = make_runner(
        alice,
        "support-agent",
        db_conn,
        trace_dir,
        script=[
            ModelResponse(tool_call=ToolCallRequest(id="c1", name="get_customer", arguments={})),  # missing customer_id
            ModelResponse(text="Sorry, I need a customer id."),
        ],
    )
    result = await runner.run(human=alice, goal="who is that?")
    reasons = [e.data.get("reason") for e in _trace_events(trace_dir, result.request_id) if e.event == "tool_rejected"]
    assert "invalid_arguments" in reasons
    assert result.stopped_reason == "final_response"


async def test_unauthorized_tool_rejected(db_conn, trace_dir, alice):
    """read-only-agent is registered but not allowed to call add_case_note."""
    runner, _, _ = make_runner(
        alice,
        "read-only-agent",
        db_conn,
        trace_dir,
        script=[
            ModelResponse(tool_call=ToolCallRequest(id="c1", name="add_case_note", arguments={"customer_id": "cust_1", "note": "x"})),
            ModelResponse(text="I'm not allowed to write notes."),
        ],
    )
    result = await runner.run(human=alice, goal="add a note")
    reasons = [e.data.get("reason") for e in _trace_events(trace_dir, result.request_id) if e.event == "tool_rejected"]
    assert "unauthorized" in reasons
    # confirm nothing was actually written
    row = db_conn.execute("SELECT COUNT(*) FROM case_notes").fetchone()
    assert row[0] == 0


async def test_max_step_termination(db_conn, trace_dir, alice):
    """A model that never stops calling tools is cut off at max_steps,
    not left running forever — and the task lands in an explicit
    terminal state (failed), never stuck at running."""
    endless_tool_calls = [
        ModelResponse(tool_call=ToolCallRequest(id=f"c{i}", name="get_customer", arguments={"customer_id": "cust_1"}))
        for i in range(10)
    ]
    runner, _, _ = make_runner(alice, "support-agent", db_conn, trace_dir, script=endless_tool_calls, max_steps=3)
    result = await runner.run(human=alice, goal="keep going")
    assert result.stopped_reason == "max_steps"
    assert result.steps_used == 3
    assert result.task_status == "failed"

    events = _trace_events(trace_dir, result.request_id)
    max_step_events = [e for e in events if e.event == "max_steps_reached"]
    failed_events = [e for e in events if e.event == "task_failed"]
    assert len(max_step_events) == 1
    assert len(failed_events) == 1
    assert failed_events[0].data.get("reason") == "max_steps_exceeded"

    task = tasks.get_task(db_conn, alice, result.task_id)
    assert task.status.value == "failed"


async def test_model_error_marks_task_failed(db_conn, trace_dir, alice):
    """running -> failed on an unrecoverable model-call failure."""

    class BrokenModel(FakeModelProvider):
        async def complete(self, *, system, messages, tools):
            raise RuntimeError("upstream exploded")

    agent = register_agent_for(db_conn, alice)
    runner = AgentRunner(agent_id=agent.agent_id, model=BrokenModel(), db_conn=db_conn, trace_dir=trace_dir)
    result = await runner.run(human=alice, goal="anything")
    assert result.stopped_reason == "error"
    assert result.task_status == "failed"

    task = tasks.get_task(db_conn, alice, result.task_id)
    assert task.status.value == "failed"
    assert task.completed_at is not None


async def test_separate_user_data_stays_isolated_through_agent(db_conn, trace_dir, alice, bob):
    """bob's agent run must not surface alice's customer even though the
    row exists in the same database. Bob runs his own registered agent —
    v0.3 agents are owner-scoped, so he can't use alice's."""
    runner, _, _ = make_runner(
        bob,
        "support-agent",
        db_conn,
        trace_dir,
        script=[
            ModelResponse(tool_call=ToolCallRequest(id="c1", name="get_customer", arguments={"customer_id": "cust_1"})),
            ModelResponse(text="I couldn't find that customer."),
        ],
    )
    result = await runner.run(human=bob, goal="who is cust_1?")
    tool_errors = [e for e in _trace_events(trace_dir, result.request_id) if e.event == "tool_error"]
    assert len(tool_errors) == 1
    assert "customer_not_found" in tool_errors[0].data["detail"]

    # the run's own task/session are still bob's, unaffected
    task = tasks.get_task(db_conn, bob, result.task_id)
    assert task.human_user_id == "user-bob"


async def test_trace_created_for_each_action(db_conn, trace_dir, alice):
    runner, _, _ = make_runner(
        alice,
        "support-agent",
        db_conn,
        trace_dir,
        script=[
            ModelResponse(tool_call=ToolCallRequest(id="c1", name="get_customer", arguments={"customer_id": "cust_1"})),
            ModelResponse(text="Alice Anders is gold tier."),
        ],
    )
    result = await runner.run(human=alice, goal="who is cust_1?")

    trace_path = trace_dir / f"{result.request_id}.jsonl"
    assert trace_path.exists()
    lines = [json.loads(line) for line in trace_path.read_text(encoding="utf-8").splitlines()]

    event_names = [line["event"] for line in lines]
    for expected in (
        "run_started",
        "session_created",
        "task_created",
        "agent_instance_created",
        "task_started",
        "model_call",
        "tool_call_requested",
        "tool_arguments_validated",
        "tool_result",
        "task_state_updated",
        "final_response",
        "task_completed",
    ):
        assert expected in event_names

    # every line carries the full identity + correlation set
    for line in lines:
        assert line["request_id"] == result.request_id
        assert line["human_user_id"] == "user-alice"


async def test_trace_events_correlate_session_task_instance(db_conn, trace_dir, alice):
    """A complete run must be reconstructable from traces: every event
    from task_created onward shares the same session_id, task_id, and
    instance_id."""
    runner, _, _ = make_runner(
        alice,
        "support-agent",
        db_conn,
        trace_dir,
        script=[
            ModelResponse(tool_call=ToolCallRequest(id="c1", name="get_customer", arguments={"customer_id": "cust_1"})),
            ModelResponse(text="Alice Anders is gold tier."),
        ],
    )
    result = await runner.run(human=alice, goal="who is cust_1?")
    events = _trace_events(trace_dir, result.request_id)

    by_event = {e.event: e for e in events}
    # session_created has no task_id/instance_id yet — legitimately null.
    assert by_event["session_created"].session_id == result.session_id
    assert by_event["session_created"].task_id is None
    assert by_event["session_created"].instance_id is None

    # from task_created onward, every event carries the real task_id.
    after_task_created = events[events.index(by_event["task_created"]) :]
    for e in after_task_created:
        assert e.session_id == result.session_id
        assert e.task_id == result.task_id

    # from agent_instance_created onward, every event carries the real
    # instance_id.
    after_instance_created = events[events.index(by_event["agent_instance_created"]) :]
    for e in after_instance_created:
        assert e.instance_id == result.instance_id


async def test_agent_instance_references_task_session_human(db_conn, trace_dir, alice, support_agent):
    runner = AgentRunner(
        agent_id=support_agent.agent_id,
        model=FakeModelProvider(script=[ModelResponse(text="done")]),
        db_conn=db_conn,
        trace_dir=trace_dir,
    )
    result = await runner.run(human=alice, goal="just answer")

    instance = instances.get_instance(db_conn, alice, result.instance_id)
    assert instance is not None
    assert instance.agent_id == support_agent.agent_id
    assert instance.human_user_id == "user-alice"
    assert instance.session_id == result.session_id
    assert instance.task_id == result.task_id
    assert instance.status == "completed"
    assert instance.ended_at is not None
    assert instance.ended_at >= instance.started_at

    # the referenced agent_id really does resolve to a persisted Agent —
    # never a dangling/foreign string.
    persisted = agents.get_agent(db_conn, alice, instance.agent_id)
    assert persisted.agent_id == support_agent.agent_id


async def test_task_references_canonical_agent_id(db_conn, trace_dir, alice, support_agent):
    runner = AgentRunner(
        agent_id=support_agent.agent_id,
        model=FakeModelProvider(script=[ModelResponse(text="done")]),
        db_conn=db_conn,
        trace_dir=trace_dir,
    )
    result = await runner.run(human=alice, goal="just answer")
    task = tasks.get_task(db_conn, alice, result.task_id)
    assert task.agent_id == support_agent.agent_id
    assert task.agent_id != "support-agent"  # never the human-readable name


async def test_model_cannot_override_agent_id(db_conn, trace_dir, alice, bob):
    """Even if a (malicious) model's tool-call arguments contain an
    "agent_id"-shaped field, it's rejected outright by argument
    validation (extra="forbid"), and the task's real agent_id is
    unaffected either way."""
    runner, model, _ = make_runner(
        alice,
        "support-agent",
        db_conn,
        trace_dir,
        script=[
            ModelResponse(
                tool_call=ToolCallRequest(
                    id="c1",
                    name="get_customer",
                    arguments={"customer_id": "cust_1", "agent_id": "agt_something_else"},
                )
            ),
            ModelResponse(text="handled"),
        ],
    )
    result = await runner.run(human=alice, goal="who is cust_1?")

    reasons = [e.data.get("reason") for e in _trace_events(trace_dir, result.request_id) if e.event == "tool_rejected"]
    assert "invalid_arguments" in reasons

    task = tasks.get_task(db_conn, alice, result.task_id)
    assert task.agent_id == runner.agent_id  # unchanged — never something the model said


async def test_agent_name_cannot_substitute_for_agent_id(db_conn, trace_dir, alice, support_agent):
    """Running with the human-readable name instead of the canonical
    agent_id fails — a name is not a valid identity handle."""
    runner = AgentRunner(
        agent_id="support-agent",  # the *name*, not support_agent.agent_id
        model=FakeModelProvider(script=[ModelResponse(text="should never get here")]),
        db_conn=db_conn,
        trace_dir=trace_dir,
    )
    with pytest.raises(AgentAccessError):
        await runner.run(human=alice, goal="anything")


async def test_unknown_agent_cannot_run(db_conn, trace_dir, alice):
    runner = AgentRunner(
        agent_id="agt_does_not_exist",
        model=FakeModelProvider(script=[ModelResponse(text="unreachable")]),
        db_conn=db_conn,
        trace_dir=trace_dir,
    )
    with pytest.raises(AgentAccessError):
        await runner.run(human=alice, goal="anything")
    # nothing persisted for a run that never started
    assert db_conn.execute("SELECT COUNT(*) FROM sessions").fetchone()[0] == 0


async def test_disabled_agent_cannot_run(db_conn, trace_dir, alice, support_agent):
    agents.disable_agent(db_conn, alice, support_agent.agent_id)
    runner = AgentRunner(
        agent_id=support_agent.agent_id,
        model=FakeModelProvider(script=[ModelResponse(text="unreachable")]),
        db_conn=db_conn,
        trace_dir=trace_dir,
    )
    with pytest.raises(AgentNotActiveError):
        await runner.run(human=alice, goal="anything")


async def test_wrong_owner_cannot_launch_agent(db_conn, trace_dir, alice, bob, support_agent):
    """support_agent is alice's — bob presenting its real agent_id must
    still be rejected; ownership is checked, not just existence."""
    runner = AgentRunner(
        agent_id=support_agent.agent_id,
        model=FakeModelProvider(script=[ModelResponse(text="unreachable")]),
        db_conn=db_conn,
        trace_dir=trace_dir,
    )
    with pytest.raises(AgentAccessError):
        await runner.run(human=bob, goal="anything")
    # bob gets no session/task either — the run never starts
    assert db_conn.execute("SELECT COUNT(*) FROM sessions WHERE human_user_id = 'user-bob'").fetchone()[0] == 0


async def test_model_provider_metadata_is_not_agent_identity(db_conn, trace_dir, alice, support_agent):
    """Two runs against the exact same registered agent, using different
    FakeModelProvider scripts (standing in for "different providers"),
    still resolve to the identical agent_id/name/version — the model has
    no bearing on agent identity at all."""
    runner_a = AgentRunner(
        agent_id=support_agent.agent_id,
        model=FakeModelProvider(script=[ModelResponse(text="answer from script A")]),
        db_conn=db_conn,
        trace_dir=trace_dir,
    )
    runner_b = AgentRunner(
        agent_id=support_agent.agent_id,
        model=FakeModelProvider(script=[ModelResponse(text="answer from script B")]),
        db_conn=db_conn,
        trace_dir=trace_dir,
    )
    result_a = await runner_a.run(human=alice, goal="task A")
    result_b = await runner_b.run(human=alice, goal="task B")

    assert result_a.agent_name == result_b.agent_name == "support-agent"
    assert result_a.agent_version == result_b.agent_version == "0.1.0"
    task_a = tasks.get_task(db_conn, alice, result_a.task_id)
    task_b = tasks.get_task(db_conn, alice, result_b.task_id)
    assert task_a.agent_id == task_b.agent_id == support_agent.agent_id


async def test_trace_contains_canonical_agent_id(db_conn, trace_dir, alice, support_agent):
    runner = AgentRunner(
        agent_id=support_agent.agent_id,
        model=FakeModelProvider(script=[ModelResponse(text="done")]),
        db_conn=db_conn,
        trace_dir=trace_dir,
    )
    result = await runner.run(human=alice, goal="just answer")
    trace_path = trace_dir / f"{result.request_id}.jsonl"
    lines = [json.loads(line) for line in trace_path.read_text(encoding="utf-8").splitlines()]
    for line in lines:
        assert line["agent_id"] == support_agent.agent_id
        assert line["agent_id"] != "support-agent"


def _trace_events(trace_dir, request_id):
    path = trace_dir / f"{request_id}.jsonl"
    events = []
    for line in path.read_text(encoding="utf-8").splitlines():
        raw = json.loads(line)
        events.append(_Event(raw["event"], raw["data"], raw["session_id"], raw["task_id"], raw["instance_id"]))
    return events


class _Event:
    def __init__(self, event, data, session_id, task_id, instance_id):
        self.event = event
        self.data = data
        self.session_id = session_id
        self.task_id = task_id
        self.instance_id = instance_id
