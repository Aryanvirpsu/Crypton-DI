"""
End-to-end AgentRunner integration tests for human approval (v0.5) — the
full chain (principal, real registered agent, real Delegation, real
AgentInstance/Task, real Approval) driven through `AgentRunner.run()`
exactly the way test_agent.py drives the v0.4 delegation checks.

These approvals are deliberately *not* instance/task-bound (see
test_approval_pipeline.py for that coverage at the pipeline-unit level) —
`AgentRunner.run()` always mints a fresh instance/task per call, so an
approval usable across a real run has to be requested unbound, the same
way most of test_agent.py's delegations aren't task-bound either.
"""

from agentlab import agents, approval
from agentlab.agent import AgentRunner
from agentlab.delegation import grant_delegation, revoke_delegation
from agentlab.model_provider import FakeModelProvider, ModelResponse, ToolCallRequest
from agentlab.webauthn import FakeWebAuthnVerifier

ALL_TOOLS = frozenset({"get_customer", "get_order", "add_case_note"})
VERIFIER = FakeWebAuthnVerifier()
NOTE_ARGS = {"customer_id": "cust_1", "note": "refund approved"}


def setup_agent_and_delegation(db_conn, owner):
    agent = agents.register_agent(db_conn, owner, name="support-agent", version="0.1.0")
    grant = grant_delegation(
        db_conn,
        owner,
        agent_id=agent.agent_id,
        allowed_tools=ALL_TOOLS,
        resource_constraints={"customer_ids": {"cust_1", "cust_2"}},
        ttl_seconds=3600,
    )
    return agent, grant


def request_and_approve(db_conn, owner, agent, grant, *, note_args=None):
    note_args = note_args or NOTE_ARGS
    appr = approval.request_approval(
        db_conn,
        owner,
        agent_id=agent.agent_id,
        delegation_id=grant.delegation_id,
        tool_name="add_case_note",
        resource_type="customer_ids",
        resource_id=note_args["customer_id"],
        proposed_arguments=note_args,
        ttl_seconds=300,
    )
    return approval.approve(db_conn, owner, appr.approval_id, verifier=VERIFIER)


def add_note_script():
    return [
        ModelResponse(tool_call=ToolCallRequest(id="c1", name="add_case_note", arguments=dict(NOTE_ARGS))),
        ModelResponse(text="Noted."),
    ]


async def test_approved_action_executes_and_consumes_the_approval(db_conn, trace_dir, alice):
    agent, grant = setup_agent_and_delegation(db_conn, alice)
    appr = request_and_approve(db_conn, alice, agent, grant)

    runner = AgentRunner(
        agent_id=agent.agent_id,
        delegation_id=grant.delegation_id,
        approval_id=appr.approval_id,
        model=FakeModelProvider(script=add_note_script()),
        db_conn=db_conn,
        trace_dir=trace_dir,
    )
    result = await runner.run(human=alice, goal="add a note for cust_1")

    assert result.stopped_reason == "final_response"
    assert result.task_status == "completed"
    row = db_conn.execute("SELECT COUNT(*) FROM case_notes WHERE customer_id = 'cust_1'").fetchone()
    assert row[0] == 1

    consumed = approval.get_approval(db_conn, alice, appr.approval_id)
    assert consumed.status.value == "consumed"
    assert consumed.consumed_at is not None


async def test_missing_approval_denies_and_writes_nothing(db_conn, trace_dir, alice):
    agent, grant = setup_agent_and_delegation(db_conn, alice)
    runner = AgentRunner(
        agent_id=agent.agent_id,
        delegation_id=grant.delegation_id,
        # no approval_id at all
        model=FakeModelProvider(script=add_note_script()),
        db_conn=db_conn,
        trace_dir=trace_dir,
    )
    result = await runner.run(human=alice, goal="add a note for cust_1")

    row = db_conn.execute("SELECT COUNT(*) FROM case_notes").fetchone()
    assert row[0] == 0

    reasons = [
        e.data.get("reason")
        for e in _trace_events(trace_dir, result.request_id)
        if e.event == "tool_rejected"
    ]
    assert "approval_required" in reasons


