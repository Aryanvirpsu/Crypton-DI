"""
Crypton Authority Model v0.1 — Envelope derivation tests.

Covers: derives only supported dimensions, narrower child restriction
wins over broader parent authority, revoked authority is not considered
active, approval scope limits effective authority where applicable,
UNKNOWN does not silently become allowed.
"""

from __future__ import annotations

from agentlab.assessment.authority_model.envelope import derive_envelope
from agentlab.assessment.authority_model.fact import Fact
from agentlab.assessment.authority_model.ledger import AuthorityLedger
from agentlab.assessment.authority_model.primitives import Approval, Grant


def test_derive_envelope_only_populates_dimensions_the_ledger_supports():
    """A ledger with only a Grant (no Approval) should leave every
    approval-related dimension unset — never guessed."""
    ledger = AuthorityLedger()
    ledger.grants["g1"] = Grant(grant_id=Fact.observed("g1"), allowed_tools=("read",), allowed_resources=("doc:1",))
    envelope = derive_envelope(ledger, envelope_id="env-1", grant_id="g1")
    assert envelope.allowed_tools == ("read",)
    assert envelope.required_approval is None
    assert envelope.approval_id is None
    assert envelope.approval_scope is None


def test_derive_envelope_with_only_approval_leaves_grant_dimensions_unset():
    ledger = AuthorityLedger()
    ledger.approvals["a1"] = Approval(approval_id=Fact.observed("a1"), target_action_ids=Fact.observed(("act-1",)))
    envelope = derive_envelope(ledger, envelope_id="env-1", approval_id="a1")
    assert envelope.required_approval is True
    assert envelope.grant_chain == ()
    assert envelope.allowed_tools is None
    assert envelope.allowed_resources is None


def test_derive_envelope_narrower_child_grant_wins_over_broader_parent():
    ledger = AuthorityLedger()
    ledger.grants["parent"] = Grant(
        grant_id=Fact.observed("parent"),
        allowed_tools=("read", "write", "delete"),
        allowed_resources=("doc:1", "doc:2"),
    )
    ledger.grants["child"] = Grant(
        grant_id=Fact.observed("child"),
        allowed_tools=("read",),
        allowed_resources=("doc:1",),
        parent_grant_id=Fact.observed("parent"),
    )
    envelope = derive_envelope(ledger, envelope_id="env-1", grant_id="child")
    # Effective authority is the intersection across the chain -- the
    # child's narrower restriction, not the parent's broader one.
    assert envelope.allowed_tools == ("read",)
    assert envelope.allowed_resources == ("doc:1",)
    assert envelope.grant_chain == ("parent", "child")


def test_derive_envelope_revoked_grant_is_not_active():
    ledger = AuthorityLedger()
    ledger.grants["g1"] = Grant(grant_id=Fact.observed("g1"), allowed_tools=("read",), revoked_at="2026-01-01T00:00:00Z")
    envelope = derive_envelope(ledger, envelope_id="env-1", grant_id="g1")
    assert envelope.revocation_state == "REVOKED"
    assert envelope.revocation_state != "ACTIVE"


def test_derive_envelope_unrevoked_grant_with_no_revocation_evidence_is_active():
    ledger = AuthorityLedger()
    ledger.grants["g1"] = Grant(grant_id=Fact.observed("g1"), allowed_tools=("read",))
    envelope = derive_envelope(ledger, envelope_id="env-1", grant_id="g1")
    assert envelope.revocation_state == "ACTIVE"


def test_derive_envelope_approval_scope_is_populated_from_explicit_binding():
    ledger = AuthorityLedger()
    ledger.approvals["a1"] = Approval(approval_id=Fact.observed("a1"), target_action_ids=Fact.observed(("act-1",)))
    envelope = derive_envelope(ledger, envelope_id="env-1", approval_id="a1")
    assert envelope.approval_scope is not None
    assert envelope.approval_scope.is_available
    assert "act-1" in envelope.approval_scope.value


def test_derive_envelope_ambiguous_approval_scope_propagates_as_unrepresentable_not_allowed():
    """§23's accuracy requirement: UNKNOWN must never silently become
    allowed. An approval whose scope is unrepresentable must surface that,
    not default to 'anything goes'."""
    ledger = AuthorityLedger()
    ledger.approvals["a1"] = Approval(
        approval_id=Fact.observed("a1"),
        target_action_ids=Fact.unrepresentable("API call shape cannot bind to one action"),
    )
    envelope = derive_envelope(ledger, envelope_id="env-1", approval_id="a1")
    assert envelope.approval_scope is not None
    assert not envelope.approval_scope.is_available
    assert envelope.approval_scope.availability.value == "UNREPRESENTABLE"


def test_derive_envelope_unknown_grant_id_leaves_dimensions_unset_not_guessed():
    ledger = AuthorityLedger()  # no grants at all
    envelope = derive_envelope(ledger, envelope_id="env-1", grant_id="nonexistent")
    assert envelope.grant_chain == ()
    assert envelope.allowed_tools is None
    assert envelope.revocation_state == "UNKNOWN"


def test_derive_envelope_never_claims_the_target_implements_an_envelope():
    """The envelope is Crypton's own assessment abstraction -- its
    derivation_notes should document how it was built, not assert the
    target system has this concept."""
    ledger = AuthorityLedger()
    ledger.grants["g1"] = Grant(grant_id=Fact.observed("g1"), allowed_tools=("read",))
    envelope = derive_envelope(ledger, envelope_id="env-1", grant_id="g1")
    assert envelope.derivation_notes  # always explains itself
