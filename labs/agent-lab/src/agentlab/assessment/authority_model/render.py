"""
Crypton Authority Model v0.1 — minimal text Authority Map renderer.

Fifth layer, read-only: Ledger + Envelope + Diff -> **TEXT RENDERING**.
Generated from the model, never hand-authored (spec §16's "the Authority
Map should be generated from the model rather than hand-authored" — see
also Definition of Done item G). Renders only fields the model actually
contains; an unknown/unrepresentable value is shown as the literal word
UNKNOWN/UNREPRESENTABLE/UNVERIFIABLE, never silently omitted, since its
absence may itself be security-relevant (spec §16).

No graphical UI, no HTML — plain text only, per spec §16's explicit scope
limit for v0.1.

**Superseded for the customer-facing report surface (2026-10-01):** the
Authority Map / Authority Diff *report surface* pass added
`presentation.py` (the `AuthorityView` projection) and `render_text.py`
(the Authority Header / Diff / Evidence Strip / Action Strip renderers) —
those are what `fixtures.py`'s output generation and the report
integration use now. This module's `render_authority_map` function is
kept, unchanged, because existing tests and call sites still use its
simpler `(ledger, envelope, diff) -> str` signature directly; it is not
deprecated, just no longer the primary customer-facing renderer.
"""

from __future__ import annotations

from .diff import AuthorityDiff
from .envelope import AuthorityEnvelope
from .fact import Fact
from .ledger import AuthorityLedger


def _fact_str(value: Fact | None, label: str = "") -> str:
    if value is None:
        return "UNKNOWN" + (f" ({label})" if label else "")
    return value.render()


def _action_label(ledger: AuthorityLedger, action_id: str) -> str:
    action = ledger.actions.get(action_id)
    if action is None:
        return f"Action {action_id}"
    id_str = _fact_str(action.action_id)
    tool = action.tool or "UNKNOWN tool"
    return f"Action[{action_id}] {tool} (action_id={id_str})"


def _effect_label(ledger: AuthorityLedger, effect_id: str) -> str:
    effect = ledger.effects.get(effect_id)
    if effect is None:
        return f"Effect {effect_id}"
    etype = effect.effect_type or "UNKNOWN effect_type"
    return f"Effect[{effect_id}] {etype}"


def render_authority_map(ledger: AuthorityLedger, envelope: AuthorityEnvelope, diff: AuthorityDiff) -> str:
    lines: list[str] = []

    has_grant_dimension = envelope.grant_chain or envelope.allowed_tools is not None or envelope.allowed_resources is not None
    lines.append("EXPECTED")
    lines.append("")
    if has_grant_dimension:
        if envelope.grant_chain:
            lines.append(f"Grant chain: {' -> '.join(envelope.grant_chain)}")
        else:
            lines.append("Grant (no formal grant_id — see derivation_notes)")
        lines.append(f"    allowed_tools: {list(envelope.allowed_tools) if envelope.allowed_tools is not None else 'UNKNOWN'}")
        lines.append(
            f"    allowed_resources: {list(envelope.allowed_resources) if envelope.allowed_resources is not None else 'UNKNOWN'}"
        )
        lines.append(f"    revocation_state: {envelope.revocation_state}")
    if envelope.required_approval:
        lines.append(f"Approval {envelope.approval_id or 'UNKNOWN'}")
        lines.append(f"    scope: {_fact_str(envelope.approval_scope)}")
        lines.append("    |")
        lines.append("    v")
        if diff.expected_envelope_id:
            lines.append("    (expected: at most the action(s) this approval's own scope binds)")
    if not has_grant_dimension and not envelope.required_approval:
        lines.append("(no grant or approval dimension could be derived from this ledger's evidence)")
    lines.append("")

    lines.append("OBSERVED")
    lines.append("")
    if envelope.required_approval:
        lines.append(f"Approval {envelope.approval_id or 'UNKNOWN'}")
        for i, action_id in enumerate(diff.observed_action_ids):
            connector = "+-->" if len(diff.observed_action_ids) > 1 else "--->"
            effect_ids = [
                eid for eid in diff.observed_effect_ids
                if ledger.effects.get(eid) and ledger.effects[eid].action_id and ledger.effects[eid].action_id.value == (
                    ledger.actions[action_id].action_id.value if ledger.actions[action_id].action_id and ledger.actions[action_id].action_id.is_available else None
                )
            ]
            effect_suffix = f" --> {_effect_label(ledger, effect_ids[0])}" if effect_ids else ""
            prefix = "    |\n    " if i > 0 else "    "
            lines.append(f"{prefix}{connector} {_action_label(ledger, action_id)}{effect_suffix}")
    else:
        for action_id in diff.observed_action_ids:
            lines.append(f"    {_action_label(ledger, action_id)}")
    if not diff.observed_action_ids:
        lines.append("(no observed actions named for this diff)")
    lines.append("")

    lines.append("DIVERGENCE")
    lines.append("")
    if diff.divergence_type.value == "NONE":
        lines.append("NONE")
    else:
        lines.append(diff.divergence_point or "UNKNOWN divergence point")
        lines.append(diff.divergence_type.value)
    lines.append("")
    lines.append("Unexpected exercised authority:")
    if diff.unexpected_authority:
        for aid in diff.unexpected_authority:
            lines.append(f"  {_action_label(ledger, aid)}" if aid in ledger.actions else f"  {aid}")
    else:
        lines.append("  NONE")
    lines.append("")
    lines.append("Missing expected authority:")
    if diff.missing_expected_authority:
        for aid in diff.missing_expected_authority:
            lines.append(f"  {aid}")
    else:
        lines.append("  NONE")
    lines.append("")
    lines.append("Evidence:")
    if diff.evidence_ids:
        for eid in diff.evidence_ids:
            lines.append(f"  {eid}")
    else:
        lines.append("  NONE")
    if diff.confidence_note:
        lines.append("")
        lines.append(f"Note: {diff.confidence_note}")

    return "\n".join(lines) + "\n"
