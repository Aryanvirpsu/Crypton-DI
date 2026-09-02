"""
Agent signing identity (v0.6) — one Ed25519 keypair per agent that wants to
*issue* a delegation, not just receive one.

Answers the question DECISIONS.md's v0.6 section poses directly: "which
concrete cryptographic principal signed this delegation?" The answer is
`agent_id` — the durable, registered `Agent` (agents.py), never
`AgentInstance` (which is minted fresh every single `AgentRunner.run()`
call and would make every hop of a delegation chain need re-signing on
every run — see DECISIONS.md's "signing principal" decision for the full
reasoning). A `HumanIdentity` does not get a signing key here at all: the
root of a delegation chain is attributed by the same mechanism every other
human-initiated write in this lab already uses — an authenticated
`HumanIdentity` making the call — not a new, second identity/signing
system invented for humans. See delegation.py's root-vs-chained split.

Deliberately separate from `agents.register_agent`, the same way
`Delegation` is separate from `Agent` and `Approval` is separate from
`Delegation`: registering an agent and giving it signing authority are two
different decisions, made at two different times, by two different calls
— not every agent needs one (a leaf agent that only ever *receives*
delegated authority, like Agent C in the RAT-6 scenario, never issues a
child delegation and never needs a keypair).

Key lifecycle is deliberately minimal, mirroring `DelegationStatus`'s
one-way shape: create, resolve (by key_id, for verification), revoke.
Revocation is permanent and — per DECISIONS.md's explicit v0.6 policy —
retroactive: a delegation signed by a key that is later revoked becomes
untrusted from that point on, not grandfathered in as still-valid. There
is no rotation-with-overlap and no KMS/HSM integration — this is a
prototype proof of the signing primitive, not production key custody (see
DECISIONS.md's "what v0.6 does not build").

Private key handling: `create_signing_key` generates the keypair, persists
only the *public* half, and returns the private half to the caller exactly
once. Nothing in this module — or anywhere else in this package — stores,
logs, or traces a private key. Callers (the CLI, test fixtures) are
responsible for holding it only as long as they need to sign; there is no
production KMS/HSM story here, deliberately (see DECISIONS.md).
"""

from __future__ import annotations

import sqlite3
import time
import uuid
from dataclasses import dataclass, replace

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from cryptography.hazmat.primitives.serialization import Encoding, NoEncryption, PrivateFormat, PublicFormat

from . import agents
from .identity import HumanIdentity
from .lifecycle import SigningKeyStatus, parse_signing_key_status, validate_signing_key_transition
from .tracing import Tracer

ALGORITHM = "Ed25519"


class SigningKeyAccessError(Exception):
    """Raised when a signing key doesn't exist, or exists but its agent
    isn't owned by the calling human. Same "look identical" reasoning as
    AgentAccessError/DelegationAccessError elsewhere in this package."""


@dataclass(frozen=True)
class SigningKey:
    key_id: str
    agent_id: str
    public_key_pem: str
    algorithm: str
    created_at: float
    revoked_at: float | None
    status: SigningKeyStatus

    @property
    def active(self) -> bool:
        return self.status == SigningKeyStatus.ACTIVE


def _new_key_id() -> str:
    """The only place a signing-key id is minted — never caller-supplied,
    exactly like agent_id/delegation_id/approval_id elsewhere in this
    lab."""
    return f"ask_{uuid.uuid4().hex}"


def _signing_key_from_row(row: sqlite3.Row) -> SigningKey:
    return SigningKey(
        key_id=row["key_id"],
        agent_id=row["agent_id"],
        public_key_pem=row["public_key_pem"],
        algorithm=row["algorithm"],
        created_at=row["created_at"],
        revoked_at=row["revoked_at"],
        status=parse_signing_key_status(row["status"]),
    )


def generate_ed25519_keypair() -> tuple[str, str]:
    """Generate a fresh Ed25519 keypair (PEM-encoded), using the
    `cryptography` library's mature primitives — never hand-rolled crypto.
    Returns `(private_key_pem, public_key_pem)`."""
    private_key = Ed25519PrivateKey.generate()
    private_pem = private_key.private_bytes(
        encoding=Encoding.PEM,
        format=PrivateFormat.PKCS8,
        encryption_algorithm=NoEncryption(),
    ).decode("utf-8")
    public_pem = private_key.public_key().public_bytes(
        encoding=Encoding.PEM,
        format=PublicFormat.SubjectPublicKeyInfo,
    ).decode("utf-8")
    return private_pem, public_pem


