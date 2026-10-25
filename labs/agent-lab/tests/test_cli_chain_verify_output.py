"""
v0.6 — `cli._chain_verify_output` (the §39 developer-facing chain proof
rendering). Pure formatting function, tested directly against constructed
`ChainVerificationResult` objects rather than through a real chain, so
these stay fast and independent of the chain-verification tests
themselves (test_delegation_chain.py already proves the *decisions* this
just has to render correctly).

Also a regression guard: an earlier version of this function used "→"
(U+2192) in the authority-path rendering, which raises
`UnicodeEncodeError` when printed to a default-codepage (cp1252) Windows
console — caught only by manually running the CLI, not by the rest of the
suite. Every line this function produces must stay plain ASCII.
"""

from agentlab import authorization, chain
from agentlab.cli import _chain_verify_output


def _allow_result() -> chain.ChainVerificationResult:
    decision = authorization.AuthorizationDecision(
        decision_id="authz_test",
        decision=authorization.DECISION_ALLOW,
        reason_code=authorization.ALLOW,
        principal_user_id="user-alice",
        agent_id="agt_c",
        instance_id="inst_1",
        task_id="",
        delegation_id="dlg_d2",
        tool_name="get_customer",
        resource="customer_ids:cust_1",
        timestamp=0.0,
    )
    return chain.ChainVerificationResult(
        allowed=True,
        decision=decision,
        root_principal="user-alice",
        acting_principal="agt_c",
        delegation_depth=3,
        authority_path=[("human", "user-alice"), ("agent", "agt_a"), ("agent", "agt_b"), ("agent", "agt_c")],
        effective_tools=frozenset({"get_customer"}),
        effective_resources={"customer_ids": frozenset({"cust_1"})},
        effective_expiry=1000.0,
        chain_hash="deadbeef",
    )


def _deny_result() -> chain.ChainVerificationResult:
    decision = authorization.AuthorizationDecision(
        decision_id="authz_test_deny",
        decision=authorization.DECISION_DENY,
        reason_code=chain.CHAIN_ANCESTOR_REVOKED,
        principal_user_id="user-alice",
        agent_id="agt_c",
        instance_id="inst_1",
        task_id="",
        delegation_id="dlg_d2",
        tool_name="get_customer",
        resource="customer_ids:cust_1",
        timestamp=0.0,
    )
    return chain.ChainVerificationResult(
        allowed=False,
        decision=decision,
        root_principal=None,
        acting_principal="agt_c",
        delegation_depth=2,
        failed_hop="agt_b",
        denial_reason=chain.CHAIN_ANCESTOR_REVOKED,
    )


def test_allow_output_is_pure_ascii_and_shows_full_path():
    output = _chain_verify_output(_allow_result(), requested_tool="get_customer", requested_resource="customer_ids:cust_1")

    output.encode("ascii")  # raises UnicodeEncodeError on any non-ASCII char, same failure mode as cp1252 stdout
    assert "ROOT PRINCIPAL" in output
    assert "Human user-alice" in output
    assert "ACTING PRINCIPAL" in output
    assert "Agent agt_c" in output
    assert "DELEGATION DEPTH" in output
    assert "3" in output
    assert "AUTHORITY PATH" in output
    assert "-> Agent agt_a" in output
    assert "-> Agent agt_b" in output
    assert "-> Agent agt_c" in output
    assert "DECISION\nALLOW" in output
    assert "FAILED HOP" not in output


def test_deny_output_is_pure_ascii_and_shows_failure_reason():
    output = _chain_verify_output(_deny_result(), requested_tool="get_customer", requested_resource="customer_ids:cust_1")

    output.encode("ascii")
    assert "DECISION\nDENY" in output
    assert "FAILED HOP" in output
    assert "agt_b" in output
    assert "REASON" in output
    assert chain.CHAIN_ANCESTOR_REVOKED in output
    # A denial with no resolved chain has nothing meaningful to show for
    # the root/authority-path section — it must be omitted, not printed
    # with placeholder/garbage values.
    assert "ROOT PRINCIPAL" not in output
    assert "AUTHORITY PATH" not in output
