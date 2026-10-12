"""
v0.6.1 — Crypton Reference Agent Lab — Methodology Dry Run.

Builds a synthetic `Engagement` by driving Agent Lab's *real* primitives
(`agents.py`, `delegation.py`, `chain.py`, `approval.py` — called exactly
as `cli.py` and the existing test suite already call them, never
modified) and capturing their real output as evidence. Nothing here is
fabricated: every `EvidenceReference` below points at a real
`AuthorizationDecision`/`ChainVerificationResult`/`Approval` produced by
this run, through the primitives' own real ID-minting and hash-chaining
code paths.

**Determinism (2026-09, pre-commit audit):** the committed fixture under
`assessment-engagement/reference-agentlab-dryrun/` must not mutate on
every regeneration — a checked-in reference artifact that diffs on every
run is not a usable fixture. IDs (`dlg_*`, `agt_*`, `authz_*`, `apr_*`,
`ask_*`, instance UUIDs) and the delegation hash-chain are derived from
`uuid.uuid4()` and `time.time()` calls inside the frozen primitive
modules. Rather than editing those modules, `build_reference_engagement`
patches the process-wide `uuid.uuid4` and `time.time` functions —
*only* for the duration of the primitive-driving block below, and *only*
from this synthetic-fixture layer — to a fixed, obviously-synthetic
sequence (`_deterministic_uuid_factory`, `REFERENCE_FIXED_TIME`). This is
the same monkeypatch technique the test suite already uses for isolation
(see `test_delegation_chain.py`); it changes no production ID-generation
logic and no security semantics — only which entropy source a scripted,
single-process dry run reads from. Real runtime IDs (anything minted
outside this script) are unaffected and remain ordinary random
`uuid4().hex` values; the synthetic ones below are deliberately
zero-padded and sequential (`...4000-8000-000000000001`, `...002`, ...)
so they are visually unmistakable as fixture data, never confusable with
a real deployment's IDs.

**What this is not:** an external customer assessment, or proof that
Crypton can automatically assess an arbitrary third-party system. This
module exists to demonstrate — end to end, with real evidence — that the
Finding/Evidence/Report model in this package can represent a real
engagement's full range of outcomes (ENFORCED, VIOLATED, UNREPRESENTABLE,
NOT_APPLICABLE — see the 2026-09-30 remediation pass's canonical
five-outcome model in `models.py`) across all eight assessment areas. The
one deliberately "VIOLATED" finding here (AD-02) is a synthetic over-broad
delegation grant — a realistic *misconfiguration* shape, not a defect in
Crypton's enforcement code, and its Gate 7 (security-claim/trust-boundary
triage) is satisfied because the over-grant genuinely crosses this
workflow's own stated least-privilege boundary (E-003 shows it is
technically usable, not merely present on paper). See
`docs/ASSESSMENT_METHODOLOGY.md` and `DECISIONS.md`'s v0.6.1 section for
the full framing.

`Engagement.label` is set to the literal string
"Crypton Reference Agent Lab — Methodology Dry Run" specifically so the
rendered report can never be mistaken for a real engagement's output —
`report.render_report` prints it prominently in the header.
"""

from __future__ import annotations

import itertools
import uuid
from pathlib import Path
from typing import Any, Callable
from unittest.mock import patch

from .. import agents, approval as approval_module, authorization, chain, db, delegation, signing_keys
from .. import tools as tools_module
from ..agent import AgentRunner
from ..identity import AgentInstance, HumanIdentity
from ..model_provider import FakeModelProvider, ModelResponse, ToolCallRequest
from .engagement import Engagement
from .evidence import EvidenceManifest
from .models import (
    AssessmentArea,
    EvidenceClassification,
    EvidenceGateStatus,
    EvidenceReference,
    Finding,
    Outcome,
    Severity,
)

REFERENCE_ASSESSMENT_ID = "ENG-REFERENCE-AGENTLAB-DRYRUN"

