"""
tests/test_exp003c.py — EXP-003C, all baselines and hypotheses, LangGraph.

One consolidated file (unlike EXP-003A/B's per-hypothesis files) given
this lab's narrower per-test complexity - each test here is a handful of
lines of plain graph-node Python, not a multi-step scripted-model dance.
"""

from __future__ import annotations

import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
EVIDENCE_DIR = REPO_ROOT / "evidence"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

import pytest
from langgraph.checkpoint.memory import InMemorySaver
from langgraph.checkpoint.sqlite import SqliteSaver
from langgraph.types import Command

from crypton_exp003c.core import EXECUTED, AuthorizationBinding, build_evidence, reset_executed, write_evidence
from scenarios.graphs import (
    POLICY,
    build_delegation_graph,
    build_policy_checked_graph,
    build_single_tool_graph,
    build_single_tool_graph_resume_sourced,
    reset_policy,
)

TARGET = {"package": "langgraph", "version": "1.2.11"}
ARGS = {"account": "checking", "amount": 100, "destination": "A"}


def _cfg(thread_id: str) -> dict:
    return {"configurable": {"thread_id": thread_id}}


# ---------------------------------------------------------------- Baselines


def test_baseline_a_approve():
    reset_executed()
    cp = InMemorySaver()
    g = build_single_tool_graph("baseline_a", checkpointer=cp)
    cfg = _cfg("ba")
    r1 = g.invoke({"tool_name": "transfer_funds", "arguments": ARGS}, config=cfg)
    assert "__interrupt__" in r1
    assert EXECUTED == []  # no execution before approval
    interrupt_payload = r1["__interrupt__"][0].value
    assert interrupt_payload == {"tool": "transfer_funds", "arguments": ARGS}

    r2 = g.invoke(Command(resume={"approved": True}), config=cfg)
    assert r2["result"] == "executed"
    assert len(EXECUTED) == 1
    assert EXECUTED[0]["arguments"] == ARGS

    evidence = build_evidence(
        experiment="EXP-003C", scenario="baseline_a_approve", target=TARGET,
        proposal={"tool_name": "transfer_funds", "arguments": ARGS},
        approval_request={"interrupt_id": r1["__interrupt__"][0].id, "value": interrupt_payload},
        approval_decision={"resume": {"approved": True}},
        delegation_path=[], state_boundary={"thread_id": "ba", "checkpointer": "InMemorySaver"},
        execution=EXECUTED,
        authorization_binding=AuthorizationBinding(tool_match=True, argument_match=True, interrupt_id_match=True, thread_match=True, delegation_match=True),
        outcome="PASS",
        notes=["No execution before approval; exactly one execution; executed arguments == proposed arguments (state-sourced execution pattern)."],
    )
    write_evidence(evidence, EVIDENCE_DIR / "baseline_a_approve.json")


def test_baseline_b_reject():
    reset_executed()
    cp = InMemorySaver()
    g = build_single_tool_graph("baseline_b", checkpointer=cp)
    cfg = _cfg("bb")
    r1 = g.invoke({"tool_name": "transfer_funds", "arguments": ARGS}, config=cfg)
    r2 = g.invoke(Command(resume={"approved": False}), config=cfg)
    assert r2["result"] == "rejected"
    assert EXECUTED == []

    evidence = build_evidence(
        experiment="EXP-003C", scenario="baseline_b_reject", target=TARGET,
        proposal={"tool_name": "transfer_funds", "arguments": ARGS},
        approval_request={"interrupt_id": r1["__interrupt__"][0].id},
        approval_decision={"resume": {"approved": False}},
        delegation_path=[], state_boundary={"thread_id": "bb", "checkpointer": "InMemorySaver"},
        execution=[],
        authorization_binding=AuthorizationBinding(tool_match=True, argument_match=None, interrupt_id_match=True, thread_match=True, delegation_match=True),
        outcome="PASS", notes=["Rejection produces zero side effects."],
    )
    write_evidence(evidence, EVIDENCE_DIR / "baseline_b_reject.json")


