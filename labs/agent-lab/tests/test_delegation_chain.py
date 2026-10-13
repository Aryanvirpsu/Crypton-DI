"""
v0.6 — RAT-6: provable multi-hop delegation with scope attenuation.

The central question this file exists to answer, end to end:

    Can Crypton prove that an action performed by Agent C was authorized
    through a Human -> Agent A -> Agent B -> Agent C chain, where every
    hop is cryptographically attributable and authority can only narrow
    downstream?

`three_hop_chain` builds the persisted RAT-6 scenario once (D0/D1/D2, real
Ed25519-signed chained delegations); every test below either proves the
allowed case, or breaks exactly one property of the chain and proves the
verifier denies it — with zero tool execution, checked via a real spy on
`tools.TOOL_FUNCTIONS`, not by trusting a returned string alone.

Tool/resource naming follows existing Agent Lab conventions (get_customer /
get_order / add_case_note, customer_ids) rather than inventing
"customer.read"-style tool infrastructure the codebase doesn't have.
"""

from __future__ import annotations

import time
from dataclasses import dataclass

import pytest

from agentlab import agents, approval, authorization, chain, delegation, signing_keys, tools as tools_module
from agentlab.agent import AgentRunner
from agentlab.identity import AgentInstance
from agentlab.model_provider import FakeModelProvider, ModelResponse, ToolCallRequest
from agentlab.webauthn import FakeWebAuthnVerifier

VERIFIER = FakeWebAuthnVerifier()


@dataclass
class ThreeHopChain:
    agent_a: agents.Agent
    agent_b: agents.Agent
    agent_c: agents.Agent
    key_a: signing_keys.SigningKey
    priv_a: str
    key_b: signing_keys.SigningKey
    priv_b: str
    d0: delegation.Delegation
    d1: delegation.Delegation
    d2: delegation.Delegation


@pytest.fixture
def three_hop_chain(db_conn, alice) -> ThreeHopChain:
    """Human alice -> Agent A -> Agent B -> Agent C, narrowing at every
    hop, matching the v0.6 spec's acceptance scenario:

        D0 (alice  -> A): {get_customer, get_order, add_case_note}, {customer_ids: cust_1, cust_2}, T+3h
        D1 (A      -> B): {get_customer, get_order},                {customer_ids: cust_1, cust_2}, T+2h
        D2 (B      -> C): {get_customer},                           {customer_ids: cust_1},          T+1h

    `cust_1` is alice's real seeded customer (db.seed) — the ALLOW case
    can actually execute `get_customer` against it; `cust_2` belongs to
    bob in the seed data and is used only to prove a *broader* resource
    scope is representable at D0/D1 without ever being executed against.
    """
    agent_a = agents.register_agent(db_conn, alice, name="agent-a", version="0.1.0")
    agent_b = agents.register_agent(db_conn, alice, name="agent-b", version="0.1.0")
    agent_c = agents.register_agent(db_conn, alice, name="agent-c", version="0.1.0")

    key_a, priv_a = signing_keys.create_signing_key(db_conn, alice, agent_a.agent_id)
    key_b, priv_b = signing_keys.create_signing_key(db_conn, alice, agent_b.agent_id)
    # Agent C never issues a child delegation in this scenario — it does
    # not need a signing key at all (see signing_keys.py's module
    # docstring: not every agent needs one).

    d0 = delegation.grant_delegation(
        db_conn,
        alice,
        agent_id=agent_a.agent_id,
        allowed_tools=frozenset({"get_customer", "get_order", "add_case_note"}),
        resource_constraints={"customer_ids": {"cust_1", "cust_2"}},
        ttl_seconds=3 * 3600,
    )
    d1 = delegation.grant_chained_delegation(
        db_conn,
        alice,
        issuer_agent_id=agent_a.agent_id,
        issuer_private_key_pem=priv_a,
        issuer_signing_key_id=key_a.key_id,
        parent_delegation_id=d0.delegation_id,
        subject_agent_id=agent_b.agent_id,
        allowed_tools=frozenset({"get_customer", "get_order"}),
        resource_constraints={"customer_ids": {"cust_1", "cust_2"}},
        ttl_seconds=2 * 3600,
    )
    d2 = delegation.grant_chained_delegation(
        db_conn,
        alice,
        issuer_agent_id=agent_b.agent_id,
        issuer_private_key_pem=priv_b,
        issuer_signing_key_id=key_b.key_id,
        parent_delegation_id=d1.delegation_id,
        subject_agent_id=agent_c.agent_id,
        allowed_tools=frozenset({"get_customer"}),
        resource_constraints={"customer_ids": {"cust_1"}},
        ttl_seconds=1 * 3600,
    )
    return ThreeHopChain(
        agent_a=agent_a, agent_b=agent_b, agent_c=agent_c, key_a=key_a, priv_a=priv_a, key_b=key_b, priv_b=priv_b,
        d0=d0, d1=d1, d2=d2,
    )


