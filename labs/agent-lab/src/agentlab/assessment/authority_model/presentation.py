"""
Crypton Authority Map / Authority Diff — presentation projection (v0.1).

This module sits strictly *downstream* of the frozen Authority Model
(`ledger.py`/`envelope.py`/`diff.py`/`primitives.py`/`fact.py`/`enums.py` —
none of those files are touched by this pass). It derives an `AuthorityView`
— a flat, report-ready projection — from an `AuthorityLedger`,
`AuthorityEnvelope`, and `AuthorityDiff` that already exist. It is itself a
*projection*, not a new source of truth: every field on `AuthorityView`
traces back to a specific Authority Model fact, a `Fact`'s own provenance/
availability, or an explicit, already-existing `Finding` field
(`recommendation`, `retest_condition`, `status`) — never a value invented
here for display purposes.

**System Path vs. Authority Path (spec §10):** this module only ever
builds an Authority Path (Principal -> Grant -> Actor -> Approval ->
Action -> Resource -> Effect). It has no System Path concept at all — a
System Path field exists on `AuthorityView` purely as an optional,
separately-rendered passthrough for whatever architecture context a
caller already has (e.g. from an engagement's `workflow` prose); this
module never derives one from Authority Model relationships, and never
lets the two graphs merge.

**Provenance visual language (spec §6)** is attached per path step as a
plain string value, not the frozen `Provenance` enum directly — most of
the time it IS one of that enum's values (copied via `.value` from the
object/Fact a step was built from), but this layer also needs one
presentation-only category the frozen model deliberately does not have:
`ASSESSED_INVARIANT` (2026-10 freeze-pass addition).

**`ASSESSED_INVARIANT` — presentation-layer-only, never added to
`enums.Provenance`.** Some EXPECTED-side facts are not read from the
target at all — they are Crypton's own stated security expectation (e.g.
"an approval should resolve at most one pending action"). That invariant
is always fully known and statable; it is not a gap, and it must never be
rendered as UNREPRESENTABLE/UNVERIFIABLE/UNKNOWN. What *can* be a genuine
target-side gap is a separate fact: whether the target's own API/schema is
even capable of representing which specific action an approval binds to.
Collapsing these two into one UNREPRESENTABLE badge (as Authority Model
v0.1's first report-surface pass did) wrongly implies the *security
property itself* is missing, when only the *target's representation* of
it is. See `build_authority_view`'s approval-handling section for where
this split is made, and the presentation accuracy rule below.

**Presentation accuracy rule (freeze-pass addition):** expected/permitted
authority and observed target state are separate semantic categories.
Expected/permitted authority may originate from an assessed invariant,
configured policy, a source contract, or customer-stated intended policy.
Observed authority must always originate from runtime evidence — a trace,
a state read, a sink/effect record. An assessed invariant must never be
labeled as an observed target property, and a target's representational
gap must never be labeled as a weakness in the invariant itself.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from ..models import Finding, Outcome
from .diff import AuthorityDiff
from .enums import DivergenceType, Provenance, SemanticAvailability
from .envelope import AuthorityEnvelope
from .fact import Fact
from .ledger import AuthorityLedger


def _fact_display(f: Fact | None) -> str:
    """Render one Fact's value for display, or its availability label if
    unavailable — never a generic placeholder, never silently blank."""
    if f is None:
        return "UNKNOWN"
    if f.is_available:
        return str(f.value)
    return f.availability.value


def _truncate(value: str, length: int = 12) -> str:
    """Compact display for long ids (e.g. a UUID run_id) — presentational
    only, the full value is always preserved in evidence/JSON output."""
    return value if len(value) <= length else value[:length] + "…"


# Presentation-only provenance category — never added to `enums.Provenance`
# (which stays frozen). See the module docstring's "ASSESSED_INVARIANT"
# section for exactly what this does and does not mean.
ASSESSED_INVARIANT = "ASSESSED_INVARIANT"


@dataclass(frozen=True)
class PathStep:
    """One node in an expected/observed Authority Path. `provenance` is a
    plain string — usually one of `enums.Provenance`'s values (via
    `Provenance.X.value`), but may also be `ASSESSED_INVARIANT` for an
    EXPECTED-side node that states Crypton's own security expectation
    rather than a value read from the target."""

    label: str
    provenance: str = "UNKNOWN"
    detail: str | None = None  # e.g. "args = {'amount': 20}"
    unexpected: bool = False  # strong marker (spec §6's "unexpected exercised path")
    availability: SemanticAvailability = SemanticAvailability.AVAILABLE
    evidence_ids: tuple[str, ...] = field(default_factory=tuple)

    def to_dict(self) -> dict[str, Any]:
        return {
            "label": self.label,
            "provenance": self.provenance,
            "detail": self.detail,
            "unexpected": self.unexpected,
            "availability": self.availability.value,
            "evidence_ids": list(self.evidence_ids),
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "PathStep":
        return cls(
            label=data["label"],
            provenance=data.get("provenance", "UNKNOWN"),
            detail=data.get("detail"),
            unexpected=data.get("unexpected", False),
            availability=SemanticAvailability(data.get("availability", "AVAILABLE")),
            evidence_ids=tuple(data.get("evidence_ids", [])),
        )


@dataclass(frozen=True)
class AuthorityHeader:
    """Component A. Only fields the Authority Model actually supports are
    populated; the renderer omits a field entirely rather than show a
    generic placeholder for one that isn't (spec §3)."""

    workflow: str | None = None
    principal: str | None = None
    actor: str | None = None
    execution_context: str | None = None
    property: str | None = None
    outcome: str | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "workflow": self.workflow,
            "principal": self.principal,
            "actor": self.actor,
            "execution_context": self.execution_context,
            "property": self.property,
            "outcome": self.outcome,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "AuthorityHeader":
        return cls(**data)


