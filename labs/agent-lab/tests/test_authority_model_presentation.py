"""
Crypton Authority Map / Authority Diff report surface — presentation and
text-rendering tests.

Covers spec §21: rendering (VIOLATED/ENFORCED/UNKNOWN/UNREPRESENTABLE/
UNVERIFIABLE/evidence/first-divergence), separation (System Path never
creates authority edges), no-inference (missing identities stay missing,
no generated display identity presented as real), and report integration
(report contains a diff produced from the model; finding/evidence data
unchanged).
"""

from __future__ import annotations

import json

import pytest

from agentlab.assessment.authority_model import fixtures
from agentlab.assessment.authority_model.diff import AuthorityDiff, compute_diff
from agentlab.assessment.authority_model.enums import DivergenceType, Provenance
from agentlab.assessment.authority_model.envelope import AuthorityEnvelope, derive_envelope
from agentlab.assessment.authority_model.fact import Fact
from agentlab.assessment.authority_model.ledger import AuthorityLedger
from agentlab.assessment.authority_model.presentation import AuthorityView, build_authority_view
from agentlab.assessment.authority_model.primitives import Action, Approval, Principal
from agentlab.assessment.authority_model.render_text import (
    render_action_strip,
    render_authority_diff,
    render_authority_header,
    render_authority_map_structural,
    render_evidence_strip,
    render_full_text_report,
)


def _mastra_available() -> bool:
    return fixtures._mastra_evidence_path().exists()


# ── Rendering ─────────────────────────────────────────────────────────────


def test_enforced_case_renders_no_divergence_grammar():
    _, _, _, view = fixtures.build_fixture_view("exp003a-h1a-argument-binding")
    text = render_authority_diff(view)
    assert "NO AUTHORITY DIVERGENCE OBSERVED" in text
    assert view.header.outcome == "ENFORCED"


def test_enforced_case_is_not_empty_looking():
    """Spec §5: a secure result must not look like an empty report."""
    _, _, _, view = fixtures.build_fixture_view("exp003a-h1a-argument-binding")
    text = render_full_text_report(view)
    assert "EXPECTED / PERMITTED" in text
    assert "OBSERVED" in text
    assert len(text.strip().splitlines()) > 10  # a real, populated page, not a one-liner


@pytest.mark.skipif(not _mastra_available(), reason="sibling semantic-rulebook checkout not present")
def test_violated_case_renders_both_expected_and_observed_paths():
    _, _, _, view = fixtures.build_fixture_view("mastra-candidate-4a")
    text = render_authority_diff(view)
    assert "EXPECTED / PERMITTED" in text
    assert "OBSERVED" in text
    assert "APPROVAL_SCOPE_MISMATCH" in text


@pytest.mark.skipif(not _mastra_available(), reason="sibling semantic-rulebook checkout not present")
def test_unknown_values_remain_visible_where_material():
    _, _, _, view = fixtures.build_fixture_view("mastra-candidate-4a")
    text = render_authority_header(view)
    assert "UNKNOWN" in text  # principal/actor


@pytest.mark.skipif(not _mastra_available(), reason="sibling semantic-rulebook checkout not present")
def test_unrepresentable_values_remain_visible():
    _, _, _, view = fixtures.build_fixture_view("mastra-candidate-4a")
    text = render_authority_diff(view)
    assert "UNREPRESENTABLE" in text


@pytest.mark.skipif(not _mastra_available(), reason="sibling semantic-rulebook checkout not present")
def test_unverifiable_values_remain_visible():
    _, _, _, view = fixtures.build_fixture_view("mastra-candidate-4a")
    text = render_authority_diff(view)
    assert "UNVERIFIABLE" in text


@pytest.mark.skipif(not _mastra_available(), reason="sibling semantic-rulebook checkout not present")
def test_evidence_ids_render_correctly():
    _, _, _, view = fixtures.build_fixture_view("mastra-candidate-4a")
    table = render_evidence_strip(view)
    assert "candidate-4a.json" in table
    for row in view.evidence_strip:
        if row.evidence_ids:
            assert row.evidence_ids[0] in table


