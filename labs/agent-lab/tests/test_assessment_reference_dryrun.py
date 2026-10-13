"""
v0.6.1 — the reference engagement: one synthetic engagement, built from
*real* Agent Lab primitive calls, renders successfully end-to-end.

This deliberately does NOT re-test chain.py/delegation.py/authorization.py/
approval.py's own security properties — those are already certified by
test_delegation_chain.py, test_attenuation.py, test_enforcement_boundary.py,
etc. This file tests only the assessment-delivery layer's ability to
build a real, evidenced engagement and render it — plus, just as
importantly, that the output is unmistakably labeled as a synthetic
self-assessment, never as an external customer result.

2026-09-30 remediation pass: updated for the canonical five-outcome model
(ENFORCED/VIOLATED/UNREPRESENTABLE/UNVERIFIABLE/NOT_APPLICABLE) and the
expanded engagement metadata schema.
"""

from __future__ import annotations

from agentlab.assessment.engagement import Engagement
from agentlab.assessment.models import Outcome
from agentlab.assessment.reference_dryrun import build_reference_engagement
from agentlab.assessment.report import render_report


def test_reference_engagement_builds_successfully(tmp_path):
    engagement = build_reference_engagement(tmp_path / "dryrun.db")
    assert len(engagement.findings) > 0
    assert len(engagement.evidence) > 0


def test_reference_engagement_covers_four_of_the_five_outcomes(tmp_path):
    """UNREPRESENTABLE is not expected here — this synthetic dry run's one
    non-evidenced finding (AD-01) is an evidence-access gap
    (UNVERIFIABLE), not a case where the underlying concept has no
    representation at all; see LIMITATIONS_TEXT for why."""
    engagement = build_reference_engagement(tmp_path / "dryrun.db")
    statuses = {f.status for f in engagement.findings}
    assert statuses == {Outcome.ENFORCED, Outcome.VIOLATED, Outcome.UNVERIFIABLE, Outcome.NOT_APPLICABLE}


def test_reference_engagement_covers_all_eight_areas(tmp_path):
    engagement = build_reference_engagement(tmp_path / "dryrun.db")
    areas = {f.area for f in engagement.findings}
    assert len(areas) == 8


def test_reference_engagement_enforced_and_violated_findings_have_evidence_and_satisfied_gates(tmp_path):
    engagement = build_reference_engagement(tmp_path / "dryrun.db")
    for f in engagement.findings:
        if f.status in (Outcome.ENFORCED, Outcome.VIOLATED):
            assert f.evidence_ids, f"{f.finding_id} is {f.status} but cites no evidence"
            assert f.evidence_gate_status is not None, f"{f.finding_id} is {f.status} but has no evidence_gate_status"
            assert f.evidence_gate_status.all_satisfied(), f"{f.finding_id} is {f.status} but not all gates are satisfied"


def test_reference_engagement_renders_without_error(tmp_path):
    engagement = build_reference_engagement(tmp_path / "dryrun.db")
    report = render_report(engagement)
    assert "## 5. Finding Cards" in report
    assert "## 9. Evidence Index" in report


def test_reference_engagement_populates_structured_metadata(tmp_path):
    """The 2026-09-30 remediation pass's expanded engagement schema should
    be populated for this dry run wherever genuinely known, not left as
    the thin, prose-only shape the 2026-09-30 audit flagged."""
    engagement = build_reference_engagement(tmp_path / "dryrun.db")
    assert engagement.assessment_id
    assert engagement.target_product == "labs/agent-lab"
    assert engagement.target_version
    assert engagement.assessment_type


# ── the dry run must never be mistaken for an external assessment ───────


def test_reference_engagement_is_labeled_as_synthetic(tmp_path):
    engagement = build_reference_engagement(tmp_path / "dryrun.db")
    assert engagement.label == "Crypton Reference Agent Lab — Methodology Dry Run"


def test_rendered_report_states_it_is_not_an_external_result(tmp_path):
    engagement = build_reference_engagement(tmp_path / "dryrun.db")
    report = render_report(engagement)
    assert "not an external customer" in report.lower() or "not an external customer result" in report.lower()


def test_rendered_report_never_claims_customer_validation_language(tmp_path):
    """Guards against exactly the mistake this milestone was commissioned
    to avoid: Crypton's own reference-implementation results being
    presented as if they validated an external customer's system."""
    engagement = build_reference_engagement(tmp_path / "dryrun.db")
    report = render_report(engagement)
    lowered = report.lower()
    for forbidden_phrase in [
        "customer's system passed",
        "validated the customer",
        "external assessment confirms",
        "proves crypton's differentiation",
    ]:
        assert forbidden_phrase not in lowered


# ── evidence is genuinely tied to real primitive output, not fabricated ──


def test_evidence_descriptions_cite_real_decision_ids(tmp_path):
    """Every authorization_log-typed evidence entry must cite a real
    decision_id string this run's own primitive calls actually minted
    (authz_<hex>) — not a placeholder."""
    engagement = build_reference_engagement(tmp_path / "dryrun.db")
    authz_evidence = [e for e in engagement.evidence if e.evidence_type == "authorization_log"]
    assert authz_evidence, "expected at least one authorization_log evidence entry"
    for e in authz_evidence:
        assert "decision_id=authz_" in e.description


def test_violated_finding_evidence_shows_the_over_broad_grant_is_real(tmp_path):
    engagement = build_reference_engagement(tmp_path / "dryrun.db")
    violated_findings = [f for f in engagement.findings if f.status == Outcome.VIOLATED]
    assert len(violated_findings) == 1
    violated_finding = violated_findings[0]
    cited_evidence = [engagement.evidence.get(eid) for eid in violated_finding.evidence_ids]
    assert any("dlg_" in (e.description or "") for e in cited_evidence if e is not None)


def test_evidence_entries_carry_classification(tmp_path):
    engagement = build_reference_engagement(tmp_path / "dryrun.db")
    for e in engagement.evidence:
        assert e.classification is not None, f"{e.evidence_id} has no classification set"


# ── save/load round-trip through the file-based engagement structure ────


def test_reference_engagement_round_trips_through_save_and_load(tmp_path):
    engagement = build_reference_engagement(tmp_path / "dryrun.db")
    engagement_dir = tmp_path / "saved-engagement"
    engagement.save(engagement_dir)

    reloaded = Engagement.load(engagement_dir)
    assert reloaded.title == engagement.title
    assert reloaded.label == engagement.label
    assert reloaded.assessment_id == engagement.assessment_id
    assert reloaded.target_product == engagement.target_product
    assert len(reloaded.findings) == len(engagement.findings)
    assert len(reloaded.evidence) == len(engagement.evidence)
    assert render_report(reloaded) == render_report(engagement)
