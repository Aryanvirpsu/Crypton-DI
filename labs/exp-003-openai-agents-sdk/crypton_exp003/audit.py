"""
crypton_exp003/audit.py — the immutable execution-record sink.

Every fake side-effect tool (fixtures.py) appends one AuditRecord here
when it actually executes. This answers "WHAT ACTUALLY EXECUTED" in the
four-stage comparison (proposed / presented for approval / approved /
executed) — see README.md. Records are append-only; nothing in this
module ever mutates or removes a record once appended.

Fields captured, and why each is (or isn't) populated:
  - tool_name, arguments, call_id: always observable from ToolContext.
  - agent_name: the CURRENT agent executing the tool (ToolContext.agent).
  - tool_namespace: ToolContext.tool_namespace — None when the SDK
    doesn't assign one for this call shape (recorded as None, not
    invented).
  - root_agent_name / delegation_call_id: populated ONLY when a scenario
    explicitly threads this through (e.g. via AgentToolInvocation on a
    nested RunResult) — see scenarios/nested_agent.py. Left None
    otherwise, since the SDK's public API does not expose a
    call-time "root of this delegation chain" pointer directly inside a
    tool invocation.
  - approval_state: populated when the scenario code can observe it
    (ToolContext.get_approval_status / is_tool_approved); None + a note
    otherwise.
"""

from __future__ import annotations

import itertools
import json
import time
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any


@dataclass(frozen=True)
class AuditRecord:
    sequence: int
    timestamp: float
    scenario_id: str
    tool_name: str
    arguments: dict[str, Any]
    call_id: str | None
    agent_name: str | None
    tool_namespace: str | None
    root_agent_name: str | None
    delegation_call_id: str | None
    approval_state: str | None
    notes: list[str] = field(default_factory=list)


class AuditSink:
    """One sink per scenario run. Not thread-safe by design — these
    scenarios run one tool call at a time, deterministically."""

    def __init__(self, scenario_id: str):
        self.scenario_id = scenario_id
        self._counter = itertools.count(1)
        self.records: list[AuditRecord] = []

    def record(
        self,
        *,
        tool_name: str,
        arguments: dict[str, Any],
        call_id: str | None,
        agent_name: str | None,
        tool_namespace: str | None,
        root_agent_name: str | None = None,
        delegation_call_id: str | None = None,
        approval_state: str | None = None,
        notes: list[str] | None = None,
    ) -> AuditRecord:
        rec = AuditRecord(
            sequence=next(self._counter),
            timestamp=time.time(),
            scenario_id=self.scenario_id,
            tool_name=tool_name,
            arguments=dict(arguments),
            call_id=call_id,
            agent_name=agent_name,
            tool_namespace=tool_namespace,
            root_agent_name=root_agent_name,
            delegation_call_id=delegation_call_id,
            approval_state=approval_state,
            notes=list(notes or []),
        )
        self.records.append(rec)
        return rec

    def as_list(self) -> list[dict[str, Any]]:
        return [asdict(r) for r in self.records]

    def write(self, path: Path) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(self.as_list(), indent=2, sort_keys=True) + "\n", encoding="utf-8")

    def count_for_tool(self, tool_name: str) -> int:
        return sum(1 for r in self.records if r.tool_name == tool_name)
