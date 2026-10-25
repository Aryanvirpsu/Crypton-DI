"""
v0.6.1 — Finding data + engagement metadata -> Markdown report.

**2026-09-30 remediation pass:** this is now the canonical report
structure (item 11 of the remediation spec,
`docs/audits/2026-09-30-evidence-report-readiness-audit.md` §4/§5.6) —
Decision Sheet, Authority Map / Authority Diff, Action Queue, Findings
Summary, Finding Cards, Verified Controls, Limitations, Retest Record,
Evidence Index, Technical Appendix, in that order. The three previously
overlapping report-template *documents*
(`docs/business/ASSESSMENT_PACK.md` §E, `docs/methodology/07-customer-report-template.md`,
`assessment-kit/v1.0/09-customer-deliverable-templates.md`) are reconciled
there, not here; this module is the one piece of *code* that actually
renders a report, and it now matches that reconciliation.

**Report-from-evidence invariant (item 12):** every section below either
renders a structured field straight from the `Engagement`/`Finding` data
it was given, or — for the two sections that have no natural per-finding
derivation (Authority Map/Diff, and prose Action Queue/Verified-Controls
narrative) — renders an explicit "not populated for this engagement"
statement rather than fabricating content to make the template look
complete. No section invents a material fact that isn't traceable to
engagement metadata, a Finding field, or an evidence reference.

Deterministic on purpose: the same `Engagement` always renders to the
exact same string. No `datetime.now()`, no random ordering, no
environment-dependent formatting — findings are sorted by `finding_id`,
evidence by `evidence_id`, and every other section pulls only from the
`Engagement` object it was given. This is what makes
`test_assessment_report.py::test_render_is_deterministic` a meaningful
test rather than a tautology, and it's what lets a report be regenerated
later (new findings appended, an old one corrected) without the rest of
the document silently reflowing in confusing ways.

This module renders whatever `Engagement` it is given — it does not
decide what counts as evidence, does not invent findings, and does not
call into `chain.py`/`delegation.py`/`authorization.py`/`approval.py` at
all. Whether the underlying findings are trustworthy is a question for
the assessor and the methodology doc, not this renderer.
"""

from __future__ import annotations

from .engagement import Engagement
from .models import AssessmentArea, Finding, Outcome, Severity

_OUTCOME_ORDER = {
    Outcome.VIOLATED: 0,
    Outcome.UNVERIFIABLE: 1,
    Outcome.UNREPRESENTABLE: 2,
    Outcome.NOT_APPLICABLE: 3,
    Outcome.ENFORCED: 4,
}

_SEVERITY_ORDER = {
    Severity.CRITICAL: 0,
    Severity.HIGH: 1,
    Severity.MEDIUM: 2,
    Severity.LOW: 3,
    Severity.INFORMATIONAL: 4,
}

_OUTCOME_MARK = {
    Outcome.ENFORCED: "ENFORCED",
    Outcome.VIOLATED: "VIOLATED",
    Outcome.UNREPRESENTABLE: "UNREPRESENTABLE",
    Outcome.UNVERIFIABLE: "UNVERIFIABLE",
    Outcome.NOT_APPLICABLE: "NOT APPLICABLE",
}

_GATE_LABELS = (
    ("g1_representability", "G1 Representability"),
    ("g2_observability", "G2 Observability"),
    ("g3_controlled_execution", "G3 Controlled execution"),
    ("g4_effect_confirmation", "G4 Effect confirmation"),
    ("g5_calibration_control", "G5 Calibration/control"),
    ("g6_reproduction", "G6 Reproduction"),
    ("g7_security_claim_triage", "G7 Security-claim triage"),
)


def _sorted_findings(findings: tuple[Finding, ...]) -> list[Finding]:
    return sorted(findings, key=lambda f: f.finding_id)


def _gate_tristate(value: bool | None) -> str:
    if value is True:
        return "satisfied"
    if value is False:
        return "not satisfied"
    return "not evaluated"


def _render_metadata(engagement: Engagement) -> str:
    lines = ["## Engagement Metadata", ""]
    rows = [
        ("Assessment ID", engagement.assessment_id),
        ("Client identifier", engagement.client_identifier),
        ("System identifier", engagement.system_identifier),
        ("Assessment type", engagement.assessment_type),
        ("Tested workflow", engagement.tested_workflow),
        ("Tested layer(s)", ", ".join(engagement.tested_layers) if engagement.tested_layers else None),
        ("Environment", engagement.environment),
        ("Target product", engagement.target_product),
        ("Target version", engagement.target_version),
        ("Target commit", engagement.target_commit),
        ("Testing window", engagement.testing_window),
        ("Configuration", engagement.configuration),
        ("Excluded scope", engagement.excluded_scope),
    ]
    lines.append("| Field | Value |")
    lines.append("|---|---|")
    any_set = False
    for label, value in rows:
        if value:
            any_set = True
        lines.append(f"| {label} | {value if value else '*(not recorded)*'} |")
    lines.append("")
    if not any_set:
        lines.append(
            "*No structured engagement metadata was recorded for this engagement — see Scope and "
            "Workflow in the Technical Appendix for whatever prose detail exists instead.*"
        )
        lines.append("")
    return "\n".join(lines)


