"""
v0.6.1 — assessment.evidence: EvidenceManifest integrity rules.
"""

import pytest

from agentlab.assessment.evidence import DanglingEvidenceReferenceError, DuplicateEvidenceIdError, EvidenceManifest
from agentlab.assessment.models import AssessmentArea, EvidenceGateStatus, EvidenceReference, Finding, Outcome

_FULL_GATES = EvidenceGateStatus(
    g1_representability=True,
    g2_observability=True,
    g3_controlled_execution=True,
    g4_effect_confirmation=True,
    g5_calibration_control=True,
    g6_reproduction=True,
    g7_security_claim_triage=True,
)


def _evidence(evidence_id: str) -> EvidenceReference:
    return EvidenceReference(evidence_id=evidence_id, evidence_type="config_excerpt", source="s", description="d")


def _finding(finding_id: str, evidence_ids: tuple[str, ...]) -> Finding:
    return Finding(
        finding_id=finding_id,
        area=AssessmentArea.DELEGATION_AUTHORITY_FLOW,
        title="t",
        status=Outcome.ENFORCED if evidence_ids else Outcome.NOT_APPLICABLE,
        observation="o",
        expected_control="e",
        evidence_ids=evidence_ids,
        evidence_gate_status=_FULL_GATES if evidence_ids else None,
    )


# ── evidence IDs resolve ─────────────────────────────────────────────────


def test_added_evidence_resolves_by_id():
    manifest = EvidenceManifest()
    manifest.add(_evidence("E-001"))
    assert "E-001" in manifest
    resolved = manifest.get("E-001")
    assert resolved is not None
    assert resolved.evidence_id == "E-001"


def test_unknown_evidence_id_resolves_to_none():
    manifest = EvidenceManifest()
    assert manifest.get("E-999") is None
    assert "E-999" not in manifest


def test_manifest_preserves_insertion_order():
    manifest = EvidenceManifest()
    manifest.add(_evidence("E-003"))
    manifest.add(_evidence("E-001"))
    manifest.add(_evidence("E-002"))
    assert manifest.ids() == ["E-003", "E-001", "E-002"]


# ── duplicate evidence IDs fail validation ───────────────────────────────


def test_duplicate_evidence_id_on_add_is_rejected():
    manifest = EvidenceManifest()
    manifest.add(_evidence("E-001"))
    with pytest.raises(DuplicateEvidenceIdError):
        manifest.add(_evidence("E-001"))


def test_duplicate_evidence_id_on_construction_is_rejected():
    with pytest.raises(DuplicateEvidenceIdError):
        EvidenceManifest([_evidence("E-001"), _evidence("E-001")])


# ── dangling evidence references fail validation ─────────────────────────


def test_validate_findings_passes_when_all_references_resolve():
    manifest = EvidenceManifest([_evidence("E-001"), _evidence("E-002")])
    findings = [_finding("AD-01", ("E-001",)), _finding("AD-02", ("E-001", "E-002"))]
    manifest.validate_findings(findings)  # must not raise


def test_validate_findings_rejects_a_dangling_reference():
    manifest = EvidenceManifest([_evidence("E-001")])
    findings = [_finding("AD-01", ("E-001", "E-999"))]
    with pytest.raises(DanglingEvidenceReferenceError, match="E-999"):
        manifest.validate_findings(findings)


def test_validate_findings_allows_findings_with_no_evidence():
    manifest = EvidenceManifest()
    findings = [_finding("AD-01", ())]
    manifest.validate_findings(findings)  # must not raise


# ── round-trip through JSON files ────────────────────────────────────────


def test_manifest_round_trips_through_json_file(tmp_path):
    manifest = EvidenceManifest([_evidence("E-001"), _evidence("E-002")])
    path = tmp_path / "evidence_manifest.json"
    manifest.save(path)

    restored = EvidenceManifest.load(path)
    assert restored.ids() == manifest.ids()
    assert restored.get("E-001") == manifest.get("E-001")


def test_manifest_load_rejects_duplicate_ids_in_file(tmp_path):
    path = tmp_path / "evidence_manifest.json"
    path.write_text(
        '[{"evidence_id": "E-001", "evidence_type": "t", "source": "s", "description": "d"},'
        ' {"evidence_id": "E-001", "evidence_type": "t", "source": "s", "description": "d"}]',
        encoding="utf-8",
    )
    with pytest.raises(DuplicateEvidenceIdError):
        EvidenceManifest.load(path)