def create_signing_key(
    conn: sqlite3.Connection, human: HumanIdentity, agent_id: str, *, tracer: Tracer | None = None
) -> tuple[SigningKey, str]:
    """Mint a fresh Ed25519 signing identity for `agent_id`, owned (via the
    agent) by `human`. Raises AgentAccessError (via `agents.get_agent`)
    unless `human` owns `agent_id` — a principal can only give signing
    authority to an agent they actually control, same discipline as
    granting a Delegation over one.

    Returns `(SigningKey, private_key_pem)` — the private half is returned
    exactly once and never persisted anywhere by this function. An agent
    may hold more than one signing key over its lifetime (e.g. after
    revoking a compromised one and creating a replacement); there is no
    uniqueness constraint forcing "at most one" — callers that want "the"
    current key should ask for the newest active one
    (`list_signing_keys`, filtered)."""
    agents.get_agent(conn, human, agent_id)  # ownership-enforced; raises AgentAccessError

    private_key_pem, public_key_pem = generate_ed25519_keypair()
    key_id = _new_key_id()
    now = time.time()
    conn.execute(
        "INSERT INTO agent_signing_keys "
        "(key_id, agent_id, public_key_pem, algorithm, created_at, revoked_at, status) "
        "VALUES (?, ?, ?, ?, ?, NULL, ?)",
        (key_id, agent_id, public_key_pem, ALGORITHM, now, SigningKeyStatus.ACTIVE.value),
    )
    conn.commit()

    if tracer is not None:
        tracer.record("signing_key_created", key_id=key_id, agent_id=agent_id, algorithm=ALGORITHM)

    key = SigningKey(
        key_id=key_id,
        agent_id=agent_id,
        public_key_pem=public_key_pem,
        algorithm=ALGORITHM,
        created_at=now,
        revoked_at=None,
        status=SigningKeyStatus.ACTIVE,
    )
    return key, private_key_pem


def get_signing_key(conn: sqlite3.Connection, human: HumanIdentity, key_id: str) -> SigningKey:
    """Ownership-enforced load (via the key's agent) — raises
    SigningKeyAccessError unless `key_id` exists AND its agent is owned by
    `human`."""
    row = conn.execute("SELECT * FROM agent_signing_keys WHERE key_id = ?", (key_id,)).fetchone()
    if row is None:
        raise SigningKeyAccessError(f"signing_key_not_found: {key_id}")
    key = _signing_key_from_row(row)
    try:
        agents.get_agent(conn, human, key.agent_id)
    except agents.AgentAccessError as e:
        raise SigningKeyAccessError(f"signing_key_not_found: {key_id}") from e
    return key


def get_signing_key_unchecked(conn: sqlite3.Connection, key_id: str) -> SigningKey | None:
    """Raw lookup with no ownership check — used only by the chain
    verifier (chain.py), which needs to resolve *any* delegation's signing
    key regardless of who is currently running the CLI, exactly the same
    precedent as delegation.get_delegation_unchecked. Never used to expose
    a key's existence/contents outside that pipeline."""
    row = conn.execute("SELECT * FROM agent_signing_keys WHERE key_id = ?", (key_id,)).fetchone()
    return None if row is None else _signing_key_from_row(row)


def list_signing_keys(conn: sqlite3.Connection, human: HumanIdentity, agent_id: str) -> list[SigningKey]:
    """Every signing key ever created for `agent_id`, active or revoked —
    ownership-enforced via the agent itself."""
    agents.get_agent(conn, human, agent_id)
    rows = conn.execute(
        "SELECT * FROM agent_signing_keys WHERE agent_id = ? ORDER BY created_at", (agent_id,)
    ).fetchall()
    return [_signing_key_from_row(row) for row in rows]


def revoke_signing_key(
    conn: sqlite3.Connection, human: HumanIdentity, key_id: str, *, tracer: Tracer | None = None
) -> SigningKey:
    """The only function allowed to change `agent_signing_keys.status`.
    One-way: `active -> revoked`; revoking an already-revoked key raises
    LifecycleError. Never deletes the row — a revoked key is retained
    forever as evidence, same principle as a revoked Delegation.

    Per DECISIONS.md's v0.6 key-lifecycle policy, revocation is
    retroactive: chain verification (chain.py) treats *every* delegation
    ever signed by this key as untrusted from the moment it's revoked, not
    only delegations signed after. This function does not need to (and
    does not) touch any delegation row to make that true — chain
    verification re-resolves the signing key's live status on every
    check, the same "never cache, always re-check the live row" discipline
    Delegation's own expiry/revocation checks already use."""
    key = get_signing_key(conn, human, key_id)
    validate_signing_key_transition(key.status, SigningKeyStatus.REVOKED)

    now = time.time()
    conn.execute(
        "UPDATE agent_signing_keys SET status = ?, revoked_at = ? WHERE key_id = ?",
        (SigningKeyStatus.REVOKED.value, now, key_id),
    )
    conn.commit()

    if tracer is not None:
        tracer.record("signing_key_revoked", key_id=key_id, agent_id=key.agent_id)

    return replace(key, status=SigningKeyStatus.REVOKED, revoked_at=now)