# Every ENFORCED/VIOLATED finding below cites this fully-satisfied gate
# record, consistent with `models.py::Finding.__post_init__`'s structural
# enforcement that no ENFORCED/VIOLATED finding may omit one. Honest basis
# for each gate, for *this specific synthetic self-assessment*:
#   G1-G4  directly demonstrated: real primitive calls, real captured
#          decisions/results (E-003 through E-008), each tied to a real
#          persisted row this run actually produced.
#   G5     AD-04/AD-05 and AD-06's first-consume/replay-denial are each
#          other's within-run positive/negative control; AD-02's control
#          is the documented stated task need (workflow.md) the actual
#          grant is compared against. For AD-03/AD-08, calibration
#          confidence comes from Agent Lab's own adversarial test suite
#          (test_attenuation.py, test_delegation_chain.py's RAT-6 attack
#          cases) — cited analogically, per
#          docs/ASSESSMENT_METHODOLOGY.md's sanctioned exception for a
#          self-assessment of Crypton's own reference implementation; this
#          dry run does not re-run a fresh negative case per finding.
#   G6     the underlying primitives (chain verification, authorization,
#          approval) are deterministic given the same persisted state, and
#          are independently reproduced 263 times over in Agent Lab's own
#          test suite — this dry run itself runs each check once, not
#          repeatedly.
#   G7     reasoning is specific to each finding; stated in its own
#          `observation`/`risk_impact` text below, not asserted generically
#          here.
_FULLY_SATISFIED_GATES = EvidenceGateStatus(
    g1_representability=True,
    g2_observability=True,
    g3_controlled_execution=True,
    g4_effect_confirmation=True,
    g5_calibration_control=True,
    g6_reproduction=True,
    g7_security_claim_triage=True,
    notes=(
        "Synthetic self-assessment of Crypton's own Agent Lab reference implementation — see this "
        "module's source comment above this constant for the specific, honest basis of each gate "
        "in this context; G5/G6 lean on Agent Lab's own 263-test adversarial suite as calibration/"
        "reproduction evidence, per docs/ASSESSMENT_METHODOLOGY.md's sanctioned self-assessment "
        "exception, not a fresh negative case or repeated run constructed separately per finding."
    ),
)

REFERENCE_HUMAN = HumanIdentity(user_id="user-reference", username="reference-operator", cred_id="dev-cred-reference")

# Fixed synthetic wall-clock used only while `build_reference_engagement`'s
# primitive-driving block runs (see the module docstring's "Determinism"
# note). Value is arbitrary and unrelated to `Engagement.date` below —
# it exists purely so `issued_at`/hash-chain math is reproducible, not to
# represent when this fixture was authored.
REFERENCE_FIXED_TIME = 1_800_000_000.0


def _deterministic_uuid_factory() -> Callable[[], "uuid.UUID"]:
    """Returns a callable that yields a fixed, sequential, obviously-fake
    UUID sequence (`00000000-0000-4000-8000-000000000001`, `...002`, ...)
    each time it's called. Used only as a `uuid.uuid4` patch target, and
    only for the scope of one `build_reference_engagement` call — see the
    module docstring."""
    counter = itertools.count(1)

    def _next_uuid() -> "uuid.UUID":
        n = next(counter)
        return uuid.UUID(f"00000000-0000-4000-8000-{n:012x}")

    return _next_uuid

SCOPE_TEXT = """\
This is a **synthetic, self-assessed reference engagement**, not an external customer assessment.

The workflow assessed is Crypton's own Agent Lab reference implementation
(`labs/agent-lab`) — a small, real (not simulated) agent loop with three tools
(`get_customer`, `get_order`, `add_case_note`), backed by real Delegation,
Approval, and multi-hop chain-verification primitives.

Purpose: demonstrate that the Agent Authorization & Delegation Assessment
methodology (`docs/ASSESSMENT_METHODOLOGY.md`) and its Finding/Evidence/Report
model can represent a real engagement's full range of outcomes — ENFORCED,
VIOLATED, UNREPRESENTABLE, and NOT_APPLICABLE — using genuine evidence, not
fabricated results.

This dry run does not validate Crypton's differentiation against any specific
competitor or claim external customer readiness. It validates the delivery
kit's reporting mechanics against a known, controlled workflow.
"""

WORKFLOW_TEXT = """\
**Reference workflow:** a single registered agent ("reference-agent"), granted
a Delegation by a human operator, executing tool calls against Agent Lab's
seeded demo data (`cust_1` owned by the reference operator; `cust_2` owned by
a different principal in the seed data and used here only as an out-of-scope
resource id, never actually read).

**Stated task need for this workflow (as would appear in a real engagement's
scope-of-work document):** the agent needs to read one customer's profile
(`get_customer` on `cust_1`) and, with human approval, add a case note for
that same customer. Nothing else.

**Actual grant issued (see AD-02):** the delegation used in this dry run
grants `get_customer`, `get_order`, and `add_case_note` against both `cust_1`
and `cust_2` — broader than the stated task need, on purpose, to produce a
realistic over-provisioning finding.

A second, narrower chained delegation (Agent A -> Agent B) is also issued to
demonstrate multi-hop scope attenuation (AD-03).
"""

