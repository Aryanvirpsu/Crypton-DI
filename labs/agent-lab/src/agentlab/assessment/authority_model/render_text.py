"""
Crypton Authority Map / Authority Diff — text rendering (v0.1).

Renders an `AuthorityView` (the presentation projection — see
`presentation.py`) into the three customer-facing components: the
Authority Header, the Authority Diff (or the plain Authority Map, when no
comparison is the point), the Evidence Strip, and the Action Strip. This
module never reads a `Ledger`/`Envelope`/`Diff` directly — it renders
exactly what `AuthorityView` already contains, so every line on the page
traces back through one place (`presentation.build_authority_view`) to the
Authority Model.

**Provenance visual language (spec §6):** one consistent grammar, grayscale-
safe (no reliance on color):

    OBSERVED           ──────
    CONFIGURED         - - - -
    CUSTOMER_ASSERTED  ········
    DERIVED            [derived]
    ASSESSED_INVARIANT [invariant]  (presentation-only category — see presentation.py)
    UNKNOWN            ?
    unexpected edge    ======  (used instead of the normal connector)

Plain text only — no HTML, no graphics, per spec §16/§18's explicit v0.1
scope limit.

`PathStep.provenance` is a plain string (not the frozen `Provenance` enum
directly — see `presentation.py`'s module docstring for why), so the
connector table below is keyed by string, not enum member.
"""

from __future__ import annotations

from .enums import DivergenceType, SemanticAvailability
from .presentation import ASSESSED_INVARIANT, AuthorityView, PathStep

_CONNECTOR = {
    "OBSERVED": "────",  # ────
    "CONFIGURED": "- - -",
    "CUSTOMER_ASSERTED": "········",  # ········
    "DERIVED": "[derived]",
    ASSESSED_INVARIANT: "[invariant]",
    "UNKNOWN": "?",
}
_UNEXPECTED_CONNECTOR = "════"  # ════


def _connector(step: PathStep) -> str:
    if step.unexpected:
        return _UNEXPECTED_CONNECTOR
    return _CONNECTOR.get(step.provenance, "?")


def _step_text(step: PathStep) -> list[str]:
    lines = [step.label]
    if step.availability != SemanticAvailability.AVAILABLE:
        lines[0] = f"{step.label} [{step.availability.value}]"
    if step.detail:
        # detail may itself be multi-line (e.g. "expected scope: ...\nbasis:
        # assessed invariant") -- indent every physical line, not just the
        # string's first one.
        lines.extend(f"    {detail_line}" for detail_line in step.detail.split("\n"))
    if step.unexpected:
        lines.append("    << UNEXPECTED AUTHORITY")
    return lines


def _render_linear_chain(steps: tuple[PathStep, ...], indent: str = "") -> list[str]:
    lines: list[str] = []
    for i, step in enumerate(steps):
        for text_line in _step_text(step):
            lines.append(f"{indent}{text_line}")
        if i < len(steps) - 1:
            lines.append(f"{indent}  |")
            lines.append(f"{indent}  {_connector(steps[i + 1])}")
            lines.append(f"{indent}  v")
    return lines


def _render_trunk_and_branches(trunk: tuple[PathStep, ...], branches: tuple[tuple[PathStep, ...], ...]) -> list[str]:
    """Renders a shared prefix (e.g. one Approval or Grant event) followed
    by one or more sibling branches (e.g. each observed action) -- never
    flattens multiple branches into one chain, which would misleadingly
    imply the second action followed from the first rather than both
    resolving under the same shared authority event (spec §4)."""
    lines: list[str] = []
    lines.extend(_render_linear_chain(trunk))

    if not branches:
        return lines

    if len(branches) == 1:
        # No real branching to show -- render as a simple continuation of
        # the trunk, connector drawn from the branch's own first step.
        if trunk:
            lines.append("  |")
            lines.append(f"  {_connector(branches[0][0])}")
            lines.append("  v")
        lines.extend(_render_linear_chain(branches[0]))
        return lines

    if trunk:
        lines.append("  |")
    for i, branch in enumerate(branches):
        # Branch joint itself carries the "unexpected" emphasis (spec §6's
        # strong marker); per-step provenance (OBSERVED/CONFIGURED/...)
        # still shows on each node via `_step_text`'s [AVAILABILITY] tag
        # when relevant, so nothing is lost by keeping the joint simple.
        marker = "+==" if branch[0].unexpected else "+--"
        first_lines = _step_text(branch[0])
        lines.append(f"  {marker}> {first_lines[0]}")
        for extra_line in first_lines[1:]:
            lines.append(f"  |   {extra_line}")
        for step in branch[1:]:
            lines.append("  |       |")
            lines.append("  |       v")
            for text_line in _step_text(step):
                lines.append(f"  |   {text_line}")
        if i < len(branches) - 1:
            lines.append("  |")
    return lines