def _instance_for(chain_fixture: ThreeHopChain, alice) -> AgentInstance:
    inst = AgentInstance.new()
    return AgentInstance(
        instance_id=inst.instance_id,
        started_at=inst.started_at,
        agent_id=chain_fixture.agent_c.agent_id,
        human_user_id=alice.user_id,
        session_id="",
        task_id="",
    )


def _verify(db_conn, chain_fixture: ThreeHopChain, alice, *, tool: str, resource_type: str | None, resource_id: str | None):
    return chain.verify_delegation_chain(
        db_conn,
        principal=alice,
        agent_id=chain_fixture.agent_c.agent_id,
        instance=_instance_for(chain_fixture, alice),
        task_id="",
        delegation_id=chain_fixture.d2.delegation_id,
        tool_name=tool,
        resource_type=resource_type,
        resource_id=resource_id,
    )


# ── §28: RAT-6 allowed case ──────────────────────────────────────────────


def test_rat6_allowed_action_verifies(db_conn, three_hop_chain, alice):
    result = _verify(db_conn, three_hop_chain, alice, tool="get_customer", resource_type="customer_ids", resource_id="cust_1")

    assert result.allowed, result.denial_reason
    assert result.root_principal == alice.user_id
    assert result.acting_principal == three_hop_chain.agent_c.agent_id
    assert result.delegation_depth == 3
    assert result.authority_path == [
        ("human", alice.user_id),
        ("agent", three_hop_chain.agent_a.agent_id),
        ("agent", three_hop_chain.agent_b.agent_id),
        ("agent", three_hop_chain.agent_c.agent_id),
    ]
    assert result.chain_hash is not None


async def test_rat6_allowed_action_executes_exactly_once_through_the_real_loop(db_conn, trace_dir, three_hop_chain, alice, monkeypatch):
    """The same scenario, driven through the *real* AgentRunner.run() loop
    (not just the verifier called directly), with a spy on
    tools.TOOL_FUNCTIONS proving the tool actually ran — not merely that
    the decision object claims ALLOW."""
    calls = []
    original = tools_module.TOOL_FUNCTIONS["get_customer"]

    def spy(*args, **kwargs):
        calls.append(1)
        return original(*args, **kwargs)

    monkeypatch.setitem(tools_module.TOOL_FUNCTIONS, "get_customer", spy)

    runner = AgentRunner(
        agent_id=three_hop_chain.agent_c.agent_id,
        delegation_id=three_hop_chain.d2.delegation_id,
        model=FakeModelProvider(
            script=[
                ModelResponse(tool_call=ToolCallRequest(id="c1", name="get_customer", arguments={"customer_id": "cust_1"})),
                ModelResponse(text="Alice Anders is a gold-tier customer."),
            ]
        ),
        db_conn=db_conn,
        trace_dir=trace_dir,
    )
    result = await runner.run(human=alice, goal="look up cust_1")

    assert result.stopped_reason == "final_response"
    assert len(calls) == 1


# ── §29: permission amplification ────────────────────────────────────────


def test_rat6_permission_amplification_denied(db_conn, three_hop_chain, alice):
    """add_case_note was dropped going D0 -> D1 and never re-granted —
    Agent C cannot reclaim it just by asking."""
    result = _verify(db_conn, three_hop_chain, alice, tool="add_case_note", resource_type="customer_ids", resource_id="cust_1")

    assert not result.allowed
    assert result.denial_reason == authorization.TOOL_NOT_ALLOWED


