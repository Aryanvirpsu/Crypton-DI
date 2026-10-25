"""
Multi-hop delegation-chain verification (v0.6) — the RAT-6 primitive.

Answers one question, authoritatively, in one place (see module-level
`verify_delegation_chain`): given a leaf delegation, a requested tool, and
a requested resource, can Crypton *prove* that the acting agent's
authority traces back — unbroken, monotonically narrowing, and
cryptographically attributable at every hop — to a human?

This module does not duplicate `authorization.py`'s leaf-level checks; it
calls them (`authorization.authorize_tool_call`) as its first step, then
extends the result upward through however many parent delegations exist.
A root-only (v0.5-shaped) delegation has zero ancestors to walk, so this
degenerates to exactly `authorization.authorize_tool_call`'s existing
behavior — no new work, no new failure mode, for every pre-v0.6 caller.
See DECISIONS.md's v0.6 section for the full design rationale.

Nothing here trusts a caller-supplied chain. Every delegation in the walk
is loaded fresh from the database by ID, exactly the same
`delegation.get_delegation_unchecked` the leaf check already uses — an
attacker handing this function a pre-built, self-consistent-looking chain
object gains nothing, because only the persisted rows are ever consulted.
"""

from __future__ import annotations

import time
from dataclasses import dataclass, field

from . import authorization, delegation as delegation_module, signing_keys
from .delegation import Delegation
from .identity import AgentInstance, HumanIdentity

# A prototype-appropriate bound, not a business rule: the RAT-6 milestone
# only needs >=3 hops end to end. Chosen generously above that so a
# realistic chain never brushes against it, while still making an
# unbounded/malicious chain (or a cycle that somehow evaded detection)
# fail fast rather than walk forever. See DECISIONS.md.
MAX_CHAIN_DEPTH = 32

# ── Chain-specific denial reason codes ──────────────────────────────────
# Reuses authorization.py's ALLOW/DECISION_* vocabulary and evidence
# ledger (authorization_decisions) — these are additional reason codes in
# the same style (stable, machine-readable, UPPER_SNAKE), not a second
# decision system. A leaf-level denial (TOOL_NOT_ALLOWED, DELEGATION_EXPIRED,
# etc.) still comes from authorization.py unchanged; these only ever fire
# for a failure found while walking *ancestors*.

CHAIN_PARENT_NOT_FOUND = "CHAIN_PARENT_NOT_FOUND"
CHAIN_CYCLE_DETECTED = "CHAIN_CYCLE_DETECTED"
CHAIN_DEPTH_EXCEEDED = "CHAIN_DEPTH_EXCEEDED"
CHAIN_FORMAT_UNSUPPORTED = "CHAIN_FORMAT_UNSUPPORTED"
CHAIN_ROOT_PRINCIPAL_MISMATCH = "CHAIN_ROOT_PRINCIPAL_MISMATCH"
CHAIN_SIGNING_KEY_MISSING = "CHAIN_SIGNING_KEY_MISSING"
CHAIN_SIGNING_KEY_NOT_FOUND = "CHAIN_SIGNING_KEY_NOT_FOUND"
CHAIN_SIGNING_KEY_REVOKED = "CHAIN_SIGNING_KEY_REVOKED"
CHAIN_SIGNATURE_INVALID = "CHAIN_SIGNATURE_INVALID"
CHAIN_ISSUER_CONTINUITY_VIOLATION = "CHAIN_ISSUER_CONTINUITY_VIOLATION"
CHAIN_PARENT_HASH_MISMATCH = "CHAIN_PARENT_HASH_MISMATCH"
CHAIN_ATTENUATION_VIOLATION = "CHAIN_ATTENUATION_VIOLATION"
CHAIN_ANCESTOR_REVOKED = "CHAIN_ANCESTOR_REVOKED"
CHAIN_ANCESTOR_EXPIRED = "CHAIN_ANCESTOR_EXPIRED"
CHAIN_VERIFICATION_ERROR = "CHAIN_VERIFICATION_ERROR"


class ChainVerificationError(Exception):
    """Raised only for a programming/data error the caller must not treat
    as an ordinary denial (e.g. a malformed resource_constraints value) —
    `verify_delegation_chain` itself never raises this outward; it is
    caught internally and converted to a DENY (see the top-level
    try/except in `verify_delegation_chain`), consistent with invariant 8:
    unknown must never mean ALLOW."""


# ── Attenuation engine (§13-16) ──────────────────────────────────────────


