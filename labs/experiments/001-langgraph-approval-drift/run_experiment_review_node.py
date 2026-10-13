"""
Orchestrator / harness for Experiment #001c — structurally identical to
`run_experiment.py` (#001) and `run_experiment_idiomatic.py` (#001b),
reusing `capture.py` and `oracle_judge.py` UNCHANGED. Only the target
module and the interrupt payload's key names differ, dictated by
`target_agent_review_node.py`'s doc-cited interrupt shape
(`{"question": ..., "tool_call": {...}}`, resumed with
`{"action": "continue"}` per the source notebook).

Usage:
    python run_experiment_review_node.py            # review-node TARGET
    python run_experiment_review_node.py --runs 5    # repeat N times
"""

from __future__ import annotations

import argparse
import json

from langgraph.checkpoint.memory import InMemorySaver
from langgraph.types import Command

from capture import CaptureRecorder, wrap_tool
from oracle_judge import Verdict, judge
from target_agent_review_node import build_graph, raw_isolate_action


def run_once() -> dict:
    recorder = CaptureRecorder()
    wrapped_tool = wrap_tool(raw_isolate_action, tool_name="isolate_endpoint", resource=None, recorder=recorder)
    app = build_graph(tool_impl=wrapped_tool, checkpointer=InMemorySaver())
    config = {"configurable": {"thread_id": recorder.run_id}}

    state = app.invoke({"messages": []}, config)  # first call_llm turn proposes the narrow action itself

    while "__interrupt__" in state:
        interrupts = state["__interrupt__"]
        assert len(interrupts) == 1, f"experiment expects exactly one interrupt at a time, got {len(interrupts)}"
        payload = interrupts[0].value
        tool_call = payload["tool_call"]
        host = tool_call["args"]["host"]
        recorder.record_proposed(
            tool_name=tool_call["name"], resource=None, args={"host": host}, note=payload.get("question", "")
        )
        # Diligent, non-adversarial operator: approves exactly what was proposed ("continue").
        recorder.record_approved(
            tool_name=tool_call["name"],
            resource=None,
            args={"host": host},
            note="operator approved via 'continue' (as presented, no edits)",
        )
        state = app.invoke(Command(resume={"action": "continue"}), config)

    return recorder.to_trace()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runs", type=int, default=1)
    parser.add_argument("--quiet", action="store_true")
    args = parser.parse_args()

    verdicts = []
    for i in range(args.runs):
        trace = run_once()
        result = judge(approved=trace["approved"], executed=trace["executed"])
        verdicts.append(result.verdict)

        if not args.quiet:
            print(f"--- run {i + 1}/{args.runs} (review-node TARGET, #001c) ---")
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
    print(f"=== SUMMARY: REVIEW-NODE TARGET (#001c), {args.runs} run(s) ===")
    print(counts)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