async def test_rat6_permission_amplification_denied_through_real_loop(db_conn, trace_dir, three_hop_chain, alice):
    def _note_count():
        return db_conn.execute("SELECT COUNT(*) FROM case_notes WHERE customer_id = 'cust_1'").fetchone()[0]

    runner = AgentRunner(
        agent_id=three_hop_chain.agent_c.agent_id,
        delegation_id=three_hop_chain.d2.delegation_id,
        model=FakeModelProvider(
            script=[
                ModelResponse(
                    tool_call=ToolCallRequest(id="c1", name="add_case_note", arguments={"customer_id": "cust_1", "note": "hi"})
                ),
                ModelResponse(text="done"),
            ]
        ),
        db_conn=db_conn,
        trace_dir=trace_dir,
    )
    await runner.run(human=alice, goal="add a note")
    assert _note_count() == 0


# ── §30: undelegated action attack ───────────────────────────────────────


def test_rat6_undelegated_action_denied(db_conn, three_hop_chain, alice):
    """delete_customer never existed anywhere in this chain's lineage —
    not even D0 granted it."""
    result = _verify(db_conn, three_hop_chain, alice, tool="delete_customer", resource_type="customer_ids", resource_id="cust_1")

    assert not result.allowed
    assert result.denial_reason == authorization.TOOL_NOT_ALLOWED


# ── §31: resource escape ─────────────────────────────────────────────────


def test_rat6_resource_escape_denied(db_conn, three_hop_chain, alice):
    """cust_2 is in D0/D1's broader resource scope but D2 narrowed to
    cust_1 only — the leaf's own narrowed scope is what's actually
    checked against the request, and it excludes cust_2."""
    result = _verify(db_conn, three_hop_chain, alice, tool="get_customer", resource_type="customer_ids", resource_id="cust_2")

    assert not result.allowed
    assert result.denial_reason == authorization.RESOURCE_NOT_ALLOWED


# ── §32: ancestor revocation ──────────────────────────────────────────────


def test_rat6_ancestor_revocation_denied(db_conn, three_hop_chain, alice):
    delegation.revoke_delegation(db_conn, alice, three_hop_chain.d1.delegation_id)

    result = _verify(db_conn, three_hop_chain, alice, tool="get_customer", resource_type="customer_ids", resource_id="cust_1")

    assert not result.allowed
    assert result.denial_reason == chain.CHAIN_ANCESTOR_REVOKED
    assert result.failed_hop == three_hop_chain.agent_b.agent_id


def test_rat6_leaf_revocation_denied(db_conn, three_hop_chain, alice):
    """The leaf itself (D2) being revoked is caught by the existing
    leaf-level check (authorization.py) — unchanged from v0.5.1."""
    delegation.revoke_delegation(db_conn, alice, three_hop_chain.d2.delegation_id)

    result = _verify(db_conn, three_hop_chain, alice, tool="get_customer", resource_type="customer_ids", resource_id="cust_1")

    assert not result.allowed
    assert result.denial_reason == authorization.DELEGATION_REVOKED
    assert result.failed_hop == "leaf"


# ── §33: ancestor expiry ──────────────────────────────────────────────────
#
# Attenuation requires child.expires_at <= parent.expires_at at every hop
# (test_attenuation.py), so in any honestly-issued, non-tampered chain the
# *leaf* always expires first, never an ancestor while a descendant
# remains valid — "ancestor expired, descendant still locally valid" is
# therefore a state that can only arise from tampering a signed
# expires_at field, which is already caught by CHAIN_SIGNATURE_INVALID /
# CHAIN_PARENT_HASH_MISMATCH (see test_ancestor_content_tampered_after_issuance_denied).
# These two tests isolate the *live* ancestor-liveness check itself as
# defense-in-depth — same justification, same bypass technique, as
# test_cycle_is_denied.


def _bypass_signature_and_hash_checks(monkeypatch, db_conn, *delegation_ids: str) -> None:
    """Shared setup for the defense-in-depth tests above: make every
    signature check trivially pass and every parent_hash check trivially
    match, so a directly-SQL-mutated row (which would otherwise be caught
    by those checks first, one layer earlier) can reach the specific,
    deeper check under test instead."""
    monkeypatch.setattr(delegation, "verify_delegation_signature", lambda *a, **k: True)
    monkeypatch.setattr(delegation, "delegation_digest", lambda d: "bypassed")
    for delegation_id in delegation_ids:
        db_conn.execute("UPDATE delegations SET parent_hash = 'bypassed' WHERE delegation_id = ?", (delegation_id,))
    db_conn.commit()


