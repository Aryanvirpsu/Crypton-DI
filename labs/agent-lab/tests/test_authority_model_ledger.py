"""
Crypton Authority Model v0.1 — Ledger tests.

Covers: event serialize/deserialize, optional execution-context
identifiers staying distinct, missing identities staying missing,
provenance preserved, evidence references preserved.
"""

from __future__ import annotations

import json

import pytest

from agentlab.assessment.authority_model.enums import EventType, Provenance, SemanticAvailability
from agentlab.assessment.authority_model.fact import Fact
from agentlab.assessment.authority_model.ledger import AuthorityLedger, LedgerEvent
from agentlab.assessment.authority_model.primitives import Action, ExecutionContext, Principal


def test_ledger_event_round_trips_through_dict():
    ctx = ExecutionContext(run_id="run-1", session_id=None, thread_id="thread-9")
    event = LedgerEvent(
        event_id="evt-1",
        event_type=EventType.ACTION_EXECUTED,
        assessment_id="ENG-1",
        test_id="T-1",
        timestamp="2026-10-01T00:00:00Z",
        sequence=3,
        execution_context=ctx,
        principal_id="p-1",
        actor_id="a-1",
        action_id="act-1",
        evidence_ids=("E-001", "E-002"),
        provenance=Provenance.OBSERVED,
    )
    restored = LedgerEvent.from_dict(event.to_dict())
    assert restored == event


def test_ledger_event_round_trips_through_json_text():
    event = LedgerEvent(event_id="evt-1", event_type=EventType.GRANT_CREATED)
    restored = LedgerEvent.from_dict(json.loads(json.dumps(event.to_dict())))
    assert restored == event


def test_execution_context_identifiers_remain_individually_distinct():
    """Spec §3: do not collapse run_id/session_id/thread_id/workflow_id/
    task_id/request_id/instance_id into one generic session id — only the
    identifiers actually exposed should be populated, and each is its own
    field, not merged."""
    ctx = ExecutionContext(run_id="run-1", thread_id="thread-1")
    assert ctx.run_id == "run-1"
    assert ctx.thread_id == "thread-1"
    assert ctx.session_id is None
    assert ctx.workflow_id is None
    assert ctx.task_id is None
    assert ctx.request_id is None
    assert ctx.instance_id is None
    restored = ExecutionContext.from_dict(ctx.to_dict())
    assert restored.run_id == "run-1"
    assert restored.thread_id == "thread-1"
    assert restored.session_id is None


def test_ledger_event_never_fabricates_an_absent_reference():
    """A LedgerEvent with no grant/approval/resource involved should carry
    None for those fields, not an invented placeholder."""
    event = LedgerEvent(event_id="evt-1", event_type=EventType.CONTEXT_BOUND)
    assert event.grant_id is None
    assert event.approval_id is None
    assert event.resource_id is None
    assert event.effect_id is None


def test_missing_principal_identity_remains_missing_through_round_trip():
    principal = Principal(principal_type="human", principal_id=Fact.unknown("not named in cited evidence"))
    restored = Principal.from_dict(principal.to_dict())
    assert restored.principal_id.value is None
    assert restored.principal_id.availability == SemanticAvailability.UNKNOWN
    assert "not named" in restored.principal_id.note


def test_provenance_is_preserved_across_round_trip():
    action = Action(tool="transfer", provenance=Provenance.CONFIGURED, evidence_ids=("E-009",))
    restored = Action.from_dict(action.to_dict())
    assert restored.provenance == Provenance.CONFIGURED


def test_evidence_references_are_preserved_across_round_trip():
    action = Action(tool="transfer", evidence_ids=("E-001", "E-002", "E-003"))
    restored = Action.from_dict(action.to_dict())
    assert restored.evidence_ids == ("E-001", "E-002", "E-003")


def test_ledger_save_and_load_round_trips(tmp_path):
    ledger = AuthorityLedger(assessment_id="ENG-1")
    ledger.add_event(LedgerEvent(event_id="evt-1", event_type=EventType.GRANT_CREATED, assessment_id="ENG-1"))
    path = tmp_path / "authority-ledger.json"
    ledger.save(path)
    restored = AuthorityLedger.load(path)
    assert restored.to_dict() == ledger.to_dict()


def test_events_of_filters_by_type():
    ledger = AuthorityLedger()
    ledger.add_event(LedgerEvent(event_id="e1", event_type=EventType.GRANT_CREATED))
    ledger.add_event(LedgerEvent(event_id="e2", event_type=EventType.ACTION_EXECUTED))
    ledger.add_event(LedgerEvent(event_id="e3", event_type=EventType.ACTION_EXECUTED))
    assert len(ledger.events_of(EventType.ACTION_EXECUTED)) == 2
    assert len(ledger.events_of(EventType.GRANT_CREATED)) == 1
    assert len(ledger.events_of(EventType.GRANT_REVOKED)) == 0


def test_fact_rejects_a_value_alongside_non_available_status():
    with pytest.raises(ValueError):
        Fact(value="x", provenance=Provenance.UNKNOWN, availability=SemanticAvailability.UNKNOWN)


def test_fact_rejects_available_status_with_no_value():
    with pytest.raises(ValueError):
        Fact(value=None, provenance=Provenance.OBSERVED, availability=SemanticAvailability.AVAILABLE)


@pytest.mark.parametrize("ctor_name", ["unrepresentable", "unverifiable"])
def test_fact_requires_a_note_for_unrepresentable_and_unverifiable(ctor_name):
    ctor = getattr(Fact, ctor_name)
    with pytest.raises(ValueError):
        ctor("")


def test_fact_unknown_does_not_require_a_note():
    fact = Fact.unknown()
    assert fact.value is None
    assert fact.availability == SemanticAvailability.UNKNOWN