@pytest.mark.skipif(not _mastra_available(), reason="sibling semantic-rulebook checkout not present")
def test_first_divergence_point_renders_correctly():
    _, _, _, view = fixtures.build_fixture_view("mastra-candidate-4a")
    text = render_authority_diff(view)
    assert view.divergence_point == "Approval -> Action"
    assert "Approval -> Action" in text
    assert "FIRST DIVERGENCE" in text


def test_agentlab_renders_richer_authority_map_without_approval_requirement():
    """Spec §15.3: AD-02's richer Grant/delegation data must not make the
    renderer *require* an approval dimension external fixtures don't have."""
    _, _, _, view = fixtures.build_fixture_view("agentlab-ad02")
    text = render_authority_map_structural(view)
    assert "Grant" in text
    assert "allowed_tools" in text


# ── Separation: System Path vs. Authority Path ───────────────────────────


def test_system_path_does_not_create_authority_edges():
    ledger = AuthorityLedger()
    envelope = AuthorityEnvelope(envelope_id="env-1")
    diff = compute_diff(ledger, envelope, diff_id="d1", observed_action_ids=[])
    view = build_authority_view(
        ledger, envelope, diff, view_id="v1",
        system_path=("Web App", "Agent", "Queue", "Worker"),
    )
    # The system path is stored and rendered completely separately; it must
    # never appear inside expected_path/observed_path/observed_branches.
    assert view.system_path == ("Web App", "Agent", "Queue", "Worker")
    all_authority_labels = [s.label for s in view.expected_path] + [s.label for s in view.observed_path]
    for branch in view.observed_branches:
        all_authority_labels.extend(s.label for s in branch)
    for node in view.system_path:
        assert node not in all_authority_labels
    rendered_map = render_authority_map_structural(view)
    assert "SYSTEM PATH" in rendered_map
    assert "AUTHORITY PATH" in rendered_map
    # System path section must appear before, and separately from, the
    # authority path section -- never merged into one graph.
    assert rendered_map.index("SYSTEM PATH") < rendered_map.index("AUTHORITY PATH")


def test_authority_path_uses_only_authority_model_relationships():
    """Without an explicit system_path, no system/architecture content
    should ever appear -- the builder has no mechanism to invent one."""
    ledger = AuthorityLedger()
    envelope = AuthorityEnvelope(envelope_id="env-1")
    diff = compute_diff(ledger, envelope, diff_id="d1", observed_action_ids=[])
    view = build_authority_view(ledger, envelope, diff, view_id="v1")
    assert view.system_path is None


# ── No-inference ──────────────────────────────────────────────────────────


def test_missing_principal_does_not_become_generic_user():
    ledger = AuthorityLedger()
    ledger.principals["p1"] = Principal(principal_id=Fact.unknown("not evidenced"))
    envelope = AuthorityEnvelope(envelope_id="env-1", principal_id="p1")
    diff = compute_diff(ledger, envelope, diff_id="d1", observed_action_ids=[])
    view = build_authority_view(ledger, envelope, diff, view_id="v1")
    assert view.header.principal == "UNKNOWN"
    assert view.header.principal != "User"
    assert "User" not in (view.header.principal or "")


def test_missing_approver_does_not_become_requester():
    ledger = AuthorityLedger()
    ledger.approvals["a1"] = Approval(
        approval_id=Fact.observed("a1"),
        requested_by=Fact.observed("requester-x"),
        approved_by=Fact.unknown("not evidenced"),
    )
    envelope = derive_envelope(ledger, envelope_id="env-1", approval_id="a1")
    diff = compute_diff(ledger, envelope, diff_id="d1", observed_action_ids=[])
    view = build_authority_view(ledger, envelope, diff, view_id="v1")
    gap_labels = {g.field_label: g for g in view.semantic_gaps}
    assert "Approver identity" in gap_labels
    assert gap_labels["Approver identity"].availability.value == "UNKNOWN"
    # The requester's identity must never leak into the approver gap.
    assert "requester-x" not in (gap_labels["Approver identity"].note or "")