def test_rat6_root_expiry_denied_even_though_leaf_unexpired(db_conn, three_hop_chain, alice, monkeypatch):
    """Reaching CHAIN_ANCESTOR_EXPIRED specifically (rather than
    CHAIN_ATTENUATION_VIOLATION or CHAIN_SIGNATURE_INVALID/CHAIN_PARENT_HASH_MISMATCH
    one layer earlier) requires bypassing all three — a genuine, notable
    property of this design: expiry is triple-defended (signed, hash-
    chained, *and* attenuation-ordered), so the live clock check below it
    is the last of four independent lines of defense against the same
    fact, not the only one."""
    _bypass_signature_and_hash_checks(monkeypatch, db_conn, three_hop_chain.d1.delegation_id, three_hop_chain.d2.delegation_id)
    monkeypatch.setattr(chain, "verify_attenuation", lambda parent, child: (True, None))
    db_conn.execute(
        "UPDATE delegations SET expires_at = ? WHERE delegation_id = ?",
        (time.time() - 1, three_hop_chain.d0.delegation_id),
    )
    db_conn.commit()

    result = _verify(db_conn, three_hop_chain, alice, tool="get_customer", resource_type="customer_ids", resource_id="cust_1")

    assert not result.allowed
    assert result.denial_reason == chain.CHAIN_ANCESTOR_EXPIRED
    assert result.failed_hop == "root"


def test_rat6_intermediate_expiry_denied(db_conn, three_hop_chain, alice, monkeypatch):
    _bypass_signature_and_hash_checks(monkeypatch, db_conn, three_hop_chain.d1.delegation_id, three_hop_chain.d2.delegation_id)
    monkeypatch.setattr(chain, "verify_attenuation", lambda parent, child: (True, None))
    db_conn.execute(
        "UPDATE delegations SET expires_at = ? WHERE delegation_id = ?",
        (time.time() - 1, three_hop_chain.d1.delegation_id),
    )
    db_conn.commit()

    result = _verify(db_conn, three_hop_chain, alice, tool="get_customer", resource_type="customer_ids", resource_id="cust_1")

    assert not result.allowed
    assert result.denial_reason == chain.CHAIN_ANCESTOR_EXPIRED
    assert result.failed_hop == three_hop_chain.agent_b.agent_id


# ── §34/§35 boundary: parent *content* tampered after issuance ──────────
#
# Distinct from "tampered child" (mutating the delegation's *own* row —
# always caught by its own signature, since every signed field lives in
# its own envelope) and from "child re-pointed to a different parent"
# (mutating a child's `parent_delegation_id` — also always caught by the
# child's own signature, since `parent_delegation_id` is itself inside
# the signed payload; see test_rat6_parent_substitution_denied below).
# This proves the *third*, distinct property `parent_hash` exists for:
# the referenced *parent's own row* being altered after a child already
# captured a hash of its original content — something no child's own
# signature could ever cover, since a signature only ever attests to the
# delegation that carries it, never to what its parent contains.


def test_ancestor_content_tampered_after_issuance_denied(db_conn, three_hop_chain, alice):
    """D1's own content is rewritten directly (broadened resource scope)
    after D2 was already signed against D1's original state — D2's
    parent_hash, captured at issuance, no longer matches D1's current
    canonical bytes."""
    db_conn.execute(
        "UPDATE delegations SET resource_constraints = ? WHERE delegation_id = ?",
        ('{"customer_ids": ["cust_1", "cust_2", "cust_999"]}', three_hop_chain.d1.delegation_id),
    )
    db_conn.commit()

    result = _verify(db_conn, three_hop_chain, alice, tool="get_customer", resource_type="customer_ids", resource_id="cust_1")

    assert not result.allowed
    assert result.denial_reason == chain.CHAIN_PARENT_HASH_MISMATCH


# ── §34: tampered child ───────────────────────────────────────────────────


