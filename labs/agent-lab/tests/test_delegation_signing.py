"""
v0.6 — canonical delegation serialization + Ed25519 signing (§10-11).

Two properties, proven independently:
  1. Canonicalization is deterministic: the same logical delegation always
     produces the same bytes/digest, regardless of construction order.
  2. Any protected field mutation changes the digest, and — the stronger,
     actually-security-relevant property — invalidates a signature that
     was computed before the mutation.
"""

from dataclasses import replace

import pytest

from agentlab import delegation, signing_keys
from agentlab.delegation import Delegation
from agentlab.lifecycle import DelegationStatus


def _delegation(**overrides) -> Delegation:
    base = dict(
        delegation_id="dlg_test",
        principal_user_id="user-alice",
        agent_id="agt_subject",
        task_id=None,
        allowed_tools=frozenset({"get_customer", "get_order"}),
        resource_constraints={"customer_ids": frozenset({"cust_1", "cust_2"})},
        issued_at=1000.0,
        expires_at=2000.0,
        status=DelegationStatus.ACTIVE,
        revoked_at=None,
        format_version=1,
        issuer_type="agent",
        issuer_id="agt_issuer",
        parent_delegation_id="dlg_parent",
        parent_hash="deadbeef",
        signing_key_id="ask_test",
        signature=None,
    )
    base.update(overrides)
    return Delegation(**base)


# ── §10: canonical serialization is deterministic ───────────────────────


def test_same_logical_delegation_same_bytes():
    """Two independently-constructed Delegation objects with identical
    logical content produce byte-identical canonical output — including
    when the frozenset/dict fields are built via different insertion
    orders (sets/dicts have no guaranteed iteration order in general)."""
    a = _delegation(
        allowed_tools=frozenset({"get_customer", "get_order", "add_case_note"}),
        resource_constraints={"customer_ids": frozenset({"cust_2", "cust_1"}), "order_ids": frozenset({"ord_1"})},
    )
    b = _delegation(
        allowed_tools=frozenset({"add_case_note", "get_order", "get_customer"}),
        resource_constraints={"order_ids": frozenset({"ord_1"}), "customer_ids": frozenset({"cust_1", "cust_2"})},
    )
    assert delegation.canonical_delegation_bytes(a) == delegation.canonical_delegation_bytes(b)
    assert delegation.delegation_digest(a) == delegation.delegation_digest(b)


def test_canonical_bytes_are_deterministic_across_repeated_calls():
    d = _delegation()
    first = delegation.canonical_delegation_bytes(d)
    second = delegation.canonical_delegation_bytes(d)
    assert first == second


def test_canonical_bytes_include_format_version():
    d = _delegation(format_version=1)
    bumped = replace(d, format_version=2)
    assert delegation.canonical_delegation_bytes(d) != delegation.canonical_delegation_bytes(bumped)


# ── §10: any protected field change changes the digest ──────────────────


@pytest.mark.parametrize(
    "field_name,new_value",
    [
        ("delegation_id", "dlg_other"),
        ("issuer_type", "human"),
        ("issuer_id", "agt_someone_else"),
        ("principal_user_id", "user-bob"),
        ("agent_id", "agt_different_subject"),
        ("task_id", "task_123"),
        ("allowed_tools", frozenset({"get_customer"})),
        ("resource_constraints", {"customer_ids": frozenset({"cust_9"})}),
        ("issued_at", 9999.0),
        ("expires_at", 9999.0),
        ("parent_delegation_id", "dlg_other_parent"),
        ("parent_hash", "cafebabe"),
        ("signing_key_id", "ask_other"),
    ],
)
def test_any_protected_field_change_changes_digest(field_name, new_value):
    original = _delegation()
    mutated = replace(original, **{field_name: new_value})
    assert delegation.delegation_digest(original) != delegation.delegation_digest(mutated), (
        f"mutating {field_name!r} did not change the digest — it is not integrity-protected"
    )


def test_status_and_revoked_at_are_deliberately_not_signed():
    """Revocation is a live, separately-checked fact (chain.py re-reads
    `status` fresh on every verification) — it is deliberately outside the
    signed envelope, so revoking a delegation must not itself invalidate
    a signature that was valid at issuance. See delegation.py's module
    comment on why."""
    active = _delegation(status=DelegationStatus.ACTIVE, revoked_at=None)
    revoked = replace(active, status=DelegationStatus.REVOKED, revoked_at=1500.0)
    assert delegation.canonical_delegation_bytes(active) == delegation.canonical_delegation_bytes(revoked)


# ── §11: signing + verification, mutation matrix ─────────────────────────


@pytest.fixture
def keypair():
    return signing_keys.generate_ed25519_keypair()


@pytest.fixture
def other_keypair():
    return signing_keys.generate_ed25519_keypair()


def test_valid_signature_verifies(keypair):
    private_pem, public_pem = keypair
    d = _delegation(signature=None)
    signature = delegation.sign_delegation(private_pem, d)
    signed = replace(d, signature=signature)

    assert delegation.verify_delegation_signature(public_pem, signed) is True


@pytest.mark.parametrize(
    "field_name,new_value",
    [
        ("issuer_id", "agt_attacker"),
        ("agent_id", "agt_different_subject"),
        ("allowed_tools", frozenset({"get_customer", "get_order", "add_case_note", "delete_customer"})),
        ("resource_constraints", {"customer_ids": frozenset({"cust_1", "cust_2", "cust_999"})}),
        ("expires_at", 999999.0),
        ("parent_delegation_id", "dlg_a_different_parent"),
        ("parent_hash", "0000000000000000000000000000000000000000000000000000000000000000"),
        ("resource_constraints", {"customer_ids": frozenset({"cust_1"})}),  # a narrowing edit also invalidates
    ],
)
def test_mutated_field_invalidates_signature(keypair, field_name, new_value):
    """The core integrity property: ANY change to a signed field —
    whether it looks like an attack (broadened tools) or looks harmless
    (a narrower resource set) — breaks the signature. A signature is
    over the delegation *as issued*; it is never re-validated as "close
    enough"."""
    private_pem, public_pem = keypair
    d = _delegation(signature=None)
    signature = delegation.sign_delegation(private_pem, d)
    signed = replace(d, signature=signature)

    tampered = replace(signed, **{field_name: new_value})
    assert delegation.verify_delegation_signature(public_pem, tampered) is False


def test_wrong_key_fails_verification(keypair, other_keypair):
    private_pem, _public_pem = keypair
    _other_private, other_public_pem = other_keypair
    d = _delegation(signature=None)
    signature = delegation.sign_delegation(private_pem, d)
    signed = replace(d, signature=signature)

    assert delegation.verify_delegation_signature(other_public_pem, signed) is False


def test_malformed_signature_fails_verification(keypair):
    _private_pem, public_pem = keypair
    d = _delegation(signature="not-valid-hex-!!")
    assert delegation.verify_delegation_signature(public_pem, d) is False


def test_missing_signature_fails_verification(keypair):
    _private_pem, public_pem = keypair
    d = _delegation(signature=None)
    assert delegation.verify_delegation_signature(public_pem, d) is False


def test_malformed_public_key_fails_verification(keypair):
    private_pem, _public_pem = keypair
    d = _delegation(signature=None)
    signature = delegation.sign_delegation(private_pem, d)
    signed = replace(d, signature=signature)

    assert delegation.verify_delegation_signature("not a real PEM", signed) is False
