"""
Crypton Authority Model v0.1 — Diff tests.

Covers: detects approval scope expansion, detects no divergence
correctly, missing evidence does not create a false divergence,
unrepresentable fields remain semantic gaps, first divergence point is
stable.
"""

from __future__ import annotations

from agentlab.assessment.authority_model.diff import compute_diff
from agentlab.assessment.authority_model.enums import DivergenceType
from agentlab.assessment.authority_model.envelope import AuthorityEnvelope, derive_envelope
from agentlab.assessment.authority_model.fact import Fact
from agentlab.assessment.authority_model.ledger import AuthorityLedger
from agentlab.assessment.authority_model.primitives import Action, Approval


def _action(tool: str, resource: str | None = None, real_id: str | None = None) -> Action:
    return Action(
        action_id=Fact.observed(real_id) if real_id else None,
        tool=tool,
        actual_resource=resource,
        state="executed",
    )


def test_compute_diff_detects_approval_scope_expansion_explicit_binding():
    ledger = AuthorityLedger()
    ledger.approvals["a1"] = Approval(approval_id=Fact.observed("a1"), target_action_ids=Fact.observed(("real-1",)))
    ledger.actions["act-1"] = _action("transfer", real_id="real-1")
    ledger.actions["act-2"] = _action("transfer", real_id="real-2")  # not in the approval's bound set
    envelope = derive_envelope(ledger, envelope_id="env-1", approval_id="a1")
    diff = compute_diff(ledger, envelope, diff_id="d1", observed_action_ids=["act-1", "act-2"])
    assert diff.divergence_type == DivergenceType.APPROVAL_SCOPE_MISMATCH
    assert diff.divergence_point == "Approval -> Action"
    assert "act-2" in diff.unexpected_authority
    assert "act-1" not in diff.unexpected_authority


def test_compute_diff_detects_approval_scope_expansion_ambiguous_scope():
    """The Mastra Candidate 4a shape: scope itself is unrepresentable
    (ambiguous), and more than one action resolved under it."""
    ledger = AuthorityLedger()
    ledger.approvals["a1"] = Approval(
        approval_id=Fact.observed("a1"),
        target_action_ids=Fact.unrepresentable("no field binds to one action"),
    )
    ledger.actions["act-1"] = _action("transfer")
    ledger.actions["act-2"] = _action("transfer")
    envelope = derive_envelope(ledger, envelope_id="env-1", approval_id="a1")
    diff = compute_diff(ledger, envelope, diff_id="d1", observed_action_ids=["act-1", "act-2"])
    assert diff.divergence_type == DivergenceType.APPROVAL_SCOPE_MISMATCH
    assert diff.divergence_point == "Approval -> Action"
    assert diff.unexpected_authority == ("act-2",)  # the second-observed action, not the first


def test_compute_diff_detects_no_divergence_correctly():
    ledger = AuthorityLedger()
    ledger.approvals["a1"] = Approval(approval_id=Fact.observed("a1"), target_action_ids=Fact.observed(("real-1",)))
    ledger.actions["act-1"] = _action("transfer", real_id="real-1")
    envelope = derive_envelope(ledger, envelope_id="env-1", approval_id="a1")
    diff = compute_diff(ledger, envelope, diff_id="d1", observed_action_ids=["act-1"])
    assert diff.divergence_type == DivergenceType.NONE
    assert diff.divergence_point is None
    assert diff.unexpected_authority == ()
    assert diff.missing_expected_authority == ()


def test_compute_diff_ambiguous_scope_with_exactly_one_action_is_not_a_divergence():
    """A single action resolved under an ambiguous-scope approval is not
    itself a problem -- only resolving MORE than one is."""
    ledger = AuthorityLedger()
    ledger.approvals["a1"] = Approval(
        approval_id=Fact.observed("a1"),
        target_action_ids=Fact.unrepresentable("no field binds to one action"),
    )
    ledger.actions["act-1"] = _action("transfer")
    envelope = derive_envelope(ledger, envelope_id="env-1", approval_id="a1")
    diff = compute_diff(ledger, envelope, diff_id="d1", observed_action_ids=["act-1"])
    assert diff.divergence_type == DivergenceType.NONE