def test_rat6_tampered_child_denied(db_conn, three_hop_chain, alice):
    """Mutate D2's resource_constraints directly via SQL after it was
    signed — the stored signature no longer matches D2's (now different)
    canonical bytes."""
    db_conn.execute(
        "UPDATE delegations SET resource_constraints = ? WHERE delegation_id = ?",
        ('{"customer_ids": ["cust_1", "cust_2"]}', three_hop_chain.d2.delegation_id),
    )
    db_conn.commit()

    result = _verify(db_conn, three_hop_chain, alice, tool="get_customer", resource_type="customer_ids", resource_id="cust_2")

    assert not result.allowed
    assert result.denial_reason == chain.CHAIN_SIGNATURE_INVALID


# ── §35: parent substitution ──────────────────────────────────────────────


def test_rat6_parent_substitution_denied(db_conn, three_hop_chain, alice):
    """Point D2 at a *different*, otherwise-valid delegation (D0 itself)
    instead of D1. `parent_delegation_id` is itself one of the fields D2's
    own signature covers (see test_delegation_signing.py's mutation
    matrix), so re-pointing it — without also re-signing, which requires
    Agent B's private key — is caught the same way any other tampered
    field on D2 would be: D2's stored signature no longer matches D2's
    (now different) canonical bytes. This is invariant 4 holding by
    construction: parent linkage is integrity-protected precisely because
    it lives inside the signed envelope, not bolted on beside it."""
    db_conn.execute(
        "UPDATE delegations SET parent_delegation_id = ? WHERE delegation_id = ?",
        (three_hop_chain.d0.delegation_id, three_hop_chain.d2.delegation_id),
    )
    db_conn.commit()

    result = _verify(db_conn, three_hop_chain, alice, tool="get_customer", resource_type="customer_ids", resource_id="cust_1")

    assert not result.allowed
    assert result.denial_reason == chain.CHAIN_SIGNATURE_INVALID


# ── §36: unauthorized issuer ──────────────────────────────────────────────


def test_rat6_unauthorized_issuer_rejected_at_issuance(db_conn, three_hop_chain, alice):
    """Agent A (not Agent B) attempts to issue a child off of D1's
    authority, even though D1's subject is Agent B — rejected immediately,
    before any row is even written."""
    with pytest.raises(delegation.DelegationIssuerMismatchError):
        delegation.grant_chained_delegation(
            db_conn,
            alice,
            issuer_agent_id=three_hop_chain.agent_a.agent_id,  # wrong — D1's subject is agent_b
            issuer_private_key_pem=three_hop_chain.priv_a,
            issuer_signing_key_id=three_hop_chain.key_a.key_id,
            parent_delegation_id=three_hop_chain.d1.delegation_id,
            subject_agent_id=three_hop_chain.agent_c.agent_id,
            allowed_tools=frozenset({"get_customer"}),
            resource_constraints={"customer_ids": {"cust_1"}},
            ttl_seconds=3600,
        )


def test_rat6_unauthorized_issuer_denied_defensively_at_verification(db_conn, three_hop_chain, alice):
    """Even if a row bypassing issuance-time validation existed (simulated
    here via a direct SQL UPDATE that rewrites D2's issuer_id to Agent A,
    without re-signing — which alone would already fail on signature; to
    isolate the continuity check specifically, this also re-signs D2 with
    Agent A's key so the *only* broken property is issuer/subject
    continuity), chain verification independently re-derives and rejects
    it — never trusting that issuance-time checks ran."""
    d2 = three_hop_chain.d2
    forged_issuer_id = three_hop_chain.agent_a.agent_id
    forged_signing_key_id = three_hop_chain.key_a.key_id
    from dataclasses import replace

    forged_unsigned = replace(d2, issuer_id=forged_issuer_id, signing_key_id=forged_signing_key_id, signature=None)
    forged_signature = delegation.sign_delegation(three_hop_chain.priv_a, forged_unsigned)

    db_conn.execute(
        "UPDATE delegations SET issuer_id = ?, signing_key_id = ?, signature = ? WHERE delegation_id = ?",
        (forged_issuer_id, forged_signing_key_id, forged_signature, d2.delegation_id),
    )
    db_conn.commit()

    result = _verify(db_conn, three_hop_chain, alice, tool="get_customer", resource_type="customer_ids", resource_id="cust_1")

    assert not result.allowed
    assert result.denial_reason == chain.CHAIN_ISSUER_CONTINUITY_VIOLATION


# ── revoked/missing signing key ───────────────────────────────────────────


