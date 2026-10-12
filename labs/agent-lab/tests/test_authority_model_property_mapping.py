"""
Crypton Authority Model / Report Surface freeze pass — regression tests.

Covers:
(1) property labels come from the stable PROP-* mapping, never an
    H-number assumption (the exact cross-framework comparison error the
    2026-09-30 evidence-readiness audit found, now guarded at the report
    layer too);
(2) Mastra's assessment invariant is kept separate from the target's
    UNREPRESENTABLE approval-action binding — the invariant is never
    itself rendered as a gap, and the target limitation is never rendered
    as if it were a weakness in the invariant.
"""

from __future__ import annotations

import pytest

from agentlab.assessment.authority_model import fixtures
from agentlab.assessment.authority_model.diff import DivergenceType
from agentlab.assessment.authority_model.property_mapping import resolve_property
from agentlab.assessment.authority_model.render_text import render_authority_diff, render_evidence_strip


def _mastra_available() -> bool:
    return fixtures._mastra_evidence_path().exists()


# ── H-number drift regression (spec point 2) ─────────────────────────────


def test_exp003a_h1_resolves_to_argument_binding():
    prop = resolve_property("EXP-003A", "H1")
    assert prop is not None
    assert prop.property_id == "PROP-ARG-BINDING"
    assert prop.short_name == "Argument binding / drift"


def test_h3_means_different_properties_in_exp003a_vs_exp003b():
    """The exact mismatch the mapping document exists to correct: EXP-003A's
    H3 (standing approval) and EXP-003B's H3 (checkpoint replay) are
    different properties. The mapping must disambiguate by (experiment,
    label), never collapse on the bare string "H3"."""
    a = resolve_property("EXP-003A", "H3")
    b = resolve_property("EXP-003B", "H3")
    assert a is not None and b is not None
    assert a.property_id != b.property_id
    assert a.property_id == "PROP-STANDING-APPROVAL"
    assert b.property_id == "PROP-CHECKPOINT-STALE-REPLAY"


def test_h5_means_different_properties_in_exp003b_vs_exp003a_and_c():
    """Mirror case: EXP-003B's H5 (standing approval) is EXP-003A's/C's H3
    equivalent, not their H5 (policy TOCTOU)."""
    b5 = resolve_property("EXP-003B", "H5")
    a5 = resolve_property("EXP-003A", "H5")
    assert b5 is not None and a5 is not None
    assert b5.property_id == "PROP-STANDING-APPROVAL"
    assert a5.property_id == "PROP-POLICY-TOCTOU"
    assert b5.property_id != a5.property_id


def test_unmapped_local_label_resolves_to_none_not_a_guess():
    """Spec: 'do not derive the property name from an H-number alone' --
    an (experiment, label) pair the mapping document doesn't define must
    resolve to None, never a fabricated best-guess property."""
    assert resolve_property("EXP-003A", "H6") is None  # A has no H6 at all
    assert resolve_property("EXP-003Z", "H1") is None  # not a real experiment


def test_exp003a_fixture_view_renders_the_stable_property_not_approval_scope():
    """End-to-end: the actual EXP-003A view must show the stable property
    label, not the generic 'Approval Scope' fallback a bare H-number
    assumption would have produced."""
    _, _, _, view = fixtures.build_fixture_view("exp003a-h1a-argument-binding")
    assert view.header.property == "Argument binding / drift (PROP-ARG-BINDING)"
    assert view.header.property != "Approval Scope"


def test_mastra_fixture_correctly_uses_the_generic_fallback_not_an_exp003_mapping():
    """Mastra Candidate 4a is not part of the EXP-003 series -- it must NOT
    be assigned a PROP-* label (that would be a fabricated cross-reference),
    and the generic fallback remains correct for it."""
    if not _mastra_available():
        pytest.skip("sibling semantic-rulebook checkout not present")
    _, _, _, view = fixtures.build_fixture_view("mastra-candidate-4a")
    assert view.header.property == "Approval Scope"
    assert "PROP-" not in (view.header.property or "")


# ── Mastra invariant-vs-representation separation (spec points 3-5) ──────