def verify_attenuation(parent: Delegation, child: Delegation) -> tuple[bool, str | None]:
    """The one authoritative subset check: is `child`'s authority a valid
    narrowing of `parent`'s? Returns `(True, None)` if every dimension
    narrows or stays equal; `(False, <reason>)` on the first violation
    found. Used both at issuance time (`delegation.grant_chained_delegation`
    — reject obvious invalidity immediately) and at verification time
    (`verify_delegation_chain` — never trust that issuance-time validation
    actually ran). Exact equality between parent and child is valid at
    every dimension — attenuation means "never broader," not "must
    shrink."

    Tools/actions: `child.allowed_tools` must be a subset of
    `parent.allowed_tools` (frozenset `<=`) — a child may drop a tool, it
    may never add one the parent never granted.

    Resources: for every resource_type the child scopes at all, the
    parent must scope that *same* type too, and the child's id set must
    be a subset of the parent's for it. A resource_type the child names
    that the parent never constrained at all is ambiguous — is that
    "parent didn't care" (unconstrained, so anything is fine) or "parent
    never delegated authority over this resource type at all"? This
    codebase's existing resource-constraint semantics (authorization.py's
    step 13: "absence of a resource_type key means nothing of that type
    is authorized — safe default deny, never unconstrained") already
    answers that for the parent's *own* execution-time check; the same
    reading applies here: an unscoped-by-parent type can never be
    legitimately narrowed into by a child, so it denies. No prefix/
    wildcard matching is used or assumed — the existing model is exact id
    sets, and subset-of-exact-sets is the only safe, unambiguous
    narrowing semantic for it (a naive string-prefix scheme would let
    "customer/1" appear to narrow "customer/123", which is backwards —
    deliberately not implemented).

    Expiry: `child.expires_at <= parent.expires_at`. A child may expire
    sooner; it may never outlive the authority it derives from.
    """
    if not child.allowed_tools <= parent.allowed_tools:
        return False, "child allowed_tools is not a subset of parent allowed_tools"

    for resource_type, child_ids in child.resource_constraints.items():
        parent_ids = parent.resource_constraints.get(resource_type)
        if parent_ids is None:
            return False, f"child scopes resource_type {resource_type!r} the parent never constrained"
        if not child_ids <= parent_ids:
            return False, f"child resource ids for {resource_type!r} are not a subset of the parent's"

    if child.expires_at > parent.expires_at:
        return False, "child expires_at is later than parent expires_at"

    return True, None


# ── Chain verification result ────────────────────────────────────────────


@dataclass(frozen=True)
class ChainVerificationResult:
    allowed: bool
    decision: authorization.AuthorizationDecision
    root_principal: str | None  # human_user_id at the root, or None if verification failed before resolving one
    acting_principal: str | None  # agent_id of the leaf (the agent actually executing)
    delegation_depth: int  # number of delegation hops walked, including the leaf (root-only = 1)
    authority_path: list[tuple[str, str]] = field(default_factory=list)  # [("human", uid), ("agent", agt_id), ...]
    effective_tools: frozenset[str] = frozenset()
    effective_resources: dict[str, frozenset[str]] = field(default_factory=dict)
    effective_expiry: float | None = None
    chain_hash: str | None = None  # delegation_digest of the leaf — a fingerprint of the verified state
    failed_hop: str | None = None  # human-readable "who->who" label of the hop that failed, if any
    denial_reason: str | None = None

    @property
    def reason_code(self) -> str:
        return self.decision.reason_code


def _authority_path(chain_leaf_to_root: list[Delegation]) -> list[tuple[str, str]]:
    """Build the root-to-leaf principal path for display/tracing:
    `[("human", H), ("agent", A), ("agent", B), ("agent", C)]`.
    `chain_leaf_to_root` is every delegation walked, leaf first, root
    last (the root delegation itself included) — this reverses it and
    prefixes the human accountable for the whole chain."""
    root = chain_leaf_to_root[-1]
    path: list[tuple[str, str]] = [("human", root.principal_user_id)]
    for d in reversed(chain_leaf_to_root):
        path.append(("agent", d.agent_id))
    return path


