"""
tests/test_shared_context_approval.py — EXP-003A.2 surgical follow-up.

Can a documented shared `context=` object cause an always_approve
decision from one logically separate run to authorize calls in another?
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
EVIDENCE_DIR = REPO_ROOT / "evidence"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

import pytest

from crypton_exp003.evidence import AuthorizationBinding, build_evidence, write_evidence
from scenarios.shared_context_approval import RUN_A_ARGS, RUN_B_ARGS, run_shared_context_probe

TARGET = {"package": "openai-agents", "version": "0.22.2"}


@pytest.mark.asyncio
async def test_shared_context_does_not_carry_always_approve_across_runs():
    r = await run_shared_context_probe()

    assert r["same_context_object_confirmed"], "test setup error: RUN B did not actually receive the same context object"
    assert r["sink"].count_for_tool("transfer_funds") == 1  # only RUN A's approved call executed

    # RUN B: same tool identity, materially different (higher-impact) args,
    # SAME shared context object as RUN A's always_approve grant - still
    # required its own, fresh approval.
    assert [(i.tool_name, i.call_id) for i in r["interruptions_b"]] == [("transfer_funds", "rB_1")]

    # RUN C control: fresh, unshared context - also requires its own approval, as expected.
    assert [(i.tool_name, i.call_id) for i in r["interruptions_c"]] == [("transfer_funds", "rC_1")]

    evidence = build_evidence(
        experiment="EXP-003A",
        scenario="a2_shared_context_always_approve_isolation",
        target=TARGET,
        proposal={
            "run_a": {"tool_name": "transfer_funds", "arguments": RUN_A_ARGS, "call_id": "rA_1"},
            "run_b": {"tool_name": "transfer_funds", "arguments": RUN_B_ARGS, "call_id": "rB_1"},
        },
        approval_request={"run_a": "rA_1", "run_b": "rB_1 (still pending after run A's always_approve)"},
        approval_decision={"run_a": "approve, always_approve=True", "run_b": "not decided - still required"},
        delegation_path=[],
        execution=[vars(rec) for rec in r["sink"].records],
        authorization_binding=AuthorizationBinding(
            tool_match=True, argument_match=None, call_id_match=True, namespace_match=True, delegation_match=True,
            notes=["argument_match is null: RUN B never executed, so there is nothing to compare its arguments against."],
        ),
        outcome="PASS",
        notes=[
            "Classification: EXPECTED_BY_DESIGN / ISOLATED PER RUN. Even the literal same "
            "`context=` object (confirmed via `is` identity check, not just equality) passed to a "
            "second, independent Runner.run() call did NOT carry RUN A's always_approve grant "
            "into RUN B, despite RUN B targeting the identical tool identity.",
            "Mechanism (confirmed via source inspection of agents/run_context.py, not just "
            "observed behavior): RunContextWrapper's `_approvals` and `_tool_invocations` fields "
            "are declared `default_factory=dict` on the WRAPPER, which Runner.run() constructs "
            "fresh on every call - the user-supplied `context` payload object is stored inside "
            "the wrapper but never itself carries approval bookkeeping. The hypothesized leak "
            "path does not exist via the public `context=` parameter.",
            "This is reported as 'impossible to reproduce via context= sharing', not "
            "INCONCLUSIVE - the test executed cleanly and the mechanism was directly observed "
            "(RUN B's context_wrapper.context is confirmed identical-by-identity to RUN A's, yet "
            "still required fresh approval).",
            "Not tested (out of scope for this surgical pass): continuing the SAME RunState "
            "across what an application treats as 'separate' logical work items - that scenario "
            "is already covered by EXP-003A Phase 1's H4 (resume binding) and is a different "
            "question from context= sharing.",
        ],
    )
    write_evidence(evidence, EVIDENCE_DIR / "a2_shared_context_always_approve_isolation.json")