@pytest.mark.skipif(not _mastra_available(), reason="sibling semantic-rulebook checkout not present")
def test_mastra_expected_side_states_the_invariant_not_a_representation_gap():
    _, _, _, view = fixtures.build_fixture_view("mastra-candidate-4a")
    approval_step = next(s for s in view.expected_path if s.label.startswith("Approval"))
    assert approval_step.provenance == "ASSESSED_INVARIANT"
    # The invariant itself must never carry an UNREPRESENTABLE/UNVERIFIABLE
    # availability badge -- it is fully known and statable.
    assert approval_step.availability.value == "AVAILABLE"
    assert "assessed invariant" in (approval_step.detail or "").lower()
    assert "UNREPRESENTABLE" not in (approval_step.detail or "")


@pytest.mark.skipif(not _mastra_available(), reason="sibling semantic-rulebook checkout not present")
def test_mastra_observed_side_states_the_target_representation_gap():
    _, _, _, view = fixtures.build_fixture_view("mastra-candidate-4a")
    approval_trunk_step = view.observed_path[0]
    assert approval_trunk_step.label.startswith("Approval")
    assert "target action binding" in (approval_trunk_step.detail or "").lower()
    assert "UNREPRESENTABLE" in (approval_trunk_step.detail or "")


@pytest.mark.skipif(not _mastra_available(), reason="sibling semantic-rulebook checkout not present")
def test_mastra_semantic_gaps_distinguish_expected_scope_from_target_representation():
    _, _, _, view = fixtures.build_fixture_view("mastra-candidate-4a")
    gap_labels = {g.field_label for g in view.semantic_gaps}
    # The gap list must name the TARGET's representation, never the
    # expected/assessed scope itself (that is never a gap).
    assert "Target representation of approval scope" in gap_labels
    assert "Expected approval scope" not in gap_labels
    assert not any("expected" in label.lower() and "scope" in label.lower() for label in gap_labels if "target" not in label.lower())


@pytest.mark.skipif(not _mastra_available(), reason="sibling semantic-rulebook checkout not present")
def test_mastra_evidence_strip_has_separate_invariant_and_representation_rows():
    _, _, _, view = fixtures.build_fixture_view("mastra-candidate-4a")
    rows_by_element = {r.element: r for r in view.evidence_strip}
    assert "Expected approval scope" in rows_by_element
    assert rows_by_element["Expected approval scope"].status == "ASSESSED_INVARIANT"
    assert rows_by_element["Expected approval scope"].evidence_ids == ()  # not an evidential claim about the target

    assert "Target representation of approval scope" in rows_by_element
    assert rows_by_element["Target representation of approval scope"].status == "UNREPRESENTABLE"
    assert rows_by_element["Target representation of approval scope"].evidence_ids  # IS evidenced


@pytest.mark.skipif(not _mastra_available(), reason="sibling semantic-rulebook checkout not present")
def test_mastra_divergence_and_first_divergence_point_are_unchanged_by_the_semantic_fix():
    """The separation is purely presentational -- the underlying divergence
    classification must not move."""
    _, _, _, view = fixtures.build_fixture_view("mastra-candidate-4a")
    assert view.divergence_type == DivergenceType.APPROVAL_SCOPE_MISMATCH.value
    assert view.divergence_point == "Approval -> Action"


@pytest.mark.skipif(not _mastra_available(), reason="sibling semantic-rulebook checkout not present")
def test_mastra_rendered_text_keeps_invariant_and_gap_visually_separate():
    _, _, _, view = fixtures.build_fixture_view("mastra-candidate-4a")
    text = render_authority_diff(view)
    expected_section = text.split("OBSERVED")[0]
    observed_section = text.split("OBSERVED")[1].split("FIRST DIVERGENCE")[0]
    assert "UNREPRESENTABLE" not in expected_section
    assert "assessed invariant" in expected_section.lower()
    assert "UNREPRESENTABLE" in observed_section


# ── Clean case (EXP-003A) must not fabricate an Action Strip ─────────────


def test_exp003a_has_no_action_strip():
    """Spec point 6: do not show an Action Strip unless there is a real
    Finding source for one -- EXP-003A has none."""
    _, _, _, view = fixtures.build_fixture_view("exp003a-h1a-argument-binding")
    assert view.action is None


@pytest.mark.skipif(not _mastra_available(), reason="sibling semantic-rulebook checkout not present")
def test_mastra_has_no_action_strip_either_since_it_has_no_formal_finding():
    _, _, _, view = fixtures.build_fixture_view("mastra-candidate-4a")
    assert view.action is None
