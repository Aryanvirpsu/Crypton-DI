"""
Crypton Authority Model v0.1 — fixture-level acceptance tests.

These are the Definition-of-Done checks (spec §24): a real agent-lab
divergence case, a real external-framework violation case (Mastra
Candidate 4a), and a real external-framework no-divergence case
(EXP-003A H1a) each produce a correct, evidence-backed Authority Diff —
plus the cross-cutting accuracy requirements from spec §23 (no inferred
principal/approver identity, no invented divergence from absent
information, proprietary classification preserved).
"""

from __future__ import annotations

import json

import pytest

from agentlab.assessment.authority_model import fixtures, render_authority_map
from agentlab.assessment.authority_model.diff import AuthorityDiff
from agentlab.assessment.authority_model.enums import DivergenceType
from agentlab.assessment.authority_model.envelope import AuthorityEnvelope
from agentlab.assessment.authority_model.ledger import AuthorityLedger
from agentlab.assessment.models import EvidenceClassification


def _mastra_available() -> bool:
    try:
        fixtures._mastra_evidence_path()
        return fixtures._mastra_evidence_path().exists()
    except Exception:
        return False


# ── Fixture 1: Agent Lab AD-02 (rich, own-system divergence case) ───────


def test_agentlab_ad02_produces_scope_expansion_divergence():
    ledger, envelope, diff = fixtures.build_agentlab_ad02_fixture()
    assert diff.divergence_type == DivergenceType.SCOPE_EXPANSION
    assert diff.divergence_point == "Grant"
    assert diff.evidence_ids  # cites real evidence, never empty for a VIOLATED-shaped result
    assert set(diff.evidence_ids) <= {"E-001", "E-002", "E-003"}  # only the evidence AD-02 itself cites


def test_agentlab_ad02_principal_is_honestly_unknown():
    """AD-02's own cited evidence never names the granting human -- the
    model must say so, not borrow an identity from a different finding's
    evidence (e.g. E-004) in the same engagement."""
    ledger, envelope, diff = fixtures.build_agentlab_ad02_fixture()
    principal = ledger.principals["ad02-principal"]
    assert principal.principal_id.value is None
    assert not principal.principal_id.is_available


def test_agentlab_ad02_renders_without_error():
    ledger, envelope, diff = fixtures.build_agentlab_ad02_fixture()
    text = render_authority_map(ledger, envelope, diff)
    assert "SCOPE_EXPANSION" in text
    assert "Grant" in text


def test_agentlab_ad02_ledger_envelope_diff_round_trip(tmp_path):
    ledger, envelope, diff = fixtures.build_agentlab_ad02_fixture()
    l2 = AuthorityLedger.from_dict(json.loads(json.dumps(ledger.to_dict())))
    e2 = AuthorityEnvelope.from_dict(json.loads(json.dumps(envelope.to_dict())))
    d2 = AuthorityDiff.from_dict(json.loads(json.dumps(diff.to_dict())))
    assert l2.to_dict() == ledger.to_dict()
    assert e2.to_dict() == envelope.to_dict()
    assert d2.to_dict() == diff.to_dict()


def test_building_the_agentlab_fixture_does_not_modify_the_source_evidence_files():
    """Historical-compatibility requirement: reading an engagement's
    findings/evidence to build a ledger must never write back to it."""
    path = fixtures._repo_root() / "assessment-engagement" / "reference-agentlab-dryrun" / "findings.json"
    before = path.read_bytes()
    fixtures.build_agentlab_ad02_fixture()
    after = path.read_bytes()
    assert before == after


# ── Fixture 2: Mastra Candidate 4a (external-framework violation case) ──


@pytest.mark.skipif(not _mastra_available(), reason="sibling semantic-rulebook checkout with captured evidence not present")
def test_mastra_candidate_4a_produces_approval_scope_mismatch():
    ledger, envelope, diff = fixtures.build_mastra_candidate_4a_fixture()
    assert diff.divergence_type == DivergenceType.APPROVAL_SCOPE_MISMATCH
    assert diff.divergence_point == "Approval -> Action"  # per spec §11, not "Effect"
    assert len(diff.unexpected_authority) == 1
    assert diff.evidence_ids


@pytest.mark.skipif(not _mastra_available(), reason="sibling semantic-rulebook checkout with captured evidence not present")
def test_mastra_candidate_4a_approver_identity_is_honestly_unknown_not_inferred():
    ledger, envelope, diff = fixtures.build_mastra_candidate_4a_fixture()
    approval = ledger.approvals["c4a-approval"]
    assert approval.approved_by.value is None
    assert not approval.approved_by.is_available
    principal = ledger.principals["c4a-principal"]
    assert principal.principal_id.value is None


@pytest.mark.skipif(not _mastra_available(), reason="sibling semantic-rulebook checkout with captured evidence not present")
def test_mastra_candidate_4a_action_ids_are_unverifiable_not_fabricated():
    """The raw sink ledger recorded callId='unknown' for both actions --
    the model must preserve that as UNVERIFIABLE, never invent a
    TC-A/TC-B-style identifier the evidence doesn't contain."""
    ledger, envelope, diff = fixtures.build_mastra_candidate_4a_fixture()
    for key in ("c4a-action-1", "c4a-action-2"):
        action = ledger.actions[key]
        assert action.action_id.value is None
        assert action.action_id.availability.value == "UNVERIFIABLE"