def test_revoked_signing_key_denies_every_delegation_it_signed(db_conn, three_hop_chain, alice):
    """Per DECISIONS.md's v0.6 key-lifecycle policy: revocation is
    retroactive. Revoking Agent B's key untrusts D2 even though D2 was
    validly signed before the revocation."""
    signing_keys.revoke_signing_key(db_conn, alice, three_hop_chain.key_b.key_id)

    result = _verify(db_conn, three_hop_chain, alice, tool="get_customer", resource_type="customer_ids", resource_id="cust_1")

    assert not result.allowed
    assert result.denial_reason == chain.CHAIN_SIGNING_KEY_REVOKED


# ── cycle + depth protection ───────────────────────────────────────────────


def test_cycle_is_denied(db_conn, three_hop_chain, alice, monkeypatch):
    """Force a cycle (D1's parent becomes D2) directly via SQL. Because
    `parent_delegation_id` lives inside the signed envelope (see
    test_rat6_parent_substitution_denied), a cycle constructed this way is
    *already* unreachable without breaking a signature or a parent_hash
    first — which is a real, useful property, not a gap: in this design,
    forging a cycle is cryptographically no easier than forging any other
    tampered chain. To exercise the cycle guard itself — the defense-in-
    depth measure that stops an unbounded/looping walk even if every
    crypto check were somehow bypassed (a compromised verifier, a future
    bug in the signature path) — this test explicitly neutralizes the
    signature and parent-hash checks and proves the *traversal* still
    terminates via CHAIN_CYCLE_DETECTED, not an infinite loop or a crash.
    """
    _bypass_signature_and_hash_checks(monkeypatch, db_conn, three_hop_chain.d1.delegation_id, three_hop_chain.d2.delegation_id)
    db_conn.execute(
        "UPDATE delegations SET parent_delegation_id = ? WHERE delegation_id = ?",
        (three_hop_chain.d2.delegation_id, three_hop_chain.d1.delegation_id),
    )
    db_conn.commit()

    result = _verify(db_conn, three_hop_chain, alice, tool="get_customer", resource_type="customer_ids", resource_id="cust_1")

    assert not result.allowed
    assert result.denial_reason == chain.CHAIN_CYCLE_DETECTED


def test_depth_limit_exceeded_is_denied(db_conn, three_hop_chain, alice, monkeypatch):
    """Exercise the depth guard itself without constructing 32+ real hops:
    lower MAX_CHAIN_DEPTH to below this fixture's real depth (3) and prove
    the walk still terminates cleanly with a DENY, never an infinite loop
    or a crash."""
    monkeypatch.setattr(chain, "MAX_CHAIN_DEPTH", 1)

    result = _verify(db_conn, three_hop_chain, alice, tool="get_customer", resource_type="customer_ids", resource_id="cust_1")

    assert not result.allowed
    assert result.denial_reason == chain.CHAIN_DEPTH_EXCEEDED


# ── §24: approval cannot rescue an invalid chain ─────────────────────────


async def test_valid_approval_cannot_rescue_an_invalid_chain(db_conn, trace_dir, three_hop_chain, alice):
    """add_case_note requires approval — but it also isn't in D2's (or
    D1's) allowed_tools, so this is denied by the chain/authorization
    layer before approval is ever consulted. To isolate "approval present
    but chain still broken" specifically (not just "tool not allowed"),
    this test instead breaks the chain via ancestor revocation on a
    request that *would* otherwise need approval, granted through a
    fresh single-hop delegation that does include add_case_note, then
    revokes it — proving a fully valid, human-approved Approval sitting
    in the database still cannot rescue a chain that fails validation."""

    def _note_count():
        return db_conn.execute("SELECT COUNT(*) FROM case_notes WHERE customer_id = 'cust_1'").fetchone()[0]

    agent = agents.register_agent(db_conn, alice, name="approval-chain-agent", version="0.1.0")
    grant = delegation.grant_delegation(
        db_conn,
        alice,
        agent_id=agent.agent_id,
        allowed_tools=frozenset({"add_case_note"}),
        resource_constraints={"customer_ids": {"cust_1"}},
        ttl_seconds=3600,
    )
    note_args = {"customer_id": "cust_1", "note": "refund approved"}
    appr = approval.request_approval(
        db_conn,
        alice,
        agent_id=agent.agent_id,
        delegation_id=grant.delegation_id,
        tool_name="add_case_note",
        resource_type="customer_ids",
        resource_id="cust_1",
        proposed_arguments=note_args,
        ttl_seconds=300,
    )
    approved = approval.approve(db_conn, alice, appr.approval_id, verifier=VERIFIER)

    # Now break the (root-only, single-hop) chain the only way a root can
    # break: revoke it directly.
    delegation.revoke_delegation(db_conn, alice, grant.delegation_id)

    runner = AgentRunner(
        agent_id=agent.agent_id,
        delegation_id=grant.delegation_id,
        approval_id=approved.approval_id,
        model=FakeModelProvider(
            script=[
                ModelResponse(tool_call=ToolCallRequest(id="c1", name="add_case_note", arguments=note_args)),
                ModelResponse(text="done"),
            ]
        ),
        db_conn=db_conn,
        trace_dir=trace_dir,
    )
    await runner.run(human=alice, goal="add a note")

    assert _note_count() == 0


