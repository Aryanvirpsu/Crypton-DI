"""
crypton_exp003c/core.py — audit sink + evidence schema for EXP-003C
(LangGraph). Consolidated into one module (unlike EXP-003A/B's separate
audit.py/evidence.py) given this lab's narrower scope - no meaningful
size pressure to split it.

Schema note (see README's cross-framework comparison): LangGraph's
interrupt() is a generic pause primitive with no built-in notion of
"tool name", "call_id", or "approval" at all - those are entirely
application-defined shapes passed as the `value` to interrupt(). So
unlike EXP-003A/B, there is no framework-supplied identity to record
here beyond `Interrupt.id` (a unique per-interrupt identifier) and
`thread_id` (the checkpoint namespace). Everything else in AuditRecord
is populated by the TEST'S OWN graph nodes, not read off a framework
object - this is itself evidence for Phase 1's concept mapping.
"""

from __future__ import annotations

import itertools
import json
import time
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Literal

EXECUTED: list[dict[str, Any]] = []  # process-wide raw execution log, reset per test via reset_executed()


def reset_executed() -> None:
    EXECUTED.clear()


@dataclass(frozen=True)
class AuditRecord:
    sequence: int
    timestamp: float
    scenario_id: str
    tool_name: str
    arguments: dict[str, Any]
    thread_id: str | None
    interrupt_id: str | None
    approval_state: str | None
    notes: list[str] = field(default_factory=list)


_counter = itertools.count(1)


def record_execution(*, scenario_id: str, tool_name: str, arguments: dict[str, Any], thread_id: str | None, interrupt_id: str | None, approval_state: str | None = "approved") -> AuditRecord:
    rec = AuditRecord(
        sequence=next(_counter),
        timestamp=time.time(),
        scenario_id=scenario_id,
        tool_name=tool_name,
        arguments=dict(arguments),
        thread_id=thread_id,
        interrupt_id=interrupt_id,
        approval_state=approval_state,
    )
    EXECUTED.append(asdict(rec))
    return rec


Outcome = Literal[
    "PASS", "FAIL", "INCONCLUSIVE", "NOT_APPLICABLE",
    "PASS_WITH_CAVEAT",  # security property held, but a related non-security behavior deviated from the naive expectation
    "CANDIDATE_FINDING_APPLICATION_LEVEL",  # demonstrates an application-design trust-boundary consideration, not a framework defect
]


@dataclass
class AuthorizationBinding:
    tool_match: bool | None
    argument_match: bool | None
    interrupt_id_match: bool | None
    thread_match: bool | None
    delegation_match: bool | None
    notes: list[str] = field(default_factory=list)

    def as_dict(self) -> dict[str, Any]:
        return {
            "tool_match": self.tool_match,
            "argument_match": self.argument_match,
            "interrupt_id_match": self.interrupt_id_match,
            "thread_match": self.thread_match,
            "delegation_match": self.delegation_match,
            "notes": self.notes,
        }


def build_evidence(
    *,
    experiment: str,
    scenario: str,
    target: dict[str, Any],
    proposal: dict[str, Any] | None,
    approval_request: dict[str, Any] | None,
    approval_decision: dict[str, Any] | None,
    delegation_path: list[dict[str, Any]],
    state_boundary: dict[str, Any],
    execution: Any,
    authorization_binding: AuthorizationBinding,
    outcome: Outcome,
    notes: list[str] | None = None,
) -> dict[str, Any]:
    return {
        "experiment": experiment,
        "scenario": scenario,
        "target": target,
        "proposal": proposal,
        "approval_request": approval_request,
        "approval_decision": approval_decision,
        "delegation_path": delegation_path,
        "state_boundary": state_boundary,
        "execution": execution,
        "authorization_binding": authorization_binding.as_dict(),
        "outcome": outcome,
        "notes": notes or [],
    }


def write_evidence(evidence: dict[str, Any], path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(evidence, indent=2, sort_keys=True, default=str) + "\n", encoding="utf-8")
