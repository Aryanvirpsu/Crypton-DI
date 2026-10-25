"""
v0.6.1 — assessment.models: Finding/EvidenceReference shape and
validation. Pure data-model tests — no chain/delegation/authorization
primitives involved.

2026-09-30 remediation pass: rewritten for the canonical five-outcome
model (ENFORCED/VIOLATED/UNREPRESENTABLE/UNVERIFIABLE/NOT_APPLICABLE), the
seven-gate evidence-sufficiency model, and the expanded evidence/finding
schemas. See docs/audits/2026-09-30-evidence-report-readiness-audit.md and
docs/ASSESSMENT_METHODOLOGY.md for the full rationale.
"""

import pytest

from agentlab.assessment.models import (
    AssessmentArea,
    EvidenceClassification,
    EvidenceGateStatus,
    EvidenceReference,
    Finding,
    InconclusiveReason,
    Outcome,
    Severity,
    migrate_legacy_outcome,
)

_FULL_GATES = EvidenceGateStatus(
    g1_representability=True,
    g2_observability=True,
    g3_controlled_execution=True,
    g4_effect_confirmation=True,
    g5_calibration_control=True,
    g6_reproduction=True,
    g7_security_claim_triage=True,
)


def _evidence(evidence_id="E-001") -> EvidenceReference:
    return EvidenceReference(
        evidence_id=evidence_id,
        evidence_type="authorization_log",
        source="test",
        description="a real decision row",
    )


def _finding(**overrides) -> Finding:
    base = dict(
        finding_id="AD-01",
        area=AssessmentArea.PRE_EXECUTION_BOUNDARY,
        title="A control",
        status=Outcome.ENFORCED,
        observation="what was observed",
        expected_control="what should hold",
        evidence_ids=("E-001",),
        evidence_gate_status=_FULL_GATES,
    )
    base.update(overrides)
    return Finding(**base)


# ── required fields ──────────────────────────────────────────────────────


@pytest.mark.parametrize("field_name", ["evidence_id", "evidence_type", "description"])
def test_evidence_reference_rejects_empty_required_fields(field_name):
    kwargs = dict(evidence_id="E-001", evidence_type="config_excerpt", source="s", description="d")
    kwargs[field_name] = "   "
    with pytest.raises(ValueError):
        EvidenceReference(**kwargs)


@pytest.mark.parametrize("field_name", ["finding_id", "title", "observation", "expected_control"])
def test_finding_rejects_empty_required_fields(field_name):
    kwargs = dict(
        finding_id="AD-01",
        area=AssessmentArea.PRE_EXECUTION_BOUNDARY,
        title="t",
        status=Outcome.NOT_APPLICABLE,  # avoid tripping the evidence-required rule for this test
        observation="o",
        expected_control="e",
    )
    kwargs[field_name] = "   "
    with pytest.raises(ValueError):
        Finding(**kwargs)


# ── ENFORCED/VIOLATED require evidence + fully satisfied gates ──────────


@pytest.mark.parametrize("status", [Outcome.ENFORCED, Outcome.VIOLATED])
def test_enforced_or_violated_without_evidence_is_rejected(status):
    with pytest.raises(ValueError, match="evidence"):
        _finding(status=status, evidence_ids=(), evidence_gate_status=_FULL_GATES)


@pytest.mark.parametrize("status", [Outcome.ENFORCED, Outcome.VIOLATED])
def test_enforced_or_violated_without_gate_status_is_rejected(status):
    with pytest.raises(ValueError, match="seven-gate"):
        _finding(status=status, evidence_gate_status=None)


@pytest.mark.parametrize(
    "incomplete_gates",
    [
        EvidenceGateStatus(g1_representability=True),  # everything else None
        EvidenceGateStatus(
            g1_representability=True,
            g2_observability=True,
            g3_controlled_execution=True,
            g4_effect_confirmation=True,
            g5_calibration_control=True,
            g6_reproduction=True,
            g7_security_claim_triage=False,  # explicitly failed, not just missing
        ),
    ],
)
def test_enforced_or_violated_with_any_unsatisfied_gate_is_rejected(incomplete_gates):
    with pytest.raises(ValueError, match="seven-gate"):
        _finding(status=Outcome.VIOLATED, evidence_gate_status=incomplete_gates)