def _render_decision_sheet(engagement: Engagement) -> str:
    lines = ["## 1. Decision Sheet", ""]
    lines.append(engagement.executive_summary.strip())
    lines.append("")
    counts: dict[Outcome, int] = {o: 0 for o in Outcome}
    for f in engagement.findings:
        counts[f.status] += 1
    lines.append("| Outcome | Count |")
    lines.append("|---|---|")
    for outcome in sorted(Outcome, key=lambda o: _OUTCOME_ORDER[o]):
        lines.append(f"| {_OUTCOME_MARK[outcome]} | {counts[outcome]} |")
    lines.append("")

    violated = [f for f in engagement.findings if f.status == Outcome.VIOLATED]
    if violated:
        rated = [f for f in violated if f.severity is not None]
        if rated:
            highest = min(rated, key=lambda f: _SEVERITY_ORDER[f.severity])
            lines.append(f"**Highest severity among VIOLATED findings:** {highest.severity.value} ({highest.finding_id})")
        else:
            lines.append("**VIOLATED findings exist, but none carry a severity rating.**")
        lines.append("")
    else:
        lines.append("**No VIOLATED findings in this engagement.**")
        lines.append("")
    return "\n".join(lines)


def _render_authority_map(engagement: Engagement) -> str:
    lines = ["## 2. Authority Map / Authority Diff", ""]
    if engagement.authority_map:
        lines.append(engagement.authority_map.strip())
    else:
        lines.append(
            "*Not populated for this engagement.* An Authority Map/Diff (expected authority — "
            "principal, delegation path, approval binding, action, resource, effect — versus "
            "observed) is not fabricated here; it is only rendered when an assessor has actually "
            "evidenced both sides. See Limitations for why, if applicable."
        )
    lines.append("")
    return "\n".join(lines)


def _render_action_queue(engagement: Engagement) -> str:
    lines = ["## 3. Action Queue", ""]
    violated = [f for f in _sorted_findings(engagement.findings) if f.status == Outcome.VIOLATED]
    if violated:
        lines.append("| Finding | Severity | Recommendation | Retest condition |")
        lines.append("|---|---|---|---|")
        for f in violated:
            sev = f.severity.value if f.severity else "(unrated)"
            rec = f.recommendation or "*(none recorded)*"
            retest = f.retest_condition or "*(not recorded)*"
            lines.append(f"| {f.finding_id} | {sev} | {rec} | {retest} |")
        lines.append("")
    if engagement.action_queue:
        lines.append(engagement.action_queue.strip())
        lines.append("")
    if not violated and not engagement.action_queue:
        lines.append("No outstanding actions recorded for this engagement.")
        lines.append("")
    return "\n".join(lines)


def _render_findings_summary(findings: tuple[Finding, ...]) -> str:
    lines = ["## 4. Findings Summary", ""]
    lines.append("| Outcome | Count |")
    lines.append("|---|---|")
    counts: dict[Outcome, int] = {o: 0 for o in Outcome}
    for f in findings:
        counts[f.status] += 1
    for outcome in sorted(Outcome, key=lambda o: _OUTCOME_ORDER[o]):
        lines.append(f"| {_OUTCOME_MARK[outcome]} | {counts[outcome]} |")
    lines.append("")

    violated_findings = [f for f in findings if f.status == Outcome.VIOLATED]
    if violated_findings:
        lines.append("**VIOLATED findings by severity:**")
        lines.append("")
        lines.append("| Severity | Count |")
        lines.append("|---|---|")
        sev_counts: dict[Severity, int] = {s: 0 for s in Severity}
        unrated = 0
        for f in violated_findings:
            if f.severity is None:
                unrated += 1
            else:
                sev_counts[f.severity] += 1
        for severity in sorted(Severity, key=lambda s: _SEVERITY_ORDER[s]):
            if sev_counts[severity]:
                lines.append(f"| {severity.value} | {sev_counts[severity]} |")
        if unrated:
            lines.append(f"| (unrated) | {unrated} |")
        lines.append("")

    lines.append("**Coverage by area:**")
    lines.append("")
    lines.append("| Area | Findings |")
    lines.append("|---|---|")
    for area in AssessmentArea:
        count = sum(1 for f in findings if f.area == area)
        lines.append(f"| {area.label} | {count} |")
    lines.append("")
    return "\n".join(lines)


