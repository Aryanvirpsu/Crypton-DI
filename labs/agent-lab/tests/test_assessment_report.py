"""
v0.6.1 — assessment.report: deterministic Markdown rendering.

2026-09-30 remediation pass: rewritten for the canonical 10-section report
structure (Decision Sheet, Authority Map/Diff, Action Queue, Findings
Summary, Finding Cards, Verified Controls, Limitations, Retest Record,
Evidence Index, Technical Appendix) and the five-outcome vocabulary. See
docs/audits/2026-09-30-evidence-report-readiness-audit.md §4/§5.6 and
item 11 of the remediation spec.
"""

import pytest

from agentlab.assessment.engagement import Engagement, EngagementLoadError
from agentlab.assessment.evidence import EvidenceManifest
from agentlab.assessment.models import (
    AssessmentArea,
    EvidenceClassification,
    EvidenceGateStatus,
    EvidenceReference,
    Finding,
    Outcome,
    Severity,
)
from agentlab.assessment.report import render_report

_FULL_GATES = EvidenceGateStatus(
    g1_representability=True,
    g2_observability=True,
    g3_controlled_execution=True,
    g4_effect_confirmation=True,
    g5_calibration_control=True,
    g6_reproduction=True,
    g7_security_claim_triage=True,
)


def _sample_engagement(**overrides) -> Engagement:
    evidence = EvidenceManifest(
        [
            EvidenceReference(
                evidence_id="E-001",
                evidence_type="authorization_log",
                source="s1",
                description="a real ALLOW decision",
                classification=EvidenceClassification.CUSTOMER_CONFIDENTIAL,
            ),
            EvidenceReference(evidence_id="E-002", evidence_type="config_excerpt", source="s2", description="an over-broad grant"),
        ]
    )
    findings = (
        Finding(
            finding_id="AD-01",
            area=AssessmentArea.PRE_EXECUTION_BOUNDARY,
            title="In-scope call allowed",
            status=Outcome.ENFORCED,
            observation="observed A",
            expected_control="expect A",
            evidence_ids=("E-001",),
            recommendation="Enforce authorization immediately before execution.",
            evidence_gate_status=_FULL_GATES,
        ),
        Finding(
            finding_id="AD-02",
            area=AssessmentArea.DELEGATION_AUTHORITY_FLOW,
            title="Over-broad grant",
            status=Outcome.VIOLATED,
            severity=Severity.MEDIUM,
            observation="observed B",
            expected_control="expect B",
            evidence_ids=("E-002",),
            risk_impact="some impact",
            recommendation="Scope grants to least privilege.",
            retest_condition="Re-verify after the grant is narrowed.",
            evidence_gate_status=_FULL_GATES,
        ),
        Finding(
            finding_id="AD-03",
            area=AssessmentArea.IDENTITY_OWNERSHIP,
            title="Provisioning process",
            status=Outcome.UNVERIFIABLE,
            observation="observed C",
            expected_control="expect C",
            evidence_ids=(),
            limitations="no external record source",
        ),
        Finding(
            finding_id="AD-04",
            area=AssessmentArea.REVOCATION_EXPIRY,
            title="Live timing",
            status=Outcome.NOT_APPLICABLE,
            observation="observed D",
            expected_control="expect D",
            evidence_ids=(),
        ),
    )
    base = dict(
        title="Sample Engagement",
        label="Crypton Reference Agent Lab — Methodology Dry Run",
        date="2026-09-02",
        assessor="Test Assessor",
        scope="Sample scope text.",
        workflow="Sample workflow text.",
        limitations="Sample limitations text.",
        executive_summary="Sample executive summary.",
        findings=findings,
        evidence=evidence,
    )
    base.update(overrides)
    return Engagement(**base)


# ── all ten canonical sections render ────────────────────────────────────


def test_report_contains_all_ten_canonical_sections():
    report = render_report(_sample_engagement())
    for heading in [
        "## 1. Decision Sheet",
        "## 2. Authority Map / Authority Diff",
        "## 3. Action Queue",
        "## 4. Findings Summary",
        "## 5. Finding Cards",
        "## 6. Verified Controls",
        "## 7. Limitations",
        "## 8. Retest Record",
        "## 9. Evidence Index",
        "## 10. Technical Appendix",
    ]:
        assert heading in report, f"missing section: {heading}"