def test_violated_requires_gate7_true_not_just_g1_through_g6():
    """The exact regression case Gate 7 exists to prevent: G1-G6 all
    satisfied, G7 not — must still be rejected for VIOLATED. This is the
    EXP-001d/e closure encoded as a structural test."""
    g1_to_6_only = EvidenceGateStatus(
        g1_representability=True,
        g2_observability=True,
        g3_controlled_execution=True,
        g4_effect_confirmation=True,
        g5_calibration_control=True,
        g6_reproduction=True,
        g7_security_claim_triage=None,
    )
    with pytest.raises(ValueError, match="Gate 7"):
        _finding(status=Outcome.VIOLATED, evidence_gate_status=g1_to_6_only)


@pytest.mark.parametrize("status", [Outcome.ENFORCED, Outcome.VIOLATED])
def test_enforced_or_violated_with_fully_satisfied_gates_is_valid(status):
    finding = _finding(status=status, evidence_gate_status=_FULL_GATES)
    assert finding.status == status
    assert finding.evidence_gate_status.all_satisfied()


# ── UNREPRESENTABLE/UNVERIFIABLE require a stated reason, not evidence ──


@pytest.mark.parametrize("status", [Outcome.UNREPRESENTABLE, Outcome.UNVERIFIABLE])
def test_unrepresentable_or_unverifiable_without_limitations_is_rejected(status):
    with pytest.raises(ValueError, match=status.value):
        _finding(status=status, evidence_ids=(), evidence_gate_status=None, limitations=None)


@pytest.mark.parametrize("status", [Outcome.UNREPRESENTABLE, Outcome.UNVERIFIABLE])
def test_unrepresentable_or_unverifiable_with_limitations_and_no_evidence_is_allowed(status):
    finding = _finding(status=status, evidence_ids=(), evidence_gate_status=None, limitations="a stated reason")
    assert finding.status == status
    assert finding.evidence_ids == ()


def test_not_applicable_requires_neither_evidence_nor_limitations():
    finding = _finding(status=Outcome.NOT_APPLICABLE, evidence_ids=(), evidence_gate_status=None, limitations=None)
    assert finding.status == Outcome.NOT_APPLICABLE


# ── severity is independent of outcome ───────────────────────────────────


def test_enforced_finding_has_no_severity_by_default():
    finding = _finding(status=Outcome.ENFORCED)
    assert finding.severity is None


def test_violated_finding_may_carry_a_severity():
    finding = _finding(status=Outcome.VIOLATED, severity=Severity.HIGH)
    assert finding.severity == Severity.HIGH


def test_severity_construction_does_not_require_a_particular_status():
    """The model itself does not enforce "only VIOLATED may have a
    severity" — that's assessor judgment (an INFORMATIONAL note on an
    ENFORCED finding is legitimate, per the methodology doc), not a
    structural rule."""
    finding = _finding(status=Outcome.ENFORCED, severity=Severity.INFORMATIONAL)
    assert finding.severity == Severity.INFORMATIONAL


# ── all five outcomes are valid, distinct enum members ──────────────────


def test_all_five_outcomes_exist_and_are_distinct():
    values = {o.value for o in Outcome}
    assert values == {"ENFORCED", "VIOLATED", "UNREPRESENTABLE", "UNVERIFIABLE", "NOT_APPLICABLE"}


def test_all_five_severities_exist():
    values = {s.value for s in Severity}
    assert values == {"CRITICAL", "HIGH", "MEDIUM", "LOW", "INFORMATIONAL"}


def test_all_eight_areas_exist_with_labels():
    assert len(list(AssessmentArea)) == 8
    for area in AssessmentArea:
        assert isinstance(area.label, str)
        assert area.label  # non-empty


def test_all_four_evidence_classifications_exist():
    values = {c.value for c in EvidenceClassification}
    assert values == {"PUBLIC", "RELEASE_APPROVED", "CUSTOMER_CONFIDENTIAL", "CRYPTON_PROPRIETARY"}


# ── seven-gate evidence-sufficiency model ────────────────────────────────


def test_gate_status_all_satisfied_requires_every_gate_true():
    assert _FULL_GATES.all_satisfied()
    assert not EvidenceGateStatus().all_satisfied()  # all None by default
    assert not EvidenceGateStatus(g1_representability=True).all_satisfied()  # partial


