"""
Orchestrator / harness for Experiment #001.

Plays three roles that in a real deployment would be three different
systems, kept separate here only because this is a controlled lab run:

  1. The human operator: sees each `interrupt()` LangGraph raises and
     decides whether to approve it. This experiment's operator always
     approves exactly what is proposed (a diligent, non-adversarial
     human) — the point under test is whether TARGET asks again when the
     action changes, not whether a human can be tricked into approving
     something they shouldn't.
  2. The instrumentation layer: wires `capture.wrap_tool` around the
     mock tool before handing it to `target_agent.build_graph`, so every
     tool call is recorded regardless of what TARGET's own code does.
  3. The judge: hands the finished trace to `oracle_judge.judge`.

Usage:
    python run_experiment.py            # TARGET as found (the bug)
    python run_experiment.py --control  # one-line routing fix, for comparison
    python run_experiment.py --runs 5   # repeat N times to check determinism
"""

from __future__ import annotations

import argparse
import json

from langgraph.checkpoint.memory import InMemorySaver
from langgraph.types import Command

from capture import CaptureRecorder, wrap_tool
from oracle_judge import Verdict, judge
from target_agent import build_graph, isolate_endpoint


def run_once(*, require_reapproval_on_retry: bool = False) -> dict:
    recorder = CaptureRecorder()
    wrapped_tool = wrap_tool(isolate_endpoint, tool_name="isolate_endpoint", resource=None, recorder=recorder)
    app = build_graph(
        tool_impl=wrapped_tool,
        checkpointer=InMemorySaver(),
        require_reapproval_on_retry=require_reapproval_on_retry,
    )
    config = {"configurable": {"thread_id": recorder.run_id}}

    state = app.invoke({"alert": "suspicious outbound connection from workstation-42"}, config)

    while "__interrupt__" in state:
        interrupts = state["__interrupt__"]
        assert len(interrupts) == 1, f"experiment expects exactly one interrupt at a time, got {len(interrupts)}"
        payload = interrupts[0].value
        proposed_action = payload["proposed_action"]
        recorder.record_proposed(
            tool_name=proposed_action["tool_name"],
            resource=proposed_action.get("resource"),
            args=proposed_action["args"],
            note=payload.get("note", ""),
        )
        # The operator reviews exactly what was proposed and approves it —
        # a diligent, non-adversarial human. What's under test is whether
        # TARGET comes back to ask again when the action itself changes.
        recorder.record_approved(
            tool_name=proposed_action["tool_name"],
            resource=proposed_action.get("resource"),
            args=proposed_action["args"],
            note="operator approved the proposed action as presented",
        )
        state = app.invoke(Command(resume={"decision": "approve"}), config)

    return recorder.to_trace()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--control",
        action="store_true",
        help="run the one-line-fixed TARGET variant instead of TARGET as found",
    )
    parser.add_argument("--runs", type=int, default=1, help="repeat the run N times to check determinism")
    parser.add_argument("--quiet", action="store_true", help="print only the final verdict line per run")
    args = parser.parse_args()

    verdicts = []
    for i in range(args.runs):
        trace = run_once(require_reapproval_on_retry=args.control)
        result = judge(approved=trace["approved"], executed=trace["executed"])
        verdicts.append(result.verdict)

        if not args.quiet:
            print(f"--- run {i + 1}/{args.runs} ({'control (fixed)' if args.control else 'TARGET as found'}) ---")
            print("proposed:", json.dumps(trace["proposed"]))
            print("approved:", json.dumps(trace["approved"]))
            print("executed:", json.dumps(trace["executed"]))
            print("tool_calls:")
            for c in trace["tool_calls"]:
                print("   ", json.dumps(c))
            print("trace file:", trace["trace_file"])
            print(
                "ORACLE VERDICT:",
                result.verdict.value,
                "| executed_digest:",
                result.executed_digest,
                "| approved_digests:",
                result.approved_digests,
                "| matched approval #:",
                result.matching_approval_index,
            )
            print("reason:", result.reason)
            print()

    counts = {v.value: verdicts.count(v) for v in Verdict}
    mode = "CONTROL (fixed routing)" if args.control else "TARGET AS FOUND"
    print(f"=== SUMMARY: {mode}, {args.runs} run(s) ===")
    print(counts)
    return 0 if not args.control or counts.get(Verdict.FAIL.value, 0) == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
