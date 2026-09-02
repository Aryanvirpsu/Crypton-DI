"""
v0.5.1 — consolidated pre-execution enforcement regression suite.

Every scenario below drives the *real* `AgentRunner.run()` loop
(agent.py's `_handle_tool_call`) end to end, and proves the security
invariant the same way — with a spy on the one tool that has a durable
side effect (`add_case_note` inserts a `case_notes` row) — never by
trusting the error message alone:

    No protected tool side effect may occur unless authorization has
    returned ALLOW and any required approval has succeeded.

This file exists to make that invariant provable in one place, even
though several of these scenarios are already exercised individually
elsewhere (test_agent.py, test_agent_approval.py, test_authorization.py)
— see each test's docstring for the specific denial path it proves.
"""

import time

import pytest

from agentlab import agents, approval, authorization, chain
from agentlab.agent import AgentRunner
from agentlab.delegation import grant_delegation, revoke_delegation
from agentlab.model_provider import FakeModelProvider, ModelResponse, ToolCallRequest
from agentlab.webauthn import FakeWebAuthnVerifier

ALL_TOOLS = frozenset({"get_customer", "get_order", "add_case_note"})
NOTE_ARGS = {"customer_id": "cust_1", "note": "refund approved"}
VERIFIER = FakeWebAuthnVerifier()


def _note_count(db_conn) -> int:
    return db_conn.execute("SELECT COUNT(*) FROM case_notes WHERE customer_id = 'cust_1'").fetchone()[0]


def _add_note_script() -> list[ModelResponse]:
    return [
        ModelResponse(tool_call=ToolCallRequest(id="c1", name="add_case_note", arguments=dict(NOTE_ARGS))),
        ModelResponse(text="Noted."),
    ]


def _setup(db_conn, owner, *, allowed_tools=ALL_TOOLS):
    agent = agents.register_agent(db_conn, owner, name="support-agent", version="0.1.0")
    grant = grant_delegation(
        db_conn,
        owner,
        agent_id=agent.agent_id,
        allowed_tools=allowed_tools,
        resource_constraints={"customer_ids": {"cust_1"}},
        ttl_seconds=3600,
    )
    return agent, grant


async def _run(db_conn, trace_dir, owner, agent, delegation_id, *, approval_id=None):
    runner = AgentRunner(
        agent_id=agent.agent_id,
        delegation_id=delegation_id,
        approval_id=approval_id,
        model=FakeModelProvider(script=_add_note_script()),
        db_conn=db_conn,
        trace_dir=trace_dir,
    )
    return await runner.run(human=owner, goal="add a note for cust_1")


# ── 1. authorization DENY (TOOL_NOT_ALLOWED) → tool not invoked ────────────


async def test_authorization_deny_prevents_execution(db_conn, trace_dir, alice):
    """A delegation that never granted add_case_note denies it deterministically
    (TOOL_NOT_ALLOWED) before the tool function is ever reached."""
    agent, grant = _setup(db_conn, alice, allowed_tools=frozenset({"get_customer", "get_order"}))

    result = await _run(db_conn, trace_dir, alice, agent, grant.delegation_id)

    assert _note_count(db_conn) == 0
    decisions = authorization.get_decisions_for_instance(db_conn, result.instance_id)
    assert any(d.reason_code == authorization.TOOL_NOT_ALLOWED for d in decisions)
    assert not any(d.decision == authorization.DECISION_ALLOW and d.tool_name == "add_case_note" for d in decisions)


# ── 2. authorization error (unexpected exception) → tool not invoked ───────