@pytest.mark.skipif(not _mastra_available(), reason="sibling semantic-rulebook checkout with captured evidence not present")
def test_mastra_candidate_4a_approval_scope_is_unrepresentable_not_allowed():
    ledger, envelope, diff = fixtures.build_mastra_candidate_4a_fixture()
    assert envelope.approval_scope is not None
    assert envelope.approval_scope.availability.value == "UNREPRESENTABLE"


@pytest.mark.skipif(not _mastra_available(), reason="sibling semantic-rulebook checkout with captured evidence not present")
def test_mastra_candidate_4a_evidence_is_classified_crypton_proprietary():
    ledger, envelope, diff = fixtures.build_mastra_candidate_4a_fixture()
    for action in ledger.actions.values():
        assert action.classification == EvidenceClassification.CRYPTON_PROPRIETARY
    for effect in ledger.effects.values():
        assert effect.classification == EvidenceClassification.CRYPTON_PROPRIETARY


@pytest.mark.skipif(not _mastra_available(), reason="sibling semantic-rulebook checkout with captured evidence not present")
def test_mastra_candidate_4a_classification_survives_serialization():
    ledger, envelope, diff = fixtures.build_mastra_candidate_4a_fixture()
    restored = AuthorityLedger.from_dict(json.loads(json.dumps(ledger.to_dict())))
    for key, action in ledger.actions.items():
        assert restored.actions[key].classification == action.classification == EvidenceClassification.CRYPTON_PROPRIETARY


@pytest.mark.skipif(not _mastra_available(), reason="sibling semantic-rulebook checkout with captured evidence not present")
def test_building_the_mastra_fixture_does_not_modify_the_source_evidence_file():
    path = fixtures._mastra_evidence_path()
    before = path.read_bytes()
    fixtures.build_mastra_candidate_4a_fixture()
    after = path.read_bytes()
    assert before == after


@pytest.mark.skipif(not _mastra_available(), reason="sibling semantic-rulebook checkout with captured evidence not present")
def test_mastra_candidate_4a_renders_without_error():
    ledger, envelope, diff = fixtures.build_mastra_candidate_4a_fixture()
    text = render_authority_map(ledger, envelope, diff)
    assert "APPROVAL_SCOPE_MISMATCH" in text
    assert "UNREPRESENTABLE" in text  # the semantic gap stays visible, not hidden


def test_mastra_evidence_unavailable_raises_a_clear_error(monkeypatch, tmp_path):
    monkeypatch.setattr(fixtures, "_mastra_evidence_path", lambda: tmp_path / "does-not-exist.json")
    with pytest.raises(fixtures.MastraEvidenceUnavailable):
        fixtures.build_mastra_candidate_4a_fixture()


# ── Fixture 3: EXP-003A H1a (external-framework, no-divergence case) ────


def test_exp003a_h1a_produces_no_divergence():
    ledger, envelope, diff = fixtures.build_exp003a_h1a_fixture()
    assert diff.divergence_type == DivergenceType.NONE
    assert diff.divergence_point is None
    assert diff.unexpected_authority == ()
    assert diff.missing_expected_authority == ()


def test_exp003a_h1a_approver_identity_is_honestly_unknown():
    ledger, envelope, diff = fixtures.build_exp003a_h1a_fixture()
    approval = ledger.approvals["h1a-approval"]
    assert approval.approved_by.value is None
    assert not approval.approved_by.is_available


def test_exp003a_h1a_action_arguments_match_what_was_approved():
    ledger, envelope, diff = fixtures.build_exp003a_h1a_fixture()
    action = ledger.actions["h1a-action"]
    approval = ledger.approvals["h1a-approval"]
    assert action.arguments.value == {"account": "checking", "amount": 100, "destination": "vendor-A"}
    assert "100" in approval.approval_scope or "checking" in approval.approval_scope


def test_exp003a_h1a_renders_without_error():
    ledger, envelope, diff = fixtures.build_exp003a_h1a_fixture()
    text = render_authority_map(ledger, envelope, diff)
    assert "NONE" in text


def test_building_the_exp003a_fixture_does_not_modify_the_source_evidence_file():
    path = fixtures._repo_root() / "labs" / "exp-003-openai-agents-sdk" / "evidence" / "h1a_post_approval_mutation.json"
    before = path.read_bytes()
    fixtures.build_exp003a_h1a_fixture()
    after = path.read_bytes()
    assert before == after


def test_exp003a_h1a_ledger_envelope_diff_round_trip():
    ledger, envelope, diff = fixtures.build_exp003a_h1a_fixture()
    l2 = AuthorityLedger.from_dict(json.loads(json.dumps(ledger.to_dict())))
    e2 = AuthorityEnvelope.from_dict(json.loads(json.dumps(envelope.to_dict())))
    d2 = AuthorityDiff.from_dict(json.loads(json.dumps(diff.to_dict())))
    assert l2.to_dict() == ledger.to_dict()
    assert e2.to_dict() == envelope.to_dict()
    assert d2.to_dict() == diff.to_dict()