def verify_delegation_chain(
    conn,
    *,
    principal: HumanIdentity,
    agent_id: str,
    instance: AgentInstance,
    task_id: str,
    delegation_id: str,
    tool_name: str,
    resource_type: str | None,
    resource_id: str | None,
) -> ChainVerificationResult:
    """The single authoritative chain verifier (§18). Walks the
    authoritative persisted chain from `delegation_id` (the leaf) up to a
    root, in this fixed order:

      1. leaf-level check (`authorization.authorize_tool_call`) — agent
         identity/instance/principal match, delegation exists, not
         revoked/expired, tool + resource permitted by the leaf's own
         (already-attenuated) scope.
      2. for the leaf, and then for each ancestor in turn, walking toward
         the root:
         a. depth/cycle guard
         b. format_version supported
         c. root-principal continuity (every hop must share the same
            accountable human)
         d. if non-root: resolve + verify its own signature against its
            registered signing key (key exists, key not revoked,
            signature valid over its own canonical bytes)
         e. resolve its parent; parent must exist
         f. issuer/subject continuity: this delegation's issuer must be
            exactly its parent's subject
         g. parent_hash must match the parent's *current* canonical digest
            (parent-substitution protection)
         h. attenuation: this delegation's authority must be a subset of
            its parent's
      3. once a root is reached: the root itself must not be revoked or
         expired.

    Returns a `ChainVerificationResult` — never raises for an ordinary
    denial (every DENY path returns a populated result with `allowed=False`
    and a `decision` already persisted via `authorization.record_decision`,
    exactly like `authorize_tool_call`'s own contract). An unexpected
    internal error (a malformed row, a crypto library exception outside
    the narrow cases `verify_delegation_signature` already swallows) is
    caught here and converted to a DENY with `CHAIN_VERIFICATION_ERROR` —
    per invariant 8, a verifier that doesn't know the answer must never
    default to ALLOW."""
    try:
        return _verify_delegation_chain_inner(
            conn,
            principal=principal,
            agent_id=agent_id,
            instance=instance,
            task_id=task_id,
            delegation_id=delegation_id,
            tool_name=tool_name,
            resource_type=resource_type,
            resource_id=resource_id,
        )
    except Exception as e:  # noqa: BLE001 - deliberate: any internal failure is a DENY, never a silent ALLOW
        resource = f"{resource_type}:{resource_id}" if resource_type else None
        decision = authorization.record_decision(
            conn,
            decision=authorization.DECISION_DENY,
            reason_code=CHAIN_VERIFICATION_ERROR,
            principal_user_id=principal.user_id,
            agent_id=agent_id,
            instance_id=instance.instance_id,
            task_id=task_id,
            delegation_id=delegation_id,
            tool_name=tool_name,
            resource=resource,
        )
        return ChainVerificationResult(
            allowed=False,
            decision=decision,
            root_principal=None,
            acting_principal=agent_id,
            delegation_depth=0,
            denial_reason=f"{CHAIN_VERIFICATION_ERROR}: {e}",
        )