async def test_valid_chain_plus_required_approval_executes_once(db_conn, trace_dir, three_hop_chain, alice):
    """The positive mirror of the test above: a fully valid 3-hop chain
    AND a required, satisfied approval both present -> execution happens,
    exactly once. Uses a dedicated single-tool delegation (add_case_note)
    granted directly to Agent C so the approval's tool/resource/argument
    hash lines up with what's actually requested, while still exercising
    a real (root-only) chain-verification pass through the same code path
    the 3-hop scenario uses."""

    def _note_count():
        return db_conn.execute("SELECT COUNT(*) FROM case_notes WHERE customer_id = 'cust_1'").fetchone()[0]

    grant = delegation.grant_delegation(
        db_conn,
        alice,
        agent_id=three_hop_chain.agent_c.agent_id,
        allowed_tools=frozenset({"add_case_note"}),
        resource_constraints={"customer_ids": {"cust_1"}},
        ttl_seconds=3600,
    )
    note_args = {"customer_id": "cust_1", "note": "refund approved"}
    appr = approval.request_approval(
        db_conn,
        alice,
        agent_id=three_hop_chain.agent_c.agent_id,
        delegation_id=grant.delegation_id,
        tool_name="add_case_note",
        resource_type="customer_ids",
        resource_id="cust_1",
        proposed_arguments=note_args,
        ttl_seconds=300,
    )
    approved = approval.approve(db_conn, alice, appr.approval_id, verifier=VERIFIER)

    runner = AgentRunner(
        agent_id=three_hop_chain.agent_c.agent_id,
        delegation_id=grant.delegation_id,
        approval_id=approved.approval_id,
        model=FakeModelProvider(
            script=[
                ModelResponse(tool_call=ToolCallRequest(id="c1", name="add_case_note", arguments=note_args)),
                ModelResponse(text="done"),
            ]
        ),
        db_conn=db_conn,
        trace_dir=trace_dir,
    )
    result = await runner.run(human=alice, goal="add a note")

    assert result.stopped_reason == "final_response"
    assert _note_count() == 1


# ── §43: single-hop compatibility (degenerate case) ──────────────────────


def test_root_only_delegation_verifies_identically_to_v05(db_conn, alice, support_agent, support_agent_delegation):
    """A plain v0.5-shaped Delegation (support_agent_delegation, from
    conftest.py, granted with the pre-v0.6 grant_delegation signature)
    verifies through the *same* chain verifier with depth 1 and no
    ancestor-walk overhead — proving old and new are one system, not two."""
    instance = AgentInstance.new()
    instance = AgentInstance(
        instance_id=instance.instance_id,
        started_at=instance.started_at,
        agent_id=support_agent.agent_id,
        human_user_id=alice.user_id,
        session_id="",
        task_id="",
    )
    result = chain.verify_delegation_chain(
        db_conn,
        principal=alice,
        agent_id=support_agent.agent_id,
        instance=instance,
        task_id="",
        delegation_id=support_agent_delegation.delegation_id,
        tool_name="get_customer",
        resource_type="customer_ids",
        resource_id="cust_1",
    )
    assert result.allowed
    assert result.delegation_depth == 1
    assert result.authority_path == [("human", alice.user_id), ("agent", support_agent.agent_id)]