async def test_authorization_error_prevents_execution(db_conn, trace_dir, alice, monkeypatch):
    """v0.6 update (was: the run crashes with an uncaught RuntimeError —
    see git history for the pre-v0.6 version of this test): boundary 2 now
    calls `chain.verify_delegation_chain`, which wraps the whole
    verification — including its call into `authorization.authorize_tool_call`
    — in a catch-all that converts *any* internal exception into a
    persisted, evidenced DENY (`chain.CHAIN_VERIFICATION_ERROR`) rather
    than letting it propagate as a raw crash. This is a deliberate v0.6
    strengthening, not a regression: DECISIONS.md's invariant 8 ("fail
    closed") and the v0.6 test matrix both require "verifier internal
    error -> DENY/no execution", not "verifier internal error -> crash".
    The security property this test exists to prove is unchanged either
    way and is what's actually asserted below: the tool is structurally
    unreachable when authorization can't produce a real ALLOW."""
    agent, grant = _setup(db_conn, alice)

    def _boom(*args, **kwargs):
        raise RuntimeError("simulated authorization backend failure")

    monkeypatch.setattr(authorization, "authorize_tool_call", _boom)
    # agent.py -> chain.verify_delegation_chain -> authorization.authorize_tool_call,
    # all by module-attribute reference, so patching the module attribute
    # here is still visible through the whole call chain without needing
    # to reload anything.

    result = await _run(db_conn, trace_dir, alice, agent, grant.delegation_id)

    assert _note_count(db_conn) == 0
    decisions = authorization.get_decisions_for_instance(db_conn, result.instance_id)
    assert any(d.reason_code == chain.CHAIN_VERIFICATION_ERROR for d in decisions)


# ── 3. missing delegation → tool not invoked ────────────────────────────────


async def test_missing_delegation_prevents_execution(db_conn, trace_dir, alice):
    """A delegation_id that was never granted (DELEGATION_NOT_FOUND)."""
    agent = agents.register_agent(db_conn, alice, name="support-agent", version="0.1.0")

    result = await _run(db_conn, trace_dir, alice, agent, "dlg_does_not_exist")

    assert _note_count(db_conn) == 0
    decisions = authorization.get_decisions_for_instance(db_conn, result.instance_id)
    assert any(d.reason_code == authorization.DELEGATION_NOT_FOUND for d in decisions)


# ── 4. expired delegation → tool not invoked ────────────────────────────────


async def test_expired_delegation_prevents_execution(db_conn, trace_dir, alice):
    """Forced into the past — no need to sleep out a real TTL, same
    technique test_authorization.py::test_expired_delegation_denied uses at
    the unit level; here it runs through the full AgentRunner loop."""
    agent, grant = _setup(db_conn, alice)
    db_conn.execute(
        "UPDATE delegations SET expires_at = ? WHERE delegation_id = ?", (time.time() - 1, grant.delegation_id)
    )
    db_conn.commit()

    result = await _run(db_conn, trace_dir, alice, agent, grant.delegation_id)

    assert _note_count(db_conn) == 0
    decisions = authorization.get_decisions_for_instance(db_conn, result.instance_id)
    assert any(d.reason_code == authorization.DELEGATION_EXPIRED for d in decisions)


# ── 5. revoked delegation → tool not invoked ────────────────────────────────


async def test_revoked_delegation_prevents_execution(db_conn, trace_dir, alice):
    agent, grant = _setup(db_conn, alice)
    revoke_delegation(db_conn, alice, grant.delegation_id)

    result = await _run(db_conn, trace_dir, alice, agent, grant.delegation_id)

    assert _note_count(db_conn) == 0
    decisions = authorization.get_decisions_for_instance(db_conn, result.instance_id)
    assert any(d.reason_code == authorization.DELEGATION_REVOKED for d in decisions)


# ── 6. required approval missing → tool not invoked ─────────────────────────


async def test_missing_required_approval_prevents_execution(db_conn, trace_dir, alice):
    """No approval_id at all on the runner — same scenario
    test_agent_approval.py::test_missing_approval_denies_and_writes_nothing
    covers; kept here too so this file is a complete, standalone reference
    for every enforcement-boundary denial path."""
    agent, grant = _setup(db_conn, alice)

    result = await _run(db_conn, trace_dir, alice, agent, grant.delegation_id, approval_id=None)

    assert _note_count(db_conn) == 0
    decisions = authorization.get_decisions_for_instance(db_conn, result.instance_id)
    assert any(d.reason_code == approval.APPROVAL_REQUIRED for d in decisions)


# ── 7. approval denied (cancelled) → tool not invoked ───────────────────────