def test_gate_status_unsatisfied_gates_lists_exactly_the_non_true_ones():
    partial = EvidenceGateStatus(g1_representability=True, g2_observability=False)
    unsatisfied = partial.unsatisfied_gates()
    assert "g1_representability" not in unsatisfied
    assert "g2_observability" in unsatisfied
    assert "g7_security_claim_triage" in unsatisfied  # never set -> None -> unsatisfied


def test_gate_status_round_trips_through_dict():
    restored = EvidenceGateStatus.from_dict(_FULL_GATES.to_dict())
    assert restored == _FULL_GATES


def test_gate_status_none_is_distinct_from_false():
    """An unevaluated gate (None) must not satisfy all_satisfied(), the
    same as an explicitly failed one (False) — both block ENFORCED/VIOLATED."""
    unevaluated = EvidenceGateStatus(
        g1_representability=True,
        g2_observability=True,
        g3_controlled_execution=True,
        g4_effect_confirmation=True,
        g5_calibration_control=True,
        g6_reproduction=True,
        g7_security_claim_triage=None,
    )
    failed = EvidenceGateStatus(
        g1_representability=True,
        g2_observability=True,
        g3_controlled_execution=True,
        g4_effect_confirmation=True,
        g5_calibration_control=True,
        g6_reproduction=True,
        g7_security_claim_triage=False,
    )
    assert not unevaluated.all_satisfied()
    assert not failed.all_satisfied()


# ── legacy outcome migration ─────────────────────────────────────────────


def test_migrate_pass_always_migrates_to_enforced():
    result = migrate_legacy_outcome("PASS")
    assert result.outcome == Outcome.ENFORCED
    assert not result.requires_assessor_resolution


def test_migrate_not_applicable_always_migrates_to_not_applicable():
    result = migrate_legacy_outcome("NOT_APPLICABLE")
    assert result.outcome == Outcome.NOT_APPLICABLE
    assert not result.requires_assessor_resolution


def test_migrate_fail_without_gate7_requires_assessor_resolution():
    """The core regression test for the remediation spec's central rule:
    evidence sufficiency alone (no Gate 7 input at all) must never promote
    FAIL to VIOLATED."""
    result = migrate_legacy_outcome("FAIL")
    assert result.outcome is None
    assert result.requires_assessor_resolution
    assert "Gate 7" in result.rationale or "G7" in result.rationale


def test_migrate_fail_with_gate7_false_requires_assessor_resolution():
    """This is the exact EXP-001d/e shape: a FAIL exists, but the
    trust-boundary triage says no claimed boundary was crossed — must not
    become VIOLATED."""
    result = migrate_legacy_outcome("FAIL", security_claim_crossed=False)
    assert result.outcome is None
    assert result.requires_assessor_resolution


def test_migrate_fail_with_gate7_true_migrates_to_violated():
    result = migrate_legacy_outcome("FAIL", security_claim_crossed=True)
    assert result.outcome == Outcome.VIOLATED
    assert not result.requires_assessor_resolution


def test_migrate_inconclusive_without_reason_requires_assessor_resolution():
    result = migrate_legacy_outcome("INCONCLUSIVE")
    assert result.outcome is None
    assert result.requires_assessor_resolution


def test_migrate_inconclusive_missing_semantic_state_migrates_to_unrepresentable():
    result = migrate_legacy_outcome("INCONCLUSIVE", inconclusive_reason=InconclusiveReason.MISSING_SEMANTIC_STATE)
    assert result.outcome == Outcome.UNREPRESENTABLE
    assert not result.requires_assessor_resolution


def test_migrate_inconclusive_insufficient_evidence_migrates_to_unverifiable():
    result = migrate_legacy_outcome(
        "INCONCLUSIVE", inconclusive_reason=InconclusiveReason.INSUFFICIENT_PERMITTED_EVIDENCE
    )
    assert result.outcome == Outcome.UNVERIFIABLE
    assert not result.requires_assessor_resolution


def test_migrate_rejects_an_unknown_legacy_status():
    with pytest.raises(ValueError):
        migrate_legacy_outcome("MAYBE")


def test_migrate_is_case_insensitive_and_strips_whitespace():
    assert migrate_legacy_outcome("  pass  ").outcome == Outcome.ENFORCED


# ── serialization round-trip ─────────────────────────────────────────────


