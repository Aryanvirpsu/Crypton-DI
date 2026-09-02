"""
Delegation (v0.4) — explicit authority a human principal grants to a
canonical agent identity.

Replaces the v0.1/v0.3 static, name-keyed permission map (authz.py,
removed) with a persisted, owned, expiring grant of authority. The bug
that motivated this: authz.py's REGISTRY was keyed by `AgentIdentity.name`
(a descriptive label two different registrations can share — e.g. the
same human's "support-agent" v0.1.0 and v0.2.0), not by the canonical
`agent_id` that actually identifies *which* registration this is. Two
agents with the same name but different agent_ids must never inherit each
other's authority just because they share a label — see
authorization.py's `authorize_tool_call` and LEARNING.md's v0.4 addendum.

A Delegation is keyed by `agent_id`, always. It is never resolved by name,
and granting one requires the principal to already own the agent it names
(agents.get_agent enforces this) — the same "you can only delegate
authority over an agent you actually own" rule that governs every other
write in this lab.

Kept deliberately NOT a policy engine:
  - `allowed_tools` is a flat set of tool names, checked with an `in`.
  - `resource_constraints` is a flat dict of resource_type -> explicit id
    set (e.g. {"customer_ids": {"cust_1"}}). Absence of a resource_type
    key means "nothing of that type is authorized" (safe default deny),
    not "unconstrained" — see authorization.py.
No wildcard, no expression language, no resource hierarchy. If this ever
needs to grow past that, it should become a deliberate, versioned
decision (see DECISIONS.md), not an incremental one here.
"""

from __future__ import annotations

import hashlib
import json
import sqlite3
import time
import uuid
from dataclasses import dataclass, field, replace
from typing import Iterable

from cryptography.exceptions import InvalidSignature
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from cryptography.hazmat.primitives.serialization import load_pem_private_key, load_pem_public_key

from . import agents, signing_keys
from .identity import HumanIdentity
from .lifecycle import DelegationStatus, parse_delegation_status, validate_delegation_transition
from .tracing import Tracer

MAX_TTL_SECONDS = 60 * 60 * 24 * 30  # 30 days — a delegation must always be finite, and bounded

# v0.6: the canonical-serialization format this module signs/hashes.
# Bumping this is a breaking change to every existing signature — see
# `canonical_delegation_bytes`'s docstring.
DELEGATION_FORMAT_VERSION = 1

ISSUER_HUMAN = "human"
ISSUER_AGENT = "agent"


class DelegationAccessError(Exception):
    """Raised when a delegation doesn't exist, or exists but isn't owned
    (as principal) by the caller. Same "look identical" reasoning as
    AgentAccessError/TaskAccessError — never reveal that another
    principal's delegation exists."""


class DelegationValidationError(Exception):
    """Raised when a delegation grant's parameters are invalid: no tools,
    an unknown tool name, a non-positive or excessive TTL, or resource
    constraints in the wrong shape."""


class DelegationIssuerMismatchError(DelegationValidationError):
    """v0.6: raised at issuance time when the agent asked to issue a child
    delegation is not the exact agent the parent delegation names as its
    subject — see `grant_chained_delegation` and DECISIONS.md's invariant
    2/§17 issuer/subject continuity. Agent X cannot mint a child off of
    D1's authority just because it knows D1's delegation_id; only the
    agent D1 actually names as `agent_id` (its subject) may issue from
    it. A `DelegationValidationError` subclass so existing callers that
    only catch the parent type still see this."""


class DelegationAttenuationError(DelegationValidationError):
    """v0.6: raised at issuance time when a proposed child delegation
    would grant broader authority than its parent — an added tool, a
    resource id the parent never scoped, or a longer expiry. See
    `verify_attenuation` in chain.py, the same function chain
    verification uses defensively at authorization time; this is the
    early, issuance-time half of "reject obvious invalidity immediately,
    but never rely on it instead of runtime verification" (§38)."""


