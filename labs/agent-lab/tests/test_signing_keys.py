"""
v0.6 — agent signing-key lifecycle (signing_keys.py).

Covers: create/resolve/revoke, ownership enforcement, and the one-way
revocation transition. Chain-verification-time consequences of a revoked
key (a delegation it signed becoming untrusted) are covered in
test_delegation_chain.py, where the full chain context exists.
"""

import pytest

from agentlab import agents, signing_keys
from agentlab.identity import HumanIdentity
from agentlab.lifecycle import LifecycleError, SigningKeyStatus


def test_create_signing_key_returns_key_and_private_pem(db_conn, alice, support_agent):
    key, private_pem = signing_keys.create_signing_key(db_conn, alice, support_agent.agent_id)

    assert key.key_id.startswith("ask_")
    assert key.agent_id == support_agent.agent_id
    assert key.algorithm == "Ed25519"
    assert key.status == SigningKeyStatus.ACTIVE
    assert key.active
    assert "BEGIN PUBLIC KEY" in key.public_key_pem
    assert "BEGIN PRIVATE KEY" in private_pem
    # The private key is never derivable from the public one persisted —
    # trivial sanity check that they're not literally the same text.
    assert private_pem != key.public_key_pem


def test_create_signing_key_requires_ownership(db_conn, alice, bob, support_agent):
    """alice owns support_agent; bob does not — bob must not be able to
    mint a signing identity for an agent he doesn't control."""
    with pytest.raises(agents.AgentAccessError):
        signing_keys.create_signing_key(db_conn, bob, support_agent.agent_id)


def test_get_signing_key_unchecked_resolves_by_id(db_conn, alice, support_agent):
    key, _ = signing_keys.create_signing_key(db_conn, alice, support_agent.agent_id)
    resolved = signing_keys.get_signing_key_unchecked(db_conn, key.key_id)
    assert resolved is not None
    assert resolved.key_id == key.key_id
    assert resolved.public_key_pem == key.public_key_pem


def test_get_signing_key_unchecked_returns_none_for_unknown_id(db_conn):
    assert signing_keys.get_signing_key_unchecked(db_conn, "ask_does_not_exist") is None


def test_get_signing_key_enforces_ownership(db_conn, alice, bob, support_agent):
    key, _ = signing_keys.create_signing_key(db_conn, alice, support_agent.agent_id)
    with pytest.raises(signing_keys.SigningKeyAccessError):
        signing_keys.get_signing_key(db_conn, bob, key.key_id)
    # alice herself can read it back fine.
    assert signing_keys.get_signing_key(db_conn, alice, key.key_id).key_id == key.key_id


def test_revoke_signing_key_is_one_way(db_conn, alice, support_agent):
    key, _ = signing_keys.create_signing_key(db_conn, alice, support_agent.agent_id)
    revoked = signing_keys.revoke_signing_key(db_conn, alice, key.key_id)

    assert revoked.status == SigningKeyStatus.REVOKED
    assert not revoked.active
    assert revoked.revoked_at is not None

    with pytest.raises(LifecycleError):
        signing_keys.revoke_signing_key(db_conn, alice, key.key_id)

    # The row is retained, not deleted — still resolvable, just inactive.
    still_there = signing_keys.get_signing_key_unchecked(db_conn, key.key_id)
    assert still_there is not None
    assert still_there.status == SigningKeyStatus.REVOKED


def test_revoke_signing_key_requires_ownership(db_conn, alice, bob, support_agent):
    key, _ = signing_keys.create_signing_key(db_conn, alice, support_agent.agent_id)
    with pytest.raises(signing_keys.SigningKeyAccessError):
        signing_keys.revoke_signing_key(db_conn, bob, key.key_id)


def test_agent_may_hold_multiple_keys_over_its_lifetime(db_conn, alice, support_agent):
    """No uniqueness constraint forces "at most one" — an agent may
    accumulate several keys (e.g. after revoking a compromised one and
    minting a replacement)."""
    key1, _ = signing_keys.create_signing_key(db_conn, alice, support_agent.agent_id)
    signing_keys.revoke_signing_key(db_conn, alice, key1.key_id)
    key2, _ = signing_keys.create_signing_key(db_conn, alice, support_agent.agent_id)

    assert key1.key_id != key2.key_id
    keys = signing_keys.list_signing_keys(db_conn, alice, support_agent.agent_id)
    assert {k.key_id for k in keys} == {key1.key_id, key2.key_id}
