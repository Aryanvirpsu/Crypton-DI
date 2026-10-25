"""EXP-007 Scenario 007-A runner: controller substitution.

Usage (from labs/exp-007-principal-continuity):
    python -m exp007.scenario_007a [--evidence | --json]

Exit code 0 only if the test separates the two policies AND the secure
policy separates controller substitution from ordinary authorization
failure. Non-zero means the measured distinction collapsed.
"""

from __future__ import annotations

import json
import sys
from dataclasses import dataclass
from typing import Callable

from .agent import NOT_EXECUTED, Agent
from .authorization import (
    CONTINUITY_UNVERIFIED,
    CONTINUITY_VERIFIED,
    CONTROLLER_NOT_AUTHORIZED,
    NO_CAPABILITY,
    Decision,
    Grant,
    PolicyContext,
    Verdict,
    agent_authenticated,
    capability_valid,
    controller_authenticated,
    controller_authorized,
    secure_policy,
    vulnerable_policy,
)
from .control_channel import ControlMessage, build_forged, build_signed, build_unsigned
from .identity import KeyDirectory, Principal, Session, derive_key
from .resource import MockResource
from .tracing import EvidenceRecord, Recorder

PASS, FAIL, QUALITY_ISSUE, NOT_APPLICABLE = "PASS", "FAIL", "QUALITY ISSUE", "N/A"

U = Principal("pid:U", "User U")
B = Principal("pid:B", "Controller B")
AGENT_ID = "agent:A"
ACTION = "SEND_MESSAGE"
U_PARAMS = {"to": "alice", "body": "meeting at 3"}
B_PARAMS = {"to": "attacker@example.invalid", "body": "exfiltrated"}


def build_world(capabilities: tuple[str, ...] = (ACTION,)):
    session = Session("sess-001", U.pid, "task-001", "nonce-U-0001")
    u_key = derive_key("U")
    keys = KeyDirectory()
    keys.register(U.pid, u_key)
    keys.register(B.pid, derive_key("B"))  # B has a real, registered identity of its own
    # Authority chain rooted at U. There is deliberately NO U -> B delegation.
    grant = Grant(
        AGENT_ID, U.pid, frozenset(capabilities), session.session_id,
        authorized_controllers=frozenset({U.pid}),
    )
    return session, u_key, PolicyContext(AGENT_ID, grant, session, keys)


@dataclass(frozen=True)
class Leg:
    name: str
    description: str
    true_actor: str
    expect_execute: bool
    expected_reason: str  # reason a correct secure policy must give
    build: Callable[[Session, bytes], ControlMessage]


LEGS = [
    Leg("a", "U signed, in-session", U.pid, True, CONTINUITY_VERIFIED,
        lambda s, k: build_signed(k, s, U.pid, ACTION, U_PARAMS)),
    Leg("b", "B unsigned, spoofing U", B.pid, False, CONTINUITY_UNVERIFIED,
        lambda s, k: build_unsigned(s, U.pid, ACTION, B_PARAMS)),
    Leg("c", "B forged signature", B.pid, False, CONTINUITY_UNVERIFIED,
        lambda s, k: build_forged(derive_key("B-guess"), s, U.pid, ACTION, B_PARAMS)),
    Leg("d", "B authenticated as B, no U->B delegation", B.pid, False, CONTROLLER_NOT_AUTHORIZED,
        lambda s, k: build_signed(derive_key("B"), s, B.pid, ACTION, B_PARAMS)),
]


@dataclass(frozen=True)
class LegResult:
    leg: str
    description: str
    decision: Decision
    executed: bool
    outcome: str  # OK / FAIL / QUALITY ISSUE
    note: str
    record: EvidenceRecord


def classify(leg: Leg, decision: Decision, executed: bool) -> tuple[str, str]:
    if leg.expect_execute:
        if executed:
            return "OK", "legitimate action executed"
        return QUALITY_ISSUE, "legitimate action blocked (availability failure)"
    if executed:
        return FAIL, "substituted controller's action executed"
    if decision.reason != leg.expected_reason:
        return QUALITY_ISSUE, (
            f"blocked, but reason {decision.reason} != expected {leg.expected_reason}"
        )
    return "OK", f"blocked on {decision.reason} ({decision.detail})"


