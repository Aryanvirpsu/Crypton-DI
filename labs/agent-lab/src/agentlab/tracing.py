"""
Full execution tracing.

Every run writes one JSON-lines file, one line per event, named after the
run's `request_id` — reconstructing a run means reading its file top to
bottom. Reuses the same correlation-ID concept crypton-gateway already
uses for `x-request-id` (see services/crypton-gateway/src/main.rs)
rather than inventing a second tracing concept.

v0.2: the trace file is keyed by `request_id`, not `instance_id`. Since
`AgentRunner.run()` now creates/resumes a Session and creates a Task
*before* the AgentInstance exists (see agent.py), `instance_id` isn't
available yet at the moment the first event needs to be written.
`request_id` is minted first, before any of that, and covers the whole
call end to end — so it's the identifier the trace file is named after.
`session_id`/`task_id`/`instance_id` still appear on every event, but
start as `None` on a fresh Tracer and are set (`tracer.session_id = ...`,
etc.) as each object comes into existence during the run — so the one or
two events recorded before the task exists (session_created /
session_resumed) legitimately carry `task_id: null`; every event from
`task_created` onward carries a real one.

Never logs: JWTs, JWT_SECRET, API keys, or raw Authorization headers —
only the identity fields already meant to be visible and tool call/
result payloads.
"""

from __future__ import annotations

import json
import time
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

DEFAULT_TRACE_DIR = Path(__file__).resolve().parents[2] / "data" / "traces"


@dataclass
class TraceEvent:
    event: str  # e.g. "run_started", "session_created", "tool_call_requested", ...
    request_id: str
    session_id: str | None
    task_id: str | None
    agent_id: str
    instance_id: str | None
    human_user_id: str
    timestamp: float
    data: dict[str, Any]


class Tracer:
    """One Tracer per call to `AgentRunner.run()`. Keeps events in memory
    (`.events`, for tests/assertions) and appends each one to a JSONL
    file on disk as it happens — not buffered until the end, so a crash
    mid-run still leaves a reconstructible partial trace.

    `request_id`, `agent_id`, and `human_user_id` are fixed for the life
    of the Tracer (all known before any DB work happens). `session_id`,
    `task_id`, and `instance_id` are mutable and get set once each
    corresponding object is created — see agent.py's `run()`."""

    def __init__(
        self,
        *,
        request_id: str,
        agent_id: str,
        human_user_id: str,
        trace_dir: Path | str = DEFAULT_TRACE_DIR,
    ):
        self.request_id = request_id
        self.agent_id = agent_id
        self.human_user_id = human_user_id
        self.session_id: str | None = None
        self.task_id: str | None = None
        self.instance_id: str | None = None
        self.events: list[TraceEvent] = []
        trace_dir = Path(trace_dir)
        trace_dir.mkdir(parents=True, exist_ok=True)
        self.path = trace_dir / f"{request_id}.jsonl"

    def record(self, event: str, **data: Any) -> TraceEvent:
        evt = TraceEvent(
            event=event,
            request_id=self.request_id,
            session_id=self.session_id,
            task_id=self.task_id,
            agent_id=self.agent_id,
            instance_id=self.instance_id,
            human_user_id=self.human_user_id,
            timestamp=time.time(),
            data=data,
        )
        self.events.append(evt)
        with self.path.open("a", encoding="utf-8") as f:
            f.write(json.dumps(asdict(evt), default=str) + "\n")
        return evt

    def as_list(self) -> list[dict]:
        return [asdict(e) for e in self.events]