@dataclass(frozen=True)
class Delegation:
    delegation_id: str
    principal_user_id: str
    agent_id: str
    task_id: str | None
    allowed_tools: frozenset[str]
    resource_constraints: dict[str, frozenset[str]]
    issued_at: float
    expires_at: float
    status: DelegationStatus
    revoked_at: float | None
    # v0.6 — see module docstring's "Chained delegation" section below.
    # All default to the values that make an existing (pre-v0.6, or
    # freshly root-granted) Delegation a valid *root*: issued directly by
    # a human, no parent, unsigned.
    format_version: int = DELEGATION_FORMAT_VERSION
    issuer_type: str = ISSUER_HUMAN
    issuer_id: str = field(default="")  # human_user_id (root) or agent_id (chained); "" only for stale rows
    parent_delegation_id: str | None = None
    parent_hash: str | None = None
    signing_key_id: str | None = None
    signature: str | None = None

    @property
    def is_root(self) -> bool:
        """A root delegation is issued directly by a HumanIdentity, with
        no parent — the degenerate, single-hop case v0.5's model already
        was. Every pre-v0.6 delegation reads back as one (see
        db.py's additive-column migration)."""
        return self.parent_delegation_id is None


def _new_delegation_id() -> str:
    """The only place a delegation_id is minted — never caller-supplied,
    exactly like agent_id/session_id/task_id/instance_id elsewhere."""
    return f"dlg_{uuid.uuid4().hex}"


def _encode_resource_constraints(constraints: dict[str, Iterable[str]]) -> str:
    return json.dumps({k: sorted(set(v)) for k, v in constraints.items()})


def _decode_resource_constraints(raw: str) -> dict[str, frozenset[str]]:
    return {k: frozenset(v) for k, v in json.loads(raw).items()}


def _delegation_from_row(row: sqlite3.Row) -> Delegation:
    keys = row.keys()
    # `issuer_type`/`issuer_id` may be NULL on a row written before v0.6's
    # additive columns existed (or, in principle, absent entirely on a very
    # old row dict passed to a test double) — such a row is, by definition,
    # a pre-v0.6 root grant, so it reads back as issued directly by its own
    # `principal_user_id`, exactly the fact that was already true of it.
    issuer_type = row["issuer_type"] if "issuer_type" in keys and row["issuer_type"] is not None else ISSUER_HUMAN
    issuer_id = (
        row["issuer_id"]
        if "issuer_type" in keys and row["issuer_id"] is not None
        else row["principal_user_id"]
    )
    return Delegation(
        delegation_id=row["delegation_id"],
        principal_user_id=row["principal_user_id"],
        agent_id=row["agent_id"],
        task_id=row["task_id"],
        allowed_tools=frozenset(json.loads(row["allowed_tools"])),
        resource_constraints=_decode_resource_constraints(row["resource_constraints"]),
        issued_at=row["issued_at"],
        expires_at=row["expires_at"],
        status=parse_delegation_status(row["status"]),
        revoked_at=row["revoked_at"],
        format_version=row["format_version"] if "format_version" in keys and row["format_version"] else 1,
        issuer_type=issuer_type,
        issuer_id=issuer_id,
        parent_delegation_id=row["parent_delegation_id"] if "parent_delegation_id" in keys else None,
        parent_hash=row["parent_hash"] if "parent_hash" in keys else None,
        signing_key_id=row["signing_key_id"] if "signing_key_id" in keys else None,
        signature=row["signature"] if "signature" in keys else None,
    )