def run_policy(name: str, policy) -> tuple[list[LegResult], LegResult, Recorder]:
    recorder = Recorder(run_label=name)
    resource = MockResource()
    session, u_key, ctx = build_world()
    agent = Agent(ctx, policy, name, resource, recorder)

    results = []
    for leg in LEGS:
        decision, step = agent.handle(leg.build(session, u_key), leg.name)
        rec = recorder.finalize(step, leg.true_actor)
        executed = rec.execution_result != NOT_EXECUTED
        outcome, note = classify(leg, decision, executed)
        results.append(LegResult(leg.name, leg.description, decision, executed, outcome, note, rec))

    # Calibration probe (not a 007-A leg): U signs correctly but the agent lacks the
    # capability. This is ordinary authorization failure; it must not look like a
    # continuity failure, or the test could not tell the two apart.
    session2, u_key2, ctx2 = build_world(capabilities=("READ_INBOX",))
    probe_agent = Agent(ctx2, policy, name, resource, recorder)
    msg = build_signed(u_key2, session2, U.pid, ACTION, U_PARAMS)
    decision, step = probe_agent.handle(msg, "cal-no-capability")
    rec = recorder.finalize(step, U.pid)
    probe = LegResult(
        "cal-no-capability", "U signed, agent lacks capability (ordinary authz failure)",
        decision, rec.execution_result != NOT_EXECUTED, "OK", "calibration only", rec,
    )
    return results, probe, recorder


def policy_verdict(results: list[LegResult]) -> str:
    outcomes = {r.outcome for r in results}
    if FAIL in outcomes:
        return FAIL
    if QUALITY_ISSUE in outcomes:
        return QUALITY_ISSUE
    return PASS


def substitute_preconditions() -> list[str]:
    """Direct proof that leg d isolates continuity: everything else about it is valid.

    If any of these fail, leg d would not be measuring principal continuity, so the
    gate fails instead of trusting the secure policy's own reason code.
    """
    session, u_key, ctx = build_world()
    msg = LEGS[3].build(session, u_key)
    g = ctx.grant
    checks = {
        "controller authenticates as B (signature verifies, key registered)":
            controller_authenticated(msg, ctx) is None and msg.controller_pid == B.pid,
        "agent A authenticated": agent_authenticated(ctx),
        "capability SEND_MESSAGE valid for A": capability_valid(msg, ctx),
        "capability not expired": g.expires_at is None or ctx.now < g.expires_at,
        "capability not revoked": not g.revoked,
        "message bound to live session/task/nonce":
            (msg.session_id, msg.task_id, msg.nonce)
            == (session.session_id, session.task_id, session.nonce),
        "no U -> B delegation (B not an authorized controller)":
            not controller_authorized(msg, ctx) and B.pid not in g.authorized_controllers,
        "grant owner (authority root) is U": g.owner_pid == U.pid,
    }
    return [f"precondition failed: {name}" for name, ok in checks.items() if not ok]


def distinction_problems(secure: list[LegResult], probe: LegResult) -> list[str]:
    """Reasons the test would fail to separate substitution from ordinary auth failure."""
    a, b, c, d = secure
    problems = []
    if a.decision.reason != CONTINUITY_VERIFIED:
        problems.append("secure policy did not verify continuity on the legitimate leg")
    for leg in (b, c):
        if leg.decision.reason != CONTINUITY_UNVERIFIED:
            problems.append(f"leg {leg.leg} not rejected as a continuity failure")
        if leg.decision.verdict is not Verdict.STEP_UP:
            problems.append(f"leg {leg.leg} did not require step-up")
        if not leg.record.controller_changed:
            problems.append(f"leg {leg.leg} not flagged controller_changed")
        if not leg.record.reconstruction_identifies_true_chain:
            problems.append(f"leg {leg.leg} trace does not identify the true causal chain")
        if leg.record.controller_authenticated is not False:
            problems.append(f"leg {leg.leg} should record controller_authenticated=False")
    rd = d.record
    if d.decision.reason != CONTROLLER_NOT_AUTHORIZED:
        problems.append("leg d not rejected as CONTROLLER_NOT_AUTHORIZED")
    if d.decision.verdict is not Verdict.STEP_UP:
        problems.append("leg d did not require step-up")
    if rd.controller_identity != B.pid:
        problems.append("leg d trace does not name B as the controller")
    if rd.controller_authenticated is not True:
        problems.append("leg d trace should record controller_authenticated=True")
    if rd.controller_authorized_for_authority_chain is not False:
        problems.append("leg d trace should record controller not authorized for the chain")
    if not (rd.agent_authenticated and rd.capability_valid):
        problems.append("leg d trace should record agent authenticated and capability valid")
    if rd.expected_authority_root != U.pid:
        problems.append("leg d trace should record U as the expected authority root")
    if not rd.controller_changed or not rd.reconstruction_identifies_true_chain:
        problems.append("leg d trace does not identify the true causal chain")
    if probe.decision.reason != NO_CAPABILITY:
        problems.append("ordinary authz failure not reported as NO_CAPABILITY")
    reasons = [a.decision.reason, b.decision.reason, d.decision.reason, probe.decision.reason]
    if len(set(reasons)) != 4:
        problems.append("reason codes for legit/unauth-substitute/auth-substitute/no-capability "
                        f"are not four distinct values: {reasons}")
    return problems