def render_authority_header(view: AuthorityView) -> str:
    rows = [
        ("WORKFLOW", view.header.workflow),
        ("PRINCIPAL", view.header.principal),
        ("ACTOR", view.header.actor),
        ("EXECUTION CONTEXT", view.header.execution_context),
        ("PROPERTY", view.header.property),
        ("OUTCOME", view.header.outcome),
    ]
    lines = [f"{label:<20}{value}" for label, value in rows if value is not None]
    return "\n".join(lines) + "\n"


def render_authority_map_structural(view: AuthorityView) -> str:
    """The plain Authority Map (spec §9) — structure only, no comparison.
    Used when explaining authority shape rather than reporting a finding."""
    lines: list[str] = []
    if view.system_path:
        lines.append("SYSTEM PATH")
        lines.append("")
        lines.append(" -> ".join(view.system_path))
        lines.append("")
        lines.append("")
    lines.append("AUTHORITY PATH")
    lines.append("")
    if view.expected_path:
        lines.extend(_render_linear_chain(view.expected_path))
    else:
        lines.append("(no authority structure could be derived from this ledger's evidence)")
    return "\n".join(lines) + "\n"


def render_authority_diff(view: AuthorityView) -> str:
    """The main customer-facing visual (spec §4/§5) — a two-state
    comparison for a divergence, or the explicit "NO AUTHORITY DIVERGENCE
    OBSERVED" grammar for a clean result. Never renders an empty-looking
    page for an ENFORCED case (spec §5's explicit rule)."""
    lines: list[str] = []

    lines.append("EXPECTED / PERMITTED")
    lines.append("")
    if view.expected_path:
        lines.extend(_render_linear_chain(view.expected_path))
    else:
        lines.append("(no expected-authority dimension could be derived from this ledger's evidence)")
    lines.append("")
    lines.append("")

    lines.append("OBSERVED")
    lines.append("")
    if view.observed_path or view.observed_branches:
        lines.extend(_render_trunk_and_branches(view.observed_path, view.observed_branches))
    else:
        lines.append("(no observed actions named for this diff)")
    lines.append("")
    lines.append("")

    if view.divergence_type == DivergenceType.NONE.value:
        lines.append("NO AUTHORITY DIVERGENCE OBSERVED")
        lines.append("")
        if view.header.property:
            lines.append(view.header.property)
        if view.header.outcome:
            lines.append(view.header.outcome)
    else:
        lines.append("FIRST DIVERGENCE")
        lines.append(view.divergence_point or "UNKNOWN")
        lines.append("")
        lines.append("TYPE")
        lines.append(view.divergence_type)
        lines.append("")
        lines.append("UNEXPECTED EXERCISE")
        if view.unexpected_exercise:
            for item in view.unexpected_exercise:
                lines.append(item)
        else:
            lines.append("NONE")

    if view.semantic_gaps:
        lines.append("")
        lines.append("")
        lines.append("SEMANTIC GAPS")
        lines.append("")
        for gap in view.semantic_gaps:
            note = f" -- {gap.note}" if gap.note else ""
            lines.append(f"{gap.field_label}")
            lines.append(f"{gap.availability.value}{note}")
            lines.append("")

    return "\n".join(lines).rstrip() + "\n"


def render_evidence_strip(view: AuthorityView) -> str:
    """Markdown table (spec §8) — derived entirely from `view.evidence_strip`,
    never hand-authored independently of the model."""
    lines = ["| Element | Status / Basis | Evidence |", "|---|---|---|"]
    if not view.evidence_strip:
        lines.append("| *(none)* | | |")
    for row in view.evidence_strip:
        evidence = ", ".join(row.evidence_ids) if row.evidence_ids else "*(none)*"
        lines.append(f"| {row.element} | {row.status} | {evidence} |")
    return "\n".join(lines) + "\n"


def render_action_strip(view: AuthorityView) -> str:
    if view.action is None:
        return "No action required -- no divergence observed.\n"
    lines = ["ACTION REQUIRED", ""]
    lines.append(view.action.required_action or "*(not yet recorded)*")
    lines.append("")
    lines.append("OWNER")
    lines.append(view.action.owner)
    lines.append("")
    lines.append("RETEST PASSES WHEN")
    lines.append(view.action.retest_condition or "*(not yet recorded)*")
    return "\n".join(lines) + "\n"


def render_full_text_report(view: AuthorityView) -> str:
    """Convenience: header + diff + evidence strip + action strip, in the
    finding-card order from spec §13/§14. This is what `authority-diff.txt`
    contains."""
    parts = [
        render_authority_header(view),
        "",
        render_authority_diff(view),
        "",
        "EVIDENCE",
        "",
        render_evidence_strip(view),
    ]
    if view.action is not None:
        parts.append("")
        parts.append(render_action_strip(view))
    return "\n".join(parts).rstrip() + "\n"