LIMITATIONS_TEXT = """\
- This is a **self-assessment of Crypton's own reference implementation**,
  not an independent assessment of a customer's system. Findings here
  demonstrate the *methodology and reporting mechanics*, not Crypton's
  differentiation versus any competitor or category.
- No external identity/HR/IT provisioning system exists for this synthetic
  engagement — AD-01 is UNVERIFIABLE for exactly that reason (evidence could
  not be obtained within this engagement's scope), not because the
  underlying concept is unrepresentable, and not because the question is
  unimportant.
- No live/production traffic exists for this engagement — AD-07 is
  NOT_APPLICABLE for the same reason.
- Agent Lab's WebAuthn step-up is a documented simulation
  (`webauthn.FakeWebAuthnVerifier`) — AD-06's approval-binding evidence
  demonstrates the *protocol design* (argument-hash binding, one-shot
  consumption), not a live authenticator ceremony.
- All findings trace to real, reproducible primitive calls made by
  `agentlab.assessment.reference_dryrun.build_reference_engagement` —
  re-running it against a fresh database reproduces the same
  ENFORCED/VIOLATED/UNVERIFIABLE/NOT_APPLICABLE pattern *and* the same
  evidence IDs byte-for-byte (delegation_id, decision_id, chain_hash, etc.
  are deterministic within this synthetic fixture — see the module
  docstring's "Determinism" note); a real customer engagement's evidence
  IDs are ordinary random values, not fixed like this reference fixture's.
- This engagement predates the 2026-09-30 schema expansion for most
  structured metadata fields (`target_commit`, `testing_window`, etc.) —
  those are set where genuinely known (this *is* Agent Lab, so its own
  product/version/commit are knowable) and left `None` where they
  describe a concept this synthetic, single-run engagement doesn't have
  (e.g. no multi-day `testing_window` — it is one scripted run).
"""

EXECUTIVE_SUMMARY_TEXT = """\
This reference dry run exercises all eight assessment areas against a known,
controlled workflow (Crypton's own Agent Lab), using real primitive calls
rather than fabricated results, to validate the assessment delivery kit's
Finding/Evidence/Report model before it is used on an external engagement.

Of eight areas assessed: five reached **ENFORCED** with cited evidence and a
fully satisfied seven-gate evidence record (scope attenuation across a signed
multi-hop chain, pre-execution enforcement, bypass-attempt denial, approval
one-shot/replay protection, and evidence traceability itself), one reached
**VIOLATED** (a delegation grant broader than the stated task's need — a
realistic over-provisioning finding, not a defect in enforcement; Gate 7
confirms the over-grant genuinely crosses this workflow's own stated
least-privilege boundary, not merely that something looked excessive), one is
**UNVERIFIABLE** (agent identity provisioning process — no external record
source exists to inspect in this synthetic engagement, which is an evidence
gap, not evidence that the underlying concept is unrepresentable), and one is
**NOT_APPLICABLE** (live revocation-propagation timing — no production
traffic exists to time).

This is not an external customer result and should not be read as one.
"""


def _authz_decision_evidence(
    evidence_id: str, decision, *, source: str, description: str
) -> EvidenceReference:
    return EvidenceReference(
        evidence_id=evidence_id,
        evidence_type="authorization_log",
        source=source,
        description=(
            f"{description} — decision_id={decision.decision_id}, decision={decision.decision}, "
            f"reason_code={decision.reason_code}"
        ),
        reference=f"authorization_decisions.decision_id = '{decision.decision_id}'",
        assessment_id=REFERENCE_ASSESSMENT_ID,
        target_product="labs/agent-lab",
        target_version="v0.6.1",
        classification=EvidenceClassification.RELEASE_APPROVED,
    )


def _current_git_commit() -> str | None:
    """Best-effort real git commit hash for `labs/agent-lab` at the moment
    this dry run is built — never fabricated; returns `None` (not a guess)
    if git isn't available or this isn't a git checkout."""
    import subprocess

    try:
        result = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=Path(__file__).resolve().parent,
            capture_output=True,
            text=True,
            timeout=5,
            check=False,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    commit = result.stdout.strip()
    return commit if result.returncode == 0 and commit else None