def test_compute_diff_missing_evidence_does_not_create_a_false_divergence():
    """No grant, no approval evidenced at all (e.g. UNVERIFIABLE finding) --
    an empty envelope against zero observed actions must never manufacture
    a divergence from pure absence."""
    ledger = AuthorityLedger()
    envelope = AuthorityEnvelope(envelope_id="env-empty")
    diff = compute_diff(ledger, envelope, diff_id="d1", observed_action_ids=[])
    assert diff.divergence_type == DivergenceType.NONE
    assert diff.unexpected_authority == ()
    assert diff.missing_expected_authority == ()


def test_compute_diff_unrepresentable_scope_with_no_actions_remains_a_semantic_gap_not_a_divergence():
    ledger = AuthorityLedger()
    ledger.approvals["a1"] = Approval(
        approval_id=Fact.observed("a1"),
        target_action_ids=Fact.unrepresentable("no field binds to one action"),
    )
    envelope = derive_envelope(ledger, envelope_id="env-1", approval_id="a1")
    diff = compute_diff(ledger, envelope, diff_id="d1", observed_action_ids=[])
    assert diff.divergence_type == DivergenceType.NONE  # absence of actions is not itself a violation
    assert envelope.approval_scope is not None
    assert not envelope.approval_scope.is_available  # the gap is visible on the envelope, not hidden


def test_compute_diff_action_with_unknowable_real_id_is_not_silently_cleared():
    """An action whose real id cannot be established must not be silently
    treated as 'within scope' just because it can't be checked."""
    ledger = AuthorityLedger()
    ledger.approvals["a1"] = Approval(approval_id=Fact.observed("a1"), target_action_ids=Fact.observed(("real-1",)))
    ledger.actions["act-1"] = _action("transfer", real_id=None)  # unknown real id
    envelope = derive_envelope(ledger, envelope_id="env-1", approval_id="a1")
    diff = compute_diff(ledger, envelope, diff_id="d1", observed_action_ids=["act-1"])
    assert diff.divergence_type == DivergenceType.NONE  # not asserted as a mismatch...
    assert "could not be checked" in (diff.confidence_note or "")  # ...but flagged as an unresolved gap, not cleared silently


def test_compute_diff_scope_expansion_outside_allowed_tools():
    envelope = AuthorityEnvelope(envelope_id="env-1", allowed_tools=("get_customer",), allowed_resources=("cust_1",))
    ledger = AuthorityLedger()
    ledger.actions["act-1"] = _action("get_order", resource="cust_2")
    diff = compute_diff(ledger, envelope, diff_id="d1", observed_action_ids=["act-1"])
    assert diff.divergence_type == DivergenceType.SCOPE_EXPANSION
    assert diff.divergence_point == "Grant"
    assert diff.unexpected_authority == ("act-1",)


def test_compute_diff_first_divergence_point_is_stable_across_repeated_calls():
    ledger = AuthorityLedger()
    ledger.approvals["a1"] = Approval(
        approval_id=Fact.observed("a1"),
        target_action_ids=Fact.unrepresentable("no field binds to one action"),
    )
    ledger.actions["act-1"] = _action("transfer")
    ledger.actions["act-2"] = _action("transfer")
    envelope = derive_envelope(ledger, envelope_id="env-1", approval_id="a1")
    first = compute_diff(ledger, envelope, diff_id="d1", observed_action_ids=["act-1", "act-2"])
    second = compute_diff(ledger, envelope, diff_id="d1", observed_action_ids=["act-1", "act-2"])
    assert first.divergence_point == second.divergence_point == "Approval -> Action"
    assert first.divergence_type == second.divergence_type


def test_diff_round_trips_through_dict():
    ledger = AuthorityLedger()
    envelope = AuthorityEnvelope(envelope_id="env-1")
    diff = compute_diff(ledger, envelope, diff_id="d1", observed_action_ids=[])
    from agentlab.assessment.authority_model.diff import AuthorityDiff

    restored = AuthorityDiff.from_dict(diff.to_dict())
    assert restored == diff