def grant_delegation(
    conn: sqlite3.Connection,
    principal: HumanIdentity,
    *,
    agent_id: str,
    allowed_tools: Iterable[str],
    resource_constraints: dict[str, Iterable[str]] | None = None,
    task_id: str | None = None,
    ttl_seconds: int,
    tracer: Tracer | None = None,
) -> Delegation:
    """Grant explicit, finite-lived authority over `agent_id` to itself,
    on `principal`'s behalf. Raises AgentAccessError (via agents.get_agent)
    unless `principal` already owns `agent_id` — a principal can only
    delegate authority over an agent they own, never an arbitrary one.

    `task_id`, when given, task-binds this delegation: it will only ever
    authorize calls made within that exact task (see authorization.py,
    step "delegation task matches when task-bound"). Left `None` for a
    delegation that authorizes any task run by this agent on this
    principal's behalf — task-bound is preferred where the caller has a
    task already in hand, but is not mandatory.
    """
    # Ownership check first: never let a caller grant authority over an
    # agent_id they don't own, and never leak whether it exists otherwise.
    agents.get_agent(conn, principal, agent_id)

    allowed_tools = frozenset(allowed_tools)
    if not allowed_tools:
        raise DelegationValidationError("allowed_tools must not be empty")

    resource_constraints = {k: frozenset(v) for k, v in (resource_constraints or {}).items()}

    if ttl_seconds <= 0:
        raise DelegationValidationError("ttl_seconds must be positive — a delegation must have a finite expiration")
    if ttl_seconds > MAX_TTL_SECONDS:
        raise DelegationValidationError(f"ttl_seconds must be at most {MAX_TTL_SECONDS}")

    delegation_id = _new_delegation_id()
    now = time.time()
    expires_at = now + ttl_seconds
    conn.execute(
        "INSERT INTO delegations "
        "(delegation_id, principal_user_id, agent_id, task_id, allowed_tools, resource_constraints, "
        "issued_at, expires_at, status, revoked_at, "
        "format_version, issuer_type, issuer_id, parent_delegation_id, parent_hash, signing_key_id, signature) "
        "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, ?, ?, ?, NULL, NULL, NULL, NULL)",
        (
            delegation_id,
            principal.user_id,
            agent_id,
            task_id,
            json.dumps(sorted(allowed_tools)),
            _encode_resource_constraints(resource_constraints),
            now,
            expires_at,
            DelegationStatus.ACTIVE.value,
            DELEGATION_FORMAT_VERSION,
            ISSUER_HUMAN,
            principal.user_id,
        ),
    )
    conn.commit()

    if tracer is not None:
        tracer.record(
            "delegation_granted",
            delegation_id=delegation_id,
            agent_id=agent_id,
            task_id=task_id,
            allowed_tools=sorted(allowed_tools),
        )

    return Delegation(
        delegation_id=delegation_id,
        principal_user_id=principal.user_id,
        agent_id=agent_id,
        task_id=task_id,
        allowed_tools=allowed_tools,
        resource_constraints=resource_constraints,
        issued_at=now,
        expires_at=expires_at,
        status=DelegationStatus.ACTIVE,
        revoked_at=None,
        format_version=DELEGATION_FORMAT_VERSION,
        issuer_type=ISSUER_HUMAN,
        issuer_id=principal.user_id,
        parent_delegation_id=None,
        parent_hash=None,
        signing_key_id=None,
        signature=None,
    )


def get_delegation(conn: sqlite3.Connection, principal: HumanIdentity, delegation_id: str) -> Delegation:
    """Ownership-enforced load — the only way application code (CLI,
    tests) may read a delegation back. Raises DelegationAccessError unless
    `delegation_id` exists AND its principal is `principal`."""
    row = conn.execute("SELECT * FROM delegations WHERE delegation_id = ?", (delegation_id,)).fetchone()
    if row is None or row["principal_user_id"] != principal.user_id:
        raise DelegationAccessError(f"delegation_not_found: {delegation_id}")
    return _delegation_from_row(row)


def get_delegation_unchecked(conn: sqlite3.Connection, delegation_id: str) -> Delegation | None:
    """Raw lookup with no ownership check — used only by the pre-execution
    authorization pipeline (authorization.py), which needs to distinguish
    "no such delegation" from "exists, but not this principal's" as two
    separate, individually-evidenced reason codes (DELEGATION_NOT_FOUND vs
    PRINCIPAL_MISMATCH). Never used to grant access to a delegation's
    contents outside that pipeline."""
    row = conn.execute("SELECT * FROM delegations WHERE delegation_id = ?", (delegation_id,)).fetchone()
    return None if row is None else _delegation_from_row(row)


def revoke_delegation(
    conn: sqlite3.Connection, principal: HumanIdentity, delegation_id: str, *, tracer: Tracer | None = None
) -> Delegation:
    """The only function allowed to change `delegations.status`. Revoking
    an already-revoked delegation raises LifecycleError (status
    transitions are one-way and re-revoking isn't a real transition) —
    callers that want idempotent revoke should catch that explicitly.
    Never deletes the row: a revoked delegation is retained forever as
    evidence/history (see module docstring)."""
    delegation = get_delegation(conn, principal, delegation_id)
    validate_delegation_transition(delegation.status, DelegationStatus.REVOKED)

    now = time.time()
    conn.execute(
        "UPDATE delegations SET status = ?, revoked_at = ? WHERE delegation_id = ?",
        (DelegationStatus.REVOKED.value, now, delegation_id),
    )
    conn.commit()

    if tracer is not None:
        tracer.record("delegation_revoked", delegation_id=delegation_id, agent_id=delegation.agent_id)

    return replace(delegation, status=DelegationStatus.REVOKED, revoked_at=now)


