"""
Crypton Authority Model v0.1 — controlled vocabularies.

Everything here is deliberately a small, closed enum, per the governing
rule of this module: this is a *derived* layer over existing evidence, not
a second place to invent new taxonomy. Add a new member only when existing,
already-captured evidence genuinely requires it — not speculatively ahead
of a real fixture needing it.
"""

from __future__ import annotations

from enum import Enum


class Provenance(str, Enum):
    """How a fact in the Authority Model was established. First-class on
    every important fact — see `primitives.Fact` and
    `docs/ASSESSMENT_METHODOLOGY.md`'s Authority Model section.

        OBSERVED          Direct runtime, trace, audit, state, sink, or
                           execution evidence.
        CONFIGURED        Established from code, policy, configuration,
                           schema, or documented contract but not observed
                           in the tested runtime path.
        DERIVED           Deterministically computed from other supported
                           facts. No guessing.
        CUSTOMER_ASSERTED Supplied by the customer but not independently
                           verified by Crypton.
        UNKNOWN           Current evidence does not establish the fact.
                           Never a synonym for NOT_APPLICABLE — NOT_APPLICABLE
                           is an *outcome* concept (does the control apply
                           at all), UNKNOWN is a *fact-availability*
                           concept (do we know the value).
    """

    OBSERVED = "OBSERVED"
    CONFIGURED = "CONFIGURED"
    DERIVED = "DERIVED"
    CUSTOMER_ASSERTED = "CUSTOMER_ASSERTED"
    UNKNOWN = "UNKNOWN"


class SemanticAvailability(str, Enum):
    """Whether a fact's *value* is meaningfully knowable at all — separate
    from `Provenance` (which says *how* a known fact was established).

        AVAILABLE        The value is populated and meaningful.
        UNREPRESENTABLE  The target system has no semantic state to
                          express this fact at all (the Representability
                          Gate, G1, fails) — not merely "we didn't look."
        UNVERIFIABLE     The fact is representable, but this specific
                          capture could not establish its value.
        UNKNOWN          Neither representability nor verifiability has
                          been assessed yet for this fact.

    Do not default to UNREPRESENTABLE for an ordinary missing value — use
    it only when there is a specific, stated reason the underlying concept
    cannot be expressed by the system at all (see each `Fact`'s `note`).
    """

    AVAILABLE = "AVAILABLE"
    UNREPRESENTABLE = "UNREPRESENTABLE"
    UNVERIFIABLE = "UNVERIFIABLE"
    UNKNOWN = "UNKNOWN"


class ActorType(str, Enum):
    HUMAN = "HUMAN"
    AGENT = "AGENT"
    SUB_AGENT = "SUB_AGENT"
    SERVICE = "SERVICE"
    WORKER = "WORKER"
    TOOL_EXECUTOR = "TOOL_EXECUTOR"
    OTHER = "OTHER"


class EventType(str, Enum):
    """The minimal controlled event vocabulary (spec §4). Add a member
    only when existing evidence genuinely requires a new one — this is not
    a place for speculative event classes."""

    GRANT_CREATED = "GRANT_CREATED"
    GRANT_DELEGATED = "GRANT_DELEGATED"
    GRANT_REVOKED = "GRANT_REVOKED"
    GRANT_EXPIRED = "GRANT_EXPIRED"

    APPROVAL_REQUESTED = "APPROVAL_REQUESTED"
    APPROVAL_GRANTED = "APPROVAL_GRANTED"
    APPROVAL_DENIED = "APPROVAL_DENIED"
    APPROVAL_CONSUMED = "APPROVAL_CONSUMED"

    ACTION_PENDING = "ACTION_PENDING"
    ACTION_ATTEMPTED = "ACTION_ATTEMPTED"
    ACTION_EXECUTED = "ACTION_EXECUTED"
    ACTION_DENIED = "ACTION_DENIED"

    EFFECT_OBSERVED = "EFFECT_OBSERVED"

    STATE_SUSPENDED = "STATE_SUSPENDED"
    STATE_RESUMED = "STATE_RESUMED"

    CONTEXT_BOUND = "CONTEXT_BOUND"
    CONTEXT_CHANGED = "CONTEXT_CHANGED"


class DivergenceType(str, Enum):
    """Supported divergence classifications (spec §10), plus the explicit
    `NONE` result for a genuinely clean no-divergence case (spec §14's
    worked example renders the literal word "NONE" — this is that value,
    not an absent/null field)."""

    NONE = "NONE"
    SCOPE_EXPANSION = "SCOPE_EXPANSION"
    PRINCIPAL_OR_ACTOR_MISMATCH = "PRINCIPAL_OR_ACTOR_MISMATCH"
    DELEGATION_EXPANSION = "DELEGATION_EXPANSION"
    APPROVAL_SCOPE_MISMATCH = "APPROVAL_SCOPE_MISMATCH"
    ACTION_OR_ARGUMENT_SUBSTITUTION = "ACTION_OR_ARGUMENT_SUBSTITUTION"
    RESOURCE_SUBSTITUTION = "RESOURCE_SUBSTITUTION"
    CONTEXT_MISMATCH = "CONTEXT_MISMATCH"
    STALE_OR_REVOKED_AUTHORITY = "STALE_OR_REVOKED_AUTHORITY"
    REPLAY_OR_DOUBLE_CONSUMPTION = "REPLAY_OR_DOUBLE_CONSUMPTION"
    EFFECT_OUTSIDE_ENVELOPE = "EFFECT_OUTSIDE_ENVELOPE"
