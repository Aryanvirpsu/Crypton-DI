"""
crypton_exp003b/evidence.py — the EXP-003B evidence record.

Compatible in spirit with EXP-003A's schema (crypton_exp003.evidence),
not identical. Two additions the task asked for, kept local until a
promotion decision is made (see README):

  - `state_boundary` {workflow, session, checkpoint}: Microsoft Agent
    Framework exposes workflow/checkpoint identity as first-class
    concepts EXP-003A had no equivalent for.
  - `authorization_binding` gained `request_match` and `checkpoint_match`
    (EXP-003A only had tool/argument/call_id/namespace/delegation match)
    - request_id is this framework's primary correlation handle for a
      pending approval, distinct from the underlying tool call_id.

Never invent a value for a field the public API doesn't expose - use
null + a note.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Literal

# 2026-09-30 remediation note: "CANDIDATE_FINDING" was already being written
# to tests/test_checkpoint_binding.py's evidence record (the strongest single
# result in the EXP-003 series) without being a declared member of this type -
# a real schema/type gap flagged by the 2026-09-30 evidence-readiness audit
# (harmless at runtime since Python doesn't enforce Literal, but it meant no
# static check could have caught an undeclared outcome value). Added here,
# not renamed/removed - this is this experiment's own local research
# vocabulary (see module docstring), independent of the separate, frozen,
# customer-facing five-outcome model in `labs/agent-lab/src/agentlab/
# assessment/models.py::Outcome`. No historical evidence JSON is rewritten.
Outcome = Literal["PASS", "FAIL", "INCONCLUSIVE", "BLOCKED_PROVIDER", "CANDIDATE_FINDING"]


@dataclass
class StateBoundary:
    workflow: str | None
    session: str | None
    checkpoint: str | None

    def as_dict(self) -> dict[str, Any]:
        return {"workflow": self.workflow, "session": self.session, "checkpoint": self.checkpoint}


@dataclass
class AuthorizationBinding:
    tool_match: bool | None
    argument_match: bool | None
    request_match: bool | None
    agent_match: bool | None
    delegation_match: bool | None
    checkpoint_match: bool | None
    notes: list[str] = field(default_factory=list)

    def as_dict(self) -> dict[str, Any]:
        return {
            "tool_match": self.tool_match,
            "argument_match": self.argument_match,
            "request_match": self.request_match,
            "agent_match": self.agent_match,
            "delegation_match": self.delegation_match,
            "checkpoint_match": self.checkpoint_match,
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
    state_boundary: StateBoundary,
    execution: dict[str, Any] | list[dict[str, Any]] | None,
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
        "state_boundary": state_boundary.as_dict(),
        "execution": execution,
        "authorization_binding": authorization_binding.as_dict(),
        "outcome": outcome,
        "notes": notes or [],
    }


def write_evidence(evidence: dict[str, Any], path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(evidence, indent=2, sort_keys=True, default=str) + "\n", encoding="utf-8")


def approval_content_to_dict(content: Any) -> dict[str, Any]:
    """Normalizes an agent_framework Content(type='function_approval_request')
    into a plain dict. Only reads public attributes; raises rather than
    silently recording a wrong value if a future version removes one.

    Confirmed empirically: the approval-request Content's own `.name`/
    `.tool_name`/`.arguments`/`.call_id` are None on the WRAPPER - the
    actual function identity/arguments live on the nested `.function_call`
    Content it wraps."""
    inner = content.function_call
    arguments = inner.arguments if inner is not None else None
    if isinstance(arguments, str):
        try:
            arguments = json.loads(arguments)
        except json.JSONDecodeError:
            pass
    return {
        "id": content.id,
        "tool_name": (inner.name if inner is not None else None),
        "call_id": (inner.call_id if inner is not None else None),
        "arguments": arguments,
    }