def bootstrap_delegation(
    conn: sqlite3.Connection,
    principal: HumanIdentity,
    agent_id: str,
    *,
    allowed_tools: Iterable[str],
    resource_constraints: dict[str, Iterable[str]] | None = None,
    ttl_seconds: int = 3600,
    tracer: Tracer | None = None,
) -> Delegation:
    """Local-dev convenience only — must be explicitly requested (see
    cli.py's `--bootstrap-delegation` flag; never invoked implicitly
    during a normal run). Unlike `agents.bootstrap_support_agent`, this is
    NOT idempotent: a delegation is a time-bounded grant, not a stable
    registration, so every call mints a fresh one. Exists purely so the
    lab's `--fake` CLI flow has something to authorize against without a
    separate manual `grant_delegation` step first."""
    return grant_delegation(
        conn,
        principal,
        agent_id=agent_id,
        allowed_tools=allowed_tools,
        resource_constraints=resource_constraints,
        ttl_seconds=ttl_seconds,
        tracer=tracer,
    )


# ── v0.6: canonical serialization + signing ─────────────────────────────
#
# A signature can only cover *bytes*, not a Python object — and Python's
# dict/set ordering, `repr()`, and pickling are all explicitly unsuitable
# (unstable across runs/versions, not cross-language, not a security
# boundary). This is the same problem `approval.canonicalize_arguments`
# already solved for tool-call arguments; the same discipline (sorted
# keys, no whitespace, deterministic) is applied here to a delegation's
# own security-relevant fields.
#
# Only the fields that define what the delegation *grants* are covered —
# not `status`/`revoked_at`. Revocation is a live, separately-checked fact
# (see chain.py), deliberately outside the signed envelope: a delegation's
# signature attests to the terms it was issued with, which must never
# change after the fact; whether it's *currently usable* is a different,
# time-varying question the signature was never meant to answer, the same
# way crypton-id's own Ed25519 JWTs are signed over identity claims, not
# over "is this session still valid right now" (that's the Redis
# denylist's job, checked live, every time — see jwt.rs's AuthUser).


def canonical_delegation_bytes(d: Delegation) -> bytes:
    """The exact bytes a delegation's issuer signs, and a verifier
    recomputes to check that signature. Deterministic and versioned
    (`format_version` is itself part of the signed payload, so a future
    format change can never be silently reinterpreted as v1): sorted JSON
    keys, no whitespace ambiguity (`separators=(",", ":")`), tool/resource
    sets sorted before encoding so set-iteration order can never affect
    the result. `default=str` only ever fires on a field type this
    function doesn't already handle explicitly — every field below is a
    plain str/int/float/None, so it should never actually trigger; kept as
    a defensive backstop, not a design choice to serialize opaquely."""
    payload = {
        "format_version": d.format_version,
        "delegation_id": d.delegation_id,
        "issuer_type": d.issuer_type,
        "issuer_id": d.issuer_id,
        "principal_user_id": d.principal_user_id,
        "subject_agent_id": d.agent_id,
        "task_id": d.task_id,
        "allowed_tools": sorted(d.allowed_tools),
        "resource_constraints": {k: sorted(v) for k, v in sorted(d.resource_constraints.items())},
        "issued_at": d.issued_at,
        "expires_at": d.expires_at,
        "parent_delegation_id": d.parent_delegation_id,
        "parent_hash": d.parent_hash,
        "signing_key_id": d.signing_key_id,
    }
    return json.dumps(payload, sort_keys=True, separators=(",", ":"), default=str).encode("utf-8")