# ── §38: issuance-time attenuation rejection ──────────────────────────────


def test_issuance_rejects_broadened_tools(db_conn, three_hop_chain, alice):
    with pytest.raises(delegation.DelegationAttenuationError):
        delegation.grant_chained_delegation(
            db_conn,
            alice,
            issuer_agent_id=three_hop_chain.agent_b.agent_id,
            issuer_private_key_pem=three_hop_chain.priv_b,
            issuer_signing_key_id=three_hop_chain.key_b.key_id,
            parent_delegation_id=three_hop_chain.d1.delegation_id,
            subject_agent_id=three_hop_chain.agent_c.agent_id,
            allowed_tools=frozenset({"get_customer", "add_case_note"}),  # D1 never granted add_case_note
            resource_constraints={"customer_ids": {"cust_1"}},
            ttl_seconds=3600,
        )


def test_issuance_rejects_broadened_resources(db_conn, three_hop_chain, alice):
    with pytest.raises(delegation.DelegationAttenuationError):
        delegation.grant_chained_delegation(
            db_conn,
            alice,
            issuer_agent_id=three_hop_chain.agent_b.agent_id,
            issuer_private_key_pem=three_hop_chain.priv_b,
            issuer_signing_key_id=three_hop_chain.key_b.key_id,
            parent_delegation_id=three_hop_chain.d1.delegation_id,
            subject_agent_id=three_hop_chain.agent_c.agent_id,
            allowed_tools=frozenset({"get_customer"}),
            resource_constraints={"customer_ids": {"cust_1", "cust_999"}},  # cust_999 never in D1
            ttl_seconds=3600,
        )


def test_issuance_rejects_extended_expiry(db_conn, three_hop_chain, alice):
    with pytest.raises(delegation.DelegationAttenuationError):
        delegation.grant_chained_delegation(
            db_conn,
            alice,
            issuer_agent_id=three_hop_chain.agent_b.agent_id,
            issuer_private_key_pem=three_hop_chain.priv_b,
            issuer_signing_key_id=three_hop_chain.key_b.key_id,
            parent_delegation_id=three_hop_chain.d1.delegation_id,
            subject_agent_id=three_hop_chain.agent_c.agent_id,
            allowed_tools=frozenset({"get_customer"}),
            resource_constraints={"customer_ids": {"cust_1"}},
            ttl_seconds=4 * 3600,  # within MAX_TTL_SECONDS, but past D1's own ~2h remaining expiry
        )


def test_issuance_rejects_a_revoked_parent(db_conn, three_hop_chain, alice):
    delegation.revoke_delegation(db_conn, alice, three_hop_chain.d1.delegation_id)
    with pytest.raises(delegation.DelegationValidationError):
        delegation.grant_chained_delegation(
            db_conn,
            alice,
            issuer_agent_id=three_hop_chain.agent_b.agent_id,
            issuer_private_key_pem=three_hop_chain.priv_b,
            issuer_signing_key_id=three_hop_chain.key_b.key_id,
            parent_delegation_id=three_hop_chain.d1.delegation_id,
            subject_agent_id=three_hop_chain.agent_c.agent_id,
            allowed_tools=frozenset({"get_customer"}),
            resource_constraints={"customer_ids": {"cust_1"}},
            ttl_seconds=60,
        )


def test_issuance_rejects_an_expired_parent(db_conn, three_hop_chain, alice):
    db_conn.execute(
        "UPDATE delegations SET expires_at = ? WHERE delegation_id = ?",
        (time.time() - 1, three_hop_chain.d1.delegation_id),
    )
    db_conn.commit()
    with pytest.raises(delegation.DelegationValidationError):
        delegation.grant_chained_delegation(
            db_conn,
            alice,
            issuer_agent_id=three_hop_chain.agent_b.agent_id,
            issuer_private_key_pem=three_hop_chain.priv_b,
            issuer_signing_key_id=three_hop_chain.key_b.key_id,
            parent_delegation_id=three_hop_chain.d1.delegation_id,
            subject_agent_id=three_hop_chain.agent_c.agent_id,
            allowed_tools=frozenset({"get_customer"}),
            resource_constraints={"customer_ids": {"cust_1"}},
            ttl_seconds=60,
        )
