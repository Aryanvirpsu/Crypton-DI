"""
crypton_exp003b/audit.py — the immutable execution-record sink.

Adapted from EXP-003A's audit.py (see README's schema comparison for what
changed and why). Microsoft Agent Framework surfaces more authority
dimensions than OpenAI's SDK did (workflow, checkpoint, request_id), so
this AuditRecord grew fields EXP-003A's lacked - added locally per the
task's instruction, not yet promoted to any shared schema.
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
    originating_agent: str | None
    executing_agent: str | None
    workflow_name: str | None
    checkpoint_id: str | None
    request_id: str | None
    approval_state: str | None
    notes: list[str] = field(default_factory=list)


class AuditSink:
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
        originating_agent: str | None,
        executing_agent: str | None,
        workflow_name: str | None = None,
        checkpoint_id: str | None = None,
        request_id: str | None = None,
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
            originating_agent=originating_agent,
            executing_agent=executing_agent,
            workflow_name=workflow_name,
            checkpoint_id=checkpoint_id,
            request_id=request_id,
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
