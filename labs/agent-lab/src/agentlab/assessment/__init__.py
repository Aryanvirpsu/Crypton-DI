"""
Crypton Agent Authorization & Delegation Assessment — delivery layer
(v0.6.1).

Turns the existing, frozen security primitives (`chain.py`, `delegation.py`,
`authorization.py`, `approval.py`) into a repeatable, evidence-backed,
client-deliverable methodology — without pretending Crypton can
automatically inspect an arbitrary customer's system. See
`docs/ASSESSMENT_METHODOLOGY.md` for the procedure this package supports,
and `DECISIONS.md`'s v0.6.1 section for why it's shaped the way it is.

This package contains no security logic of its own. It does not decide
whether a delegation chain is valid, whether an approval is fresh, or
whether a tool call should have been allowed — those questions belong
entirely to the modules listed above, called exactly as they always have
been. This package only models findings/evidence (`models.py`), indexes
evidence (`evidence.py`), structures an engagement (`engagement.py`),
renders a report (`report.py`), and demonstrates the whole flow against
Crypton's own reference implementation (`reference_dryrun.py`).
"""

from .engagement import Engagement, EngagementLoadError
from .evidence import DanglingEvidenceReferenceError, DuplicateEvidenceIdError, EvidenceManifest
from .models import (
    AssessmentArea,
    EvidenceClassification,
    EvidenceGateStatus,
    EvidenceReference,
    Finding,
    InconclusiveReason,
    LegacyOutcomeMigration,
    Outcome,
    Severity,
    migrate_legacy_outcome,
)
from .report import render_report

__all__ = [
    "AssessmentArea",
    "DanglingEvidenceReferenceError",
    "DuplicateEvidenceIdError",
    "Engagement",
    "EngagementLoadError",
    "EvidenceClassification",
    "EvidenceGateStatus",
    "EvidenceManifest",
    "EvidenceReference",
    "Finding",
    "InconclusiveReason",
    "LegacyOutcomeMigration",
    "Outcome",
    "Severity",
    "migrate_legacy_outcome",
    "render_report",
]
