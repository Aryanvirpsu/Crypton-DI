"""
Crypton Authority Model v0.1 — the Authority Ledger.

The first derived layer: RAW EVIDENCE -> **AUTHORITY LEDGER** -> Authority
Envelope -> Authority Diff -> Finding/Action. A ledger is a flat,
time-ordered sequence of `LedgerEvent`s, each referencing (by local key,
never by inventing one) the canonical objects it concerns — plus the full
objects themselves, held separately so an event doesn't duplicate an
object's facts on every reference.

A ledger is built once per fixture by normalization code in `fixtures.py`
that reads an existing engagement/evidence record — never by an assessor
hand-authoring `authority-ledger.json` from scratch (that would make this
a second, manually maintained source of truth, exactly what this module's
governing design rule forbids).
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from ..models import EvidenceClassification
from .enums import EventType, Provenance
from .primitives import Action, Actor, Approval, Effect, ExecutionContext, Grant, Principal, Resource


@dataclass(frozen=True)
class LedgerEvent:
    """One entry. Every execution-context identifier and every
    authority-related reference is individually optional — a ledger entry
    never invents an ID merely because the model supports the object (spec
    §3's closing rule); it carries only the identifiers the target/evidence
    actually exposed."""

    event_id: str
    event_type: EventType
    assessment_id: str | None = None
    test_id: str | None = None
    timestamp: str | None = None
    sequence: int | None = None

    execution_context: ExecutionContext | None = None

    principal_id: str | None = None
    actor_id: str | None = None
    grant_id: str | None = None
    approval_id: str | None = None
    action_id: str | None = None
    resource_id: str | None = None
    effect_id: str | None = None

    before_state: Any | None = None
    after_state: Any | None = None

    evidence_ids: tuple[str, ...] = field(default_factory=tuple)
    provenance: Provenance = Provenance.UNKNOWN
    classification: EvidenceClassification | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "event_id": self.event_id,
            "event_type": self.event_type.value,
            "assessment_id": self.assessment_id,
            "test_id": self.test_id,
            "timestamp": self.timestamp,
            "sequence": self.sequence,
            "execution_context": self.execution_context.to_dict() if self.execution_context else None,
            "principal_id": self.principal_id,
            "actor_id": self.actor_id,
            "grant_id": self.grant_id,
            "approval_id": self.approval_id,
            "action_id": self.action_id,
            "resource_id": self.resource_id,
            "effect_id": self.effect_id,
            "before_state": self.before_state,
            "after_state": self.after_state,
            "evidence_ids": list(self.evidence_ids),
            "provenance": self.provenance.value,
            "classification": self.classification.value if self.classification else None,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "LedgerEvent":
        execution_context = data.get("execution_context")
        classification = data.get("classification")
        return cls(
            event_id=data["event_id"],
            event_type=EventType(data["event_type"]),
            assessment_id=data.get("assessment_id"),
            test_id=data.get("test_id"),
            timestamp=data.get("timestamp"),
            sequence=data.get("sequence"),
            execution_context=ExecutionContext.from_dict(execution_context) if execution_context else None,
            principal_id=data.get("principal_id"),
            actor_id=data.get("actor_id"),
            grant_id=data.get("grant_id"),
            approval_id=data.get("approval_id"),
            action_id=data.get("action_id"),
            resource_id=data.get("resource_id"),
            effect_id=data.get("effect_id"),
            before_state=data.get("before_state"),
            after_state=data.get("after_state"),
            evidence_ids=tuple(data.get("evidence_ids", [])),
            provenance=Provenance(data.get("provenance", "UNKNOWN")),
            classification=EvidenceClassification(classification) if classification else None,
        )


@dataclass
class AuthorityLedger:
    """A time-ordered event sequence plus the full canonical objects those
    events reference, keyed by local reference id (the same strings used
    in each `LedgerEvent`'s `*_id` fields — a local ledger key, not a claim
    that the target system itself exposes that exact string, unless the
    referenced object's own `Fact`-wrapped id says otherwise)."""

    assessment_id: str | None = None
    events: list[LedgerEvent] = field(default_factory=list)
    principals: dict[str, Principal] = field(default_factory=dict)
    actors: dict[str, Actor] = field(default_factory=dict)
    grants: dict[str, Grant] = field(default_factory=dict)
    approvals: dict[str, Approval] = field(default_factory=dict)
    actions: dict[str, Action] = field(default_factory=dict)
    resources: dict[str, Resource] = field(default_factory=dict)
    effects: dict[str, Effect] = field(default_factory=dict)

    def add_event(self, event: LedgerEvent) -> None:
        self.events.append(event)

    def events_of(self, *event_types: EventType) -> list[LedgerEvent]:
        wanted = set(event_types)
        return [e for e in self.events if e.event_type in wanted]

    def to_dict(self) -> dict[str, Any]:
        return {
            "assessment_id": self.assessment_id,
            "events": [e.to_dict() for e in sorted(self.events, key=lambda e: (e.sequence or 0, e.event_id))],
            "principals": {k: v.to_dict() for k, v in self.principals.items()},
            "actors": {k: v.to_dict() for k, v in self.actors.items()},
            "grants": {k: v.to_dict() for k, v in self.grants.items()},
            "approvals": {k: v.to_dict() for k, v in self.approvals.items()},
            "actions": {k: v.to_dict() for k, v in self.actions.items()},
            "resources": {k: v.to_dict() for k, v in self.resources.items()},
            "effects": {k: v.to_dict() for k, v in self.effects.items()},
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "AuthorityLedger":
        return cls(
            assessment_id=data.get("assessment_id"),
            events=[LedgerEvent.from_dict(e) for e in data.get("events", [])],
            principals={k: Principal.from_dict(v) for k, v in data.get("principals", {}).items()},
            actors={k: Actor.from_dict(v) for k, v in data.get("actors", {}).items()},
            grants={k: Grant.from_dict(v) for k, v in data.get("grants", {}).items()},
            approvals={k: Approval.from_dict(v) for k, v in data.get("approvals", {}).items()},
            actions={k: Action.from_dict(v) for k, v in data.get("actions", {}).items()},
            resources={k: Resource.from_dict(v) for k, v in data.get("resources", {}).items()},
            effects={k: Effect.from_dict(v) for k, v in data.get("effects", {}).items()},
        )

    def save(self, path: str | Path) -> None:
        Path(path).write_text(json.dumps(self.to_dict(), indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    @classmethod
    def load(cls, path: str | Path) -> "AuthorityLedger":
        return cls.from_dict(json.loads(Path(path).read_text(encoding="utf-8")))
