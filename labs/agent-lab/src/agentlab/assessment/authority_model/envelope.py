"""
Crypton Authority Model v0.1 — the Authority Envelope.

Third layer: Raw Evidence -> Authority Ledger -> **AUTHORITY ENVELOPE** ->
Authority Diff -> Finding/Action. An envelope answers: *what effective
authority did this actor have, for this action/resource, in this
execution context, at this point* — derived deterministically from the
ledger, never asserted independently of it.

`derive_envelope` only ever populates a dimension (grant_chain,
approval_scope, revocation_state, ...) when the ledger actually contains
events that dimension can be computed from (spec §9 — "derive only
dimensions that existing evidence explicitly supports"). It never performs
general policy evaluation; it does exactly one thing — intersect/narrow
across whatever grant and approval events exist — and leaves every other
dimension `None`.

**The envelope is an assessment abstraction.** Constructing one never
implies the target system itself has an "envelope" concept — see each
fixture's `derivation_notes`.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from .enums import EventType
from .fact import Fact
from .ledger import AuthorityLedger


@dataclass(frozen=True)
class AuthorityEnvelope:
    envelope_id: str
    assessment_id: str | None = None
    test_id: str | None = None

    principal_id: str | None = None
    actor_id: str | None = None
    execution_context_id: str | None = None  # the ExecutionContext.run_id (or other identifier) this envelope concerns

    grant_chain: tuple[str, ...] = field(default_factory=tuple)  # grant_ids, root -> leaf, where evidenced
    allowed_actions: tuple[str, ...] | None = None
    allowed_tools: tuple[str, ...] | None = None
    allowed_resources: tuple[str, ...] | None = None
    argument_constraints: str | None = None

    required_approval: bool | None = None
    approval_id: str | None = None
    approval_scope: Fact | None = None  # Fact[str] — may be UNREPRESENTABLE (ambiguous-by-construction scope)

    valid_from: str | None = None
    expires_at: str | None = None
    revocation_state: str = "UNKNOWN"  # ACTIVE | REVOKED | EXPIRED | UNKNOWN

    delegation_constraints: str | None = None
    terminal_or_consumption_state: str | None = None

    source_evidence_ids: tuple[str, ...] = field(default_factory=tuple)
    derivation_notes: str | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "envelope_id": self.envelope_id,
            "assessment_id": self.assessment_id,
            "test_id": self.test_id,
            "principal_id": self.principal_id,
            "actor_id": self.actor_id,
            "execution_context_id": self.execution_context_id,
            "grant_chain": list(self.grant_chain),
            "allowed_actions": list(self.allowed_actions) if self.allowed_actions is not None else None,
            "allowed_tools": list(self.allowed_tools) if self.allowed_tools is not None else None,
            "allowed_resources": list(self.allowed_resources) if self.allowed_resources is not None else None,
            "argument_constraints": self.argument_constraints,
            "required_approval": self.required_approval,
            "approval_id": self.approval_id,
            "approval_scope": self.approval_scope.to_dict() if self.approval_scope else None,
            "valid_from": self.valid_from,
            "expires_at": self.expires_at,
            "revocation_state": self.revocation_state,
            "delegation_constraints": self.delegation_constraints,
            "terminal_or_consumption_state": self.terminal_or_consumption_state,
            "source_evidence_ids": list(self.source_evidence_ids),
            "derivation_notes": self.derivation_notes,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "AuthorityEnvelope":
        allowed_actions = data.get("allowed_actions")
        allowed_tools = data.get("allowed_tools")
        allowed_resources = data.get("allowed_resources")
        approval_scope = data.get("approval_scope")
        return cls(
            envelope_id=data["envelope_id"],
            assessment_id=data.get("assessment_id"),
            test_id=data.get("test_id"),
            principal_id=data.get("principal_id"),
            actor_id=data.get("actor_id"),
            execution_context_id=data.get("execution_context_id"),
            grant_chain=tuple(data.get("grant_chain", [])),
            allowed_actions=tuple(allowed_actions) if allowed_actions is not None else None,
            allowed_tools=tuple(allowed_tools) if allowed_tools is not None else None,
            allowed_resources=tuple(allowed_resources) if allowed_resources is not None else None,
            argument_constraints=data.get("argument_constraints"),
            required_approval=data.get("required_approval"),
            approval_id=data.get("approval_id"),
            approval_scope=Fact.from_dict(approval_scope),
            valid_from=data.get("valid_from"),
            expires_at=data.get("expires_at"),
            revocation_state=data.get("revocation_state", "UNKNOWN"),
            delegation_constraints=data.get("delegation_constraints"),
            terminal_or_consumption_state=data.get("terminal_or_consumption_state"),
            source_evidence_ids=tuple(data.get("source_evidence_ids", [])),
            derivation_notes=data.get("derivation_notes"),
        )


def derive_envelope(
    ledger: AuthorityLedger,
    *,
    envelope_id: str,
    test_id: str | None = None,
    grant_id: str | None = None,
    approval_id: str | None = None,
    principal_id: str | None = None,
    actor_id: str | None = None,
    execution_context_id: str | None = None,
) -> AuthorityEnvelope:
    """Derive the effective-authority envelope for one `grant_id` and/or
    one `approval_id` already present in `ledger`. The caller names which
    grant/approval this envelope concerns (an envelope is scoped to one
    actor/action/context question, per spec §8 — it is not a whole-ledger
    summary) — `derive_envelope` does not search the ledger to guess which
    one is "the" relevant one.

    Only populates a dimension when the named grant/approval, or the
    ledger's revocation-related events for it, actually supply it. Returns
    `None` for every other dimension rather than guessing.
    """
    source_evidence: list[str] = []
    notes: list[str] = []

    grant_chain: tuple[str, ...] = ()
    allowed_tools: tuple[str, ...] | None = None
    allowed_resources: tuple[str, ...] | None = None
    allowed_actions: tuple[str, ...] | None = None
    argument_constraints: str | None = None
    delegation_constraints: str | None = None
    valid_from: str | None = None
    expires_at: str | None = None
    revocation_state = "UNKNOWN"

    if grant_id is not None and grant_id in ledger.grants:
        # Walk the grant chain root -> leaf via parent_grant_id, narrowing
        # at each hop by intersection — only as many hops as are actually
        # present in the ledger.
        chain: list[str] = []
        current_id: str | None = grant_id
        visited: set[str] = set()
        tool_sets: list[set[str]] = []
        resource_sets: list[set[str]] = []
        while current_id is not None and current_id not in visited and current_id in ledger.grants:
            visited.add(current_id)
            g = ledger.grants[current_id]
            chain.append(current_id)
            source_evidence.extend(g.evidence_ids)
            if g.allowed_tools is not None:
                tool_sets.append(set(g.allowed_tools))
            if g.allowed_resources is not None:
                resource_sets.append(set(g.allowed_resources))
            if g.allowed_actions is not None:
                allowed_actions = tuple(g.allowed_actions) if allowed_actions is None else allowed_actions
            if g.argument_constraints and argument_constraints is None:
                argument_constraints = g.argument_constraints
            if g.delegation_allowed is not None or g.delegation_depth_limit is not None:
                delegation_constraints = (
                    f"delegation_allowed={g.delegation_allowed}, depth_limit={g.delegation_depth_limit}"
                )
            if valid_from is None:
                valid_from = g.valid_from
            if expires_at is None:
                expires_at = g.expires_at
            if g.revoked_at:
                revocation_state = "REVOKED"
            current_pf = g.parent_grant_id
            current_id = current_pf.value if (current_pf is not None and current_pf.is_available) else None
        grant_chain = tuple(reversed(chain))  # root -> leaf
        if tool_sets:
            allowed_tools = tuple(sorted(set.intersection(*tool_sets)))
        if resource_sets:
            allowed_resources = tuple(sorted(set.intersection(*resource_sets)))
        if revocation_state == "UNKNOWN":
            revocation_state = "ACTIVE"
        notes.append(
            f"Grant dimension derived from grant_chain={list(grant_chain)} "
            f"({'intersected across ' + str(len(grant_chain)) + ' hop(s)' if len(grant_chain) > 1 else 'single grant'})."
        )
    else:
        notes.append("No grant evidenced in this ledger for the named grant_id — grant-related dimensions left unset.")

    required_approval: bool | None = None
    approval_id_out: str | None = None
    approval_scope: Fact | None = None
    terminal_or_consumption_state: str | None = None

    if approval_id is not None and approval_id in ledger.approvals:
        required_approval = True
        approval_id_out = approval_id
        appr = ledger.approvals[approval_id]
        source_evidence.extend(appr.evidence_ids)
        if appr.approval_scope:
            approval_scope = Fact.observed(appr.approval_scope)
        elif appr.target_action_ids is not None:
            if appr.target_action_ids.is_available:
                approval_scope = Fact.observed(f"bound to action(s): {appr.target_action_ids.value}")
            else:
                approval_scope = appr.target_action_ids  # propagate the UNREPRESENTABLE/UNVERIFIABLE fact as-is
        if appr.state == "consumed" or appr.consumed_at:
            terminal_or_consumption_state = "consumed"
        elif appr.state:
            terminal_or_consumption_state = appr.state
        if expires_at is None:
            expires_at = appr.expires_at
        if valid_from is None:
            valid_from = appr.granted_at
        notes.append(f"Approval dimension derived from approval_id={approval_id!r}.")
    else:
        notes.append("No approval evidenced in this ledger for the named approval_id — approval dimension left unset.")

    return AuthorityEnvelope(
        envelope_id=envelope_id,
        assessment_id=ledger.assessment_id,
        test_id=test_id,
        principal_id=principal_id,
        actor_id=actor_id,
        execution_context_id=execution_context_id,
        grant_chain=grant_chain,
        allowed_actions=allowed_actions,
        allowed_tools=allowed_tools,
        allowed_resources=allowed_resources,
        argument_constraints=argument_constraints,
        required_approval=required_approval,
        approval_id=approval_id_out,
        approval_scope=approval_scope,
        valid_from=valid_from,
        expires_at=expires_at,
        revocation_state=revocation_state,
        delegation_constraints=delegation_constraints,
        terminal_or_consumption_state=terminal_or_consumption_state,
        source_evidence_ids=tuple(dict.fromkeys(source_evidence)),  # de-duplicate, preserve order
        derivation_notes=" ".join(notes),
    )
