"""
v0.6.1 — data shapes for the Agent Authorization & Delegation Assessment
delivery layer.

**2026-09-30 remediation pass:** this module now implements the canonical,
customer-facing *five*-outcome model (`Outcome`: ENFORCED / VIOLATED /
UNREPRESENTABLE / UNVERIFIABLE / NOT_APPLICABLE), the seven-gate
evidence-sufficiency model (`EvidenceGateStatus`), and an expanded
property/test-record shape on `Finding`. This is a forward-looking schema
change to the *engagement-delivery* layer only — it does not rewrite any
historical research artifact. EXP-003's PASS/FAIL/INCONCLUSIVE evidence
JSON, EXP-005's findings, the Validation Benchmark's verdicts, and every
other frozen research artifact in `labs/` keep their original vocabulary
exactly as recorded; see `migrate_legacy_outcome` below for how (and when)
a specific historical conclusion may be deliberately promoted into this
model, and `docs/ASSESSMENT_METHODOLOGY.md` for the full outcome
definitions and the seven gates.

This module is deliberately dumb: plain dataclasses, no business logic,
no calls into `chain.py`/`delegation.py`/`authorization.py`/`approval.py`.
It exists so a `Finding` produced by a human assessor (from source review,
config inspection, a staging test, or a Crypton-reference demonstration)
and a `Finding` produced by `reference_dryrun.py` (from a real chain
verification call) are the exact same shape — the report renderer never
has to know which one it's looking at.

Two things this module is careful to get right, per
`docs/ASSESSMENT_METHODOLOGY.md`:

  1. Outcome and severity are independent. An ENFORCED finding normally has
     no severity at all — severity answers "how bad is the gap," and a
     holding control has no gap. `Finding.severity` is `None` unless the
     assessor deliberately sets it (e.g. an INFORMATIONAL note attached
     to an otherwise-ENFORCED control).
  2. ENFORCED and VIOLATED conclusions require evidence *and* a fully
     satisfied seven-gate evidence-sufficiency record.
     `Finding.__post_init__` enforces this structurally — an ENFORCED or
     VIOLATED `Finding` with an empty `evidence_ids` tuple, a missing
     `evidence_gate_status`, or any gate not explicitly `True` raises
     `ValueError` at construction time, not silently later at render time.
     UNREPRESENTABLE and UNVERIFIABLE do not require evidence or a
     satisfied gate record (their whole point is often "the property
     cannot be represented" or "evidence could not be obtained"), but they
     *do* require a non-empty `limitations` stating which of the two
     applies and why — a bare UNREPRESENTABLE/UNVERIFIABLE with no stated
     reason is not acceptable practice, and a missing log field alone is
     never, by itself, sufficient grounds for UNREPRESENTABLE (see
     `docs/ASSESSMENT_METHODOLOGY.md`'s "The five outcomes" table).
     NOT_APPLICABLE requires neither.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Any


class AssessmentArea(str, Enum):
    """The eight core areas from `docs/ASSESSMENT_METHODOLOGY.md`. Kept as
    a fixed, small enum on purpose — this is the assessment's table of
    contents, not an open-ended taxonomy a caller can extend ad hoc."""

    IDENTITY_OWNERSHIP = "identity_ownership"
    DELEGATION_AUTHORITY_FLOW = "delegation_authority_flow"
    SCOPE_ATTENUATION = "scope_attenuation"
    PRE_EXECUTION_BOUNDARY = "pre_execution_boundary"
    AUTHORIZATION_BYPASS = "authorization_bypass"
    APPROVAL_BINDING = "approval_binding"
    REVOCATION_EXPIRY = "revocation_expiry"
    EVIDENCE_ACCOUNTABILITY = "evidence_accountability"

    @property
    def label(self) -> str:
        """Human-readable heading, matching the methodology doc's section
        titles exactly, so a report and the methodology doc read as one
        consistent document."""
        return _AREA_LABELS[self]


_AREA_LABELS: dict[AssessmentArea, str] = {
    AssessmentArea.IDENTITY_OWNERSHIP: "Agent Identity & Ownership",
    AssessmentArea.DELEGATION_AUTHORITY_FLOW: "Delegation & Authority Flow",
    AssessmentArea.SCOPE_ATTENUATION: "Scope Attenuation / Privilege Inheritance",
    AssessmentArea.PRE_EXECUTION_BOUNDARY: "Pre-Execution Authorization Boundary & Bypass",
    AssessmentArea.AUTHORIZATION_BYPASS: "Authorization Bypass Paths",
    AssessmentArea.APPROVAL_BINDING: "Human-Approval Binding (Replay / TOCTOU / Drift)",
    AssessmentArea.REVOCATION_EXPIRY: "Revocation / Expiry Propagation",
    AssessmentArea.EVIDENCE_ACCOUNTABILITY: "Evidence / Accountability Reconstruction",
}


class Outcome(str, Enum):
    """The five, and only five, customer-facing conclusions a finding may
    reach. This is the canonical outcome model as of the 2026-09-30
    remediation pass (`docs/audits/2026-09-30-evidence-report-readiness-audit.md`
    §5.3), replacing the prior PASS/FAIL/INCONCLUSIVE/NOT_APPLICABLE
    four-outcome model in *this* engagement-delivery layer only.

        ENFORCED          The claimed security property was evidenced and
                           held, across every gate in the seven-gate
                           evidence-sufficiency model (see
                           `EvidenceGateStatus`).
        VIOLATED          The claimed security property was evidenced,
                           every gate was satisfied, AND Gate 7
                           (security-claim / trust-boundary triage)
                           affirmatively confirmed the observed behavior
                           crosses a real, claimed security boundary — not
                           merely that something unexpected happened.
        UNREPRESENTABLE   The target system does not contain the semantic
                           state needed to express this property at all
                           (e.g. no approver-identity field exists
                           anywhere in the framework to test). This is
                           never established by a single missing log
                           field alone — it requires confirming the
                           underlying semantic concept has no
                           representation anywhere reachable, not just
                           that one particular record didn't carry it.
        UNVERIFIABLE      The property is representable, but sufficient
                           permitted evidence could not be obtained within
                           this engagement's actual access/scope (a
                           customer withheld source access, a staging
                           environment was unreachable, a retention window
                           had already expired).
        NOT_APPLICABLE    The control genuinely does not apply to the
                           workflow being assessed (e.g. no human-approval
                           step exists at all, so replay/TOCTOU findings
                           for it don't apply).

    "Requires manual review" is NOT an outcome — manual review (source
    inspection, config reading, an assessor-run staging test) is a valid
    *method* of reaching ENFORCED or VIOLATED, not an excuse to default to
    UNVERIFIABLE. UNVERIFIABLE means evidence genuinely could not be
    obtained by any method available in the engagement; UNREPRESENTABLE
    means the question genuinely does not have an answer the system could
    give, not "we didn't look."

    Research-only vocabulary (PARTIAL PASS, COVERAGE GAP, SPLIT,
    PASS_WITH_CAVEAT, CANDIDATE_FINDING, and similar labels used inside
    `docs/methodology/` and the individual `labs/exp-00*` experiments) may
    continue to be used in historical and research artifacts — this
    enum governs only what a *customer engagement record* (a `Finding` in
    this module) is allowed to resolve to. See `migrate_legacy_outcome`
    for how a specific historical result is deliberately promoted into
    this vocabulary, and `docs/ASSESSMENT_METHODOLOGY.md` for the full
    outcome definitions and the seven evidence-sufficiency gates.
    """

    ENFORCED = "ENFORCED"
    VIOLATED = "VIOLATED"
    UNREPRESENTABLE = "UNREPRESENTABLE"
    UNVERIFIABLE = "UNVERIFIABLE"
    NOT_APPLICABLE = "NOT_APPLICABLE"


class Severity(str, Enum):
    """Only ever meaningful on a VIOLATED (or, occasionally, an
    INFORMATIONAL note on an otherwise-ENFORCED area) — see
    `docs/ASSESSMENT_METHODOLOGY.md`'s severity rubric for the contextual
    factors an assessor weighs to pick one of these; this enum is
    deliberately just the five labels, not a scoring formula."""

    CRITICAL = "CRITICAL"
    HIGH = "HIGH"
    MEDIUM = "MEDIUM"
    LOW = "LOW"
    INFORMATIONAL = "INFORMATIONAL"


class EvidenceClassification(str, Enum):
    """2026-09-30 addition. Every newly captured evidence item should carry
    one of these. `None` (the default on `EvidenceReference`) means the
    item predates this schema version — classification is never
    retroactively fabricated for old evidence that never recorded it (see
    `EvidenceReference.classification`'s docstring)."""

    PUBLIC = "PUBLIC"
    RELEASE_APPROVED = "RELEASE_APPROVED"
    CUSTOMER_CONFIDENTIAL = "CUSTOMER_CONFIDENTIAL"
    CRYPTON_PROPRIETARY = "CRYPTON_PROPRIETARY"


_GATE_FIELD_NAMES: tuple[str, ...] = (
    "g1_representability",
    "g2_observability",
    "g3_controlled_execution",
    "g4_effect_confirmation",
    "g5_calibration_control",
    "g6_reproduction",
    "g7_security_claim_triage",
)


@dataclass(frozen=True)
class EvidenceGateStatus:
    """The seven-gate evidence-sufficiency model (2026-09-30 remediation
    pass, extending the six-gate model from
    `docs/audits/2026-09-30-evidence-report-readiness-audit.md` §4 with a
    new Gate 7):

        G1  Representability        Does the target contain the semantic
                                      state necessary to express this
                                      property at all?
        G2  Observability           Was enough of that state actually
                                      captured to evaluate it?
        G3  Controlled execution    Was the relevant condition actually
                                      exercised, not just reasoned about?
        G4  Effect confirmation     Was the real resulting effect verified
                                      through a sink, ledger, state
                                      transition, or other objective
                                      evidence — never model/narrative
                                      text alone?
        G5  Calibration / control   Was there a negative, baseline, or
                                      calibration case demonstrating the
                                      test actually measures the intended
                                      property?
        G6  Reproduction            Was the behavior reproduced enough to
                                      support the strength of the claim?
        G7  Security-claim /        Does the observed behavior actually
            trust-boundary triage    satisfy or violate a security
                                      property the system *claims* to hold
                                      across a real, relevant trust
                                      boundary — as opposed to evidence-
                                      sufficient-but-not-actually-a-
                                      violation behavior? This is the gate
                                      the EXP-001d/e closure
                                      (`labs/experiments/001-langgraph-approval-drift/CLOSURE.md`,
                                      2026-09-27) exists to formalize:
                                      that finding passed G1-G6 cleanly
                                      and was still the wrong conclusion,
                                      because the mutating code was
                                      trusted, same-trust-domain, and
                                      explicitly permitted by the
                                      framework's own contract — no
                                      claimed boundary was actually
                                      crossed. Evidence sufficiency alone
                                      (G1-G6) must never promote a
                                      finding to VIOLATED; only an
                                      affirmative G7 may.

    Each field is `True` (satisfied), `False` (not satisfied — the gate
    was evaluated and failed), or `None` (not evaluated / no information).
    `Finding.__post_init__` requires every field to be exactly `True` —
    not merely non-`False` — before an ENFORCED or VIOLATED outcome may be
    constructed; see that method's docstring for why `None` is treated the
    same as `False` for this purpose (an unevaluated gate is not a
    satisfied one).
    """

    g1_representability: bool | None = None
    g2_observability: bool | None = None
    g3_controlled_execution: bool | None = None
    g4_effect_confirmation: bool | None = None
    g5_calibration_control: bool | None = None
    g6_reproduction: bool | None = None
    g7_security_claim_triage: bool | None = None
    notes: str | None = None

    def all_satisfied(self) -> bool:
        """True only if every one of the seven gates is explicitly `True`
        — an unevaluated (`None`) or failed (`False`) gate both count as
        not satisfied."""
        return all(getattr(self, name) is True for name in _GATE_FIELD_NAMES)

    def unsatisfied_gates(self) -> tuple[str, ...]:
        """Names of every gate that is not exactly `True`, for building a
        precise error/report message."""
        return tuple(name for name in _GATE_FIELD_NAMES if getattr(self, name) is not True)

    def to_dict(self) -> dict[str, Any]:
        return {name: getattr(self, name) for name in _GATE_FIELD_NAMES} | {"notes": self.notes}

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "EvidenceGateStatus":
        return cls(**{name: data.get(name) for name in _GATE_FIELD_NAMES}, notes=data.get("notes"))


_LEGACY_STATUSES: frozenset[str] = frozenset({"PASS", "FAIL", "INCONCLUSIVE", "NOT_APPLICABLE"})

# NOT_APPLICABLE is the one literal shared by both the legacy four-outcome
# model and the canonical five-outcome model (it migrates to itself — see
# migrate_legacy_outcome below) — it is valid to load directly and must NOT
# be treated as an ambiguous legacy value the way PASS/FAIL/INCONCLUSIVE
# are. Finding.from_dict's loud-refusal guard checks against this narrower
# set, not the full _LEGACY_STATUSES above.
_AMBIGUOUS_LEGACY_VALUES: frozenset[str] = frozenset({"PASS", "FAIL", "INCONCLUSIVE"})


class InconclusiveReason(str, Enum):
    """Why a historical INCONCLUSIVE result occurred — required to migrate
    it into the canonical model (see `migrate_legacy_outcome`). There is no
    catch-all third value on purpose: anything that isn't one of these two
    reasons must stay unmigrated and go to an assessor, per the
    remediation spec's explicit instruction that INCONCLUSIVE must never
    be mechanically converted."""

    MISSING_SEMANTIC_STATE = "missing_semantic_state"
    INSUFFICIENT_PERMITTED_EVIDENCE = "insufficient_permitted_evidence"


@dataclass(frozen=True)
class LegacyOutcomeMigration:
    """The result of attempting to migrate one historical PASS/FAIL/
    INCONCLUSIVE/NOT_APPLICABLE conclusion into the canonical `Outcome`
    model. `outcome` is `None` when the migration cannot be performed
    automatically and safely — in that case the historical artifact is
    left exactly as originally recorded, `requires_assessor_resolution`
    is `True`, and `rationale` explains what an assessor needs to decide
    before this record may appear in a customer-facing report under the
    canonical vocabulary."""

    legacy_status: str
    outcome: Outcome | None
    rationale: str
    requires_assessor_resolution: bool


def migrate_legacy_outcome(
    legacy_status: str,
    *,
    security_claim_crossed: bool | None = None,
    inconclusive_reason: InconclusiveReason | None = None,
) -> LegacyOutcomeMigration:
    """Migrate one historical four-outcome conclusion into the canonical
    five-outcome model, per the exact semantics of the 2026-09-30
    remediation spec:

        PASS            -> ENFORCED, always.
        NOT_APPLICABLE  -> NOT_APPLICABLE, always.
        FAIL            -> VIOLATED *only* when `security_claim_crossed`
                           is `True` (an explicit, positive Gate 7
                           result). Otherwise returns `outcome=None` and
                           `requires_assessor_resolution=True` — a FAIL
                           must never be mechanically promoted to VIOLATED
                           on evidence sufficiency alone. See the
                           EXP-001d/e closure
                           (`labs/experiments/001-langgraph-approval-drift/CLOSURE.md`):
                           that finding had real, reproducible,
                           multi-run evidence (G1-G6 all satisfied) and
                           was still reclassified `NO VULNERABILITY —
                           EXPECTED FRAMEWORK BEHAVIOR` on 2026-09-27,
                           because the triage question (did this actually
                           cross a claimed boundary?) had never been
                           asked. This function makes that mistake
                           structurally impossible to repeat during
                           migration.
        INCONCLUSIVE    -> UNREPRESENTABLE if `inconclusive_reason` is
                           `MISSING_SEMANTIC_STATE` (the target has no
                           semantic state to express the property at
                           all); UNVERIFIABLE if `INSUFFICIENT_PERMITTED_EVIDENCE`
                           (the property is representable, but evidence
                           could not be obtained within what was
                           observable/permitted). With no reason given,
                           returns `outcome=None` and
                           `requires_assessor_resolution=True` — never
                           mechanically converted.

    This function never mutates anything; it is advisory. Calling it does
    not touch any file. A caller promoting a historical result into a real
    engagement's `findings.json` constructs a fresh `Finding` with the
    returned `outcome` (when not `None`) themselves — this function only
    tells them which outcome is safe to use and why, or that none is.
    """
    legacy = legacy_status.strip().upper()
    if legacy not in _LEGACY_STATUSES:
        raise ValueError(f"{legacy_status!r} is not one of the four legacy statuses {sorted(_LEGACY_STATUSES)}")

    if legacy == "PASS":
        return LegacyOutcomeMigration(legacy, Outcome.ENFORCED, "PASS always migrates to ENFORCED.", False)

    if legacy == "NOT_APPLICABLE":
        return LegacyOutcomeMigration(
            legacy, Outcome.NOT_APPLICABLE, "NOT_APPLICABLE always migrates to NOT_APPLICABLE.", False
        )

    if legacy == "FAIL":
        if security_claim_crossed is True:
            return LegacyOutcomeMigration(
                legacy,
                Outcome.VIOLATED,
                "Gate 7 (security-claim / trust-boundary triage) affirmatively confirmed a real, "
                "claimed security boundary was crossed.",
                False,
            )
        return LegacyOutcomeMigration(
            legacy,
            None,
            "FAIL does not auto-migrate to VIOLATED without an explicit, positive Gate 7 result "
            "(security_claim_crossed=True). Passing G1-G6 is not sufficient — see the EXP-001d/e "
            "closure (labs/experiments/001-langgraph-approval-drift/CLOSURE.md, 2026-09-27): that "
            "finding was fully evidence-sufficient and still not a real violation, because the "
            "mutating code was trusted, same-trust-domain, and permitted by the framework's own "
            "contract. An assessor must explicitly run Gate 7 triage on this specific historical "
            "FAIL before it may be reported as VIOLATED.",
            True,
        )

    # INCONCLUSIVE
    if inconclusive_reason is InconclusiveReason.MISSING_SEMANTIC_STATE:
        return LegacyOutcomeMigration(
            legacy,
            Outcome.UNREPRESENTABLE,
            "The target system does not represent the semantic state needed to express this "
            "property at all.",
            False,
        )
    if inconclusive_reason is InconclusiveReason.INSUFFICIENT_PERMITTED_EVIDENCE:
        return LegacyOutcomeMigration(
            legacy,
            Outcome.UNVERIFIABLE,
            "The property is representable, but sufficient evidence could not be obtained within "
            "what was observable or permitted in the engagement that produced this result.",
            False,
        )
    return LegacyOutcomeMigration(
        legacy,
        None,
        "INCONCLUSIVE must never be mechanically converted without a stated reason. Provide "
        "inconclusive_reason=InconclusiveReason.MISSING_SEMANTIC_STATE or "
        "InconclusiveReason.INSUFFICIENT_PERMITTED_EVIDENCE, or leave this historical artifact's "
        "INCONCLUSIVE exactly as recorded and resolve it manually before it may appear in a "
        "customer-facing report under the canonical outcome model.",
        True,
    )


@dataclass(frozen=True)
class EvidenceReference:
    """One entry in an engagement's evidence manifest — a pointer to
    something that was actually inspected/collected, not the evidence
    content itself (see `evidence.py`'s module docstring for why this is
    an index, not a document store).

    `evidence_type` is free text on purpose (examples: "architecture_diagram",
    "source_excerpt", "authorization_log", "execution_trace",
    "approval_record", "staging_test_result", "config_excerpt",
    "identity_record", "live_observation") — the methodology doc lists the
    expected categories, but a fixed enum here would force every real
    engagement's evidence into a box drawn before anyone had seen the
    customer's actual artifacts.

    **2026-09-30 expansion:** the fields below `notes` are new. All are
    optional and default to `None` — loading an older evidence manifest
    that predates this schema version is unaffected, and no value is ever
    fabricated for a field an old record never captured (per the
    remediation spec's explicit "do NOT fabricate chain-of-custody values
    for old evidence that never recorded them" instruction). Populate
    these for every newly captured evidence item going forward.
    """

    evidence_id: str
    evidence_type: str
    source: str
    description: str
    timestamp: str | None = None  # ISO 8601 if known; evidence often predates the assessment
    reference: str | None = None  # file path, log query, ticket link, etc. ("path_or_reference")
    integrity_hash: str | None = None  # e.g. "sha256:<hex>" — only when cheap/practical to compute
    notes: str | None = None
    # --- 2026-09-30 expansion ---
    assessment_id: str | None = None
    test_id: str | None = None
    run_id: str | None = None
    target_product: str | None = None
    target_version: str | None = None
    target_commit: str | None = None
    environment_manifest_reference: str | None = None
    supports: str | None = None  # free text: what claim/finding this evidence item supports
    redaction_status: str | None = None  # e.g. "unredacted", "redacted — customer PII removed"
    classification: EvidenceClassification | None = None

    def __post_init__(self) -> None:
        if not self.evidence_id.strip():
            raise ValueError("evidence_id must not be empty")
        if not self.evidence_type.strip():
            raise ValueError("evidence_type must not be empty")
        if not self.description.strip():
            raise ValueError("description must not be empty")

    def to_dict(self) -> dict[str, Any]:
        return {
            "evidence_id": self.evidence_id,
            "evidence_type": self.evidence_type,
            "source": self.source,
            "description": self.description,
            "timestamp": self.timestamp,
            "reference": self.reference,
            "integrity_hash": self.integrity_hash,
            "notes": self.notes,
            "assessment_id": self.assessment_id,
            "test_id": self.test_id,
            "run_id": self.run_id,
            "target_product": self.target_product,
            "target_version": self.target_version,
            "target_commit": self.target_commit,
            "environment_manifest_reference": self.environment_manifest_reference,
            "supports": self.supports,
            "redaction_status": self.redaction_status,
            "classification": self.classification.value if self.classification else None,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "EvidenceReference":
        classification = data.get("classification")
        return cls(
            evidence_id=data["evidence_id"],
            evidence_type=data["evidence_type"],
            source=data["source"],
            description=data["description"],
            timestamp=data.get("timestamp"),
            reference=data.get("reference"),
            integrity_hash=data.get("integrity_hash"),
            notes=data.get("notes"),
            assessment_id=data.get("assessment_id"),
            test_id=data.get("test_id"),
            run_id=data.get("run_id"),
            target_product=data.get("target_product"),
            target_version=data.get("target_version"),
            target_commit=data.get("target_commit"),
            environment_manifest_reference=data.get("environment_manifest_reference"),
            supports=data.get("supports"),
            redaction_status=data.get("redaction_status"),
            classification=EvidenceClassification(classification) if classification else None,
        )


@dataclass(frozen=True)
class Finding:
    """One conclusion about one control, for one assessment area — and, as
    of the 2026-09-30 remediation pass, also a property/test record
    (`docs/audits/2026-09-30-evidence-report-readiness-audit.md` §4): the
    fields below `confidence` are new and optional, used when a `Finding`
    is derived from (or reports on) a specific, identifiable property
    test rather than only a narrative assessor conclusion.

    `finding_id` is assessor-assigned (e.g. "AD-04" — "AD" for
    "Agent/Delegation", numbered within the engagement), not
    auto-generated — a real engagement needs stable IDs that survive
    re-renders and can be cited in conversation with a customer
    ("the issue we discussed is AD-07"), which a hash or a fresh UUID per
    render would not give. `test_id`/`property_id` are a separate concept:
    `finding_id` identifies this conclusion *within this engagement*;
    `property_id` identifies *which stable, cross-engagement property*
    (see `docs/methodology/08-exp003-property-id-mapping.md` for why this
    distinction matters) it concerns, and `test_id` identifies the
    specific test/procedure that produced the evidence.

    `severity` is `None` unless the assessor sets it — see the module
    docstring. `recommendation` (pre-existing) and `risk_impact` are free
    text; this module does not template them. `recommendation` is this
    schema's field for what the remediation-spec calls `remediation` —
    the two are the same concept, kept under their original name rather
    than duplicated under a second field, per the remediation pass's
    "consolidate, don't duplicate" instruction. Likewise `limitations`
    (pre-existing) is this schema's field for what the spec calls
    `evidence_limitations`.
    """

    finding_id: str
    area: AssessmentArea
    title: str
    status: Outcome
    observation: str
    expected_control: str
    evidence_ids: tuple[str, ...] = field(default_factory=tuple)
    severity: Severity | None = None
    risk_impact: str | None = None
    recommendation: str | None = None
    limitations: str | None = None
    confidence: str | None = None
    # --- 2026-09-30 expansion: property/test-record fields ---
    test_id: str | None = None
    property_id: str | None = None
    property_family: str | None = None
    tested_layer: str | None = None
    required_semantic_facts: tuple[str, ...] = field(default_factory=tuple)
    invariant: str | None = None
    preconditions: str | None = None
    initial_state: str | None = None
    stimulus: str | None = None
    expected: str | None = None
    observed: str | None = None
    effect: str | None = None
    control_or_calibration: str | None = None
    reproduction: str | None = None
    retest_condition: str | None = None
    evidence_gate_status: EvidenceGateStatus | None = None

    def __post_init__(self) -> None:
        if not self.finding_id.strip():
            raise ValueError("finding_id must not be empty")
        if not self.title.strip():
            raise ValueError("title must not be empty")
        if not self.observation.strip():
            raise ValueError("observation must not be empty")
        if not self.expected_control.strip():
            raise ValueError("expected_control must not be empty")

        if self.status in (Outcome.ENFORCED, Outcome.VIOLATED):
            if not self.evidence_ids:
                raise ValueError(
                    f"finding {self.finding_id!r}: {self.status.value} conclusions require at least "
                    "one evidence reference (evidence_ids) — see docs/ASSESSMENT_METHODOLOGY.md"
                )
            if self.evidence_gate_status is None or not self.evidence_gate_status.all_satisfied():
                unsatisfied = (
                    self.evidence_gate_status.unsatisfied_gates() if self.evidence_gate_status else _GATE_FIELD_NAMES
                )
                raise ValueError(
                    f"finding {self.finding_id!r}: {self.status.value} conclusions require a fully "
                    f"satisfied seven-gate evidence_gate_status (unsatisfied: {', '.join(unsatisfied)}) "
                    "— see docs/ASSESSMENT_METHODOLOGY.md's seven-gate evidence-sufficiency model. "
                    "Passing gates G1-G6 alone is not sufficient for VIOLATED — Gate 7 "
                    "(security-claim / trust-boundary triage) must also be explicitly satisfied; see "
                    "the EXP-001d/e closure for why this is enforced structurally."
                )

        if self.status in (Outcome.UNREPRESENTABLE, Outcome.UNVERIFIABLE):
            if not (self.limitations or "").strip():
                raise ValueError(
                    f"finding {self.finding_id!r}: {self.status.value} requires a non-empty "
                    "`limitations` stating why (a missing log field alone is never, by itself, "
                    "sufficient grounds for UNREPRESENTABLE) — see docs/ASSESSMENT_METHODOLOGY.md"
                )

    def to_dict(self) -> dict[str, Any]:
        return {
            "finding_id": self.finding_id,
            "area": self.area.value,
            "title": self.title,
            "outcome": self.status.value,
            "observation": self.observation,
            "expected_control": self.expected_control,
            "evidence_ids": list(self.evidence_ids),
            "severity": self.severity.value if self.severity else None,
            "risk_impact": self.risk_impact,
            "recommendation": self.recommendation,
            "limitations": self.limitations,
            "confidence": self.confidence,
            "test_id": self.test_id,
            "property_id": self.property_id,
            "property_family": self.property_family,
            "tested_layer": self.tested_layer,
            "required_semantic_facts": list(self.required_semantic_facts),
            "invariant": self.invariant,
            "preconditions": self.preconditions,
            "initial_state": self.initial_state,
            "stimulus": self.stimulus,
            "expected": self.expected,
            "observed": self.observed,
            "effect": self.effect,
            "control_or_calibration": self.control_or_calibration,
            "reproduction": self.reproduction,
            "retest_condition": self.retest_condition,
            "evidence_gate_status": self.evidence_gate_status.to_dict() if self.evidence_gate_status else None,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "Finding":
        severity = data.get("severity")
        # Accept "outcome" (canonical, post-2026-09-30) or "status" (pre-remediation
        # key name) — but never silently reinterpret a *legacy value*
        # (PASS/FAIL/INCONCLUSIVE) found under either key. A legacy value must go
        # through migrate_legacy_outcome() explicitly; constructing an Outcome
        # enum from one directly raises ValueError, which is the correct,
        # loud failure mode here, not a guess.
        raw_outcome = data.get("outcome", data.get("status"))
        if raw_outcome in _AMBIGUOUS_LEGACY_VALUES:
            raise ValueError(
                f"finding {data.get('finding_id')!r} has a legacy outcome value {raw_outcome!r} — "
                "this must be migrated via migrate_legacy_outcome() with an explicit assessor "
                "decision before it can be loaded as a canonical Finding; it is not auto-converted."
            )
        gate_status = data.get("evidence_gate_status")
        return cls(
            finding_id=data["finding_id"],
            area=AssessmentArea(data["area"]),
            title=data["title"],
            status=Outcome(raw_outcome),
            observation=data["observation"],
            expected_control=data["expected_control"],
            evidence_ids=tuple(data.get("evidence_ids", [])),
            severity=Severity(severity) if severity else None,
            risk_impact=data.get("risk_impact"),
            recommendation=data.get("recommendation"),
            limitations=data.get("limitations"),
            confidence=data.get("confidence"),
            test_id=data.get("test_id"),
            property_id=data.get("property_id"),
            property_family=data.get("property_family"),
            tested_layer=data.get("tested_layer"),
            required_semantic_facts=tuple(data.get("required_semantic_facts", [])),
            invariant=data.get("invariant"),
            preconditions=data.get("preconditions"),
            initial_state=data.get("initial_state"),
            stimulus=data.get("stimulus"),
            expected=data.get("expected"),
            observed=data.get("observed"),
            effect=data.get("effect"),
            control_or_calibration=data.get("control_or_calibration"),
            reproduction=data.get("reproduction"),
            retest_condition=data.get("retest_condition"),
            evidence_gate_status=EvidenceGateStatus.from_dict(gate_status) if gate_status else None,
        )