def build_reference_engagement(db_path: str | Path) -> Engagement:
    """Drive real Agent Lab primitives end to end and return a populated
    `Engagement`. `db_path` should be a fresh, disposable SQLite path
    (tests pass a pytest `tmp_path`; the standalone script below uses a
    throwaway temp file)."""
    conn = db.connect(db_path)
    db.seed(conn)
    human = REFERENCE_HUMAN

    # ── Setup: two agents, a signing key, a root delegation, and one
    # chained (narrower) delegation — real registrations, real signatures.
    # ── Determinism scope: patch the process-wide random-ID source
    # (uuid.uuid4) and wall clock (time.time) for exactly this block —
    # every frozen primitive below (agents.py, delegation.py, chain.py,
    # authorization.py, approval.py, agent.py) still runs its own real,
    # unmodified code path (real grant validation, real Ed25519 signing,
    # real attenuation checks, real hash chaining); only the entropy
    # source they read from is swapped for a deterministic, obviously-
    # synthetic sequence, matching this test suite's existing monkeypatch
    # style (see test_delegation_chain.py). This makes the checked-in
    # reference fixture reproducible byte-for-byte across runs without
    # touching any frozen file or changing how production IDs are minted.
    with (
        patch("uuid.uuid4", side_effect=_deterministic_uuid_factory()),
        patch("time.time", return_value=REFERENCE_FIXED_TIME),
    ):
        agent_a = agents.register_agent(conn, human, name="reference-agent-a", version="0.1.0")
        agent_b = agents.register_agent(conn, human, name="reference-agent-b", version="0.1.0")
        key_a, priv_a = signing_keys.create_signing_key(conn, human, agent_a.agent_id)

        # Deliberately over-broad relative to the stated task need in
        # WORKFLOW_TEXT (get_customer on cust_1 only) — this is AD-02's evidence.
        root_grant = delegation.grant_delegation(
            conn,
            human,
            agent_id=agent_a.agent_id,
            allowed_tools=frozenset({"get_customer", "get_order", "add_case_note"}),
            resource_constraints={"customer_ids": {"cust_1", "cust_2"}},
            ttl_seconds=3600,
        )

        chained_grant = delegation.grant_chained_delegation(
            conn,
            human,
            issuer_agent_id=agent_a.agent_id,
            issuer_private_key_pem=priv_a,
            issuer_signing_key_id=key_a.key_id,
            parent_delegation_id=root_grant.delegation_id,
            subject_agent_id=agent_b.agent_id,
            allowed_tools=frozenset({"get_customer"}),
            resource_constraints={"customer_ids": {"cust_1"}},
            ttl_seconds=1800,
        )

        instance_a = AgentInstance(instance_id="ref-instance-a", started_at=0.0, agent_id=agent_a.agent_id, human_user_id=human.user_id)
        instance_b = AgentInstance(instance_id="ref-instance-b", started_at=0.0, agent_id=agent_b.agent_id, human_user_id=human.user_id)

        # ── AD-03 evidence: 2-hop chain, in-scope request, real signature +
        # attenuation verification.
        chain_allow = chain.verify_delegation_chain(
            conn,
            principal=human,
            agent_id=agent_b.agent_id,
            instance=instance_b,
            task_id="",
            delegation_id=chained_grant.delegation_id,
            tool_name="get_customer",
            resource_type="customer_ids",
            resource_id="cust_1",
        )
        assert chain_allow.allowed, "reference dry run setup is broken: expected the 2-hop chain to verify ALLOW"

        # ── AD-04 evidence: the same in-scope request driven through the
        # *real* AgentRunner loop (not just chain.verify_delegation_chain in
        # isolation), with a spy on tools.TOOL_FUNCTIONS proving the tool
        # actually ran exactly once — the same rigor test_enforcement_boundary.py
        # already holds itself to, applied here so this dry run's own
        # "authorized before execution" claim is demonstrated, not borrowed.
        #
        # ── AD-05 evidence: the same real-loop technique for the negative
        # case — an out-of-scope request (add_case_note, never granted to
        # agent_b) driven through the real loop, proving zero execution, not
        # just a DENY return value.
        import asyncio

        trace_dir = Path(db_path).parent / "reference_dryrun_traces"

        get_customer_calls: list[Any] = []
        original_get_customer = tools_module.TOOL_FUNCTIONS["get_customer"]

        def _spy_get_customer(*args, **kwargs):
            get_customer_calls.append(1)
            return original_get_customer(*args, **kwargs)

        add_case_note_calls: list[Any] = []
        original_add_case_note = tools_module.TOOL_FUNCTIONS["add_case_note"]

        def _spy_add_case_note(*args, **kwargs):
            add_case_note_calls.append(1)
            return original_add_case_note(*args, **kwargs)

        tools_module.TOOL_FUNCTIONS["get_customer"] = _spy_get_customer
        tools_module.TOOL_FUNCTIONS["add_case_note"] = _spy_add_case_note
        try:
            allow_runner = AgentRunner(
                agent_id=agent_a.agent_id,
                delegation_id=root_grant.delegation_id,
                model=FakeModelProvider(
                    script=[
                        ModelResponse(tool_call=ToolCallRequest(id="ref-1", name="get_customer", arguments={"customer_id": "cust_1"})),
                        ModelResponse(text="Reference customer looked up."),
                    ]
                ),
                db_conn=conn,
                trace_dir=trace_dir,
            )
            allow_result = asyncio.run(allow_runner.run(human=human, goal="look up cust_1"))

            bypass_runner = AgentRunner(
                agent_id=agent_b.agent_id,
                delegation_id=chained_grant.delegation_id,
                model=FakeModelProvider(
                    script=[
                        ModelResponse(
                            tool_call=ToolCallRequest(
                                id="ref-2", name="add_case_note", arguments={"customer_id": "cust_1", "note": "bypass attempt"}
                            )
                        ),
                        ModelResponse(text="unreachable if enforcement holds"),
                    ]
                ),
                db_conn=conn,
                trace_dir=trace_dir,
            )
            bypass_result = asyncio.run(bypass_runner.run(human=human, goal="attempt an out-of-scope write"))
        finally:
            tools_module.TOOL_FUNCTIONS["get_customer"] = original_get_customer
            tools_module.TOOL_FUNCTIONS["add_case_note"] = original_add_case_note

        assert allow_result.stopped_reason == "final_response"
        assert len(get_customer_calls) == 1, "expected get_customer to execute exactly once"
        leaf_decisions = authorization.get_decisions_for_instance(conn, allow_result.instance_id)
        leaf_allow_decision = next(d for d in leaf_decisions if d.tool_name == "get_customer" and d.allowed)

        assert len(add_case_note_calls) == 0, "expected add_case_note to never execute — the bypass attempt must be denied first"
        bypass_decisions = authorization.get_decisions_for_instance(conn, bypass_result.instance_id)
        bypass_deny_decision = next(d for d in bypass_decisions if d.tool_name == "add_case_note" and not d.allowed)

        # ── AD-02 evidence: the over-broad grant is itself technically
        # authorized against cust_2 (never intended by the stated task) —
        # proving enforcement did exactly what it was told, which is the
        # point: the gap is in the *grant*, not the *enforcement*.
        over_broad_allow = chain.verify_delegation_chain(
            conn,
            principal=human,
            agent_id=agent_a.agent_id,
            instance=instance_a,
            task_id="",
            delegation_id=root_grant.delegation_id,
            tool_name="get_order",
            resource_type="customer_ids",
            resource_id="cust_2",
        )
        assert over_broad_allow.allowed

        # ── AD-06 evidence: approval one-shot consumption + replay denial.
        note_args = {"customer_id": "cust_1", "note": "reference dry run"}
        appr = approval_module.request_approval(
            conn,
            human,
            agent_id=agent_a.agent_id,
            delegation_id=root_grant.delegation_id,
            tool_name="add_case_note",
            resource_type="customer_ids",
            resource_id="cust_1",
            proposed_arguments=note_args,
            ttl_seconds=300,
        )
        from ..webauthn import FakeWebAuthnVerifier

        approved = approval_module.approve(conn, human, appr.approval_id, verifier=FakeWebAuthnVerifier())
        first_consume = approval_module.verify_and_consume_approval(
            conn,
            approval_id=approved.approval_id,
            principal=human,
            agent_id=agent_a.agent_id,
            instance_id=instance_a.instance_id,
            task_id="",
            delegation_id=root_grant.delegation_id,
            tool_name="add_case_note",
            resource_type="customer_ids",
            resource_id="cust_1",
            validated_arguments=note_args,
        )
        assert first_consume.allowed
        replay_attempt = approval_module.verify_and_consume_approval(
            conn,
            approval_id=approved.approval_id,
            principal=human,
            agent_id=agent_a.agent_id,
            instance_id=instance_a.instance_id,
            task_id="",
            delegation_id=root_grant.delegation_id,
            tool_name="add_case_note",
            resource_type="customer_ids",
            resource_id="cust_1",
            validated_arguments=note_args,
        )
        assert not replay_attempt.allowed

    # ── Build the evidence manifest from the real results above. ────────
    evidence = EvidenceManifest()
    evidence.add(
        EvidenceReference(
            evidence_id="E-001",
            evidence_type="config_excerpt",
            source="labs/agent-lab (delegation.grant_delegation call in this dry run)",
            description=(
                f"Root delegation {root_grant.delegation_id} granted to reference-agent-a: "
                f"allowed_tools={sorted(root_grant.allowed_tools)}, "
                f"resource_constraints={{'customer_ids': {sorted(root_grant.resource_constraints['customer_ids'])}}}"
            ),
            reference=f"delegations.delegation_id = '{root_grant.delegation_id}'",
            assessment_id=REFERENCE_ASSESSMENT_ID,
            target_product="labs/agent-lab",
            target_version="v0.6.1",
            classification=EvidenceClassification.RELEASE_APPROVED,
        )
    )
    evidence.add(
        EvidenceReference(
            evidence_id="E-002",
            evidence_type="config_excerpt",
            source="assessment-engagement/reference-agentlab-dryrun/workflow.md",
            description="Stated task need for this workflow: get_customer on cust_1 only (plus approved add_case_note).",
            assessment_id=REFERENCE_ASSESSMENT_ID,
            target_product="labs/agent-lab",
            target_version="v0.6.1",
            classification=EvidenceClassification.RELEASE_APPROVED,
        )
    )
    evidence.add(_authz_decision_evidence(
        "E-003", over_broad_allow.decision, source="chain.verify_delegation_chain (this dry run)",
        description="Root grant technically authorizes get_order on cust_2, outside the stated task need",
    ))
    evidence.add(
        EvidenceReference(
            evidence_id="E-004",
            evidence_type="execution_trace",
            source="chain.verify_delegation_chain (this dry run)",
            description=(
                f"2-hop chain verification, delegation {chained_grant.delegation_id}: allowed={chain_allow.allowed}, "
                f"depth={chain_allow.delegation_depth}, authority_path={chain_allow.authority_path}, "
                f"chain_hash={chain_allow.chain_hash}"
            ),
            reference=f"delegations.delegation_id = '{chained_grant.delegation_id}'",
            assessment_id=REFERENCE_ASSESSMENT_ID,
            target_product="labs/agent-lab",
            target_version="v0.6.1",
            classification=EvidenceClassification.RELEASE_APPROVED,
        )
    )
    evidence.add(_authz_decision_evidence(
        "E-005", leaf_allow_decision, source=f"AgentRunner.run() (this dry run), instance_id={allow_result.instance_id}",
        description=(
            f"In-scope request (get_customer/cust_1) driven through the real agent loop: ALLOW, and "
            f"tools.TOOL_FUNCTIONS['get_customer'] was called exactly {len(get_customer_calls)} time(s) "
            "(spy-instrumented for this run)"
        ),
    ))
    evidence.add(_authz_decision_evidence(
        "E-006", bypass_deny_decision, source=f"AgentRunner.run() (this dry run), instance_id={bypass_result.instance_id}",
        description=(
            "Out-of-scope request (add_case_note, never granted to agent-b) driven through the real agent "
            f"loop: DENY, and tools.TOOL_FUNCTIONS['add_case_note'] was called {len(add_case_note_calls)} "
            "time(s) (spy-instrumented for this run — zero confirms no side effect occurred)"
        ),
    ))
    evidence.add(
        EvidenceReference(
            evidence_id="E-007",
            evidence_type="approval_record",
            source="approval.py (this dry run)",
            description=(
                f"Approval {approved.approval_id}: first consumption allowed "
                f"(decision_id={first_consume.decision_id}), replay attempt denied "
                f"(decision_id={replay_attempt.decision_id}, reason_code={replay_attempt.reason_code})"
            ),
            reference=f"approvals.approval_id = '{approved.approval_id}'",
            assessment_id=REFERENCE_ASSESSMENT_ID,
            target_product="labs/agent-lab",
            target_version="v0.6.1",
            classification=EvidenceClassification.RELEASE_APPROVED,
        )
    )
    evidence.add(
        EvidenceReference(
            evidence_id="E-008",
            evidence_type="execution_trace",
            source="authorization_decisions table (this dry run)",
            description=(
                "Every decision cited above (E-003, E-005, E-006, E-007) is a row in the same "
                "append-only authorization_decisions ledger, queryable by decision_id — this evidence "
                "entry demonstrates that traceability itself, not a separate control."
            ),
            assessment_id=REFERENCE_ASSESSMENT_ID,
            target_product="labs/agent-lab",
            target_version="v0.6.1",
            classification=EvidenceClassification.RELEASE_APPROVED,
        )
    )

    findings = (
        Finding(
            finding_id="AD-01",
            area=AssessmentArea.IDENTITY_OWNERSHIP,
            title="Agent identity provisioning/offboarding process",
            status=Outcome.UNVERIFIABLE,
            observation=(
                "This synthetic engagement has no external identity/HR/IT system of record — agents "
                "were registered directly by this dry run's own script, not through a reviewable "
                "provisioning process."
            ),
            expected_control="A documented, auditable process governs who may register an agent, on whose authority, and how it is deprovisioned.",
            evidence_ids=(),
            limitations=(
                "UNVERIFIABLE, not UNREPRESENTABLE: the concept of a provisioning process is entirely "
                "representable (Agent Lab's own `agents.register_agent` call is one), this synthetic "
                "engagement simply has no external provisioning/offboarding documentation or process "
                "interview to inspect — an evidence-access gap, not a semantic one."
            ),
            property_family="identity_provisioning",
            tested_layer="process/documentation",
        ),
        Finding(
            finding_id="AD-02",
            area=AssessmentArea.DELEGATION_AUTHORITY_FLOW,
            title="Delegation grant exceeds the stated task's need",
            status=Outcome.VIOLATED,
            severity=Severity.MEDIUM,
            observation=(
                "The root delegation grants get_customer, get_order, and add_case_note against both "
                "cust_1 and cust_2, while the documented workflow task (workflow.md) only requires "
                "get_customer (plus approved add_case_note) against cust_1. E-003 shows the excess scope "
                "is technically usable (get_order on cust_2 is ALLOWed), confirming this is a real, "
                "exploitable over-grant, not a theoretical one."
            ),
            expected_control="Delegated authority should be scoped to the minimum tools and resources the stated task actually requires (least privilege at grant time).",
            evidence_ids=("E-001", "E-002", "E-003"),
            risk_impact=(
                "An agent with more standing authority than its task needs increases the blast radius of "
                "a compromised or misbehaving agent instance, and of a manipulated model output that "
                "requests an in-scope-but-unintended action — enforcement will correctly ALLOW it, "
                "because it was, in fact, granted. Gate 7 (security-claim/trust-boundary triage): this "
                "workflow's own stated scope-of-work (workflow.md) is the claimed least-privilege "
                "boundary, and E-003 shows it was genuinely crossed — the excess grant is usable, not "
                "merely present in configuration."
            ),
            recommendation="Scope each delegation's allowed_tools and resource_constraints to the specific task being authorized, not a broader standing grant convenient for multiple future tasks.",
            retest_condition="Re-verify after the grant is narrowed to get_customer on cust_1 only (plus the approved add_case_note) — get_order on cust_2 should then DENY.",
            confidence="High — directly observed in the delegation's own persisted grant.",
            property_family="least_privilege_grant",
            tested_layer="delegation_grant",
            invariant="A delegation's allowed_tools/resource_constraints are a subset of the stated task's actual need.",
            evidence_gate_status=_FULLY_SATISFIED_GATES,
        ),
        Finding(
            finding_id="AD-03",
            area=AssessmentArea.SCOPE_ATTENUATION,
            title="Multi-hop delegation narrows correctly and is cryptographically verifiable",
            status=Outcome.ENFORCED,
            observation=(
                "A second-hop delegation (agent-a -> agent-b) narrows the root grant to a single tool "
                "and a single resource id, is Ed25519-signed by agent-a's registered key, and verifies "
                "as a 2-hop chain with a full, reconstructable authority path back to the human operator. "
                "Gate 7: this directly satisfies the claimed boundary that a child delegation may only "
                "narrow, never widen, its parent's grant (chain.verify_attenuation's subset rule)."
            ),
            expected_control="Authority delegated across multiple hops must only ever narrow, never widen, and each hop must be independently, cryptographically attributable.",
            evidence_ids=("E-004",),
            recommendation="Where a workflow delegates authority across more than one agent, verify — not merely document — that each hop narrows the prior one and is independently attributable.",
            property_family="scope_attenuation",
            tested_layer="delegation_chain",
            invariant="child.allowed_tools <= parent.allowed_tools and child's resource id sets are subsets of the parent's, for every hop.",
            evidence_gate_status=_FULLY_SATISFIED_GATES,
        ),
        Finding(
            finding_id="AD-04",
            area=AssessmentArea.PRE_EXECUTION_BOUNDARY,
            title="In-scope request authorized, then executed exactly once",
            status=Outcome.ENFORCED,
            observation=(
                "A request within the delegation's granted scope (get_customer/cust_1) was driven through "
                "the real agent execution loop (AgentRunner.run), not verified in isolation — a spy on the "
                "underlying tool function confirms it ran exactly once, and only after a persisted ALLOW "
                "decision was recorded. Gate 7: the claimed boundary (authorization must gate execution, "
                "not follow it) is the exact thing exercised and confirmed here, not an incidental "
                "observation."
            ),
            expected_control="Authorization must be evaluated, and must return ALLOW, before a protected action's side effect occurs — never inferred after the fact from a result.",
            evidence_ids=("E-005",),
            recommendation="Confirm the customer's own enforcement point sits before the tool/action executes, not as a post-hoc audit check.",
            property_family="pre_execution_enforcement",
            tested_layer="agent_execution_loop",
            control_or_calibration="Paired with AD-05's denied, zero-side-effect negative case driven through the same real loop.",
            evidence_gate_status=_FULLY_SATISFIED_GATES,
        ),
        Finding(
            finding_id="AD-05",
            area=AssessmentArea.AUTHORIZATION_BYPASS,
            title="Out-of-scope request denied pre-execution, zero side effects",
            status=Outcome.ENFORCED,
            observation=(
                "A request for a tool never granted to the acting agent (add_case_note, dropped at the "
                "chained hop) was driven through the real agent execution loop and denied with a specific "
                "reason code — a spy on the underlying tool function confirms it was never called. Gate 7: "
                "this is the negative case for AD-04's claimed boundary — the same boundary, confirmed from "
                "the other direction."
            ),
            expected_control="A request outside the effective delegated scope must be denied, not silently narrowed or ignored, and the denial must be evidenced.",
            evidence_ids=("E-006",),
            recommendation="Verify bypass attempts against the customer's actual enforcement point produce a denial with an auditable reason, not just an application-level error the caller could suppress.",
            property_family="pre_execution_enforcement",
            tested_layer="agent_execution_loop",
            control_or_calibration="Paired with AD-04's allowed, exactly-once positive case driven through the same real loop.",
            evidence_gate_status=_FULLY_SATISFIED_GATES,
        ),
        Finding(
            finding_id="AD-06",
            area=AssessmentArea.APPROVAL_BINDING,
            title="Approval is one-shot and replay-resistant",
            status=Outcome.ENFORCED,
            observation=(
                "An approval tied to one exact tool call is consumed successfully once; a second "
                "consumption attempt with identical arguments is denied (APPROVAL_ALREADY_CONSUMED), both "
                "evidenced. Gate 7: the claimed boundary (an approval authorizes exactly one execution) is "
                "exercised directly, with its own within-finding negative case (the replay attempt)."
            ),
            expected_control="A human approval must bind to the exact action approved and be usable at most once — replaying it must not authorize a second execution.",
            evidence_ids=("E-007",),
            limitations="The underlying WebAuthn ceremony is a documented Agent-Lab-only simulation (FakeWebAuthnVerifier) — this evidences the binding/consumption protocol, not a live authenticator.",
            recommendation="Confirm the customer's own approval mechanism binds to exact action arguments (not just tool name) and enforces single-use, not just a UI-level disabled button.",
            property_family="approval_binding",
            tested_layer="approval_consumption",
            invariant="An Approval's (approved -> consumed) transition is atomic and one-way; a second verify_and_consume_approval on the same approval_id denies.",
            control_or_calibration="The replay attempt (second consumption, same approval_id) is this finding's own negative case, run immediately after the successful first consumption.",
            evidence_gate_status=_FULLY_SATISFIED_GATES,
        ),
        Finding(
            finding_id="AD-07",
            area=AssessmentArea.REVOCATION_EXPIRY,
            title="Live revocation-propagation timing",
            status=Outcome.NOT_APPLICABLE,
            observation="This synthetic engagement has no live/production traffic against which to measure the delay between a revocation action and enforcement taking effect.",
            expected_control="Revoking a delegation or signing key should take effect on the next authorization check, with no meaningful propagation delay.",
            evidence_ids=(),
            limitations="Applicable to a live/staging engagement with real traffic; not to a one-shot synthetic dry run.",
            property_family="revocation_propagation",
            tested_layer="n/a — no live traffic in this engagement",
        ),
        Finding(
            finding_id="AD-08",
            area=AssessmentArea.EVIDENCE_ACCOUNTABILITY,
            title="Every authorization decision above is independently traceable",
            status=Outcome.ENFORCED,
            observation=(
                "AD-02 through AD-06's evidence all resolve to real rows in one append-only "
                "authorization_decisions ledger, addressable by decision_id, without needing to "
                "reconstruct anything from partial logs. Gate 7: the claimed boundary (every "
                "consequential decision must be reconstructible from durable records alone) is "
                "demonstrated by actually doing the reconstruction, not merely asserting the ledger "
                "exists."
            ),
            expected_control="Every consequential authorization decision must be reconstructible after the fact — who, what, when, and the outcome — from durable evidence, not inferred.",
            evidence_ids=("E-008",),
            recommendation="Confirm the customer's own system persists an equivalent durable, queryable decision ledger — not only human-readable logs that expire or aren't decision-scoped.",
            property_family="evidence_accountability",
            tested_layer="authorization_decisions_ledger",
            evidence_gate_status=_FULLY_SATISFIED_GATES,
        ),
    )

    engagement = Engagement(
        title="Crypton Agent Authorization & Delegation Assessment — Reference Dry Run",
        label="Crypton Reference Agent Lab — Methodology Dry Run",
        date="2026-09-02",
        assessor="Crypton (self-assessment, synthetic engagement)",
        scope=SCOPE_TEXT,
        workflow=WORKFLOW_TEXT,
        limitations=LIMITATIONS_TEXT,
        executive_summary=EXECUTIVE_SUMMARY_TEXT,
        findings=findings,
        evidence=evidence,
        assessment_id=REFERENCE_ASSESSMENT_ID,
        client_identifier="Crypton (self-assessment)",
        system_identifier="labs/agent-lab reference implementation",
        assessment_type="agent_authorization_delegation_assessment",
        tested_workflow="reference-agent tool-call workflow (get_customer / get_order / add_case_note)",
        tested_layers=("delegation", "authorization", "approval", "chain_verification", "tracing"),
        environment="Agent Lab isolated SQLite (disposable, non-production — see DECISIONS.md invariant 10)",
        target_product="labs/agent-lab",
        target_version="v0.6.1",
        target_commit=_current_git_commit(),
        testing_window=None,  # one scripted, single-session dry run — no multi-day window concept applies
        configuration="Default Agent Lab config; FakeWebAuthnVerifier (documented simulation, see limitations); deterministic uuid4/time patch scoped to this script only (see module docstring).",
        excluded_scope="Gateway enforcement, real WebAuthn wiring, production key custody, MCP — none exist yet (see DECISIONS.md v0.6 'what v0.6 deliberately did not build').",
    )
    # Everything needed is already extracted into plain Python objects
    # above — close explicitly rather than relying on GC, since a caller
    # (main() below) may want to delete the underlying db_path immediately
    # after this returns (SQLite holds an OS-level file lock on Windows
    # until the connection is closed).
    conn.close()
    return engagement


def main() -> None:
    """Standalone entry point: `python -m agentlab.assessment.reference_dryrun`.
    Builds the engagement against a throwaway temp database, writes it to
    `assessment-engagement/reference-agentlab-dryrun/`, and renders
    `report.md` alongside it."""
    import tempfile

    from .report import render_report

    repo_root = Path(__file__).resolve().parents[5]
    out_dir = repo_root / "assessment-engagement" / "reference-agentlab-dryrun"

    with tempfile.TemporaryDirectory() as tmp:
        db_path = Path(tmp) / "reference_dryrun.db"
        engagement = build_reference_engagement(db_path)

    engagement.save(out_dir)
    report_text = render_report(engagement)
    (out_dir / "report.md").write_text(report_text, encoding="utf-8")
    print(f"Wrote engagement + report.md to {out_dir}")


if __name__ == "__main__":
    main()
