"""
Identity boundary tests — asymmetric (Ed25519/EdDSA) since v0.5.1.

The central security property under test (see DECISIONS.md's "JWT:
asymmetric signing replaces the shared HS256 secret", TD-1): a verifier
holding only a public key must be structurally unable to mint a token
that verifies against a *different* public key — in particular, unable
to forge anything crypton-id's real `JWT_PUBLIC_KEY_PEM` would accept.
"""

import time

import jwt as pyjwt
import pytest

from agentlab.identity import IdentityError, generate_dev_keypair, issue_dev_token, verify_human_identity

PRIVATE_KEY_PEM, PUBLIC_KEY_PEM = generate_dev_keypair()
OTHER_PRIVATE_KEY_PEM, OTHER_PUBLIC_KEY_PEM = generate_dev_keypair()


def test_verify_roundtrip():
    token = issue_dev_token("user-alice", "alice", "cred-1", private_key=PRIVATE_KEY_PEM)
    identity = verify_human_identity(token, public_key=PUBLIC_KEY_PEM)
    assert identity.user_id == "user-alice"
    assert identity.username == "alice"
    assert identity.cred_id == "cred-1"


def test_wrong_public_key_rejected():
    """A token signed by one keypair must not verify against a
    different, unrelated keypair's public key."""
    token = issue_dev_token("user-alice", "alice", "cred-1", private_key=PRIVATE_KEY_PEM)
    with pytest.raises(IdentityError):
        verify_human_identity(token, public_key=OTHER_PUBLIC_KEY_PEM)


def test_token_signed_by_a_different_private_key_is_rejected():
    """The forgery scenario TD-1 exists to close: someone who does NOT
    hold crypton-id's real private key (here, standing in with a
    different, unrelated keypair) cannot mint anything that verifies
    against crypton-id's real public key."""
    forged_token = issue_dev_token("user-mallory", "mallory", "cred-x", private_key=OTHER_PRIVATE_KEY_PEM)
    with pytest.raises(IdentityError):
        verify_human_identity(forged_token, public_key=PUBLIC_KEY_PEM)


def test_public_key_cannot_be_used_to_mint_a_token():
    """The structural version of the property above: a public key PEM
    cannot even be *loaded* as a signing key — a verifier holding only
    `JWT_PUBLIC_KEY_PEM` has no way to call `issue_dev_token` (or
    anything else) and produce a token at all, let alone a valid one."""
    with pytest.raises(Exception):
        pyjwt.encode({"sub": "x", "username": "x", "cred_id": "x"}, PUBLIC_KEY_PEM, algorithm="EdDSA")


def test_tampered_token_rejected():
    token = issue_dev_token("user-alice", "alice", "cred-1", private_key=PRIVATE_KEY_PEM)
    header, payload, sig = token.split(".")
    tampered_payload = payload[:-1] + ("A" if payload[-1] != "A" else "B")
    tampered = f"{header}.{tampered_payload}.{sig}"
    with pytest.raises(IdentityError):
        verify_human_identity(tampered, public_key=PUBLIC_KEY_PEM)


def test_expired_token_rejected():
    token = issue_dev_token("user-alice", "alice", "cred-1", private_key=PRIVATE_KEY_PEM, ttl_seconds=-10)
    with pytest.raises(IdentityError):
        verify_human_identity(token, public_key=PUBLIC_KEY_PEM)


def test_garbage_token_rejected():
    with pytest.raises(IdentityError):
        verify_human_identity("not-a-jwt-at-all", public_key=PUBLIC_KEY_PEM)


def test_missing_public_key_rejected(monkeypatch):
    monkeypatch.delenv("JWT_PUBLIC_KEY_PEM", raising=False)
    token = issue_dev_token("user-alice", "alice", "cred-1", private_key=PRIVATE_KEY_PEM)
    with pytest.raises(IdentityError):
        verify_human_identity(token)  # no public key configured anywhere


def test_missing_required_claims_rejected():
    """A structurally valid, correctly-signed token that simply omits a
    required claim must still be rejected — signature validity alone is
    not identity validity."""
    now = int(time.time())
    token = pyjwt.encode({"sub": "user-alice", "iat": now, "exp": now + 3600}, PRIVATE_KEY_PEM, algorithm="EdDSA")
    with pytest.raises(IdentityError):
        verify_human_identity(token, public_key=PUBLIC_KEY_PEM)


def test_generate_dev_keypair_produces_unique_keys_each_call():
    """Every call mints a fresh keypair — dev tokens are never
    interchangeable with a previous run's, let alone crypton-id's real
    key."""
    priv_a, pub_a = generate_dev_keypair()
    priv_b, pub_b = generate_dev_keypair()
    assert priv_a != priv_b
    assert pub_a != pub_b