def test_report_header_shows_the_engagement_label_prominently():
    report = render_report(_sample_engagement())
    assert "Crypton Reference Agent Lab — Methodology Dry Run" in report
    # must appear before the decision sheet, not buried
    assert report.index("Crypton Reference Agent Lab") < report.index("## 1. Decision Sheet")


def test_report_renders_structured_engagement_metadata():
    engagement = _sample_engagement(
        assessment_id="ENG-0001",
        target_product="ExampleCo Agent Platform",
        target_version="3.4.0",
    )
    report = render_report(engagement)
    assert "ENG-0001" in report
    assert "ExampleCo Agent Platform" in report
    assert "3.4.0" in report


def test_report_states_when_no_structured_metadata_was_recorded():
    report = render_report(_sample_engagement())  # no assessment_id/target_* set
    assert "not recorded" in report.lower()


# ── findings reference evidence correctly ────────────────────────────────


def test_finding_evidence_citation_shows_the_evidence_description():
    report = render_report(_sample_engagement())
    assert "`E-001`" in report
    assert "a real ALLOW decision" in report
    assert "`E-002`" in report
    assert "an over-broad grant" in report


def test_evidence_index_lists_every_manifest_entry_with_classification():
    report = render_report(_sample_engagement())
    evidence_section = report.split("## 9. Evidence Index")[1].split("## 10. Technical Appendix")[0]
    assert "E-001" in evidence_section
    assert "E-002" in evidence_section
    assert "CUSTOMER_CONFIDENTIAL" in evidence_section


# ── all five outcomes render correctly ───────────────────────────────────


def test_all_five_outcomes_render_distinctly():
    report = render_report(_sample_engagement())
    assert "**Outcome:** ENFORCED" in report
    assert "**Outcome:** VIOLATED" in report
    assert "**Outcome:** UNVERIFIABLE" in report
    assert "**Outcome:** NOT APPLICABLE" in report


def test_findings_summary_counts_are_correct():
    report = render_report(_sample_engagement())
    results_section = report.split("## 4. Findings Summary")[1].split("## 5. Finding Cards")[0]
    assert "| VIOLATED | 1 |" in results_section
    assert "| UNVERIFIABLE | 1 |" in results_section
    assert "| NOT APPLICABLE | 1 |" in results_section
    assert "| ENFORCED | 1 |" in results_section


def test_violated_severity_is_shown_but_enforced_severity_is_not():
    report = render_report(_sample_engagement())
    ad01_section = report.split("### AD-01")[1].split("### AD-02")[0]
    ad02_section = report.split("### AD-02")[1].split("### AD-03")[0]
    assert "**Severity:**" not in ad01_section  # ENFORCED finding, no severity set
    assert "**Severity:** MEDIUM" in ad02_section  # VIOLATED finding, severity set


def test_finding_card_renders_evidence_gate_status_when_present():
    report = render_report(_sample_engagement())
    ad01_section = report.split("### AD-01")[1].split("### AD-02")[0]
    assert "G1 Representability: satisfied" in ad01_section
    assert "G7 Security-claim triage: satisfied" in ad01_section


# ── Decision Sheet ────────────────────────────────────────────────────────


def test_decision_sheet_shows_executive_summary_and_outcome_counts():
    report = render_report(_sample_engagement())
    decision_sheet = report.split("## 1. Decision Sheet")[1].split("## 2. Authority Map")[0]
    assert "Sample executive summary." in decision_sheet
    assert "| VIOLATED | 1 |" in decision_sheet
    assert "Highest severity among VIOLATED findings" in decision_sheet
    assert "MEDIUM (AD-02)" in decision_sheet


# ── Authority Map / Authority Diff: never fabricated ─────────────────────


def test_authority_map_states_not_populated_when_absent():
    report = render_report(_sample_engagement())
    section = report.split("## 2. Authority Map / Authority Diff")[1].split("## 3. Action Queue")[0]
    assert "Not populated for this engagement" in section


def test_authority_map_renders_the_given_text_when_present():
    report = render_report(_sample_engagement(authority_map="Human -> Agent A -> Agent B, fully evidenced."))
    section = report.split("## 2. Authority Map / Authority Diff")[1].split("## 3. Action Queue")[0]
    assert "Human -> Agent A -> Agent B, fully evidenced." in section
    assert "Not populated" not in section


# ── Action Queue: derived from VIOLATED findings, never fabricated ───────


def test_action_queue_lists_violated_findings_with_recommendation_and_retest_condition():
    report = render_report(_sample_engagement())
    section = report.split("## 3. Action Queue")[1].split("## 4. Findings Summary")[0]
    assert "AD-02" in section
    assert "Scope grants to least privilege." in section
    assert "Re-verify after the grant is narrowed." in section


