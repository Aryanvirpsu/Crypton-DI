"""
scenarios/graphs.py — the graph constructions used across EXP-003C.

No LLM/chat model anywhere: LangGraph's authorization-relevant primitive
(interrupt()/Command(resume=)/checkpointer) is fully exercised by plain,
deterministic Python node functions. This is itself a Phase-0 finding -
see README.
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any, TypedDict

REPO_ROOT = Path(__file__).resolve().parents[1]
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

from langgraph.checkpoint.memory import InMemorySaver
from langgraph.graph import StateGraph
from langgraph.types import Command, interrupt

from crypton_exp003c.core import record_execution

# Mutable module-level "policy" used only by the H5 (policy drift) scenario.
POLICY = {"limit": 1000}


def reset_policy(limit: int = 1000) -> None:
    POLICY["limit"] = limit


class SingleToolState(TypedDict):
    tool_name: str
    arguments: dict[str, Any]
    result: str


def _propose_and_execute(scenario_id: str):
    """One node: proposes `state["arguments"]` for `state["tool_name"]` via
    interrupt(), and - critically - executes using the arguments captured
    in STATE (checkpointed, not the ephemeral resume payload), matching
    the realistic/safe pattern. See H1 for what happens when this
    discipline is NOT followed, and for direct state mutation via the
    public update_state() API."""

    def node(state: SingleToolState) -> dict[str, Any]:
        decision = interrupt({"tool": state["tool_name"], "arguments": state["arguments"]})
        if isinstance(decision, dict) and decision.get("approved"):
            record_execution(
                scenario_id=scenario_id,
                tool_name=state["tool_name"],
                arguments=state["arguments"],
                thread_id=None,  # filled in by the caller from config, see tests
                interrupt_id=None,
            )
            return {"result": "executed"}
        return {"result": "rejected"}

    return node


def build_single_tool_graph(scenario_id: str, *, checkpointer):
    graph = StateGraph(SingleToolState)
    graph.add_node("propose", _propose_and_execute(scenario_id))
    graph.set_entry_point("propose")
    return graph.compile(checkpointer=checkpointer)


def build_single_tool_graph_resume_sourced(scenario_id: str, *, checkpointer):
    """The NAIVE/unsafe pattern for comparison: executes using whatever
    arguments the RESUME payload itself claims, not the originally
    proposed/checkpointed arguments. Framework-permitted; not a framework
    defect that it's possible - see H1's discussion of why."""

    def node(state: SingleToolState) -> dict[str, Any]:
        decision = interrupt({"tool": state["tool_name"], "arguments": state["arguments"]})
        if isinstance(decision, dict) and decision.get("approved"):
            executed_args = decision.get("arguments", state["arguments"])
            record_execution(scenario_id=scenario_id, tool_name=state["tool_name"], arguments=executed_args, thread_id=None, interrupt_id=None)
            return {"result": "executed"}
        return {"result": "rejected"}

    graph = StateGraph(SingleToolState)
    graph.add_node("propose", node)
    graph.set_entry_point("propose")
    return graph.compile(checkpointer=checkpointer)


def build_policy_checked_graph(scenario_id: str, *, checkpointer):
    """H5: re-checks the mutable POLICY dict AFTER resume, immediately
    before the side effect - the application-built TOCTOU-safe pattern."""

    def node(state: SingleToolState) -> dict[str, Any]:
        decision = interrupt({"tool": state["tool_name"], "arguments": state["arguments"]})
        if not (isinstance(decision, dict) and decision.get("approved")):
            return {"result": "rejected"}
        amount = state["arguments"].get("amount", 0)
        if amount > POLICY["limit"]:
            return {"result": f"blocked_by_policy(limit={POLICY['limit']}, amount={amount})"}
        record_execution(scenario_id=scenario_id, tool_name=state["tool_name"], arguments=state["arguments"], thread_id=None, interrupt_id=None)
        return {"result": "executed"}

    graph = StateGraph(SingleToolState)
    graph.add_node("propose", node)
    graph.set_entry_point("propose")
    return graph.compile(checkpointer=checkpointer)


# ---- H2: delegation boundary (parent interrupt -> subgraph with its own interrupt) ----


class DelegationState(TypedDict):
    arguments: dict[str, Any]
    result: str


def build_delegation_graph(scenario_id: str, *, checkpointer):
    def child_node(state: DelegationState) -> dict[str, Any]:
        decision = interrupt({"tool": "issue_refund", "arguments": state["arguments"], "level": "child"})
        if isinstance(decision, dict) and decision.get("approved"):
            record_execution(scenario_id=scenario_id, tool_name="issue_refund", arguments=state["arguments"], thread_id=None, interrupt_id=None)
            return {"result": "child_executed"}
        return {"result": "child_rejected"}

    child_graph = StateGraph(DelegationState)
    child_graph.add_node("child", child_node)
    child_graph.set_entry_point("child")
    compiled_child = child_graph.compile()  # no own checkpointer - runs as part of the parent's

    def parent_node(state: DelegationState) -> dict[str, Any]:
        decision = interrupt({"tool": "delegate_to_refund_agent", "arguments": state["arguments"], "level": "parent"})
        if not (isinstance(decision, dict) and decision.get("approved")):
            return {"result": "delegation_rejected"}
        # Delegation approved - invoke the child subgraph, which has its OWN interrupt.
        child_result = compiled_child.invoke({"arguments": state["arguments"]})
        return {"result": child_result["result"]}

    parent_graph = StateGraph(DelegationState)
    parent_graph.add_node("parent", parent_node)
    parent_graph.set_entry_point("parent")
    return parent_graph.compile(checkpointer=checkpointer)