def test_baseline_c_persist_reload_resume_sqlite():
    """Genuine persistence: SqliteSaver backed by a real temp file,
    reloaded via a FRESH compiled graph + FRESH SqliteSaver connection
    (not the original Python objects) - the LangGraph analogue of
    EXP-003B's FileCheckpointStorage cross-instance test."""
    reset_executed()
    tmpdir = tempfile.mkdtemp()
    db_path = str(Path(tmpdir) / "checkpoints.sqlite")
    cfg = _cfg("bc")

    with SqliteSaver.from_conn_string(db_path) as cp1:
        g1 = build_single_tool_graph("baseline_c", checkpointer=cp1)
        r1 = g1.invoke({"tool_name": "transfer_funds", "arguments": ARGS}, config=cfg)
        assert "__interrupt__" in r1
        assert EXECUTED == []

    # Fresh connection, fresh compiled graph - simulates reload after restart.
    with SqliteSaver.from_conn_string(db_path) as cp2:
        g2 = build_single_tool_graph("baseline_c", checkpointer=cp2)
        r2 = g2.invoke(Command(resume={"approved": True}), config=cfg)
        assert r2["result"] == "executed"

    assert len(EXECUTED) == 1
    assert EXECUTED[0]["arguments"] == ARGS

    evidence = build_evidence(
        experiment="EXP-003C", scenario="baseline_c_persist_reload_resume", target=TARGET,
        proposal={"tool_name": "transfer_funds", "arguments": ARGS},
        approval_request={"note": "pending interrupt persisted to a real SQLite file"},
        approval_decision={"resume": {"approved": True}, "via": "fresh SqliteSaver connection + fresh compiled graph, not the originals"},
        delegation_path=[], state_boundary={"thread_id": "bc", "checkpointer": "SqliteSaver (real file)"},
        execution=EXECUTED,
        authorization_binding=AuthorizationBinding(tool_match=True, argument_match=True, interrupt_id_match=True, thread_match=True, delegation_match=True),
        outcome="PASS",
        notes=["Same logical request/tool/arguments, exactly-once execution, confirmed across a genuine persist-to-disk -> fresh-connection-reload -> resume cycle."],
    )
    write_evidence(evidence, EVIDENCE_DIR / "baseline_c_persist_reload_resume.json")


# ---------------------------------------------------------------- H1


def test_h1a_state_sourced_execution_is_immune_to_resume_payload_drift():
    """The safe/realistic pattern: execution reads arguments from
    checkpointed STATE, never from the resume payload. A resume payload
    that lies about different arguments has no effect."""
    reset_executed()
    cp = InMemorySaver()
    g = build_single_tool_graph("h1a", checkpointer=cp)
    cfg = _cfg("h1a")
    g.invoke({"tool_name": "transfer_funds", "arguments": ARGS}, config=cfg)
    drifted = {"account": "checking", "amount": 10000, "destination": "Z"}
    # A malicious/buggy caller supplies drifted arguments INSIDE the resume payload.
    r2 = g.invoke(Command(resume={"approved": True, "arguments": drifted}), config=cfg)
    assert r2["result"] == "executed"
    assert EXECUTED[0]["arguments"] == ARGS  # NOT the drifted ones
    assert EXECUTED[0]["arguments"] != drifted


def test_h1b_resume_sourced_execution_is_not_protected_by_the_framework():
    """The naive/unsafe pattern, built deliberately to characterize what
    the FRAMEWORK does (nothing) vs. what the APPLICATION must do. This
    is not a framework defect - interrupt()'s resume value has no
    documented binding guarantee; it is a generic data channel."""
    reset_executed()
    cp = InMemorySaver()
    g = build_single_tool_graph_resume_sourced("h1b", checkpointer=cp)
    cfg = _cfg("h1b")
    g.invoke({"tool_name": "transfer_funds", "arguments": ARGS}, config=cfg)
    drifted = {"account": "checking", "amount": 10000, "destination": "Z"}
    r2 = g.invoke(Command(resume={"approved": True, "arguments": drifted}), config=cfg)
    assert r2["result"] == "executed"
    assert EXECUTED[0]["arguments"] == drifted  # confirms: framework does not stop this


