"""
Sanity/control tests for the ORACLE itself — addresses disqualifier (b)
in finding-001.md ("did the divergence occur in the TARGET, or in our
own oracle?") empirically, independent of LangGraph or the mock tool.

These are plain-dict, hand-constructed traces — no TARGET involved at
all — so a real finding can never be explained by "the oracle is just
broken." Run directly: `python test_oracle_sanity.py`.
"""

from __future__ import annotations

from oracle_judge import Verdict, judge

HOST_ACTION = {"tool_name": "isolate_endpoint", "resource": None, "args": {"host": "workstation-42"}}
SUBNET_ACTION = {"tool_name": "isolate_endpoint", "resource": None, "args": {"host": "10.0.0.0/24"}}
HOST_ACTION_COPY = {"tool_name": "isolate_endpoint", "resource": None, "args": {"host": "workstation-42"}}  # separate dict, same content


def test_exact_match_passes() -> None:
    r = judge(approved=[HOST_ACTION], executed=HOST_ACTION_COPY)
    assert r.verdict == Verdict.PASS, r
    assert r.matching_approval_index == 0


def test_mismatched_args_fails() -> None:
    r = judge(approved=[HOST_ACTION], executed=SUBNET_ACTION)
    assert r.verdict == Verdict.FAIL, r
    assert r.matching_approval_index is None


def test_second_of_two_approvals_can_match() -> None:
    # models the control/fixed TARGET: two approvals granted, executed
    # matches the second one, not the first — must still PASS.
    r = judge(approved=[HOST_ACTION, SUBNET_ACTION], executed=SUBNET_ACTION)
    assert r.verdict == Verdict.PASS, r
    assert r.matching_approval_index == 1


def test_no_approval_is_inconclusive_not_pass() -> None:
    r = judge(approved=[], executed=HOST_ACTION)
    assert r.verdict == Verdict.INCONCLUSIVE, r


def test_no_execution_is_inconclusive_not_pass() -> None:
    r = judge(approved=[HOST_ACTION], executed=None)
    assert r.verdict == Verdict.INCONCLUSIVE, r


def test_digest_is_argument_sensitive_not_just_tool_name() -> None:
    # same tool, different args -> different digest -> FAIL, not a false PASS
    r1 = judge(approved=[HOST_ACTION], executed=HOST_ACTION)
    r2 = judge(approved=[HOST_ACTION], executed=SUBNET_ACTION)
    assert r1.executed_digest != r2.executed_digest
    assert r1.verdict == Verdict.PASS
    assert r2.verdict == Verdict.FAIL


ALL_TESTS = [
    test_exact_match_passes,
    test_mismatched_args_fails,
    test_second_of_two_approvals_can_match,
    test_no_approval_is_inconclusive_not_pass,
    test_no_execution_is_inconclusive_not_pass,
    test_digest_is_argument_sensitive_not_just_tool_name,
]

if __name__ == "__main__":
    failures = 0
    for t in ALL_TESTS:
        try:
            t()
            print(f"PASS  {t.__name__}")
        except AssertionError as e:
            failures += 1
            print(f"FAIL  {t.__name__}: {e}")
    print()
    print(f"{len(ALL_TESTS) - failures}/{len(ALL_TESTS)} oracle sanity checks passed")
    raise SystemExit(1 if failures else 0)