def test_evidence_reference_round_trips_through_dict():
    original = EvidenceReference(
        evidence_id="E-001",
        evidence_type="execution_trace",
        source="data/traces/req-1.jsonl",
        description="full trace for one run",
        timestamp="2026-09-02T00:00:00Z",
        reference="req-1.jsonl:L14",
        integrity_hash="sha256:deadbeef",
        notes="collected during controlled staging test",
        assessment_id="ENG-0001",
        test_id="T-001",
        run_id="run-abc123",
        target_product="example-product",
        target_version="1.2.3",
        target_commit="abcdef0",
        environment_manifest_reference="env/manifest-001.json",
        supports="AD-02",
        redaction_status="unredacted",
        classification=EvidenceClassification.CUSTOMER_CONFIDENTIAL,
    )
    restored = EvidenceReference.from_dict(original.to_dict())
    assert restored == original


def test_evidence_reference_round_trips_with_optional_fields_omitted():
    original = EvidenceReference(evidence_id="E-002", evidence_type="source_excerpt", source="s", description="d")
    restored = EvidenceReference.from_dict(original.to_dict())
    assert restored == original
    assert restored.classification is None


def test_finding_round_trips_through_dict():
    original = _finding(
        status=Outcome.VIOLATED,
        severity=Severity.MEDIUM,
        risk_impact="some impact",
        recommendation="do the thing",
        limitations="some caveat",
        confidence="High",
        evidence_ids=("E-001", "E-002"),
        evidence_gate_status=_FULL_GATES,
        test_id="T-001",
        property_id="PROP-ARG-BINDING",
        property_family="argument_binding",
        tested_layer="agent_execution_loop",
        required_semantic_facts=("fact a", "fact b"),
        invariant="an invariant",
        preconditions="some preconditions",
        initial_state="some initial state",
        stimulus="some stimulus",
        expected="expected result",
        observed="observed result",
        effect="some effect",
        control_or_calibration="a control",
        reproduction="5/5",
        retest_condition="retest after fix",
    )
    restored = Finding.from_dict(original.to_dict())
    assert restored == original


def test_finding_round_trips_with_optional_fields_omitted():
    original = _finding(status=Outcome.NOT_APPLICABLE, evidence_ids=(), evidence_gate_status=None)
    restored = Finding.from_dict(original.to_dict())
    assert restored == original
    assert restored.severity is None
    assert restored.evidence_gate_status is None


def test_finding_to_dict_uses_outcome_key_not_status():
    finding = _finding(status=Outcome.ENFORCED)
    data = finding.to_dict()
    assert data["outcome"] == "ENFORCED"
    assert "status" not in data


def test_finding_from_dict_accepts_legacy_status_key_name_with_a_canonical_value():
    """The JSON *key* name "status" (pre-2026-09-30) is accepted for
    backward compatibility, but only with an already-canonical value —
    this is not a legacy-vocabulary migration path."""
    data = _finding(status=Outcome.ENFORCED).to_dict()
    data["status"] = data.pop("outcome")
    restored = Finding.from_dict(data)
    assert restored.status == Outcome.ENFORCED


@pytest.mark.parametrize("legacy_value", ["PASS", "FAIL", "INCONCLUSIVE"])
def test_finding_from_dict_refuses_to_silently_load_an_ambiguous_legacy_outcome_value(legacy_value):
    """A historical PASS/FAIL/INCONCLUSIVE value must never be silently
    accepted as if it were already canonical — loading it raises, pointing
    at migrate_legacy_outcome() instead of guessing. NOT_APPLICABLE is
    deliberately excluded from this parametrization: it is the one literal
    shared, unchanged, by both the legacy and canonical vocabularies (see
    migrate_legacy_outcome's "NOT_APPLICABLE always migrates to
    NOT_APPLICABLE" rule) and loading it directly is correct, not a bug."""
    data = _finding(status=Outcome.ENFORCED).to_dict()
    data["outcome"] = legacy_value
    with pytest.raises(ValueError, match="migrate_legacy_outcome"):
        Finding.from_dict(data)


def test_finding_from_dict_accepts_not_applicable_directly_without_migration():
    data = _finding(status=Outcome.ENFORCED).to_dict()
    data["outcome"] = "NOT_APPLICABLE"
    data["evidence_ids"] = []
    data["evidence_gate_status"] = None
    restored = Finding.from_dict(data)
    assert restored.status == Outcome.NOT_APPLICABLE