@dataclass(frozen=True)
class SemanticGap:
    """One missing-or-unresolved security-relevant fact (spec §7) — never
    silently dropped from the view."""

    field_label: str
    availability: SemanticAvailability
    note: str | None
    evidence_ids: tuple[str, ...] = field(default_factory=tuple)

    def to_dict(self) -> dict[str, Any]:
        return {
            "field_label": self.field_label,
            "availability": self.availability.value,
            "note": self.note,
            "evidence_ids": list(self.evidence_ids),
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "SemanticGap":
        return cls(
            field_label=data["field_label"],
            availability=SemanticAvailability(data["availability"]),
            note=data.get("note"),
            evidence_ids=tuple(data.get("evidence_ids", [])),
        )


@dataclass(frozen=True)
class EvidenceStripRow:
    element: str
    status: str  # a Provenance value (e.g. "OBSERVED") or a SemanticAvailability value (e.g. "UNREPRESENTABLE")
    evidence_ids: tuple[str, ...] = field(default_factory=tuple)

    def to_dict(self) -> dict[str, Any]:
        return {"element": self.element, "status": self.status, "evidence_ids": list(self.evidence_ids)}

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "EvidenceStripRow":
        return cls(element=data["element"], status=data["status"], evidence_ids=tuple(data.get("evidence_ids", [])))


@dataclass(frozen=True)
class ActionStripView:
    """Component — spec §12. `owner` defaults to the literal string
    'TBD' and is never filled with a guessed name (spec §12's explicit
    rule, carried over from the customer-context-gap discipline in the
    2026-09-30 remediation pass)."""

    required_action: str | None = None
    owner: str = "TBD"
    retest_condition: str | None = None

    def to_dict(self) -> dict[str, Any]:
        return {"required_action": self.required_action, "owner": self.owner, "retest_condition": self.retest_condition}

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "ActionStripView":
        return cls(required_action=data.get("required_action"), owner=data.get("owner", "TBD"), retest_condition=data.get("retest_condition"))


@dataclass(frozen=True)
class AuthorityView:
    """The full presentation projection — `authority-view.json`'s shape.
    Every field here is derived from the Authority Model (+ an optional
    `Finding` for the narrative/action fields); see `build_authority_view`."""

    view_id: str
    header: AuthorityHeader
    system_path: tuple[str, ...] | None  # optional, separate graph (spec §10) -- never derived from authority relationships
    expected_path: tuple[PathStep, ...]
    observed_path: tuple[PathStep, ...]  # the shared trunk (e.g. just "Approval X") before any branching
    observed_branches: tuple[tuple[PathStep, ...], ...]  # one sub-chain (Action [-> Effect]) per observed action -- rendered as siblings, never flattened into one misleading linear chain when there is more than one
    divergence_type: str
    divergence_point: str | None
    unexpected_exercise: tuple[str, ...]
    semantic_gaps: tuple[SemanticGap, ...]
    evidence_strip: tuple[EvidenceStripRow, ...]
    action: ActionStripView | None
    source_evidence_ids: tuple[str, ...]

    def to_dict(self) -> dict[str, Any]:
        return {
            "view_id": self.view_id,
            "header": self.header.to_dict(),
            "system_path": list(self.system_path) if self.system_path is not None else None,
            "expected_path": [s.to_dict() for s in self.expected_path],
            "observed_path": [s.to_dict() for s in self.observed_path],
            "observed_branches": [[s.to_dict() for s in branch] for branch in self.observed_branches],
            "divergence_type": self.divergence_type,
            "divergence_point": self.divergence_point,
            "unexpected_exercise": list(self.unexpected_exercise),
            "semantic_gaps": [g.to_dict() for g in self.semantic_gaps],
            "evidence_strip": [r.to_dict() for r in self.evidence_strip],
            "action": self.action.to_dict() if self.action else None,
            "source_evidence_ids": list(self.source_evidence_ids),
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "AuthorityView":
        action = data.get("action")
        system_path = data.get("system_path")
        return cls(
            view_id=data["view_id"],
            header=AuthorityHeader.from_dict(data["header"]),
            system_path=tuple(system_path) if system_path is not None else None,
            expected_path=tuple(PathStep.from_dict(s) for s in data.get("expected_path", [])),
            observed_path=tuple(PathStep.from_dict(s) for s in data.get("observed_path", [])),
            observed_branches=tuple(tuple(PathStep.from_dict(s) for s in branch) for branch in data.get("observed_branches", [])),
            divergence_type=data["divergence_type"],
            divergence_point=data.get("divergence_point"),
            unexpected_exercise=tuple(data.get("unexpected_exercise", [])),
            semantic_gaps=tuple(SemanticGap.from_dict(g) for g in data.get("semantic_gaps", [])),
            evidence_strip=tuple(EvidenceStripRow.from_dict(r) for r in data.get("evidence_strip", [])),
            action=ActionStripView.from_dict(action) if action else None,
            source_evidence_ids=tuple(data.get("source_evidence_ids", [])),
        )


# ── builder ──────────────────────────────────────────────────────────────


def _resolve_actor_label(ledger: AuthorityLedger, actor_id: str | None) -> tuple[str, Provenance]:
    if actor_id is None or actor_id not in ledger.actors:
        return "UNKNOWN", Provenance.UNKNOWN
    actor = ledger.actors[actor_id]
    return _fact_display(actor.actor_id), actor.provenance


def _resolve_principal_label(ledger: AuthorityLedger, principal_id: str | None) -> str:
    if principal_id is None or principal_id not in ledger.principals:
        return "UNKNOWN"
    return _fact_display(ledger.principals[principal_id].principal_id)


def _action_label(ledger: AuthorityLedger, action_key: str) -> tuple[str, str | None, Provenance, tuple[str, ...]]:
    action = ledger.actions.get(action_key)
    if action is None:
        return action_key, None, Provenance.UNKNOWN, ()
    real_id = _fact_display(action.action_id)
    label = f"Action {real_id}" if action.action_id and action.action_id.is_available else f"Action [{action.tool or 'UNKNOWN'}] (action_id={real_id})"
    detail = f"args = {action.arguments.value}" if action.arguments is not None and action.arguments.is_available else None
    return label, detail, action.provenance, action.evidence_ids


def _effect_label(ledger: AuthorityLedger, effect_key: str | None) -> tuple[str, Provenance, tuple[str, ...]] | None:
    if effect_key is None or effect_key not in ledger.effects:
        return None
    effect = ledger.effects[effect_key]
    label = f"Effect {effect.effect_id}" + (f" ({effect.effect_type})" if effect.effect_type else "")
    return label, effect.provenance, effect.evidence_ids


def _effect_key_for_action(ledger: AuthorityLedger, diff: AuthorityDiff, action_key: str) -> str | None:
    for effect_key in diff.observed_effect_ids:
        effect = ledger.effects.get(effect_key)
        action = ledger.actions.get(action_key)
        if effect is None or action is None:
            continue
        # Link by the local event ordering already established when the
        # fixture was built -- effects reference the same real action_id
        # (when known) or were added 1:1 with this action's local key.
        if effect.action_id is not None and action.action_id is not None and effect.action_id.is_available and action.action_id.is_available:
            if effect.action_id.value == action.action_id.value:
                return effect_key
    # Fall back to positional pairing when neither side has a resolvable
    # real id (e.g. Mastra's callId="unknown") -- both sequences were built
    # in the same order by the same fixture, so position is a presentation
    # convenience, not a claimed system fact.
    try:
        idx = list(diff.observed_action_ids).index(action_key)
        if idx < len(diff.observed_effect_ids):
            return diff.observed_effect_ids[idx]
    except ValueError:
        pass
    return None


def build_authority_view(
    ledger: AuthorityLedger,
    envelope: AuthorityEnvelope,
    diff: AuthorityDiff,
    *,
    view_id: str,
    workflow: str | None = None,
    system_path: tuple[str, ...] | None = None,
    finding: Finding | None = None,
    outcome_override: str | None = None,
    property_label_override: str | None = None,
) -> AuthorityView:
    """Project `ledger`/`envelope`/`diff` (+ optionally the real `Finding`
    this Authority Diff supports) into a flat, report-ready `AuthorityView`.

    `outcome_override` exists only for the two fixtures that have no
    formal `Finding` record of their own (Mastra Candidate 4a, EXP-003A) —
    each call site documents exactly which already-recorded evidence
    justifies the string it passes (see `fixtures.py`). This function
    itself only ever derives `NONE` divergence as "ENFORCED" on its own;
    it never prints "VIOLATED" without either a real `Finding.status` or
    an explicit, documented `outcome_override`.

    `property_label_override` (freeze-pass addition) exists for the same
    no-Finding fixtures, specifically to avoid the generic
    "Approval Scope"/"Delegation / Grant Scope" fallback below silently
    standing in for a property that already has a stable, documented
    identity (e.g. EXP-003A's H1 is `PROP-ARG-BINDING`, per
    `docs/methodology/08-exp003-property-id-mapping.md` and
    `property_mapping.py`). This function never derives a property name
    from an H-number itself — callers resolve the stable mapping and pass
    the result in.
    """
    # ── Header ────────────────────────────────────────────────────────
    principal_label = _resolve_principal_label(ledger, envelope.principal_id)
    actor_label, _ = _resolve_actor_label(ledger, envelope.actor_id)
    exec_ctx = envelope.execution_context_id
    if property_label_override is not None:
        property_label = property_label_override
    elif finding is not None:
        property_label = finding.area.label
    else:
        property_label = "Approval Scope" if envelope.required_approval else ("Delegation / Grant Scope" if envelope.grant_chain or envelope.allowed_tools is not None else None)

    if finding is not None:
        outcome_label = finding.status.value
    elif outcome_override is not None:
        outcome_label = outcome_override
    elif diff.divergence_type == DivergenceType.NONE:
        outcome_label = Outcome.ENFORCED.value
    else:
        outcome_label = None  # never guess a Gate-7-gated outcome

    header = AuthorityHeader(
        workflow=workflow,
        principal=principal_label,
        actor=actor_label,
        execution_context=_truncate(exec_ctx) if exec_ctx else None,
        property=property_label,
        outcome=outcome_label,
    )

    # ── Expected path ─────────────────────────────────────────────────
    expected_path: list[PathStep] = []
    if envelope.principal_id and envelope.principal_id in ledger.principals:
        p = ledger.principals[envelope.principal_id]
        expected_path.append(PathStep(label=f"Principal {_fact_display(p.principal_id)}", provenance=p.provenance.value, availability=p.principal_id.availability if p.principal_id else SemanticAvailability.UNKNOWN, evidence_ids=p.evidence_ids))

    if envelope.grant_chain or envelope.allowed_tools is not None or envelope.allowed_resources is not None:
        grant_obj = ledger.grants.get(envelope.grant_chain[-1]) if envelope.grant_chain else None
        grant_label = f"Grant {_fact_display(grant_obj.grant_id)}" if grant_obj else "Grant (no formal grant_id -- see derivation_notes)"
        tools = ", ".join(envelope.allowed_tools) if envelope.allowed_tools else "UNKNOWN"
        resources = ", ".join(envelope.allowed_resources) if envelope.allowed_resources else "UNKNOWN"
        expected_path.append(
            PathStep(
                label=grant_label,
                provenance=(grant_obj.provenance.value if grant_obj else Provenance.DERIVED.value),
                detail=f"allowed_tools=[{tools}], allowed_resources=[{resources}]",
                evidence_ids=envelope.source_evidence_ids,
            )
        )

    # Approval handling -- the EXPECTED side splits into two shapes,
    # depending on whether the approval's own binding to a specific action
    # is representable at all:
    #
    #   (a) Representable binding: the approval's own evidenced scope IS
    #       the expected side (unchanged from the prior pass).
    #   (b) Unrepresentable/unverifiable binding (e.g. Mastra's
    #       approveToolCall({runId}) with toolCallId omitted): the EXPECTED
    #       side must still state Crypton's assessed invariant -- "exactly
    #       one pending action should be resolved" -- as ASSESSED_INVARIANT,
    #       never as UNREPRESENTABLE. The target's inability to represent
    #       *which* action is a fact about the OBSERVED/target side, not
    #       about whether the invariant itself is known (freeze-pass fix;
    #       see module docstring).
    appr = ledger.approvals.get(envelope.approval_id) if envelope.approval_id else None
    binding_unrepresentable = (
        appr is not None and appr.target_action_ids is not None and not appr.target_action_ids.is_available
    )
    if envelope.required_approval:
        appr_label = f"Approval {_fact_display(appr.approval_id)}" if appr else "Approval"
        if binding_unrepresentable:
            expected_path.append(
                PathStep(
                    label=appr_label,
                    provenance=ASSESSED_INVARIANT,
                    detail="expected scope: one intended pending action\nbasis: assessed invariant",
                    evidence_ids=appr.evidence_ids if appr else (),
                )
            )
            expected_path.append(PathStep(label="Selected pending action", provenance=ASSESSED_INVARIANT))
        else:
            expected_path.append(
                PathStep(
                    label=appr_label,
                    provenance=(appr.provenance.value if appr else Provenance.UNKNOWN.value),
                    detail=envelope.approval_scope.value if envelope.approval_scope and envelope.approval_scope.is_available else None,
                    availability=envelope.approval_scope.availability if envelope.approval_scope else SemanticAvailability.AVAILABLE,
                    evidence_ids=appr.evidence_ids if appr else (),
                )
            )
            # Expected downstream node: the explicitly bound action, when
            # the binding is representable -- never a fabricated single
            # action when it isn't (that case is handled above).
            if appr is not None and appr.target_action_ids is not None and appr.target_action_ids.is_available:
                bound_ids = ", ".join(appr.target_action_ids.value)
                expected_path.append(PathStep(label=f"Action {bound_ids}", provenance=Provenance.OBSERVED.value))

    # ── Observed path: a shared trunk (e.g. the Approval event), plus one
    # branch per observed action -- rendered as siblings, never flattened
    # into a single chain that would misleadingly imply action 2 followed
    # from action 1 rather than both resolving under the same approval.
    observed_path: list[PathStep] = []
    if envelope.required_approval:
        appr_label = f"Approval {_fact_display(appr.approval_id)}" if appr else "Approval"
        # The OBSERVED side is where the target's representational gap
        # belongs -- stated as a plain fact about the target's own API/
        # schema, not a badge on the whole Approval node (freeze-pass fix).
        observed_detail = (
            f"target action binding: {appr.target_action_ids.availability.value}" if binding_unrepresentable else None
        )
        observed_path.append(
            PathStep(
                label=appr_label,
                provenance=(appr.provenance.value if appr else Provenance.UNKNOWN.value),
                detail=observed_detail,
            )
        )
    elif envelope.grant_chain or envelope.allowed_tools is not None:
        grant_obj = ledger.grants.get(envelope.grant_chain[-1]) if envelope.grant_chain else None
        grant_label = f"Grant {_fact_display(grant_obj.grant_id)}" if grant_obj else "Grant"
        observed_path.append(PathStep(label=grant_label, provenance=(grant_obj.provenance.value if grant_obj else Provenance.DERIVED.value)))

    def _action_display_label(action_key: str) -> str:
        """A description that is always meaningful, even when the action's
        own real id is unavailable -- falls back to tool+arguments rather
        than printing a bare availability word with no context."""
        action = ledger.actions.get(action_key)
        if action is None:
            return action_key
        if action.action_id is not None and action.action_id.is_available:
            return f"Action {action.action_id.value}"
        args = action.arguments.value if action.arguments is not None and action.arguments.is_available else None
        tool = action.tool or "UNKNOWN tool"
        return f"{tool} ({args})" if args else tool

    observed_branches: list[tuple[PathStep, ...]] = []
    for action_key in diff.observed_action_ids:
        label, detail, provenance, ev_ids = _action_label(ledger, action_key)
        is_unexpected = action_key in diff.unexpected_authority
        branch: list[PathStep] = [PathStep(label=label, provenance=provenance.value, detail=detail, unexpected=is_unexpected, evidence_ids=ev_ids)]
        effect_key = _effect_key_for_action(ledger, diff, action_key)
        effect_info = _effect_label(ledger, effect_key)
        if effect_info is not None:
            e_label, e_provenance, e_evidence = effect_info
            branch.append(PathStep(label=e_label, provenance=e_provenance.value, unexpected=is_unexpected, evidence_ids=e_evidence))
        observed_branches.append(tuple(branch))

    # ── Unexpected exercise summary (spec §4's "TC-B -> E2") -- always a
    # meaningful description, never a bare availability word.
    unexpected_exercise: list[str] = []
    for action_key in diff.unexpected_authority:
        action_display = _action_display_label(action_key)
        effect_key = _effect_key_for_action(ledger, diff, action_key)
        if effect_key and effect_key in ledger.effects:
            unexpected_exercise.append(f"{action_display} -> {ledger.effects[effect_key].effect_id}")
        else:
            unexpected_exercise.append(action_display)

    # ── Semantic gaps (spec §7) ───────────────────────────────────────
    semantic_gaps: list[SemanticGap] = []

    def _maybe_gap(label: str, f: Fact | None, evidence_ids: tuple[str, ...] = ()) -> None:
        if f is not None and not f.is_available:
            semantic_gaps.append(SemanticGap(field_label=label, availability=f.availability, note=f.note, evidence_ids=evidence_ids))

    if envelope.principal_id and envelope.principal_id in ledger.principals:
        p = ledger.principals[envelope.principal_id]
        _maybe_gap("Principal", p.principal_id, p.evidence_ids)
    if envelope.actor_id and envelope.actor_id in ledger.actors:
        a = ledger.actors[envelope.actor_id]
        _maybe_gap("Actor", a.actor_id, a.evidence_ids)
    if envelope.approval_id and envelope.approval_id in ledger.approvals:
        gap_appr = ledger.approvals[envelope.approval_id]
        _maybe_gap("Approver identity", gap_appr.approved_by, gap_appr.evidence_ids)
        # Named as the TARGET's representation, not the expected scope
        # itself -- the expected scope (an assessed invariant) is never a
        # semantic gap; see the approval-handling section above.
        _maybe_gap("Target representation of approval scope", gap_appr.target_action_ids, gap_appr.evidence_ids)
    _gap_tool_totals = {t: [ledger.actions[k].tool or k for k in diff.observed_action_ids if k in ledger.actions].count(t) for t in set(ledger.actions[k].tool or k for k in diff.observed_action_ids if k in ledger.actions)}
    _gap_tool_running: dict[str, int] = {}
    for action_key in diff.observed_action_ids:
        action = ledger.actions.get(action_key)
        if action is not None:
            # Index distinguishes multiple same-tool actions (e.g. two
            # "transfer" calls) rather than producing identical-looking gap
            # rows for genuinely different observed actions.
            tool_label = action.tool or action_key
            _gap_tool_running[tool_label] = _gap_tool_running.get(tool_label, 0) + 1
            suffix = f" #{_gap_tool_running[tool_label]}" if _gap_tool_totals[tool_label] > 1 else ""
            _maybe_gap(f"Action ID ({tool_label}{suffix})", action.action_id, action.evidence_ids)

    # ── Evidence strip (spec §8) -- derived, never hand-authored ───────
    evidence_strip: list[EvidenceStripRow] = []
    seen_rows: set[tuple[str, str]] = set()

    def _add_row(element: str, status: str, evidence_ids: tuple[str, ...]) -> None:
        key = (element, status)
        if key in seen_rows:
            return
        seen_rows.add(key)
        evidence_strip.append(EvidenceStripRow(element=element, status=status, evidence_ids=evidence_ids))

    if envelope.required_approval and envelope.approval_id in ledger.approvals:
        evidence_appr = ledger.approvals[envelope.approval_id]
        if binding_unrepresentable:
            # Two separate rows, matching the expected/observed split above:
            # the invariant itself (no target evidence needed -- it is
            # Crypton's own stated expectation, not a claim about the
            # target) and the target's representational gap (evidenced).
            _add_row("Expected approval scope", ASSESSED_INVARIANT, ())
            _add_row("Target representation of approval scope", evidence_appr.target_action_ids.availability.value, evidence_appr.evidence_ids)
        else:
            scope_status = evidence_appr.provenance.value if envelope.approval_scope is None or envelope.approval_scope.is_available else envelope.approval_scope.availability.value
            _add_row("Approval scope", scope_status, evidence_appr.evidence_ids)
        _add_row("Approver identity", evidence_appr.approved_by.availability.value if evidence_appr.approved_by and not evidence_appr.approved_by.is_available else evidence_appr.provenance.value, evidence_appr.evidence_ids)
    all_tools = [ledger.actions[k].tool or k for k in diff.observed_action_ids if k in ledger.actions]
    tool_totals = {t: all_tools.count(t) for t in set(all_tools)}
    tool_running_count: dict[str, int] = {}
    for action_key in diff.observed_action_ids:
        action = ledger.actions.get(action_key)
        if action is None:
            continue
        tool_name = action.tool or action_key
        tool_running_count[tool_name] = tool_running_count.get(tool_name, 0) + 1
        # Distinguish by occurrence index when the same tool appears more
        # than once -- otherwise two distinct, differently-evidenced rows
        # would collide under one label and the dedup-by-(element,status)
        # rule below would silently drop the second one.
        suffix = f" #{tool_running_count[tool_name]}" if tool_totals[tool_name] > 1 else ""
        _add_row(f"Action ({tool_name}{suffix})", action.provenance.value, action.evidence_ids)
        effect_key = _effect_key_for_action(ledger, diff, action_key)
        if effect_key and effect_key in ledger.effects:
            effect = ledger.effects[effect_key]
            _add_row(f"Effect ({effect.effect_type or effect_key}{suffix})", effect.provenance.value, effect.evidence_ids)
    for gap in semantic_gaps:
        _add_row(gap.field_label, gap.availability.value, gap.evidence_ids)

    # ── Action strip (spec §12 / freeze-pass point 6) ───────────────────
    # Only rendered when a real Finding backs it -- never fabricated, and
    # never shown as an empty "not yet recorded" placeholder for a fixture
    # that has no formal engagement Finding of its own (Mastra, EXP-003A).
    action_view: ActionStripView | None = None
    if finding is not None:
        action_view = ActionStripView(required_action=finding.recommendation, owner="TBD", retest_condition=finding.retest_condition)

    return AuthorityView(
        view_id=view_id,
        header=header,
        system_path=system_path,
        expected_path=tuple(expected_path),
        observed_path=tuple(observed_path),
        observed_branches=tuple(observed_branches),
        divergence_type=diff.divergence_type.value,
        divergence_point=diff.divergence_point,
        unexpected_exercise=tuple(unexpected_exercise),
        semantic_gaps=tuple(semantic_gaps),
        evidence_strip=tuple(evidence_strip),
        action=action_view,
        source_evidence_ids=diff.evidence_ids,
    )