def test_action_queue_states_no_outstanding_actions_when_nothing_violated():
    only_enforced = (
        Finding(
            finding_id="AD-01",
            area=AssessmentArea.PRE_EXECUTION_BOUNDARY,
            title="t",
            status=Outcome.ENFORCED,
            observation="o",
            expected_control="e",
            evidence_ids=("E-001",),
            evidence_gate_status=_FULL_GATES,
        ),
    )
    evidence = EvidenceManifest([EvidenceReference(evidence_id="E-001", evidence_type="t", source="s", description="d")])
    engagement = _sample_engagement(findings=only_enforced, evidence=evidence)
    report = render_report(engagement)
    section = report.split("## 3. Action Queue")[1].split("## 4. Findings Summary")[0]
    assert "No outstanding actions recorded" in section


# ── Verified Controls: derived from ENFORCED findings ────────────────────


def test_verified_controls_lists_enforced_findings():
    report = render_report(_sample_engagement())
    section = report.split("## 6. Verified Controls")[1].split("## 7. Limitations")[0]
    assert "AD-01" in section
    assert "In-scope call allowed" in section


# ── Retest Record: never fabricated ──────────────────────────────────────


def test_retest_record_states_none_when_no_finding_has_a_retest_condition():
    no_retest_findings = (
        Finding(
            finding_id="AD-01",
            area=AssessmentArea.PRE_EXECUTION_BOUNDARY,
            title="t",
            status=Outcome.ENFORCED,
            observation="o",
            expected_control="e",
            evidence_ids=("E-001",),
            evidence_gate_status=_FULL_GATES,
        ),
    )
    evidence = EvidenceManifest([EvidenceReference(evidence_id="E-001", evidence_type="t", source="s", description="d")])
    engagement = _sample_engagement(findings=no_retest_findings, evidence=evidence)
    report = render_report(engagement)
    section = report.split("## 8. Retest Record")[1].split("## 9. Evidence Index")[0]
    assert "do not infer" in section.lower()


def test_retest_record_lists_findings_with_a_retest_condition():
    report = render_report(_sample_engagement())
    section = report.split("## 8. Retest Record")[1].split("## 9. Evidence Index")[0]
    assert "AD-02" in section
    assert "Re-verify after the grant is narrowed." in section


# ── recommendations preserved in the Technical Appendix ──────────────────


def test_technical_appendix_includes_all_recommendations():
    report = render_report(_sample_engagement())
    appendix = report.split("## 10. Technical Appendix")[1]
    assert "Enforce authorization immediately before execution." in appendix
    assert "Scope grants to least privilege." in appendix


def test_technical_appendix_includes_scope_and_workflow_prose():
    report = render_report(_sample_engagement())
    appendix = report.split("## 10. Technical Appendix")[1]
    assert "Sample scope text." in appendix
    assert "Sample workflow text." in appendix


# ── deterministic output ─────────────────────────────────────────────────


def test_render_is_deterministic():
    engagement = _sample_engagement()
    first = render_report(engagement)
    second = render_report(engagement)
    assert first == second


def test_render_does_not_depend_on_finding_construction_order():
    """Findings are sorted by finding_id at render time, not by
    insertion/tuple order — a caller assembling findings in a different
    order must still produce byte-identical output."""
    e1 = _sample_engagement()
    reordered_findings = tuple(reversed(e1.findings))
    e2 = _sample_engagement(findings=reordered_findings)
    assert render_report(e1) == render_report(e2)


# ── engagement construction rejects dangling references ──────────────────


def test_engagement_construction_rejects_dangling_evidence_reference():
    evidence = EvidenceManifest([EvidenceReference(evidence_id="E-001", evidence_type="t", source="s", description="d")])
    findings = (
        Finding(
            finding_id="AD-01",
            area=AssessmentArea.PRE_EXECUTION_BOUNDARY,
            title="t",
            status=Outcome.ENFORCED,
            observation="o",
            expected_control="e",
            evidence_ids=("E-999",),  # not in the manifest
            evidence_gate_status=_FULL_GATES,
        ),
    )
    with pytest.raises(Exception):
        Engagement(
            title="t", label="l", date="d", assessor="a", scope="s", workflow="w",
            limitations="lim", executive_summary="exec", findings=findings, evidence=evidence,
        )