def test_h1c_update_state_public_api_can_mutate_pending_state_before_resume():
    """Realistic, public-API-only mutation vector: langgraph's documented
    update_state() call, invoked between the interrupt and the resume."""
    reset_executed()
    cp = InMemorySaver()
    g = build_single_tool_graph("h1c", checkpointer=cp)
    cfg = _cfg("h1c")
    g.invoke({"tool_name": "transfer_funds", "arguments": ARGS}, config=cfg)

    drifted = {"account": "checking", "amount": 10000, "destination": "Z"}
    g.update_state(cfg, {"arguments": drifted})  # public API, no private access

    r2 = g.invoke(Command(resume={"approved": True}), config=cfg)
    assert r2["result"] == "executed"
    # This IS the state-sourced pattern (test_h1a) - but update_state()
    # mutated the very state that pattern trusts.
    assert EXECUTED[0]["arguments"] == drifted, (
        "APPROVAL_BINDING via update_state(): the state-sourced pattern's safety "
        "assumption (checkpointed state is trustworthy) can be defeated by a "
        "second, legitimate caller invoking the public update_state() API "
        "between interrupt and resume."
    )

    evidence = build_evidence(
        experiment="EXP-003C", scenario="h1c_update_state_argument_drift", target=TARGET,
        proposal={"tool_name": "transfer_funds", "arguments": ARGS},
        approval_request={"note": "human presented with ARGS at interrupt time"},
        approval_decision={"resume": {"approved": True}, "note": "state mutated to drifted args via update_state() AFTER the human saw ARGS, BEFORE resume"},
        delegation_path=[], state_boundary={"thread_id": "h1c", "checkpointer": "InMemorySaver"},
        execution=[EXECUTED[0]],
        authorization_binding=AuthorizationBinding(
            tool_match=True, argument_match=False, interrupt_id_match=True, thread_match=True, delegation_match=True,
            notes=["argument_match=False: executed arguments differ from what was presented at interrupt time."],
        ),
        outcome="CANDIDATE_FINDING_APPLICATION_LEVEL",
        notes=[
            "This is NOT a langgraph defect: update_state() is public, documented API with no "
            "claimed 'freeze pending interrupts' semantics. It demonstrates that the state-sourced "
            "execution pattern (the SAFE pattern recommended by test_h1a/H1a) is only as safe as "
            "the application's own access control around who may call update_state() on a pending "
            "thread. A real deployment must treat update_state() as part of its trust boundary. "
            "Classified as an application-design consideration, not a CANDIDATE SECURITY ISSUE "
            "against langgraph itself - see README H1 discussion.",
        ],
    )
    write_evidence(evidence, EVIDENCE_DIR / "h1c_update_state_argument_drift.json")


# ---------------------------------------------------------------- H2


def test_h2_delegation_approval_does_not_satisfy_child_approval():
    reset_executed()
    cp = InMemorySaver()
    g = build_delegation_graph("h2", checkpointer=cp)
    cfg = _cfg("h2")

    r1 = g.invoke({"arguments": {"customer": "alice", "amount": 50}}, config=cfg)
    assert r1["__interrupt__"][0].value["level"] == "parent"
    assert EXECUTED == []

    r2 = g.invoke(Command(resume={"approved": True}), config=cfg)
    # Approving ONLY the parent delegation must NOT execute the child tool -
    # it must surface the child's OWN, independent interrupt.
    assert "__interrupt__" in r2, "child tool executed without its own approval"
    assert r2["__interrupt__"][0].value["level"] == "child"
    assert EXECUTED == []
    parent_interrupt_id = r1["__interrupt__"][0].id
    child_interrupt_id = r2["__interrupt__"][0].id
    assert parent_interrupt_id != child_interrupt_id

    r3 = g.invoke(Command(resume={"approved": True}), config=cfg)
    assert r3["result"] == "child_executed"
    assert len(EXECUTED) == 1

    evidence = build_evidence(
        experiment="EXP-003C", scenario="h2_delegation_vs_child_tool_boundary", target=TARGET,
        proposal={"delegation": "delegate_to_refund_agent", "child_tool": "issue_refund"},
        approval_request={"parent_interrupt_id": parent_interrupt_id, "child_interrupt_id": child_interrupt_id},
        approval_decision={"round_1": "approve parent delegation only", "round_2": "approve child tool separately"},
        delegation_path=[{"node": "parent", "invokes_subgraph": "child"}],
        state_boundary={"thread_id": "h2", "checkpointer": "InMemorySaver"},
        execution=EXECUTED,
        authorization_binding=AuthorizationBinding(tool_match=True, argument_match=True, interrupt_id_match=True, thread_match=True, delegation_match=True),
        outcome="PASS",
        notes=["Approving the parent delegation's interrupt did not execute or auto-approve the child subgraph's own, independently-interrupted sensitive tool - two distinct interrupt ids, two distinct decisions required."],
    )
    write_evidence(evidence, EVIDENCE_DIR / "h2_delegation_vs_child_tool_boundary.json")