def _verify_delegation_chain_inner(
    conn,
    *,
    principal: HumanIdentity,
    agent_id: str,
    instance: AgentInstance,
    task_id: str,
    delegation_id: str,
    tool_name: str,
    resource_type: str | None,
    resource_id: str | None,
) -> ChainVerificationResult:
    resource = f"{resource_type}:{resource_id}" if resource_type else None

    def deny(reason_code: str, *, failed_hop: str | None, depth: int) -> ChainVerificationResult:
        decision = authorization.record_decision(
            conn,
            decision=authorization.DECISION_DENY,
            reason_code=reason_code,
            principal_user_id=principal.user_id,
            agent_id=agent_id,
            instance_id=instance.instance_id,
            task_id=task_id,
            delegation_id=delegation_id,
            tool_name=tool_name,
            resource=resource,
        )
        return ChainVerificationResult(
            allowed=False,
            decision=decision,
            root_principal=None,
            acting_principal=agent_id,
            delegation_depth=depth,
            failed_hop=failed_hop,
            denial_reason=reason_code,
        )

    # Step 1: the leaf-level check — unchanged, reused exactly as v0.5.1
    # already calls it. This is what makes a root-only delegation's
    # behavior identical to before v0.6: for that case, this function
    # returns right after this block, having done nothing new.
    leaf_decision = authorization.authorize_tool_call(
        conn,
        principal=principal,
        agent_id=agent_id,
        instance=instance,
        task_id=task_id,
        delegation_id=delegation_id,
        tool_name=tool_name,
        resource_type=resource_type,
        resource_id=resource_id,
    )
    if not leaf_decision.allowed:
        return ChainVerificationResult(
            allowed=False,
            decision=leaf_decision,
            root_principal=None,
            acting_principal=agent_id,
            delegation_depth=1,
            failed_hop="leaf",
            denial_reason=leaf_decision.reason_code,
        )

    leaf = delegation_module.get_delegation_unchecked(conn, delegation_id)
    if leaf is None:
        # authorize_tool_call just proved this row exists; this branch is
        # unreachable in practice, kept only as a defensive DENY rather
        # than an unchecked None-deref.
        return deny(CHAIN_PARENT_NOT_FOUND, failed_hop="leaf", depth=1)

    now = time.time()
    visited: set[str] = {leaf.delegation_id}
    walked: list[Delegation] = []  # ancestors visited so far, leaf-first (does not include leaf itself)
    current = leaf
    depth = 1

    while not current.is_root:
        if depth > MAX_CHAIN_DEPTH:
            return deny(CHAIN_DEPTH_EXCEEDED, failed_hop=current.agent_id, depth=depth)

        if current.format_version != delegation_module.DELEGATION_FORMAT_VERSION:
            return deny(CHAIN_FORMAT_UNSUPPORTED, failed_hop=current.agent_id, depth=depth)

        if current.principal_user_id != leaf.principal_user_id:
            return deny(CHAIN_ROOT_PRINCIPAL_MISMATCH, failed_hop=current.agent_id, depth=depth)

        # Signature: every non-root delegation (the leaf included, on this
        # first iteration) must carry a valid signature from its own
        # registered, currently-active signing key. This is what catches
        # a tampered child: mutating any signed field of `current` after
        # the fact changes its canonical bytes, so the stored signature
        # (computed over the original bytes) no longer verifies.
        if not current.signing_key_id or not current.signature:
            return deny(CHAIN_SIGNING_KEY_MISSING, failed_hop=current.agent_id, depth=depth)
        key = signing_keys.get_signing_key_unchecked(conn, current.signing_key_id)
        if key is None:
            return deny(CHAIN_SIGNING_KEY_NOT_FOUND, failed_hop=current.agent_id, depth=depth)
        if not key.active:
            return deny(CHAIN_SIGNING_KEY_REVOKED, failed_hop=current.agent_id, depth=depth)
        if not delegation_module.verify_delegation_signature(key.public_key_pem, current):
            return deny(CHAIN_SIGNATURE_INVALID, failed_hop=current.agent_id, depth=depth)

        if current.status.value == "revoked":
            return deny(CHAIN_ANCESTOR_REVOKED, failed_hop=current.agent_id, depth=depth)
        if now > current.expires_at:
            return deny(CHAIN_ANCESTOR_EXPIRED, failed_hop=current.agent_id, depth=depth)

        parent_id = current.parent_delegation_id
        parent = delegation_module.get_delegation_unchecked(conn, parent_id) if parent_id else None
        if parent is None:
            return deny(CHAIN_PARENT_NOT_FOUND, failed_hop=current.agent_id, depth=depth)
        if parent.delegation_id in visited:
            return deny(CHAIN_CYCLE_DETECTED, failed_hop=current.agent_id, depth=depth)
        visited.add(parent.delegation_id)

        # Issuer/subject continuity (§17): the delegation that issued
        # `current` must be exactly the agent `parent` names as its own
        # subject — never just "some agent that knew parent's ID."
        if current.issuer_type != delegation_module.ISSUER_AGENT or current.issuer_id != parent.agent_id:
            return deny(CHAIN_ISSUER_CONTINUITY_VIOLATION, failed_hop=f"{parent.agent_id}->{current.agent_id}", depth=depth)

        # Parent-substitution protection: the hash captured at `current`'s
        # issuance must match the parent's canonical bytes *right now* —
        # if `parent_delegation_id` were swapped to point at a different,
        # otherwise-valid delegation, this comparison fails even though
        # `parent` itself loads successfully.
        if current.parent_hash != delegation_module.delegation_digest(parent):
            return deny(CHAIN_PARENT_HASH_MISMATCH, failed_hop=f"{parent.agent_id}->{current.agent_id}", depth=depth)

        ok, _reason = verify_attenuation(parent, current)
        if not ok:
            return deny(CHAIN_ATTENUATION_VIOLATION, failed_hop=f"{parent.agent_id}->{current.agent_id}", depth=depth)

        walked.append(current)
        current = parent
        depth += 1

    # `current` is now the root. A root has no signature to check (see
    # DECISIONS.md — humans don't hold a delegation-signing key in this
    # milestone); it still must be live.
    if current.status.value == "revoked":
        return deny(CHAIN_ANCESTOR_REVOKED, failed_hop="root", depth=depth)
    if now > current.expires_at:
        return deny(CHAIN_ANCESTOR_EXPIRED, failed_hop="root", depth=depth)

    path = _authority_path(walked + [current])
    return ChainVerificationResult(
        allowed=True,
        decision=leaf_decision,
        root_principal=leaf.principal_user_id,
        acting_principal=leaf.agent_id,
        delegation_depth=depth,
        authority_path=path,
        effective_tools=leaf.allowed_tools,
        effective_resources=leaf.resource_constraints,
        effective_expiry=leaf.expires_at,
        chain_hash=delegation_module.delegation_digest(leaf),
    )