def _render_finding_card(f: Finding, evidence_manifest) -> str:
    lines = [f"### {f.finding_id} — {f.title}", ""]
    lines.append(f"- **Area:** {f.area.label}")
    lines.append(f"- **Outcome:** {_OUTCOME_MARK[f.status]}")
    if f.severity is not None:
        lines.append(f"- **Severity:** {f.severity.value}")
    if f.property_id:
        lines.append(f"- **Property ID:** `{f.property_id}`" + (f" ({f.property_family})" if f.property_family else ""))
    if f.test_id:
        lines.append(f"- **Test ID:** `{f.test_id}`")
    if f.tested_layer:
        lines.append(f"- **Tested layer:** {f.tested_layer}")
    lines.append("")
    lines.append("**Observation**")
    lines.append("")
    lines.append(f.observation)
    lines.append("")
    lines.append("**Expected control**")
    lines.append("")
    lines.append(f.expected_control)
    lines.append("")
    if f.invariant:
        lines.append("**Invariant**")
        lines.append("")
        lines.append(f.invariant)
        lines.append("")
    if f.preconditions:
        lines.append("**Preconditions**")
        lines.append("")
        lines.append(f.preconditions)
        lines.append("")
    if f.stimulus:
        lines.append("**Stimulus**")
        lines.append("")
        lines.append(f.stimulus)
        lines.append("")
    if f.expected or f.observed:
        lines.append("**Expected vs. observed (test record)**")
        lines.append("")
        lines.append(f"- Expected: {f.expected or '*(not recorded)*'}")
        lines.append(f"- Observed: {f.observed or '*(not recorded)*'}")
        lines.append("")
    if f.effect:
        lines.append("**Effect**")
        lines.append("")
        lines.append(f.effect)
        lines.append("")
    if f.control_or_calibration:
        lines.append("**Control / calibration**")
        lines.append("")
        lines.append(f.control_or_calibration)
        lines.append("")
    if f.reproduction:
        lines.append(f"**Reproduction:** {f.reproduction}")
        lines.append("")
    if f.risk_impact:
        lines.append("**Risk / impact**")
        lines.append("")
        lines.append(f.risk_impact)
        lines.append("")
    if f.evidence_ids:
        lines.append("**Evidence**")
        lines.append("")
        for eid in f.evidence_ids:
            ev = evidence_manifest.get(eid)
            desc = f" — {ev.description}" if ev is not None else " — (unresolved reference)"
            lines.append(f"- `{eid}`{desc}")
        lines.append("")
    else:
        lines.append("**Evidence:** none cited.")
        lines.append("")
    if f.evidence_gate_status:
        lines.append("**Evidence gate status**")
        lines.append("")
        for field_name, label in _GATE_LABELS:
            lines.append(f"- {label}: {_gate_tristate(getattr(f.evidence_gate_status, field_name))}")
        if f.evidence_gate_status.notes:
            lines.append(f"- Notes: {f.evidence_gate_status.notes}")
        lines.append("")
    if f.recommendation:
        lines.append("**Recommendation**")
        lines.append("")
        lines.append(f.recommendation)
        lines.append("")
    if f.retest_condition:
        lines.append(f"**Retest condition:** {f.retest_condition}")
        lines.append("")
    if f.limitations:
        lines.append("**Limitations**")
        lines.append("")
        lines.append(f.limitations)
        lines.append("")
    if f.confidence:
        lines.append(f"**Confidence:** {f.confidence}")
        lines.append("")
    return "\n".join(lines)


def _render_finding_cards(findings: tuple[Finding, ...], evidence_manifest) -> str:
    lines = ["## 5. Finding Cards", ""]
    if not findings:
        lines.append("No findings recorded for this engagement.")
        lines.append("")
        return "\n".join(lines)
    for f in _sorted_findings(findings):
        lines.append(_render_finding_card(f, evidence_manifest))
    return "\n".join(lines)


def _render_verified_controls(engagement: Engagement) -> str:
    lines = ["## 6. Verified Controls", ""]
    enforced = [f for f in _sorted_findings(engagement.findings) if f.status == Outcome.ENFORCED]
    if enforced:
        lines.append("| Finding | Title | Evidence |")
        lines.append("|---|---|---|")
        for f in enforced:
            ev = ", ".join(f"`{e}`" for e in f.evidence_ids) if f.evidence_ids else "*(none)*"
            lines.append(f"| {f.finding_id} | {f.title} | {ev} |")
        lines.append("")
    if engagement.verified_controls:
        lines.append(engagement.verified_controls.strip())
        lines.append("")
    if not enforced and not engagement.verified_controls:
        lines.append("No verified controls recorded for this engagement.")
        lines.append("")
    return "\n".join(lines)