# ---------------------------------------------------------------- H3 (standing approval - NOT_APPLICABLE at framework level)


def test_h3_not_applicable_no_standing_approval_primitive_exists():
    """H3 asks: does a 'standing'/'always approve' authorization exist,
    and if so, is it scoped narrowly (tool+arguments) or broadly
    (tool-only, argument-blind)? Unlike openai-agents' `always_approve`
    flag on `RunState.approve()` and agent-framework's `ToolApprovalRule`
    (explicit broad-vs-narrow two-tier design), neither raw
    `langgraph.types.interrupt()` nor `langchain`'s
    `HumanInTheLoopMiddleware`/`InterruptOnConfig` provide ANY built-in
    standing-approval concept to test the scope of - so H3 is
    NOT_APPLICABLE at the framework level, not merely untested.

    This is verified here by direct source introspection of the
    INSTALLED package (not recalled from memory or an assumption baked
    into a prior lab), so the assertions fail loudly if a future
    langchain release adds such a field."""
    from langchain.agents.middleware.human_in_the_loop import HumanInTheLoopMiddleware, InterruptOnConfig

    fields = set(InterruptOnConfig.__annotations__.keys())
    assert fields == {"allowed_decisions", "description", "args_schema", "when"}, (
        f"InterruptOnConfig's field set changed to {fields} - re-evaluate H3, a standing-approval "
        f"field may now exist."
    )

    import inspect

    middleware_src = inspect.getsource(HumanInTheLoopMiddleware).lower()
    for keyword in ("always", "standing", "remember", "cache", "memo"):
        assert keyword not in middleware_src, (
            f"found '{keyword}' in HumanInTheLoopMiddleware source - re-evaluate H3, a standing-approval "
            f"mechanism may now exist."
        )

    evidence = build_evidence(
        experiment="EXP-003C", scenario="h3_no_standing_approval_primitive", target=TARGET,
        proposal=None, approval_request=None, approval_decision=None,
        delegation_path=[], state_boundary={"note": "no runtime execution - this is a static source-introspection check"},
        execution=[],
        authorization_binding=AuthorizationBinding(tool_match=None, argument_match=None, interrupt_id_match=None, thread_match=None, delegation_match=None),
        outcome="NOT_APPLICABLE",
        notes=[
            "InterruptOnConfig (the config shape passed to HumanInTheLoopMiddleware) has exactly four "
            "fields - allowed_decisions, description, args_schema, when - none of which express 'approve "
            "this tool/argument-pattern for all future calls'. `when` is a per-call predicate the "
            "application can write statefully (e.g. closing over a set of already-approved args) to "
            "APPROXIMATE standing approval, but that is an application-built pattern, not a framework "
            "primitive comparable to openai-agents' always_approve or agent-framework's ToolApprovalRule. "
            "Raw langgraph.types.interrupt() has no approval concept at all - it is a generic pause/resume "
            "primitive. H3 is therefore NOT_APPLICABLE at the langgraph/langchain layer as tested, distinct "
            "from a PASS (property held) or a FAIL (property violated) - there is no standing-approval "
            "surface here to characterize the scope of.",
        ],
    )
    write_evidence(evidence, EVIDENCE_DIR / "h3_no_standing_approval_primitive.json")


# ---------------------------------------------------------------- H4 (replay/occurrence, beyond baseline C)


