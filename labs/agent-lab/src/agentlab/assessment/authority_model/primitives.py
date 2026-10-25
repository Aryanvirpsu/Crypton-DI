"""
Crypton Authority Model v0.1 — the eight canonical primitives.

Principal, Actor, Grant, Approval, ExecutionContext, Action, Resource,
Effect. Every object is a plain, frozen dataclass — no business logic, no
evaluation, no calls into agent-lab's real primitives or any target
system. These are *representations* of facts already established
elsewhere (an engagement record, a property/test record, an evidence
manifest, or an explicitly-marked customer assertion), never a second
place to decide or invent one.

Identity- and binding-sensitive fields (the ones spec §23 forbids
inferring) are wrapped in `Fact` so their provenance/availability travels
with them. Structural fields (tool/resource name lists, timestamps, state
labels) are plain `Optional` values — their presence or absence is already
unambiguous, and the object's own `evidence_ids`/`provenance` cover them.

Every object is optional in a ledger — a ledger entry never fabricates one
merely because the model supports it (spec §3's closing rule).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from ..models import EvidenceClassification
from .enums import ActorType, Provenance
from .fact import Fact


def _facts_to_dict(value: Fact | None) -> dict[str, Any] | None:
    return value.to_dict() if value is not None else None


def _facts_from_dict(data: dict[str, Any] | None) -> Fact | None:
    return Fact.from_dict(data)


@dataclass(frozen=True)
class Principal:
    principal_type: str | None = None  # e.g. "human", "organization" — plain, not identity-sensitive itself
    principal_id: Fact | None = None
    authenticated_identity: Fact | None = None
    claimed_identity: Fact | None = None
    tenant_org: Fact | None = None
    evidence_ids: tuple[str, ...] = field(default_factory=tuple)
    provenance: Provenance = Provenance.UNKNOWN
    classification: EvidenceClassification | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "principal_type": self.principal_type,
            "principal_id": _facts_to_dict(self.principal_id),
            "authenticated_identity": _facts_to_dict(self.authenticated_identity),
            "claimed_identity": _facts_to_dict(self.claimed_identity),
            "tenant_org": _facts_to_dict(self.tenant_org),
            "evidence_ids": list(self.evidence_ids),
            "provenance": self.provenance.value,
            "classification": self.classification.value if self.classification else None,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "Principal":
        classification = data.get("classification")
        return cls(
            principal_type=data.get("principal_type"),
            principal_id=_facts_from_dict(data.get("principal_id")),
            authenticated_identity=_facts_from_dict(data.get("authenticated_identity")),
            claimed_identity=_facts_from_dict(data.get("claimed_identity")),
            tenant_org=_facts_from_dict(data.get("tenant_org")),
            evidence_ids=tuple(data.get("evidence_ids", [])),
            provenance=Provenance(data.get("provenance", "UNKNOWN")),
            classification=EvidenceClassification(classification) if classification else None,
        )


@dataclass(frozen=True)
class Actor:
    actor_type: ActorType | None = None
    actor_id: Fact | None = None
    framework_identity: Fact | None = None
    acts_for_principal: Fact | None = None
    evidence_ids: tuple[str, ...] = field(default_factory=tuple)
    provenance: Provenance = Provenance.UNKNOWN
    classification: EvidenceClassification | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "actor_type": self.actor_type.value if self.actor_type else None,
            "actor_id": _facts_to_dict(self.actor_id),
            "framework_identity": _facts_to_dict(self.framework_identity),
            "acts_for_principal": _facts_to_dict(self.acts_for_principal),
            "evidence_ids": list(self.evidence_ids),
            "provenance": self.provenance.value,
            "classification": self.classification.value if self.classification else None,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "Actor":
        actor_type = data.get("actor_type")
        classification = data.get("classification")
        return cls(
            actor_type=ActorType(actor_type) if actor_type else None,
            actor_id=_facts_from_dict(data.get("actor_id")),
            framework_identity=_facts_from_dict(data.get("framework_identity")),
            acts_for_principal=_facts_from_dict(data.get("acts_for_principal")),
            evidence_ids=tuple(data.get("evidence_ids", [])),
            provenance=Provenance(data.get("provenance", "UNKNOWN")),
            classification=EvidenceClassification(classification) if classification else None,
        )


@dataclass(frozen=True)
class Grant:
    grant_id: Fact | None = None
    granted_by: Fact | None = None
    granted_to: Fact | None = None
    allowed_actions: tuple[str, ...] | None = None
    allowed_tools: tuple[str, ...] | None = None
    allowed_resources: tuple[str, ...] | None = None
    argument_constraints: str | None = None
    delegation_allowed: bool | None = None
    delegation_depth_limit: int | None = None
    valid_from: str | None = None
    expires_at: str | None = None
    revoked_at: str | None = None
    parent_grant_id: Fact | None = None
    evidence_ids: tuple[str, ...] = field(default_factory=tuple)
    provenance: Provenance = Provenance.UNKNOWN
    classification: EvidenceClassification | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "grant_id": _facts_to_dict(self.grant_id),
            "granted_by": _facts_to_dict(self.granted_by),
            "granted_to": _facts_to_dict(self.granted_to),
            "allowed_actions": list(self.allowed_actions) if self.allowed_actions is not None else None,
            "allowed_tools": list(self.allowed_tools) if self.allowed_tools is not None else None,
            "allowed_resources": list(self.allowed_resources) if self.allowed_resources is not None else None,
            "argument_constraints": self.argument_constraints,
            "delegation_allowed": self.delegation_allowed,
            "delegation_depth_limit": self.delegation_depth_limit,
            "valid_from": self.valid_from,
            "expires_at": self.expires_at,
            "revoked_at": self.revoked_at,
            "parent_grant_id": _facts_to_dict(self.parent_grant_id),
            "evidence_ids": list(self.evidence_ids),
            "provenance": self.provenance.value,
            "classification": self.classification.value if self.classification else None,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "Grant":
        classification = data.get("classification")
        allowed_actions = data.get("allowed_actions")
        allowed_tools = data.get("allowed_tools")
        allowed_resources = data.get("allowed_resources")
        return cls(
            grant_id=_facts_from_dict(data.get("grant_id")),
            granted_by=_facts_from_dict(data.get("granted_by")),
            granted_to=_facts_from_dict(data.get("granted_to")),
            allowed_actions=tuple(allowed_actions) if allowed_actions is not None else None,
            allowed_tools=tuple(allowed_tools) if allowed_tools is not None else None,
            allowed_resources=tuple(allowed_resources) if allowed_resources is not None else None,
            argument_constraints=data.get("argument_constraints"),
            delegation_allowed=data.get("delegation_allowed"),
            delegation_depth_limit=data.get("delegation_depth_limit"),
            valid_from=data.get("valid_from"),
            expires_at=data.get("expires_at"),
            revoked_at=data.get("revoked_at"),
            parent_grant_id=_facts_from_dict(data.get("parent_grant_id")),
            evidence_ids=tuple(data.get("evidence_ids", [])),
            provenance=Provenance(data.get("provenance", "UNKNOWN")),
            classification=EvidenceClassification(classification) if classification else None,
        )


@dataclass(frozen=True)
class Approval:
    approval_id: Fact | None = None
    requested_by: Fact | None = None
    approved_by: Fact | None = None
    target_action_ids: Fact | None = None  # Fact[tuple[str, ...]] — binding itself may be ambiguous/unrepresentable
    target_resource_ids: tuple[str, ...] | None = None
    approval_scope: str | None = None
    granted_at: str | None = None
    expires_at: str | None = None
    state: str | None = None
    consumed_at: str | None = None
    execution_context_id: str | None = None
    evidence_ids: tuple[str, ...] = field(default_factory=tuple)
    provenance: Provenance = Provenance.UNKNOWN
    classification: EvidenceClassification | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "approval_id": _facts_to_dict(self.approval_id),
            "requested_by": _facts_to_dict(self.requested_by),
            "approved_by": _facts_to_dict(self.approved_by),
            "target_action_ids": _facts_to_dict(self.target_action_ids),
            "target_resource_ids": list(self.target_resource_ids) if self.target_resource_ids is not None else None,
            "approval_scope": self.approval_scope,
            "granted_at": self.granted_at,
            "expires_at": self.expires_at,
            "state": self.state,
            "consumed_at": self.consumed_at,
            "execution_context_id": self.execution_context_id,
            "evidence_ids": list(self.evidence_ids),
            "provenance": self.provenance.value,
            "classification": self.classification.value if self.classification else None,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "Approval":
        classification = data.get("classification")
        target_resource_ids = data.get("target_resource_ids")
        return cls(
            approval_id=_facts_from_dict(data.get("approval_id")),
            requested_by=_facts_from_dict(data.get("requested_by")),
            approved_by=_facts_from_dict(data.get("approved_by")),
            target_action_ids=_facts_from_dict(data.get("target_action_ids")),
            target_resource_ids=tuple(target_resource_ids) if target_resource_ids is not None else None,
            approval_scope=data.get("approval_scope"),
            granted_at=data.get("granted_at"),
            expires_at=data.get("expires_at"),
            state=data.get("state"),
            consumed_at=data.get("consumed_at"),
            execution_context_id=data.get("execution_context_id"),
            evidence_ids=tuple(data.get("evidence_ids", [])),
            provenance=Provenance(data.get("provenance", "UNKNOWN")),
            classification=EvidenceClassification(classification) if classification else None,
        )


@dataclass(frozen=True)
class ExecutionContext:
    run_id: str | None = None
    session_id: str | None = None
    thread_id: str | None = None
    workflow_id: str | None = None
    task_id: str | None = None
    request_id: str | None = None
    instance_id: str | None = None
    owner_principal: Fact | None = None
    owner_actor: Fact | None = None
    evidence_ids: tuple[str, ...] = field(default_factory=tuple)
    provenance: Provenance = Provenance.UNKNOWN
    classification: EvidenceClassification | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "run_id": self.run_id,
            "session_id": self.session_id,
            "thread_id": self.thread_id,
            "workflow_id": self.workflow_id,
            "task_id": self.task_id,
            "request_id": self.request_id,
            "instance_id": self.instance_id,
            "owner_principal": _facts_to_dict(self.owner_principal),
            "owner_actor": _facts_to_dict(self.owner_actor),
            "evidence_ids": list(self.evidence_ids),
            "provenance": self.provenance.value,
            "classification": self.classification.value if self.classification else None,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "ExecutionContext":
        classification = data.get("classification")
        return cls(
            run_id=data.get("run_id"),
            session_id=data.get("session_id"),
            thread_id=data.get("thread_id"),
            workflow_id=data.get("workflow_id"),
            task_id=data.get("task_id"),
            request_id=data.get("request_id"),
            instance_id=data.get("instance_id"),
            owner_principal=_facts_from_dict(data.get("owner_principal")),
            owner_actor=_facts_from_dict(data.get("owner_actor")),
            evidence_ids=tuple(data.get("evidence_ids", [])),
            provenance=Provenance(data.get("provenance", "UNKNOWN")),
            classification=EvidenceClassification(classification) if classification else None,
        )


@dataclass(frozen=True)
class Action:
    action_id: Fact | None = None
    actor_id: Fact | None = None
    tool: str | None = None
    operation: str | None = None
    arguments: Fact | None = None  # Fact[dict] — raw arguments, when available
    argument_hash: str | None = None  # populated instead of/alongside arguments when only a commitment exists
    intended_resource: str | None = None
    actual_resource: str | None = None
    state: str | None = None
    execution_context_id: str | None = None
    evidence_ids: tuple[str, ...] = field(default_factory=tuple)
    provenance: Provenance = Provenance.UNKNOWN
    classification: EvidenceClassification | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "action_id": _facts_to_dict(self.action_id),
            "actor_id": _facts_to_dict(self.actor_id),
            "tool": self.tool,
            "operation": self.operation,
            "arguments": _facts_to_dict(self.arguments),
            "argument_hash": self.argument_hash,
            "intended_resource": self.intended_resource,
            "actual_resource": self.actual_resource,
            "state": self.state,
            "execution_context_id": self.execution_context_id,
            "evidence_ids": list(self.evidence_ids),
            "provenance": self.provenance.value,
            "classification": self.classification.value if self.classification else None,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "Action":
        classification = data.get("classification")
        return cls(
            action_id=_facts_from_dict(data.get("action_id")),
            actor_id=_facts_from_dict(data.get("actor_id")),
            tool=data.get("tool"),
            operation=data.get("operation"),
            arguments=_facts_from_dict(data.get("arguments")),
            argument_hash=data.get("argument_hash"),
            intended_resource=data.get("intended_resource"),
            actual_resource=data.get("actual_resource"),
            state=data.get("state"),
            execution_context_id=data.get("execution_context_id"),
            evidence_ids=tuple(data.get("evidence_ids", [])),
            provenance=Provenance(data.get("provenance", "UNKNOWN")),
            classification=EvidenceClassification(classification) if classification else None,
        )


@dataclass(frozen=True)
class Resource:
    resource_id: Fact | None = None
    resource_type: str | None = None
    namespace_tenant: str | None = None
    attributes: dict[str, Any] | None = None
    evidence_ids: tuple[str, ...] = field(default_factory=tuple)
    provenance: Provenance = Provenance.UNKNOWN
    classification: EvidenceClassification | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "resource_id": _facts_to_dict(self.resource_id),
            "resource_type": self.resource_type,
            "namespace_tenant": self.namespace_tenant,
            "attributes": self.attributes,
            "evidence_ids": list(self.evidence_ids),
            "provenance": self.provenance.value,
            "classification": self.classification.value if self.classification else None,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "Resource":
        classification = data.get("classification")
        return cls(
            resource_id=_facts_from_dict(data.get("resource_id")),
            resource_type=data.get("resource_type"),
            namespace_tenant=data.get("namespace_tenant"),
            attributes=data.get("attributes"),
            evidence_ids=tuple(data.get("evidence_ids", [])),
            provenance=Provenance(data.get("provenance", "UNKNOWN")),
            classification=EvidenceClassification(classification) if classification else None,
        )


@dataclass(frozen=True)
class Effect:
    effect_id: str  # local bookkeeping ID for this Authority Model's own records — see module docstring
    action_id: Fact | None = None
    effect_type: str | None = None
    target_resource: str | None = None
    before_state: Any | None = None
    after_state: Any | None = None
    observed_at: str | None = None
    evidence_ids: tuple[str, ...] = field(default_factory=tuple)
    provenance: Provenance = Provenance.UNKNOWN
    classification: EvidenceClassification | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "effect_id": self.effect_id,
            "action_id": _facts_to_dict(self.action_id),
            "effect_type": self.effect_type,
            "target_resource": self.target_resource,
            "before_state": self.before_state,
            "after_state": self.after_state,
            "observed_at": self.observed_at,
            "evidence_ids": list(self.evidence_ids),
            "provenance": self.provenance.value,
            "classification": self.classification.value if self.classification else None,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "Effect":
        classification = data.get("classification")
        return cls(
            effect_id=data["effect_id"],
            action_id=_facts_from_dict(data.get("action_id")),
            effect_type=data.get("effect_type"),
            target_resource=data.get("target_resource"),
            before_state=data.get("before_state"),
            after_state=data.get("after_state"),
            observed_at=data.get("observed_at"),
            evidence_ids=tuple(data.get("evidence_ids", [])),
            provenance=Provenance(data.get("provenance", "UNKNOWN")),
            classification=EvidenceClassification(classification) if classification else None,
        )