def vulnerable_problems(v_results: list[LegResult], v_verdict: str) -> list[str]:
    problems = []
    if v_verdict != FAIL:
        problems.append(f"vulnerable_policy expected FAIL, got {v_verdict}")
    for r in v_results:
        if not r.executed:
            problems.append(f"vulnerable_policy should execute leg {r.leg}")
    d = v_results[3]
    if d.decision.verdict is not Verdict.ALLOW:
        problems.append("vulnerable_policy should ALLOW authenticated substitute (leg d)")
    if d.record.controller_authorized_for_authority_chain is not None:
        problems.append("vulnerable_policy must not claim to have evaluated controller authority")
    return problems


def gate(vulnerable=None, secure=None):
    """Run both policies; return the results plus a list of gate failures.

    Passing a broken `secure` candidate must yield failures (negative control)."""
    vulnerable = vulnerable or vulnerable_policy
    secure = secure or secure_policy
    v_results, v_probe, v_rec = run_policy("vulnerable_policy", vulnerable)
    s_results, s_probe, s_rec = run_policy("secure_policy", secure)
    v_verdict, s_verdict = policy_verdict(v_results), policy_verdict(s_results)
    failures = substitute_preconditions() + vulnerable_problems(v_results, v_verdict)
    if s_verdict != PASS:
        failures.append(f"secure_policy expected PASS, got {s_verdict}")
    failures += distinction_problems(s_results, s_probe)
    return v_results, v_probe, v_rec, s_results, s_probe, s_rec, v_verdict, s_verdict, failures


def print_summary(name, results, probe, verdict) -> None:
    print(f"\n== policy: {name} ==")
    for r in results:
        print(f"  leg {r.leg} [{r.description}]")
        print(f"      decision : {r.decision.render()}")
        print(f"      executed : {r.executed}   -> {r.outcome}: {r.note}")
    print(f"  probe {probe.leg} [{probe.description}]")
    print(f"      decision : {probe.decision.render()}")
    print(f"  VERDICT ({name}): {verdict}")


def print_matrix(v_results, v_probe, s_results, s_probe) -> None:
    print("\n== semantic result matrix ==")
    print(f"  {'case':<19}{'ctrl auth':<11}{'capab':<7}{'ctrl authority':<16}"
          f"{'vulnerable':<12}{'secure':<10}reason")
    rows = list(zip(v_results, s_results)) + [(v_probe, s_probe)]
    for v, s in rows:
        r = s.record
        auth = {True: "valid", False: "invalid", None: "-"}[r.controller_authenticated]
        cap = "valid" if r.capability_valid else "absent"
        authority = {True: "valid", False: "absent", None: "-"}[
            r.controller_authorized_for_authority_chain]
        if s.leg.startswith("cal"):
            auth, authority = "valid (U)", "valid"
        print(f"  {s.leg:<19}{auth:<11}{cap:<7}{authority:<16}"
              f"{v.decision.verdict.value:<12}{s.decision.verdict.value:<10}{s.decision.reason}")


def print_evidence(recorder: Recorder) -> None:
    for rec in recorder.dump():
        print(f"\n  -- {rec['policy']} / leg {rec['leg']} (step {rec['step']}) --")
        for k, v in rec.items():
            print(f"     {k}: {v}")


def main(argv: list[str] | None = None) -> int:
    args = sys.argv[1:] if argv is None else argv
    (v_results, v_probe, v_rec, s_results, s_probe, s_rec,
     v_verdict, s_verdict, failures) = gate(vulnerable_policy, secure_policy)

    if "--json" in args:
        print(json.dumps({"vulnerable_policy": v_rec.dump(), "secure_policy": s_rec.dump()},
                         indent=2, sort_keys=True))
    else:
        print("EXP-007 / 007-A  Controller Substitution (principal continuity)")
        print_summary("vulnerable_policy", v_results, v_probe, v_verdict)
        print_summary("secure_policy", s_results, s_probe, s_verdict)
        print_matrix(v_results, v_probe, s_results, s_probe)
        if "--evidence" in args:
            print("\n== evidence (all fields, every step) ==")
            print_evidence(v_rec)
            print_evidence(s_rec)
        if failures:
            print("\nDISTINCTION CHECK: COLLAPSED")
            for f in failures:
                print(f"  - {f}")
        else:
            print("\nDISTINCTION CHECK: OK - continuity failure (CONTINUITY_UNVERIFIED), "
                  "authenticated-substitute failure (CONTROLLER_NOT_AUTHORIZED) and ordinary "
                  "authz failure (NO_CAPABILITY) are separately reported; vulnerable policy "
                  "cannot separate them.")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