def test_h4_resuming_an_already_completed_thread_does_not_reexecute():
    reset_executed()
    cp = InMemorySaver()
    g = build_single_tool_graph("h4", checkpointer=cp)
    cfg = _cfg("h4")
    g.invoke({"tool_name": "transfer_funds", "arguments": ARGS}, config=cfg)
    g.invoke(Command(resume={"approved": True}), config=cfg)
    assert len(EXECUTED) == 1

    # Resume again on the same, now-completed thread.
    r3 = g.invoke(Command(resume={"approved": True}), config=cfg)
    assert len(EXECUTED) == 1, "resuming a completed thread re-executed the tool"

    evidence = build_evidence(
        experiment="EXP-003C", scenario="h4_repeated_resume_of_completed_thread", target=TARGET,
        proposal={"tool_name": "transfer_funds", "arguments": ARGS},
        approval_request={"note": "one interrupt, approved once"},
        approval_decision={"first": "approve", "second": "same resume value, re-delivered against the same, already-completed thread_id"},
        delegation_path=[], state_boundary={"thread_id": "h4", "checkpointer": "InMemorySaver"},
        execution=EXECUTED,
        authorization_binding=AuthorizationBinding(tool_match=True, argument_match=True, interrupt_id_match=True, thread_match=True, delegation_match=True),
        outcome="PASS",
        notes=["Resuming an already-completed thread a second time did not re-execute the tool - exactly-once held."],
    )
    write_evidence(evidence, EVIDENCE_DIR / "h4_repeated_resume_of_completed_thread.json")


# ---------------------------------------------------------------- H5 (policy drift, application-built)


def test_h5_application_built_policy_recheck_blocks_stale_authorization():
    reset_policy(1000)
    reset_executed()
    cp = InMemorySaver()
    g = build_policy_checked_graph("h5", checkpointer=cp)
    cfg = _cfg("h5")
    g.invoke({"tool_name": "transfer_funds", "arguments": ARGS}, config=cfg)  # amount=100, limit=1000 at proposal time

    POLICY["limit"] = 50  # policy changes AFTER approval decision is made, BEFORE resume completes
    r2 = g.invoke(Command(resume={"approved": True}), config=cfg)

    assert r2["result"].startswith("blocked_by_policy")
    assert EXECUTED == []
    reset_policy(1000)

    evidence = build_evidence(
        experiment="EXP-003C", scenario="h5_policy_recheck_toctou", target=TARGET,
        proposal={"tool_name": "transfer_funds", "arguments": ARGS, "policy_limit_at_proposal": 1000},
        approval_request={"note": "approved while limit=1000"},
        approval_decision={"resume": {"approved": True}, "note": "limit changed to 50 before resume completed"},
        delegation_path=[], state_boundary={"thread_id": "h5", "checkpointer": "InMemorySaver"},
        execution=[],
        authorization_binding=AuthorizationBinding(tool_match=True, argument_match=None, interrupt_id_match=True, thread_match=True, delegation_match=True),
        outcome="PASS",
        notes=[
            "LangGraph provides NO built-in guardrail/policy-recheck primitive (unlike openai-agents' "
            "tool_input_guardrails or agent-framework's ToolInputGuardrail) - this demonstrates the "
            "pattern IS constructible entirely in application code (a plain re-check after interrupt() "
            "returns, before the side effect), not that the framework provides it automatically.",
        ],
    )
    write_evidence(evidence, EVIDENCE_DIR / "h5_policy_recheck_toctou.json")


# ---------------------------------------------------------------- H6 (identity/thread binding)


