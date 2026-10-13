"""
Crypton Authority Model v0.1 — the Authority Diff.

Fourth layer: Raw Evidence -> Ledger -> Envelope -> **AUTHORITY DIFF** ->
Finding/Action. A diff compares one `AuthorityEnvelope` (the expected/
permitted authority) against an explicit, caller-named set of observed
`Action`s from the same ledger — it never scans a ledger and guesses which
actions are "the relevant ones" (spec §10's "no divergence without an
evidenced expected/permitted side and observed side").

A missing value is never, by itself, grounds for a divergence — only an
explicit mismatch between an evidenced expected dimension and an evidenced
observed fact produces one. Where the expected side itself could not be
established (UNREPRESENTABLE/UNVERIFIABLE), that is a semantic gap to
surface, not a manufactured divergence (spec §10's closing rule).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Sequence

from .enums import DivergenceType
from .ledger import AuthorityLedger
from .envelope import AuthorityEnvelope


@dataclass(frozen=True)
class AuthorityDiff:
    diff_id: str
    assessment_id: str | None = None
    test_id: str | None = None

    expected_envelope_id: str | None = None

    observed_action_ids: tuple[str, ...] = field(default_factory=tuple)
    observed_resources: tuple[str, ...] = field(default_factory=tuple)
    observed_effect_ids: tuple[str, ...] = field(default_factory=tuple)

    divergence_type: DivergenceType = DivergenceType.NONE
    divergence_point: str | None = None  # e.g. "Approval -> Action", "Grant", None when divergence_type is NONE

    unexpected_authority: tuple[str, ...] = field(default_factory=tuple)  # action_ids (or descriptions) exercised beyond envelope
    missing_expected_authority: tuple[str, ...] = field(default_factory=tuple)  # expected action_ids never observed

    evidence_ids: tuple[str, ...] = field(default_factory=tuple)
    confidence_note: str | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "diff_id": self.diff_id,
            "assessment_id": self.assessment_id,
            "test_id": self.test_id,
            "expected_envelope_id": self.expected_envelope_id,
            "observed_action_ids": list(self.observed_action_ids),
            "observed_resources": list(self.observed_resources),
            "observed_effect_ids": list(self.observed_effect_ids),
            "divergence_type": self.divergence_type.value,
            "divergence_point": self.divergence_point,
            "unexpected_authority": list(self.unexpected_authority),
            "missing_expected_authority": list(self.missing_expected_authority),
            "evidence_ids": list(self.evidence_ids),
            "confidence_note": self.confidence_note,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "AuthorityDiff":
        return cls(
            diff_id=data["diff_id"],
            assessment_id=data.get("assessment_id"),
            test_id=data.get("test_id"),
            expected_envelope_id=data.get("expected_envelope_id"),
            observed_action_ids=tuple(data.get("observed_action_ids", [])),
            observed_resources=tuple(data.get("observed_resources", [])),
            observed_effect_ids=tuple(data.get("observed_effect_ids", [])),
            divergence_type=DivergenceType(data.get("divergence_type", "NONE")),
            divergence_point=data.get("divergence_point"),
            unexpected_authority=tuple(data.get("unexpected_authority", [])),
            missing_expected_authority=tuple(data.get("missing_expected_authority", [])),
            evidence_ids=tuple(data.get("evidence_ids", [])),
            confidence_note=data.get("confidence_note"),
        )


def _resource_of(action) -> str | None:
    return action.actual_resource or action.intended_resource


def compute_diff(
    ledger: AuthorityLedger,
    envelope: AuthorityEnvelope,
    *,
    diff_id: str,
    observed_action_ids: Sequence[str],
) -> AuthorityDiff:
    """Compare `envelope` (the expected/permitted authority) against the
    explicit `observed_action_ids` (local keys into `ledger.actions`) the
    caller names as the exercised authority to check — never a ledger-wide
    scan. Each divergence rule below only fires on an explicit, evidenced
    mismatch; absence of information never manufactures a divergence."""
    evidence_ids: list[str] = list(envelope.source_evidence_ids)
    observed_resources: list[str] = []
    observed_effect_ids: list[str] = []

    actions = []
    for action_id in observed_action_ids:
        action = ledger.actions.get(action_id)
        if action is None:
            continue
        actions.append((action_id, action))
        evidence_ids.extend(action.evidence_ids)
        res = _resource_of(action)
        if res:
            observed_resources.append(res)
    for effect_id, effect in ledger.effects.items():
        if effect.action_id is not None and effect.action_id.is_available and effect.action_id.value in observed_action_ids:
            observed_effect_ids.append(effect_id)
            evidence_ids.extend(effect.evidence_ids)

    divergence_type = DivergenceType.NONE
    divergence_point: str | None = None
    unexpected: list[str] = []
    missing: list[str] = []
    confidence_parts: list[str] = []

    # ── Gate: revoked/expired authority still exercised ──────────────────
    if envelope.revocation_state == "REVOKED" and actions:
        divergence_type = DivergenceType.STALE_OR_REVOKED_AUTHORITY
        divergence_point = "Grant"
        unexpected = [aid for aid, _ in actions]
        confidence_parts.append("Envelope's governing grant is recorded REVOKED, yet action(s) were observed against it.")

    # ── Approval-scope mismatch: ambiguous or explicitly-exceeded scope ──
    elif envelope.required_approval and len(actions) > 0:
        scope = envelope.approval_scope
        if scope is not None and not scope.is_available:
            # Scope itself could not be represented/verified as bound to a
            # single action — if MORE THAN ONE action was observed resolved
            # under this approval, that is itself the divergence: an
            # approval whose own scope is unrepresentable/ambiguous cannot
            # be said to have authorized more than one action.
            if len(actions) > 1:
                divergence_type = DivergenceType.APPROVAL_SCOPE_MISMATCH
                divergence_point = "Approval -> Action"
                unexpected = [aid for aid, _ in actions[1:]]  # all but the first are "beyond" an ambiguous single-scope approval
                confidence_parts.append(
                    f"Approval's own scope is {scope.availability.value} ({scope.note}); "
                    f"{len(actions)} actions were nonetheless observed resolved under it, not at most one."
                )
        elif scope is not None and scope.is_available and isinstance(scope.value, str):
            # An explicit, available scope description exists but this v0.1
            # does not attempt free-text scope parsing — only the
            # ambiguous-scope case above and the explicit bound-ids case
            # below are evaluated structurally.
            pass

    # ── Explicit bound-action-id scope: anything beyond the bound set ────
    # Compared by each action's own REAL, system-level action_id (when
    # known) — never by this ledger's local bookkeeping keys, which live
    # in a different namespace (our own, not the target's) and must never
    # be judged against a real id set.
    if divergence_type == DivergenceType.NONE and envelope.required_approval and actions:
        appr = ledger.approvals.get(envelope.approval_id) if envelope.approval_id else None
        if appr is not None and appr.target_action_ids is not None and appr.target_action_ids.is_available:
            bound = set(appr.target_action_ids.value)
            resolved: dict[str, str | None] = {
                aid: (action.action_id.value if action.action_id is not None and action.action_id.is_available else None)
                for aid, action in actions
            }
            knowable = {aid: rid for aid, rid in resolved.items() if rid is not None}
            unknowable = [aid for aid, rid in resolved.items() if rid is None]
            beyond = sorted(aid for aid, rid in knowable.items() if rid not in bound)
            not_exercised = sorted(bound - set(knowable.values()))
            if beyond:
                divergence_type = DivergenceType.APPROVAL_SCOPE_MISMATCH
                divergence_point = "Approval -> Action"
                unexpected = beyond
                confidence_parts.append(
                    f"Approval explicitly bound to {sorted(bound)}; action(s) {beyond} (real id(s) "
                    f"{[knowable[a] for a in beyond]}) exceed that binding."
                )
            if not_exercised:
                missing = not_exercised
            if unknowable:
                confidence_parts.append(
                    f"Action(s) {unknowable} have no independently verifiable real action_id and could not "
                    "be checked against the approval's explicit binding — a semantic gap, not a cleared check."
                )

    # ── Scope expansion: an action outside the envelope's allowed tools/resources ──
    if divergence_type == DivergenceType.NONE and (envelope.allowed_tools is not None or envelope.allowed_resources is not None):
        out_of_scope: list[str] = []
        for aid, action in actions:
            if envelope.allowed_tools is not None and action.tool is not None and action.tool not in envelope.allowed_tools:
                out_of_scope.append(aid)
                continue
            res = _resource_of(action)
            if envelope.allowed_resources is not None and res is not None and res not in envelope.allowed_resources:
                out_of_scope.append(aid)
        if out_of_scope:
            divergence_type = DivergenceType.SCOPE_EXPANSION
            divergence_point = "Grant"
            unexpected = out_of_scope
            confidence_parts.append(
                f"Action(s) {out_of_scope} fall outside the envelope's allowed_tools/allowed_resources "
                f"(allowed_tools={envelope.allowed_tools}, allowed_resources={envelope.allowed_resources})."
            )

    # ── Argument/action substitution: two records for the same real action_id disagree ──
    if divergence_type == DivergenceType.NONE:
        by_real_id: dict[str, list[tuple[str, Any]]] = {}
        for aid, action in actions:
            if action.action_id is not None and action.action_id.is_available:
                by_real_id.setdefault(action.action_id.value, []).append((aid, action))
        for real_id, records in by_real_id.items():
            if len(records) > 1:
                arg_values = {
                    (a.arguments.value.__repr__() if a.arguments and a.arguments.is_available else None) for _, a in records
                }
                if len(arg_values) > 1:
                    divergence_type = DivergenceType.ACTION_OR_ARGUMENT_SUBSTITUTION
                    divergence_point = "Action -> Effect"
                    unexpected = [aid for aid, _ in records]
                    confidence_parts.append(f"Multiple records for action_id={real_id!r} disagree on arguments.")

    if divergence_type == DivergenceType.NONE:
        confidence_parts.append(
            "No evidenced mismatch found between the expected envelope and the named observed actions — "
            "this is a genuine no-divergence result, not an absence of checking."
        )

    return AuthorityDiff(
        diff_id=diff_id,
        assessment_id=envelope.assessment_id,
        test_id=envelope.test_id,
        expected_envelope_id=envelope.envelope_id,
        observed_action_ids=tuple(aid for aid, _ in actions),
        observed_resources=tuple(dict.fromkeys(observed_resources)),
        observed_effect_ids=tuple(observed_effect_ids),
        divergence_type=divergence_type,
        divergence_point=divergence_point,
        unexpected_authority=tuple(unexpected),
        missing_expected_authority=tuple(missing),
        evidence_ids=tuple(dict.fromkeys(evidence_ids)),
        confidence_note=" ".join(confidence_parts) if confidence_parts else None,
    )