def delegation_digest(d: Delegation) -> str:
    """sha256 of `canonical_delegation_bytes(d)`, hex-encoded. This is what
    a *child* delegation's `parent_hash` stores — a hash-chain link
    independent of the parent's signature (a root/human-issued parent has
    no signature at all, but still has a digest a child can bind to). Two
    delegations with identical security-relevant fields always produce the
    same digest, regardless of which Python process/run computed it
    (`test_delegation_signing.py::test_same_logical_delegation_same_bytes`
    proves this directly); changing any single covered field changes the
    digest (`test_any_protected_field_change_changes_digest`)."""
    return hashlib.sha256(canonical_delegation_bytes(d)).hexdigest()


def sign_delegation(private_key_pem: str, d: Delegation) -> str:
    """Issuer-side: sign `d`'s canonical bytes with an Ed25519 private key
    (PEM, PKCS8), returning a hex-encoded signature. Uses the
    `cryptography` library's Ed25519 implementation directly — no
    hand-rolled signing math, same discipline `signing_keys.py` and
    crypton-id's own JWT migration already follow."""
    private_key = load_pem_private_key(private_key_pem.encode("utf-8"), password=None)
    if not isinstance(private_key, Ed25519PrivateKey):
        raise DelegationValidationError("signing key is not an Ed25519 private key")
    signature = private_key.sign(canonical_delegation_bytes(d))
    return signature.hex()


def verify_delegation_signature(public_key_pem: str, d: Delegation) -> bool:
    """Verifier-side: recompute `d`'s canonical bytes and check them
    against `d.signature` using `public_key_pem`. Returns `False` for
    *any* failure — a malformed signature, a missing signature, wrong-key,
    tampered fields, or an unparseable public key — never raises past this
    function's boundary, so a caller (chain.py) can treat "signature
    invalid" and "signature verification itself errored" identically:
    both mean DENY, never ALLOW (see DECISIONS.md invariant 8, "unknown
    must never mean ALLOW")."""
    if not d.signature:
        return False
    try:
        public_key = load_pem_public_key(public_key_pem.encode("utf-8"))
        public_key.verify(bytes.fromhex(d.signature), canonical_delegation_bytes(d))
        return True
    except (InvalidSignature, ValueError, TypeError):
        return False


# ── v0.6: chained (agent-issued) delegation ─────────────────────────────