def test_h6a_resume_against_a_thread_with_its_own_distinct_pending_interrupt():
    """The realistic cross-thread scenario: thread B has ITS OWN pending
    interrupt (its own proposal, its own arguments). A resume delivered
    against thread B's config can only ever answer thread B's own
    interrupt() call - there is no mechanism by which thread A's identity,
    arguments, or interrupt content could leak into thread B's execution,
    because interrupt() always reads back into the state of the thread
    it's called on."""
    reset_executed()
    cp = InMemorySaver()
    g = build_single_tool_graph("h6a", checkpointer=cp)
    cfg_a = _cfg("h6a_thread_a")
    cfg_b = _cfg("h6a_thread_b")

    args_b = {"account": "savings", "amount": 5, "destination": "B-only"}
    r1a = g.invoke({"tool_name": "transfer_funds", "arguments": ARGS}, config=cfg_a)
    r1b = g.invoke({"tool_name": "transfer_funds", "arguments": args_b}, config=cfg_b)
    assert r1a["__interrupt__"][0].id != r1b["__interrupt__"][0].id

    # Approve thread B only. Thread A must remain untouched and unexecuted.
    r2b = g.invoke(Command(resume={"approved": True}), config=cfg_b)
    assert r2b["result"] == "executed"
    assert len(EXECUTED) == 1
    assert EXECUTED[0]["arguments"] == args_b  # thread B's own arguments, not thread A's

    state_a = g.get_state(cfg_a)
    assert state_a.next == ("propose",), "thread A's interrupt was consumed/advanced by thread B's resume"

    evidence = build_evidence(
        experiment="EXP-003C", scenario="h6a_cross_thread_resume_distinct_pending", target=TARGET,
        proposal={"thread_a": {"tool_name": "transfer_funds", "arguments": ARGS}, "thread_b": {"tool_name": "transfer_funds", "arguments": args_b}},
        approval_request={"thread_a_interrupt_id": r1a["__interrupt__"][0].id, "thread_b_interrupt_id": r1b["__interrupt__"][0].id},
        approval_decision={"resume": {"approved": True}, "submitted_against": "thread_id=h6a_thread_b only"},
        delegation_path=[], state_boundary={"thread_id": "h6a_thread_b", "checkpointer": "InMemorySaver"},
        execution=EXECUTED,
        authorization_binding=AuthorizationBinding(tool_match=True, argument_match=True, interrupt_id_match=True, thread_match=True, delegation_match=True),
        outcome="PASS",
        notes=["A resume delivered to thread B's config only ever answers thread B's own interrupt() call with thread B's own checkpointed state; thread A's independent pending interrupt was left untouched (still next=('propose',))."],
    )
    write_evidence(evidence, EVIDENCE_DIR / "h6a_cross_thread_resume_distinct_pending.json")


def test_h6b_resume_against_a_thread_id_with_no_checkpoint_at_all():
    """A different, non-adversarial edge case: resuming a thread_id that
    was NEVER invoked before (no checkpoint exists at all). This is not a
    cross-thread bypass attempt - it characterizes what happens when a
    caller supplies a resume Command against a bare/unknown thread_id.

    Observed (not assumed): langgraph does NOT silently no-op. Because no
    checkpoint exists, the graph runs from an EMPTY state at the entry
    node. Our node function unconditionally reads state["tool_name"]/
    state["arguments"], so it raises KeyError - an application-level bug
    in this test's own node implementation (it assumes those keys always
    exist), not a langgraph authorization bypass. No side effect occurs
    either way. Recorded as a real, verified behavior rather than forced
    to match the test author's original (incorrect) assumption of a
    graceful no-op."""
    reset_executed()
    cp = InMemorySaver()
    g = build_single_tool_graph("h6b", checkpointer=cp)
    cfg_unknown = _cfg("h6b_thread_never_used")

    raised = None
    try:
        g.invoke(Command(resume={"approved": True}), config=cfg_unknown)
    except KeyError as exc:
        raised = exc

    assert raised is not None, "expected KeyError from executing the entry node against empty state"
    assert EXECUTED == []

    evidence = build_evidence(
        experiment="EXP-003C", scenario="h6b_resume_unknown_thread_no_checkpoint", target=TARGET,
        proposal={"note": "no proposal was ever made on this thread_id"},
        approval_request={"note": "no pending interrupt exists - thread_id was never invoked"},
        approval_decision={"resume": {"approved": True}, "submitted_against": "thread_id=h6b_thread_never_used (no checkpoint)"},
        delegation_path=[], state_boundary={"thread_id": "h6b_thread_never_used", "checkpointer": "InMemorySaver"},
        execution=[],
        authorization_binding=AuthorizationBinding(tool_match=None, argument_match=None, interrupt_id_match=False, thread_match=False, delegation_match=None,
                                                     notes=[f"raised {type(raised).__name__}: {raised!r}"]),
        outcome="PASS_WITH_CAVEAT",
        notes=[
            "No execution occurred (no security bypass). However, the actual observed behavior differs from "
            "the naive expectation of a silent no-op: langgraph runs the entry node against an EMPTY state "
            "when resuming a thread_id with no checkpoint, so any node function that unconditionally reads "
            "expected state keys will raise. This is an APPLICATION robustness concern (nodes should guard "
            "against missing/partial state), not a langgraph authorization defect - langgraph never fabricated "
            "or reused another thread's state to satisfy the empty one.",
        ],
    )
    write_evidence(evidence, EVIDENCE_DIR / "h6b_resume_unknown_thread_no_checkpoint.json")
