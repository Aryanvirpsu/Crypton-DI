"""
Stable semantic property IDs — code mirror of
`docs/methodology/08-exp003-property-id-mapping.md`.

Exists for exactly one reason: the presentation layer must never infer a
property's identity from a local H-number alone (that is the precise
cross-framework comparison error the 2026-09-30 evidence-readiness audit
found — EXP-003B's "H5" is a different property from EXP-003A's/EXP-003C's
"H5" — and the mapping document exists to correct). This module makes that
same, already-documented mapping available to code, so a fixture's local
label (an H-number, or no label at all) resolves to a stable `PROP-*` ID
and name through one explicit table — never a guess, never derived from
the bare string "H1"/"H3"/etc.

**This file must never diverge from the mapping document.** Add an entry
here only when that document defines one; if the document is ever revised,
revise this table to match in the same change.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class StableProperty:
    property_id: str
    short_name: str  # the first clause of the mapping document's own "Name" column -- not a new name invented here

    @property
    def display_label(self) -> str:
        return f"{self.short_name} ({self.property_id})"


# Keyed by (experiment, local_label) exactly as the mapping document's
# table rows read -- an experiment's own H-number only resolves within
# that experiment, never assumed to carry across to another one.
_TABLE: dict[tuple[str, str], StableProperty] = {
    ("EXP-003A", "H1"): StableProperty("PROP-ARG-BINDING", "Argument binding / drift"),
    ("EXP-003B", "H1"): StableProperty("PROP-ARG-BINDING", "Argument binding / drift"),
    ("EXP-003C", "H1"): StableProperty("PROP-ARG-BINDING", "Argument binding / drift"),
    ("EXP-003A", "H2"): StableProperty("PROP-DELEGATION-BOUNDARY", "Delegation / handoff boundary"),
    ("EXP-003B", "H2"): StableProperty("PROP-DELEGATION-BOUNDARY", "Delegation / handoff boundary"),
    ("EXP-003C", "H2"): StableProperty("PROP-DELEGATION-BOUNDARY", "Delegation / handoff boundary"),
    ("EXP-003A", "H3"): StableProperty("PROP-STANDING-APPROVAL", "Standing / always-approve scope"),
    ("EXP-003B", "H5"): StableProperty("PROP-STANDING-APPROVAL", "Standing / always-approve scope"),
    ("EXP-003C", "H3"): StableProperty("PROP-STANDING-APPROVAL", "Standing / always-approve scope"),
    ("EXP-003B", "H3"): StableProperty("PROP-CHECKPOINT-STALE-REPLAY", "Checkpoint restore / stale-response replay"),
    ("EXP-003A", "H4"): StableProperty("PROP-PERSISTENCE-EXACTLY-ONCE", "Persistence / resume exactly-once"),
    ("EXP-003C", "H4"): StableProperty("PROP-PERSISTENCE-EXACTLY-ONCE", "Persistence / resume exactly-once"),
    ("EXP-003B", "H4"): StableProperty("PROP-MULTI-PENDING-APPROVALS", "Multiple simultaneous pending approvals"),
    ("EXP-003A", "H5"): StableProperty("PROP-POLICY-TOCTOU", "Policy / guardrail TOCTOU"),
    ("EXP-003C", "H5"): StableProperty("PROP-POLICY-TOCTOU", "Policy / guardrail TOCTOU"),
    ("EXP-003B", "H6"): StableProperty("PROP-IDENTITY-ORIGIN-BINDING", "Identity / origin binding"),
    ("EXP-003C", "H6"): StableProperty("PROP-IDENTITY-ORIGIN-BINDING", "Identity / origin binding"),
}


def resolve_property(experiment: str, local_label: str) -> StableProperty | None:
    """Returns the stable property this (experiment, local H-number) pair
    maps to, or `None` if the mapping document defines no entry for it —
    callers must not guess a value in that case, the same discipline the
    mapping document itself follows (it marks un-mapped cells explicitly
    rather than assuming equivalence)."""
    return _TABLE.get((experiment, local_label))
