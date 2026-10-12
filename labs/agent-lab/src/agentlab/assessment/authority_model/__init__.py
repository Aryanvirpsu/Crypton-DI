"""
Crypton Authority Model v0.1.

A *derived* evidence-backed layer over the existing assessment schema —
RAW EVIDENCE -> AUTHORITY LEDGER -> AUTHORITY ENVELOPE -> AUTHORITY DIFF ->
Finding/Action. Built from engagement, property/test, and evidence
records that already exist; never a second, manually maintained source of
truth. See each module's docstring for the layer it implements, and
`docs/ASSESSMENT_METHODOLOGY.md`'s Authority Model section for the
governing design rules.
"""

from .diff import AuthorityDiff, compute_diff
from .enums import ActorType, DivergenceType, EventType, Provenance, SemanticAvailability
from .envelope import AuthorityEnvelope, derive_envelope
from .fact import Fact
from .ledger import AuthorityLedger, LedgerEvent
from .presentation import (
    ActionStripView,
    AuthorityHeader,
    AuthorityView,
    EvidenceStripRow,
    PathStep,
    SemanticGap,
    build_authority_view,
)
from .primitives import Action, Actor, Approval, Effect, ExecutionContext, Grant, Principal, Resource
from .render import render_authority_map
from .render_text import (
    render_action_strip,
    render_authority_diff,
    render_authority_header,
    render_authority_map_structural,
    render_evidence_strip,
    render_full_text_report,
)

__all__ = [
    "Action",
    "ActionStripView",
    "Actor",
    "ActorType",
    "Approval",
    "AuthorityDiff",
    "AuthorityEnvelope",
    "AuthorityHeader",
    "AuthorityLedger",
    "AuthorityView",
    "DivergenceType",
    "Effect",
    "EventType",
    "EvidenceStripRow",
    "ExecutionContext",
    "Fact",
    "Grant",
    "LedgerEvent",
    "PathStep",
    "Principal",
    "Provenance",
    "Resource",
    "SemanticAvailability",
    "SemanticGap",
    "build_authority_view",
    "compute_diff",
    "derive_envelope",
    "render_action_strip",
    "render_authority_diff",
    "render_authority_header",
    "render_authority_map",
    "render_authority_map_structural",
    "render_evidence_strip",
    "render_full_text_report",
]
