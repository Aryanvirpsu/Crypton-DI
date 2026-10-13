"""
crypton_exp003/evidence.py — the machine-readable evidence record.

Builds and writes the per-scenario JSON evidence document described in
README.md's evidence schema. This is the core Crypton primitive this
experiment is built around: a four-stage comparison of

    WHAT WAS PROPOSED           (proposal)
    WHAT WAS PRESENTED FOR APPROVAL   (approval_request)
    WHAT WAS APPROVED            (approval_decision)
    WHAT ACTUALLY EXECUTED         (execution)

`authorization_binding` records whether those four stages agree
(tool/argument/call_id/namespace/delegation match), per field, as
booleans or `None` + a note when the public API doesn't expose enough to
judge. Never invent a True/False for something unobservable.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Literal

Outcome = Literal["PASS", "FAIL", "INCONCLUSIVE"]


@dataclass
class AuthorizationBinding:
    tool_match: bool | None
    argument_match: bool | None
    call_id_match: bool | None
    namespace_match: bool | None
    delegation_match: bool | None
    notes: list[str] = field(default_factory=list)

    def as_dict(self) -> dict[str, Any]:
        return {
            "tool_match": self.tool_match,
            "argument_match": self.argument_match,
            "call_id_match": self.call_id_match,
            "namespace_match": self.namespace_match,
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
        "execution": execution,
        "authorization_binding": authorization_binding.as_dict(),
        "outcome": outcome,
        "notes": notes or [],
    }


def write_evidence(evidence: dict[str, Any], path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(evidence, indent=2, sort_keys=True, default=str) + "\n", encoding="utf-8")


def approval_item_to_dict(item: Any) -> dict[str, Any]:
    """Normalizes an agents.ToolApprovalItem into a plain dict for
    evidence purposes. Only reads public attributes/methods; if a future
    SDK version removes one, this raises rather than silently recording
    a wrong value."""
    arguments = item.arguments
    if isinstance(arguments, str):
        try:
            arguments = json.loads(arguments)
        except json.JSONDecodeError:
            pass  # keep raw string rather than pretending it parsed
    return {
        "tool_name": item.tool_name,
        "arguments": arguments,
        "call_id": item.call_id,
        "tool_namespace": item.tool_namespace,
        "qualified_name": item.qualified_name,
    }