async def test_replayed_approval_denies_the_second_execution(db_conn, trace_dir, alice):
    """First execution allowed; the exact same approval used again (a
    second run, same delegation, same approval_id) is denied — one-shot
    consumption survives across separate AgentInstances/Tasks, not just
    within one run() call."""
    agent, grant = setup_agent_and_delegation(db_conn, alice)
    appr = request_and_approve(db_conn, alice, agent, grant)

    runner_1 = AgentRunner(
        agent_id=agent.agent_id,
        delegation_id=grant.delegation_id,
        approval_id=appr.approval_id,
        model=FakeModelProvider(script=add_note_script()),
        db_conn=db_conn,
        trace_dir=trace_dir,
    )
    first = await runner_1.run(human=alice, goal="add a note for cust_1")
    assert first.task_status == "completed"

    runner_2 = AgentRunner(
        agent_id=agent.agent_id,
        delegation_id=grant.delegation_id,
        approval_id=appr.approval_id,  # the SAME approval_id, already consumed
        model=FakeModelProvider(script=add_note_script()),
        db_conn=db_conn,
        trace_dir=trace_dir,
    )
    second = await runner_2.run(human=alice, goal="add a note for cust_1 again")

    row = db_conn.execute("SELECT COUNT(*) FROM case_notes WHERE customer_id = 'cust_1'").fetchone()
    assert row[0] == 1  # still just the one note from the first run

    reasons = [
        e.data.get("reason")
        for e in _trace_events(trace_dir, second.request_id)
        if e.event == "tool_rejected"
    ]
    assert "approval_already_consumed" in reasons


async def test_argument_substitution_denies_execution(db_conn, trace_dir, alice):
    """Approved for one note's text; the model tries to execute with a
    different note -> denied, nothing written."""
    agent, grant = setup_agent_and_delegation(db_conn, alice)
    appr = request_and_approve(db_conn, alice, agent, grant, note_args={"customer_id": "cust_1", "note": "amount=100"})

    runner = AgentRunner(
        agent_id=agent.agent_id,
        delegation_id=grant.delegation_id,
        approval_id=appr.approval_id,
        model=FakeModelProvider(
            script=[
                ModelResponse(
                    tool_call=ToolCallRequest(
                        id="c1", name="add_case_note", arguments={"customer_id": "cust_1", "note": "amount=1000"}
                    )
                ),
                ModelResponse(text="Noted."),
            ]
        ),
        db_conn=db_conn,
        trace_dir=trace_dir,
    )
    result = await runner.run(human=alice, goal="add a note")

    row = db_conn.execute("SELECT COUNT(*) FROM case_notes").fetchone()
    assert row[0] == 0
    reasons = [
        e.data.get("reason")
        for e in _trace_events(trace_dir, result.request_id)
        if e.event == "tool_rejected"
    ]
    assert "approval_argument_mismatch" in reasons


async def test_valid_approval_never_overrides_a_revoked_delegation(db_conn, trace_dir, alice):
    """Authorization and approval are independent checks — a fully valid,
    approved Approval must not execute if the Delegation backing it has
    since been revoked. The approval is never even reached/consumed."""
    agent, grant = setup_agent_and_delegation(db_conn, alice)
    appr = request_and_approve(db_conn, alice, agent, grant)
    revoke_delegation(db_conn, alice, grant.delegation_id)

    runner = AgentRunner(
        agent_id=agent.agent_id,
        delegation_id=grant.delegation_id,
        approval_id=appr.approval_id,
        model=FakeModelProvider(script=add_note_script()),
        db_conn=db_conn,
        trace_dir=trace_dir,
    )
    result = await runner.run(human=alice, goal="add a note for cust_1")

    row = db_conn.execute("SELECT COUNT(*) FROM case_notes").fetchone()
    assert row[0] == 0
    reasons = [
        e.data.get("reason")
        for e in _trace_events(trace_dir, result.request_id)
        if e.event == "tool_rejected"
    ]
    assert "delegation_revoked" in reasons
    assert "approval_pending" not in reasons and "approval_already_consumed" not in reasons

    # the approval itself was never touched by a run that never reached it
    untouched = approval.get_approval(db_conn, alice, appr.approval_id)
    assert untouched.status.value == "approved"
    assert untouched.consumed_at is None


async def test_reads_never_require_approval(db_conn, trace_dir, alice):
    """get_customer/get_order aren't flagged requires_human_approval — a
    run with no approval_id at all must still be able to read."""
    agent, grant = setup_agent_and_delegation(db_conn, alice)
    runner = AgentRunner(
        agent_id=agent.agent_id,
        delegation_id=grant.delegation_id,
        model=FakeModelProvider(
            script=[
                ModelResponse(tool_call=ToolCallRequest(id="c1", name="get_customer", arguments={"customer_id": "cust_1"})),
                ModelResponse(text="Alice Anders is gold tier."),
            ]
        ),
        db_conn=db_conn,
        trace_dir=trace_dir,
    )
    result = await runner.run(human=alice, goal="who is cust_1?")
    assert result.stopped_reason == "final_response"
    assert result.task_status == "completed"


def _trace_events(trace_dir, request_id):
    import json

    path = trace_dir / f"{request_id}.jsonl"
    events = []
    for line in path.read_text(encoding="utf-8").splitlines():
        raw = json.loads(line)
        events.append(_Event(raw["event"], raw["data"]))
    return events


class _Event:
    def __init__(self, event, data):
        self.event = event
        self.data = data