def _render_limitations(engagement: Engagement) -> str:
    lines = ["## 7. Limitations", ""]
    lines.append(engagement.limitations.strip())
    lines.append("")
    return "\n".join(lines)


def _render_retest_record(findings: tuple[Finding, ...]) -> str:
    lines = ["## 8. Retest Record", ""]
    with_retest = [f for f in _sorted_findings(findings) if f.retest_condition]
    if not with_retest:
        lines.append("No retest conditions recorded for this engagement — do not infer one.")
        lines.append("")
        return "\n".join(lines)
    lines.append("| Finding | Outcome | Retest condition |")
    lines.append("|---|---|---|")
    for f in with_retest:
        lines.append(f"| {f.finding_id} | {_OUTCOME_MARK[f.status]} | {f.retest_condition} |")
    lines.append("")
    return "\n".join(lines)


def _render_evidence_index(evidence_manifest) -> str:
    lines = ["## 9. Evidence Index", ""]
    entries = sorted(evidence_manifest, key=lambda e: e.evidence_id)
    if not entries:
        lines.append("No evidence recorded for this engagement.")
        lines.append("")
        return "\n".join(lines)
    lines.append("| ID | Type | Source | Description | Reference | Classification |")
    lines.append("|---|---|---|---|---|---|")
    for e in entries:
        ref = e.reference or ""
        classification = e.classification.value if e.classification else "*(not recorded)*"
        lines.append(f"| `{e.evidence_id}` | {e.evidence_type} | {e.source} | {e.description} | {ref} | {classification} |")
    lines.append("")
    return "\n".join(lines)


def _render_technical_appendix(engagement: Engagement) -> str:
    lines = ["## 10. Technical Appendix", ""]
    lines.append("### Scope")
    lines.append("")
    lines.append(engagement.scope.strip())
    lines.append("")
    lines.append("### Workflow Assessed")
    lines.append("")
    lines.append(engagement.workflow.strip())
    lines.append("")
    lines.append("### Methodology")
    lines.append("")
    lines.append(
        "Findings above use the fixed five-outcome vocabulary defined in "
        "`docs/ASSESSMENT_METHODOLOGY.md`: **ENFORCED** (the claimed security property was "
        "evidenced and held), **VIOLATED** (evidenced, every evidence-sufficiency gate satisfied, "
        "and Gate 7 security-claim/trust-boundary triage affirmatively confirmed a real claimed "
        "boundary was crossed), **UNREPRESENTABLE** (the target has no semantic state to express "
        "this property at all), **UNVERIFIABLE** (the property is representable but sufficient "
        "evidence could not be obtained), and **NOT_APPLICABLE** (the control does not apply to "
        "this workflow). Every ENFORCED and VIOLATED finding above cites at least one entry in "
        "§9's evidence index and a fully satisfied seven-gate evidence_gate_status (G1 "
        "Representability, G2 Observability, G3 Controlled execution, G4 Effect confirmation, G5 "
        "Calibration/control, G6 Reproduction, G7 Security-claim triage) — passing G1-G6 alone is "
        "never sufficient for VIOLATED. Severity, where set, follows the contextual rubric in the "
        "methodology document — it is not a fixed mapping from finding category to severity level."
    )
    lines.append("")
    unique_recs: list[str] = []
    seen: set[str] = set()
    for f in _sorted_findings(engagement.findings):
        if f.recommendation and f.recommendation not in seen:
            unique_recs.append(f"- **{f.finding_id}:** {f.recommendation}")
            seen.add(f.recommendation)
    if unique_recs:
        lines.append("### All Recommendations (also itemized per-VIOLATED-finding in §3)")
        lines.append("")
        lines.extend(unique_recs)
        lines.append("")
    return "\n".join(lines)


def render_report(engagement: Engagement) -> str:
    """Render `engagement` to a Markdown findings report. Pure function —
    same input, same output, every time."""
    parts: list[str] = []

    parts.append(f"# {engagement.title}")
    parts.append("")
    parts.append(f"**{engagement.label}**")
    parts.append("")
    parts.append(f"Date: {engagement.date}  ")
    parts.append(f"Assessor: {engagement.assessor}")
    parts.append("")
    parts.append("---")
    parts.append("")

    parts.append(_render_metadata(engagement))
    parts.append(_render_decision_sheet(engagement))
    parts.append(_render_authority_map(engagement))
    parts.append(_render_action_queue(engagement))
    parts.append(_render_findings_summary(engagement.findings))
    parts.append(_render_finding_cards(engagement.findings, engagement.evidence))
    parts.append(_render_verified_controls(engagement))
    parts.append(_render_limitations(engagement))
    parts.append(_render_retest_record(engagement.findings))
    parts.append(_render_evidence_index(engagement.evidence))
    parts.append(_render_technical_appendix(engagement))

    return "\n".join(parts).rstrip() + "\n"