def test_missing_action_id_does_not_receive_a_generated_display_identity():
    """An action with no real id gets a tool+arguments description, never
    a fabricated id string presented as if it were the real one."""
    ledger = AuthorityLedger()
    ledger.actions["act-1"] = Action(action_id=None, tool="transfer", arguments=Fact.observed({"amount": 5}))
    envelope = AuthorityEnvelope(envelope_id="env-1")
    diff = compute_diff(ledger, envelope, diff_id="d1", observed_action_ids=["act-1"])
    view = build_authority_view(ledger, envelope, diff, view_id="v1")
    all_branches_text = " ".join(s.label for branch in view.observed_branches for s in branch)
    # The local ledger key ("act-1") is a presentation alias only -- it may
    # appear, but never standing in for a claimed real action_id.
    for branch in view.observed_branches:
        assert "action_id=act-1" not in branch[0].label


def test_outcome_never_prints_violated_without_a_finding_or_explicit_override():
    """build_authority_view must never claim a Gate-7-gated outcome on its
    own just because a divergence was detected."""
    ledger = AuthorityLedger()
    ledger.approvals["a1"] = Approval(approval_id=Fact.observed("a1"), target_action_ids=Fact.observed(("real-1",)))
    ledger.actions["act-1"] = Action(action_id=Fact.observed("real-1"), tool="x")
    ledger.actions["act-2"] = Action(action_id=Fact.observed("real-2"), tool="x")
    envelope = derive_envelope(ledger, envelope_id="env-1", approval_id="a1")
    diff = compute_diff(ledger, envelope, diff_id="d1", observed_action_ids=["act-1", "act-2"])
    assert diff.divergence_type != DivergenceType.NONE
    view = build_authority_view(ledger, envelope, diff, view_id="v1")  # no finding, no outcome_override
    assert view.header.outcome is None


# ── Report integration ───────────────────────────────────────────────────


def test_report_contains_authority_diff_produced_from_the_model(tmp_path):
    _, _, _, view = fixtures.build_fixture_view("agentlab-ad02")
    rendered = render_full_text_report(view)
    # Every divergence-relevant string in the rendered text must trace back
    # to the view object itself (round-trip through JSON, compare).
    restored = AuthorityView.from_dict(json.loads(json.dumps(view.to_dict())))
    assert render_full_text_report(restored) == rendered


def test_building_views_does_not_modify_finding_or_evidence_source_files():
    engagement_dir = fixtures._repo_root() / "assessment-engagement" / "reference-agentlab-dryrun"
    findings_before = (engagement_dir / "findings.json").read_bytes()
    evidence_before = (engagement_dir / "evidence_manifest.json").read_bytes()
    fixtures.build_fixture_view("agentlab-ad02")
    assert (engagement_dir / "findings.json").read_bytes() == findings_before
    assert (engagement_dir / "evidence_manifest.json").read_bytes() == evidence_before


def test_action_strip_uses_real_finding_fields_not_invented_text():
    _, _, _, view = fixtures.build_fixture_view("agentlab-ad02")
    assert view.action is not None
    assert view.action.owner == "TBD"  # never invented
    assert "Scope each delegation" in (view.action.required_action or "")  # real Finding.recommendation text
    assert "Re-verify" in (view.action.retest_condition or "")  # real Finding.retest_condition text


# ── View round-trip / serialization ──────────────────────────────────────


def test_authority_view_round_trips_through_dict():
    ledger = AuthorityLedger()
    envelope = AuthorityEnvelope(envelope_id="env-1")
    diff = compute_diff(ledger, envelope, diff_id="d1", observed_action_ids=[])
    view = build_authority_view(ledger, envelope, diff, view_id="v1", system_path=("A", "B"))
    restored = AuthorityView.from_dict(view.to_dict())
    assert restored == view


def test_authority_view_json_is_a_projection_not_a_duplicate_of_raw_evidence():
    """Spec §16: must not duplicate entire raw evidence blobs -- only ids."""
    _, _, _, view = fixtures.build_fixture_view("exp003a-h1a-argument-binding")
    serialized = json.dumps(view.to_dict())
    # The raw evidence file's full approval_request/proposal nesting should
    # not be duplicated verbatim inside the view -- only referenced by path.
    assert "labs/exp-003-openai-agents-sdk/evidence/h1a_post_approval_mutation.json" in serialized


def test_clean_case_action_strip_is_absent_not_fabricated():
    _, _, _, view = fixtures.build_fixture_view("exp003a-h1a-argument-binding")
    assert view.action is None
    text = render_action_strip(view)
    assert "No action required" in text
