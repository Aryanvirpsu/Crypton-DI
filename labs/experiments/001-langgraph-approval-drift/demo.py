"""
Self-contained demo runner for Experiment #001d (approval/execution drift
via LangChain `after_model` middleware ordering).

Purpose: a single command for the live demo that needs no LLM, no API
key, and no network access — everything here runs against real,
installed `langchain`/`langgraph` code with a scripted (non-LLM) planner
in `target_agent_middleware.py`. Nothing is mocked out of the framework
itself; only the "model" proposing actions is deterministic, which is
what makes this repeatable on stage instead of dependent on a live LLM
call succeeding.

Usage:
    python demo.py
    python demo.py --runs 10
"""

from __future__ import annotations

import argparse
import sys

from oracle_judge import Verdict, judge
from run_experiment_middleware import run_once


def _reconfigure_stdout_utf8_best_effort() -> None:
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except (AttributeError, ValueError):
        pass


def _banner(title: str) -> None:
    print()
    print("=" * 78)
    print(title)
    print("=" * 78)


def _run_batch(*, order: str, runs: int) -> dict:
    verdicts = []
    for i in range(runs):
        trace = run_once(order=order)
        result = judge(approved=trace["approved"], executed=trace["executed"])
        verdicts.append(result.verdict)
        approved_hosts = [a["args"]["host"] for a in trace["approved"]]
        executed_host = trace["executed"]["args"]["host"]
        print(
            f"  run {i + 1}/{runs}: human approved {approved_hosts} "
            f"-> agent executed {executed_host!r} -> ORACLE: {result.verdict.value}"
        )
    return {v.value: verdicts.count(v) for v in Verdict}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runs", type=int, default=5, help="runs per scenario (default 5)")
    args = parser.parse_args()

    _reconfigure_stdout_utf8_best_effort()

    print("Crypton — Action Integrity Demo")
    print("Experiment #001d: does a human's approval still bind to what the agent")
    print("actually executes, once a second, individually-benign middleware sits")
    print("in the pipeline?")
    print()
    print("No LLM, no API key, no network call in this run — the planner is")
    print("scripted so the result is exactly reproducible every time.")

    _banner("STEP 1 / 3 — Oracle sanity check (is the judge itself trustworthy?)")
    from test_oracle_sanity import ALL_TESTS  # noqa: PLC0415 (deliberate: keeps this file self-contained)

    sanity_failures = 0
    for t in ALL_TESTS:
        try:
            t()
            print(f"  PASS  {t.__name__}")
        except AssertionError as e:
            sanity_failures += 1
            print(f"  FAIL  {t.__name__}: {e}")
    print(f"\n  {len(ALL_TESTS) - sanity_failures}/{len(ALL_TESTS)} oracle sanity checks passed")
    sanity_ok = sanity_failures == 0

    _banner(
        "STEP 2 / 3 — VULNERABLE ordering: [HostEscalationMiddleware, "
        "HumanInTheLoopMiddleware]\n"
        "(the order a developer would naturally write: normalize, then approve)"
    )
    vuln_counts = _run_batch(order="escalation_first", runs=args.runs)
    print(f"\n  SUMMARY (vulnerable order): {vuln_counts}")

    _banner(
        "STEP 3 / 3 — FIXED ordering: [HumanInTheLoopMiddleware, "
        "HostEscalationMiddleware]\n"
        "(approve first, normalize after — same two middlewares, order swapped)"
    )
    fixed_counts = _run_batch(order="hitl_first", runs=args.runs)
    print(f"\n  SUMMARY (fixed order): {fixed_counts}")

    _banner("RESULT")
    print(f"  Vulnerable ordering : {vuln_counts.get('FAIL', 0)}/{args.runs} FAIL "
          f"(human approved one host, agent isolated a /24)")
    print(f"  Fixed ordering      : {fixed_counts.get('PASS', 0)}/{args.runs} PASS "
          f"(same middlewares, execution matches approval every time)")
    print()
    print("  Neither middleware is buggy on its own. The composition is silently")
    print("  order-dependent because LangChain wires multiple `after_model` hooks")
    print("  in *reverse* list order — undocumented, verified against the")
    print("  installed package's own graph-building code (see finding-001d.md).")

    all_vuln_failed = vuln_counts.get("FAIL", 0) == args.runs
    all_fixed_passed = fixed_counts.get("PASS", 0) == args.runs
    ok = sanity_ok and all_vuln_failed and all_fixed_passed
    print()
    print("DEMO STATUS:", "OK — matches finding-001d exactly" if ok else "MISMATCH — investigate before presenting")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