def grant_chained_delegation(
    conn: sqlite3.Connection,
    human: HumanIdentity,
    *,
    issuer_agent_id: str,
    issuer_private_key_pem: str,
    issuer_signing_key_id: str,
    parent_delegation_id: str,
    subject_agent_id: str,
    allowed_tools: Iterable[str],
    resource_constraints: dict[str, Iterable[str]] | None = None,
    task_id: str | None = None,
    ttl_seconds: int,
    tracer: Tracer | None = None,
) -> Delegation:
    """Issue a *non-root* delegation: `issuer_agent_id` (an agent, not the
    root human) delegates a narrowed subset of the authority it itself
    holds via `parent_delegation_id` to `subject_agent_id`.

    `human` must own `issuer_agent_id` (agents.get_agent enforces this) —
    exactly like every other authority-granting call in this lab, there is
    no code path where an agent autonomously calls this on its own behalf;
    a human operator drives every delegation in this Lab, chained or not
    (see DECISIONS.md's v0.6 "human root" section — Agent Lab does not
    build live agent-to-agent RPC in this milestone).

    Rejects, at issuance time, every violation chain verification would
    also catch defensively later (§38: reject early, but never *instead
    of* re-checking at verification time):
      - `DelegationAccessError` if `parent_delegation_id` doesn't exist.
      - `DelegationIssuerMismatchError` if the parent's *subject*
        (`parent.agent_id`) is not `issuer_agent_id` — Agent X cannot
        mint a child off of D1's authority just because it knows D1's ID;
        only the agent D1 actually names may issue from it.
      - `DelegationValidationError` if the parent is already revoked or
        expired — authority that no longer exists cannot be narrowed into
        something new.
      - `DelegationAttenuationError` if the proposed child would grant
        broader tools, resources, or a longer expiry than the parent (see
        chain.verify_attenuation — the exact same subset check, called
        here too so the two call sites can never silently disagree).

    The new delegation is signed with `issuer_private_key_pem` over its
    own canonical bytes (`sign_delegation`), and its `parent_hash` is set
    to `delegation_digest(parent)` — computed here, from the parent row
    just read, not trusted from any caller input — the cryptographic bond
    a parent-substitution attack must break (see chain.py)."""
    # Ownership: a principal can only issue authority through an agent
    # they actually control.
    agents.get_agent(conn, human, issuer_agent_id)

    parent = get_delegation_unchecked(conn, parent_delegation_id)
    if parent is None:
        raise DelegationAccessError(f"parent_delegation_not_found: {parent_delegation_id}")

    # Issuer/subject continuity, enforced at issuance (defense #1 of 2 —
    # chain.py re-checks this from persisted state at verification time
    # regardless of what happened here).
    if parent.agent_id != issuer_agent_id:
        raise DelegationIssuerMismatchError(
            f"issuer_mismatch: {issuer_agent_id} is not the subject of {parent_delegation_id} "
            f"(subject is {parent.agent_id})"
        )

    if parent.status == DelegationStatus.REVOKED:
        raise DelegationValidationError(f"parent_revoked: {parent_delegation_id}")
    if time.time() > parent.expires_at:
        raise DelegationValidationError(f"parent_expired: {parent_delegation_id}")

    allowed_tools = frozenset(allowed_tools)
    if not allowed_tools:
        raise DelegationValidationError("allowed_tools must not be empty")
    resource_constraints = {k: frozenset(v) for k, v in (resource_constraints or {}).items()}

    if ttl_seconds <= 0:
        raise DelegationValidationError("ttl_seconds must be positive — a delegation must have a finite expiration")
    if ttl_seconds > MAX_TTL_SECONDS:
        raise DelegationValidationError(f"ttl_seconds must be at most {MAX_TTL_SECONDS}")

    now = time.time()
    expires_at = now + ttl_seconds

    # Issuance-time attenuation check — see chain.verify_attenuation for
    # the authoritative implementation; imported locally to avoid a
    # module-level circular import (chain.py imports delegation.py for
    # Delegation/get_delegation_unchecked).
    from . import chain as chain_module

    candidate = Delegation(
        delegation_id="",  # not yet minted; attenuation doesn't depend on the id
        principal_user_id=parent.principal_user_id,
        agent_id=subject_agent_id,
        task_id=task_id,
        allowed_tools=allowed_tools,
        resource_constraints=resource_constraints,
        issued_at=now,
        expires_at=expires_at,
        status=DelegationStatus.ACTIVE,
        revoked_at=None,
    )
    ok, reason = chain_module.verify_attenuation(parent, candidate)
    if not ok:
        raise DelegationAttenuationError(f"attenuation_violation: {reason}")

    delegation_id = _new_delegation_id()
    parent_hash = delegation_digest(parent)

    unsigned = replace(
        candidate,
        delegation_id=delegation_id,
        format_version=DELEGATION_FORMAT_VERSION,
        issuer_type=ISSUER_AGENT,
        issuer_id=issuer_agent_id,
        parent_delegation_id=parent_delegation_id,
        parent_hash=parent_hash,
        signing_key_id=issuer_signing_key_id,
        signature=None,
    )
    signature = sign_delegation(issuer_private_key_pem, unsigned)
    final = replace(unsigned, signature=signature)

    conn.execute(
        "INSERT INTO delegations "
        "(delegation_id, principal_user_id, agent_id, task_id, allowed_tools, resource_constraints, "
        "issued_at, expires_at, status, revoked_at, "
        "format_version, issuer_type, issuer_id, parent_delegation_id, parent_hash, signing_key_id, signature) "
        "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, ?, ?, ?, ?, ?, ?, ?)",
        (
            delegation_id,
            final.principal_user_id,
            subject_agent_id,
            task_id,
            json.dumps(sorted(allowed_tools)),
            _encode_resource_constraints(resource_constraints),
            now,
            expires_at,
            DelegationStatus.ACTIVE.value,
            DELEGATION_FORMAT_VERSION,
            ISSUER_AGENT,
            issuer_agent_id,
            parent_delegation_id,
            parent_hash,
            issuer_signing_key_id,
            signature,
        ),
    )
    conn.commit()

    if tracer is not None:
        tracer.record(
            "chained_delegation_granted",
            delegation_id=delegation_id,
            issuer_agent_id=issuer_agent_id,
            subject_agent_id=subject_agent_id,
            parent_delegation_id=parent_delegation_id,
            allowed_tools=sorted(allowed_tools),
        )

    return final