async def test_cancelled_approval_prevents_execution(db_conn, trace_dir, alice):
    """A human can withdraw a request before consuming it (`approval.cancel`).
    A cancelled approval must deny exactly like any other non-approved
    status — it is not silently treated as "no approval required"."""
    agent, grant = _setup(db_conn, alice)
    appr = approval.request_approval(
        db_conn,
        alice,
        agent_id=agent.agent_id,
        delegation_id=grant.delegation_id,
        tool_name="add_case_note",
        resource_type="customer_ids",
        resource_id="cust_1",
        proposed_arguments=NOTE_ARGS,
        ttl_seconds=300,
    )
    approval.cancel(db_conn, alice, appr.approval_id)

    result = await _run(db_conn, trace_dir, alice, agent, grant.delegation_id, approval_id=appr.approval_id)

    assert _note_count(db_conn) == 0
    decisions = authorization.get_decisions_for_instance(db_conn, result.instance_id)
    assert any(d.reason_code == approval.APPROVAL_CANCELLED for d in decisions)


# ── 8. approval verification (WebAuthn) error → tool not invoked ───────────


async def test_webauthn_verification_failure_prevents_execution(db_conn, trace_dir, alice):
    """A failed step-up ceremony must leave the approval exactly as pending
    as it was before the attempt (webauthn.py's documented contract) — and
    a run that then tries to consume that still-pending approval must be
    denied, never fall through to execution."""
    agent, grant = _setup(db_conn, alice)
    appr = approval.request_approval(
        db_conn,
        alice,
        agent_id=agent.agent_id,
        delegation_id=grant.delegation_id,
        tool_name="add_case_note",
        resource_type="customer_ids",
        resource_id="cust_1",
        proposed_arguments=NOTE_ARGS,
        ttl_seconds=300,
    )

    with pytest.raises(Exception):
        approval.approve(
            db_conn,
            alice,
            appr.approval_id,
            verifier=_FailingVerifier(),
        )

    # Approval must still be pending — the failed ceremony changed nothing.
    still_pending = approval.get_approval(db_conn, alice, appr.approval_id)
    assert still_pending.status.value == "pending"

    result = await _run(db_conn, trace_dir, alice, agent, grant.delegation_id, approval_id=appr.approval_id)

    assert _note_count(db_conn) == 0
    decisions = authorization.get_decisions_for_instance(db_conn, result.instance_id)
    assert any(d.reason_code == approval.APPROVAL_PENDING for d in decisions)


class _FailingVerifier(FakeWebAuthnVerifier):
    """Deterministically fails every step-up ceremony, regardless of
    challenge — a stand-in for a real authenticator refusing/erroring."""

    def verify_step_up(self, *, principal, challenge):  # noqa: D401 - interface override
        return super().verify_step_up(principal=principal, challenge=self.FAIL_CHALLENGE)


# ── 9. ALLOW + approval satisfied → executes exactly once ──────────────────


async def test_fully_authorized_call_executes_exactly_once(db_conn, trace_dir, alice):
    """The positive case: valid delegation + valid, consumed approval ->
    the tool runs, and running the same approval_id again is guaranteed to
    see APPROVAL_ALREADY_CONSUMED rather than a second execution (one-shot,
    proven by the row count staying at 1 across two attempts, not just the
    second call's return value)."""
    agent, grant = _setup(db_conn, alice)
    appr = approval.request_approval(
        db_conn,
        alice,
        agent_id=agent.agent_id,
        delegation_id=grant.delegation_id,
        tool_name="add_case_note",
        resource_type="customer_ids",
        resource_id="cust_1",
        proposed_arguments=NOTE_ARGS,
        ttl_seconds=300,
    )
    approved = approval.approve(db_conn, alice, appr.approval_id, verifier=VERIFIER)

    result = await _run(db_conn, trace_dir, alice, agent, grant.delegation_id, approval_id=approved.approval_id)
    assert result.stopped_reason == "final_response"
    assert _note_count(db_conn) == 1

    # Replaying the exact same approval_id in a brand-new run must not add
    # a second row — the approval was already consumed by the first run.
    result2 = await _run(db_conn, trace_dir, alice, agent, grant.delegation_id, approval_id=approved.approval_id)
    assert _note_count(db_conn) == 1
    decisions2 = authorization.get_decisions_for_instance(db_conn, result2.instance_id)
    assert any(d.reason_code == approval.APPROVAL_ALREADY_CONSUMED for d in decisions2)
