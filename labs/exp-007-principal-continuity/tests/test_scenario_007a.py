import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

from exp007 import scenario_007a as s
from exp007.authorization import (
    CONTINUITY_UNVERIFIED,
    CONTROLLER_NOT_AUTHORIZED,
    NO_CAPABILITY,
    authenticated_only_policy,
    capability_valid,
    controller_authenticated,
    controller_authorized,
    secure_policy,
    vulnerable_policy,
)
from exp007.identity import derive_key, verify

ROOT = Path(__file__).resolve().parent.parent
SIGNATURE_STYLE_DETAILS = {"UNSIGNED", "BAD_SIGNATURE", "UNREGISTERED_CONTROLLER"}


def _run(name, policy):
    return s.run_policy(name, policy)


def test_vulnerable_policy_fails_and_executes_every_substitution_leg():
    results, _, _ = _run("vulnerable_policy", vulnerable_policy)
    assert s.policy_verdict(results) == s.FAIL
    assert [r.executed for r in results] == [True, True, True, True]


def test_secure_policy_passes_and_blocks_every_substitution_leg():
    results, _, _ = _run("secure_policy", secure_policy)
    assert s.policy_verdict(results) == s.PASS
    assert [r.executed for r in results] == [True, False, False, False]
    assert results[1].decision.detail == "UNSIGNED"
    assert results[2].decision.detail == "BAD_SIGNATURE"


def test_authenticated_substitute_precondition_everything_valid_except_continuity():
    session, u_key, ctx = s.build_world()
    msg = s.LEGS[3].build(session, u_key)
    g = ctx.grant
    # B is honestly authenticated as B ...
    assert msg.controller_pid == s.B.pid
    assert verify(derive_key("B"), msg.signed_payload(), msg.signature) is True
    assert controller_authenticated(msg, ctx) is None
    # ... A and the U-granted capability are fully valid ...
    assert ctx.agent_id == g.agent_id
    assert capability_valid(msg, ctx) is True
    assert g.revoked is False and (g.expires_at is None or ctx.now < g.expires_at)
    # ... and the only thing missing is U -> B authority.
    assert g.owner_pid == s.U.pid
    assert controller_authorized(msg, ctx) is False
    assert s.substitute_preconditions() == []


def test_authenticated_substitute_reason_is_continuity_specific():
    results, probe, _ = _run("secure_policy", secure_policy)
    d = results[3]
    assert d.decision.verdict.value == "STEP_UP"
    assert d.decision.reason == CONTROLLER_NOT_AUTHORIZED
    assert d.decision.detail not in SIGNATURE_STYLE_DETAILS
    for banned in ("BAD_SIGNATURE", "INVALID_SIGNATURE", "UNSIGNED", "UNAUTHENTICATED",
                   "NO_CAPABILITY", "CAPABILITY_MISSING", "TOKEN_INVALID"):
        assert d.decision.reason != banned
    r = d.record
    assert (r.agent_identity, r.agent_authenticated) == ("agent:A", True)
    assert (r.controller_identity, r.controller_authenticated) == ("pid:B", True)
    assert r.capability_valid is True
    assert r.expected_authority_root == "pid:U"
    assert r.controller_authorized_for_authority_chain is False
    assert r.controller_changed is True
    assert r.reconstruction_identifies_true_chain is True


def test_four_reason_codes_are_distinct():
    results, probe, _ = _run("secure_policy", secure_policy)
    a, b, _c, d = results
    reasons = {a.decision.reason, b.decision.reason, d.decision.reason, probe.decision.reason}
    assert len(reasons) == 4
    assert probe.decision.reason == NO_CAPABILITY
    assert b.decision.reason == CONTINUITY_UNVERIFIED
    assert s.distinction_problems(results, probe) == []


def test_vulnerable_trace_does_not_claim_b_is_u_and_shows_unchecked_authority():
    results, _, _ = _run("vulnerable_policy", vulnerable_policy)
    d = results[3].record
    assert d.controller_identity == "pid:B"  # not relabelled as U
    assert d.controller_authorized_for_authority_chain is None  # never evaluated
    assert d.expected_authority_root == "pid:U"
    assert "controller not checked" in d.resource_decision
    assert d.reconstruction_identifies_true_chain is False  # cannot show B lacked authority
    assert d.controller_changed is False  # blind to the substitution


def test_unauthenticated_substitution_vulnerable_trace_misattributes_to_u():
    v, _, _ = _run("vulnerable_policy", vulnerable_policy)
    sec, _, _ = _run("secure_policy", secure_policy)
    assert v[1].record.controller_identity == "pid:U"
    assert v[1].record.reconstruction_identifies_true_chain is False
    assert sec[1].record.controller_identity.startswith("UNVERIFIED(")
    assert sec[1].record.reconstruction_identifies_true_chain is True


def test_gate_passes_for_reference_policies():
    assert s.gate()[-1] == []


@pytest.mark.parametrize("broken", [vulnerable_policy, authenticated_only_policy])
def test_negative_control_gate_rejects_broken_secure_policies(broken):
    *_, failures = s.gate(secure=broken)
    assert failures, "gate must reject a secure policy that ignores controller continuity"


def test_authenticated_only_mutant_fails_specifically_on_authenticated_substitute():
    results, _, _ = _run("secure_policy", authenticated_only_policy)
    assert results[3].executed is True and results[3].outcome == s.FAIL
    assert s.policy_verdict(results) == s.FAIL


def test_runner_exits_nonzero_for_broken_secure_policy(monkeypatch):
    for broken in (vulnerable_policy, authenticated_only_policy):
        monkeypatch.setattr(s, "secure_policy", broken)
        assert s.main(["--json"]) == 1


def test_runner_exits_nonzero_if_secure_policy_never_evaluated_authority_chain(monkeypatch):
    monkeypatch.setattr(s, "vulnerable_policy", secure_policy)  # vulnerable can no longer FAIL
    assert s.main(["--json"]) == 1


def test_evidence_has_all_fields():
    _, _, _, _, _, s_rec, *_ = s.gate()
    rec = s_rec.dump()[3]
    for f in ("principal_identity", "agent_identity", "controller_identity", "original_grant",
              "approval_object", "action_params", "resource_decision", "execution_result",
              "audit_note", "reconstruction_identifies_true_chain", "controller_changed",
              "step_nonce", "timestamp", "agent_authenticated", "controller_authenticated",
              "capability_valid", "expected_authority_root",
              "controller_authorized_for_authority_chain"):
        assert f in rec


def test_deterministic_across_repeated_in_process_runs(capsys):
    outputs = []
    for _ in range(10):
        assert s.main(["--json"]) == 0
        outputs.append(capsys.readouterr().out)
    assert len(set(outputs)) == 1
    json.loads(outputs[0])


def test_deterministic_across_processes_and_hash_seeds():
    outs = set()
    for seed in ("0", "1", "42", "random"):
        env = dict(os.environ, PYTHONHASHSEED=seed)
        p = subprocess.run([sys.executable, "-m", "exp007.scenario_007a", "--json"],
                           cwd=ROOT, env=env, capture_output=True, text=True)
        assert p.returncode == 0, p.stdout + p.stderr
        outs.add(p.stdout)
    assert len(outs) == 1
