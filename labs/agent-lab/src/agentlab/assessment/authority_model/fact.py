"""
Crypton Authority Model v0.1 — the `Fact` provenance wrapper.

Used for every identity-sensitive or binding-sensitive field across the
eight canonical primitives (principal/actor/approver identity, grant
delegation edges, approval-to-action binding, resource identity) — the
exact fields spec §23 forbids inferring. A bare `None` cannot distinguish
"we don't know" from "the system structurally has no such concept" from
"a human supplied this but Crypton never verified it" — `Fact` can.

Structural fields that are simply present-or-absent facts about the
*shape* of a record (timestamps, state enums, tool/resource name lists)
are not wrapped — the object's own top-level `provenance`/`evidence_ids`
cover those. Wrapping every single field would make every fixture
unreadable for no safety benefit; wrapping none would let an assessor
silently guess a principal's identity. `Fact` is reserved for exactly the
fields spec §23 names as never-to-be-inferred.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Generic, TypeVar

from .enums import Provenance, SemanticAvailability

T = TypeVar("T")


@dataclass(frozen=True)
class Fact(Generic[T]):
    """One fact, with its provenance and semantic availability attached.

    Construction is via the named constructors below, not the bare
    dataclass constructor — this is deliberate: `Fact.unknown()` forces a
    caller to say *why* a value is absent rather than quietly passing
    `value=None` and moving on, and `__post_init__` enforces that an
    unavailable fact never also carries a value (a `Fact` is either a real
    value with `AVAILABLE` availability, or an absence with a stated
    reason — never both at once).
    """

    value: T | None
    provenance: Provenance
    availability: SemanticAvailability
    note: str | None = None

    def __post_init__(self) -> None:
        if self.availability != SemanticAvailability.AVAILABLE and self.value is not None:
            raise ValueError(
                f"Fact with availability={self.availability.value} must not also carry a value "
                f"({self.value!r}) — if the value is genuinely known, availability must be AVAILABLE"
            )
        if self.availability == SemanticAvailability.AVAILABLE and self.value is None:
            raise ValueError("Fact with availability=AVAILABLE must carry a non-None value")
        if self.availability in (SemanticAvailability.UNREPRESENTABLE, SemanticAvailability.UNVERIFIABLE) and not (
            self.note or ""
        ).strip():
            raise ValueError(
                f"Fact with availability={self.availability.value} requires a non-empty `note` stating "
                "why — never a bare, unexplained absence"
            )

    @property
    def is_available(self) -> bool:
        return self.availability == SemanticAvailability.AVAILABLE

    # ── named constructors — the only intended way to build a Fact ──────

    @classmethod
    def observed(cls, value: T, note: str | None = None) -> "Fact[T]":
        return cls(value=value, provenance=Provenance.OBSERVED, availability=SemanticAvailability.AVAILABLE, note=note)

    @classmethod
    def configured(cls, value: T, note: str | None = None) -> "Fact[T]":
        return cls(value=value, provenance=Provenance.CONFIGURED, availability=SemanticAvailability.AVAILABLE, note=note)

    @classmethod
    def derived(cls, value: T, note: str | None = None) -> "Fact[T]":
        return cls(value=value, provenance=Provenance.DERIVED, availability=SemanticAvailability.AVAILABLE, note=note)

    @classmethod
    def customer_asserted(cls, value: T, note: str | None = None) -> "Fact[T]":
        return cls(
            value=value, provenance=Provenance.CUSTOMER_ASSERTED, availability=SemanticAvailability.AVAILABLE, note=note
        )

    @classmethod
    def unknown(cls, note: str | None = None) -> "Fact[T]":
        """Current evidence simply does not establish this fact — distinct
        from UNREPRESENTABLE (the system cannot express it) and
        UNVERIFIABLE (it's representable but this capture couldn't get it)."""
        return cls(value=None, provenance=Provenance.UNKNOWN, availability=SemanticAvailability.UNKNOWN, note=note)

    @classmethod
    def unrepresentable(cls, note: str) -> "Fact[T]":
        return cls(value=None, provenance=Provenance.UNKNOWN, availability=SemanticAvailability.UNREPRESENTABLE, note=note)

    @classmethod
    def unverifiable(cls, note: str) -> "Fact[T]":
        return cls(value=None, provenance=Provenance.UNKNOWN, availability=SemanticAvailability.UNVERIFIABLE, note=note)

    # ── serialization ─────────────────────────────────────────────────

    def to_dict(self) -> dict[str, Any]:
        value = list(self.value) if isinstance(self.value, tuple) else self.value
        return {
            "value": value,
            "provenance": self.provenance.value,
            "availability": self.availability.value,
            "note": self.note,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any] | None) -> "Fact[T] | None":
        if data is None:
            return None
        return cls(
            value=data.get("value"),
            provenance=Provenance(data["provenance"]),
            availability=SemanticAvailability(data["availability"]),
            note=data.get("note"),
        )

    def render(self) -> str:
        """Compact one-line rendering for the text Authority Map — never
        silently disappears when a value is absent (spec §16)."""
        if self.is_available:
            return str(self.value)
        return f"{self.availability.value}" + (f" ({self.note})" if self.note else "")
