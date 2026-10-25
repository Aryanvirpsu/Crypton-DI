"""
Crypton Authority Model v0.1 — fixture normalization code.

This is the *only* place this module reads a real, already-captured
evidence record and turns it into Ledger/Envelope/Diff objects. Per spec
§21, this is deliberately NOT an adapter framework — three plain
functions, one per fixture, each reading exactly the evidence it needs and
nothing more. If a fourth target later needs the same normalization
pattern, generalize then; building that abstraction speculatively now,
before a second real need exists, is exactly the kind of platform
infrastructure this pass was told not to build.

Every fact below traces to a specific, cited evidence item. Where a
fixture's own cited evidence does not name a principal, approver, or
action identity, this code leaves it `Fact.unknown()`/`Fact.unrepresentable()`/
`Fact.unverifiable()` — it never reaches into a *different* evidence item
in the same corpus to fill the gap, even when the answer happens to be
knowable elsewhere in the repository. A finding's Authority Model is
scoped to the evidence that finding itself cites.
"""

from __future__ import annotations

import ast
import json
import re
from pathlib import Path
from typing import Any, Callable

from ..models import EvidenceClassification, Finding
from .diff import AuthorityDiff, compute_diff
from .enums import ActorType, EventType, Provenance
from .envelope import AuthorityEnvelope, derive_envelope
from .fact import Fact
from .ledger import AuthorityLedger, LedgerEvent
from .primitives import Action, Actor, Approval, Effect, Grant, Principal


def _repo_root() -> Path:
    # labs/agent-lab/src/agentlab/assessment/authority_model/fixtures.py -> Crypton-DI
    return Path(__file__).resolve().parents[6]


# ═══════════════════════════════════════════════════════════════════════
# Fixture 1 — Agent Lab's own reference dry run, AD-02 (VIOLATED, a real
# divergence: the issued grant exceeds the workflow's own stated need).
# Source: assessment-engagement/reference-agentlab-dryrun/{findings,evidence_manifest}.json
# — read as data, not re-run.
# ═══════════════════════════════════════════════════════════════════════


def build_agentlab_ad02_fixture() -> tuple[AuthorityLedger, AuthorityEnvelope, AuthorityDiff]:
    engagement_dir = _repo_root() / "assessment-engagement" / "reference-agentlab-dryrun"
    findings = json.loads((engagement_dir / "findings.json").read_text(encoding="utf-8"))
    evidence = {e["evidence_id"]: e for e in json.loads((engagement_dir / "evidence_manifest.json").read_text(encoding="utf-8"))}

    finding = next(f for f in findings if f["finding_id"] == "AD-02")
    assert finding["evidence_ids"] == ["E-001", "E-002", "E-003"], "AD-02's cited evidence changed — update this fixture"
    e001, e002, e003 = evidence["E-001"], evidence["E-002"], evidence["E-003"]

    # ── Parse E-001 (the real, persisted root delegation) ───────────────
    m = re.search(r"Root delegation (\S+) granted to (\S+): allowed_tools=(\[[^\]]*\]), resource_constraints=(\{.*\})$", e001["description"])
    assert m, f"E-001 description format changed: {e001['description']!r}"
    actual_grant_id, granted_to_name, allowed_tools_literal, resource_constraints_literal = m.groups()
    actual_allowed_tools = tuple(sorted(ast.literal_eval(allowed_tools_literal)))
    resource_constraints: dict[str, list[str]] = ast.literal_eval(resource_constraints_literal)
    actual_allowed_resources = tuple(sorted(f"{rtype}:{rid}" for rtype, ids in resource_constraints.items() for rid in ids))

    # ── Parse E-003 (the real authorization decision that confirms the
    # excess scope is actually usable) ───────────────────────────────────
    m = re.search(r"authorizes (\w+) on (\w+).*decision_id=(\S+?), decision=(\w+), reason_code=(\w+)", e003["description"])
    assert m, f"E-003 description format changed: {e003['description']!r}"
    over_broad_tool, over_broad_resource_id, decision_id, decision, reason_code = m.groups()

    ledger = AuthorityLedger(assessment_id=finding.get("assessment_id") or "ENG-REFERENCE-AGENTLAB-DRYRUN")

    # Principal and approver: AD-02's own cited evidence (E-001/E-002/E-003)
    # never names the human principal who issued the grant — left unknown,
    # not borrowed from a different finding's evidence (e.g. E-004) in the
    # same engagement.
    principal_key = "ad02-principal"
    ledger.principals[principal_key] = Principal(
        principal_type="human",
        principal_id=Fact.unknown("Not named in AD-02's own cited evidence (E-001/E-002/E-003)."),
        evidence_ids=("E-001",),
        provenance=Provenance.OBSERVED,
        classification=EvidenceClassification(e001.get("classification")) if e001.get("classification") else None,
    )

    actor_key = "ad02-actor-agent-a"
    ledger.actors[actor_key] = Actor(
        actor_type=ActorType.AGENT,
        actor_id=Fact.observed(granted_to_name, note="registered name from E-001's evidence text, not Agent Lab's internal agent_id"),
        acts_for_principal=Fact.unknown("E-001's evidence text does not name the granting principal."),
        evidence_ids=("E-001",),
        provenance=Provenance.OBSERVED,
    )

    grant_key = "ad02-grant-actual"
    ledger.grants[grant_key] = Grant(
        grant_id=Fact.observed(actual_grant_id),
        granted_by=Fact.unknown("Not named in E-001's evidence text."),
        granted_to=Fact.observed(granted_to_name),
        allowed_tools=actual_allowed_tools,
        allowed_resources=actual_allowed_resources,
        evidence_ids=("E-001",),
        provenance=Provenance.OBSERVED,
        classification=EvidenceClassification(e001.get("classification")) if e001.get("classification") else None,
    )
    ledger.add_event(
        LedgerEvent(
            event_id="ad02-evt-1-grant-created",
            event_type=EventType.GRANT_CREATED,
            assessment_id=ledger.assessment_id,
            test_id="AD-02",
            sequence=1,
            principal_id=principal_key,
            actor_id=actor_key,
            grant_id=grant_key,
            evidence_ids=("E-001",),
            provenance=Provenance.OBSERVED,
        )
    )

    action_key = "ad02-action-get-order-cust2"
    ledger.actions[action_key] = Action(
        action_id=Fact.observed(decision_id, note="this is the authorization decision's own decision_id — no separate tool-call/action id was minted, because this evidence is a direct chain.verify_delegation_chain check, not an AgentRunner execution"),
        actor_id=Fact.observed(granted_to_name),
        tool=over_broad_tool,
        intended_resource=f"customer_ids:{over_broad_resource_id}",
        actual_resource=f"customer_ids:{over_broad_resource_id}",
        state="authorized" if decision == "ALLOW" else "denied",
        evidence_ids=("E-003",),
        provenance=Provenance.OBSERVED,
    )
    ledger.add_event(
        LedgerEvent(
            event_id="ad02-evt-2-action-attempted",
            event_type=EventType.ACTION_ATTEMPTED,
            assessment_id=ledger.assessment_id,
            test_id="AD-02",
            sequence=2,
            actor_id=actor_key,
            grant_id=grant_key,
            action_id=action_key,
            after_state=f"decision={decision}, reason_code={reason_code}",
            evidence_ids=("E-003",),
            provenance=Provenance.OBSERVED,
        )
    )

    # ── Expected envelope: built directly from E-002's stated task need,
    # NOT via derive_envelope's grant-chain walk — Agent Lab's schema has
    # no separate "intended-grant" record distinct from the one actually
    # issued (E-001); the stated need exists only as prose (workflow.md,
    # restated in E-002), so this one envelope is hand-assembled from that
    # evidence item rather than derived from a Grant ledger entry. This is
    # a deliberate, noted exception to the usual derive_envelope path, not
    # a silent inconsistency.
    envelope = AuthorityEnvelope(
        envelope_id="ad02-expected-envelope",
        assessment_id=ledger.assessment_id,
        test_id="AD-02",
        principal_id=principal_key,
        actor_id=actor_key,
        allowed_tools=("add_case_note", "get_customer"),
        allowed_resources=("customer_ids:cust_1",),
        revocation_state="ACTIVE",
        source_evidence_ids=("E-002",),
        derivation_notes=(
            "Built directly from E-002's stated task need ('get_customer on cust_1 only, plus approved "
            "add_case_note') — not derived via the grant_chain walk, because Agent Lab's schema has no "
            "separate 'intended-grant' record distinct from the grant actually issued (E-001/ad02-grant-actual, "
            "whose allowed_tools/resources are the broader, observed side of this diff, not the expected side)."
        ),
    )

    diff = compute_diff(ledger, envelope, diff_id="ad02-diff", observed_action_ids=[action_key])

    return ledger, envelope, diff


# ═══════════════════════════════════════════════════════════════════════
# Fixture 2 — Mastra Candidate 4a (external-framework violation case).
# Source: semantic-rulebook/experiments/004-candidate-4a-evidence/candidate-4a.json
# (sibling repo) — the real, machine-readable evidence captured from the
# existing, unmodified harness. Read as data; the harness is not re-run
# here.
# ═══════════════════════════════════════════════════════════════════════


class MastraEvidenceUnavailable(RuntimeError):
    """Raised when the sibling semantic-rulebook checkout (or its
    Candidate 4a evidence capture) is not present — e.g. a checkout of
    Crypton-DI alone, without the adjacent research repo."""


def _mastra_evidence_path() -> Path:
    return (
        _repo_root().parent
        / "semantic-rulebook"
        / "experiments"
        / "004-candidate-4a-evidence"
        / "candidate-4a.json"
    )


def build_mastra_candidate_4a_fixture() -> tuple[AuthorityLedger, AuthorityEnvelope, AuthorityDiff]:
    path = _mastra_evidence_path()
    if not path.exists():
        raise MastraEvidenceUnavailable(
            f"{path} not found — this fixture needs the sibling semantic-rulebook checkout's captured "
            "Candidate 4a evidence (see labs/exp-003-openai-agents-sdk/ for a same-repo alternative fixture)."
        )
    runs: list[dict[str, Any]] = json.loads(path.read_text(encoding="utf-8"))
    run = runs[0]  # this fixture uses run 1; the other 4 runs' agreement is preserved via run 1's own reproduction_record text, not re-ingested as separate ledgers (spec's no-overengineering rule)

    ledger = AuthorityLedger(assessment_id=None)  # semantic-rulebook evidence predates/is outside Crypton's engagement.json scheme — no assessment_id exists to cite
    test_id = run["test_id"]
    run_id = run["run_id"]
    evidence_ref = f"semantic-rulebook/experiments/004-candidate-4a-evidence/candidate-4a.json#run={run_id}"

    classification = EvidenceClassification(run["classification"])

    principal_key = "c4a-principal"
    ledger.principals[principal_key] = Principal(
        principal_type=None,
        principal_id=Fact.unknown("No human/organization identity is captured anywhere in this evidence — the call is a scripted harness invocation, not an authenticated human session."),
        tenant_org=Fact.unknown("Not evidenced."),
        evidence_ids=(evidence_ref,),
        provenance=Provenance.OBSERVED,
        classification=classification,
    )

    actor_key = "c4a-actor-sub-agent"
    ledger.actors[actor_key] = Actor(
        actor_type=ActorType.SUB_AGENT,
        actor_id=Fact.unknown("preconditions describe a 'sub-agent' executing the transfer tool by role only — no concrete framework-internal actor id is captured in this evidence."),
        acts_for_principal=Fact.unknown("Not evidenced."),
        evidence_ids=(evidence_ref,),
        provenance=Provenance.OBSERVED,
        classification=classification,
    )

    approval_key = "c4a-approval"
    ledger.approvals[approval_key] = Approval(
        approval_id=Fact.unknown("approveToolCall({runId}) has no discrete approval-record id in the captured evidence — only the runId scoping the call is recorded."),
        requested_by=Fact.unknown("Not evidenced."),
        approved_by=Fact.unknown("Scripted harness call (no toolCallId supplied) — not an authenticated human approver."),
        target_action_ids=Fact.unrepresentable(
            "approveToolCall({runId}) with toolCallId omitted has no parameter capable of binding to one "
            "specific pending action — the API call shape itself cannot represent a single-action binding "
            "in this form, independent of what was or wasn't observed."
        ),
        state="granted",
        execution_context_id=run_id,
        evidence_ids=(evidence_ref,),
        provenance=Provenance.OBSERVED,
        classification=classification,
    )
    ledger.add_event(
        LedgerEvent(
            event_id="c4a-evt-1-approval-granted",
            event_type=EventType.APPROVAL_GRANTED,
            test_id=test_id,
            sequence=1,
            timestamp=run["timestamp"],
            principal_id=principal_key,
            actor_id=actor_key,
            approval_id=approval_key,
            evidence_ids=(evidence_ref,),
            provenance=Provenance.OBSERVED,
            classification=classification,
        )
    )

    effect_ledger = run["effect_or_sink_ledger"]
    action_keys: list[str] = []
    for i, entry in enumerate(effect_ledger, start=1):
        action_key = f"c4a-action-{i}"
        action_keys.append(action_key)
        ledger.actions[action_key] = Action(
            action_id=Fact.unverifiable(
                "The sink ledger recorded callId as the literal string 'unknown' for this entry — Mastra's "
                "own effect-ledger did not expose a resolvable call/action id for this capture."
            ),
            actor_id=Fact.unknown("Not evidenced."),
            tool=entry["tool"],
            arguments=Fact.observed(dict(entry["args"])),
            actual_resource=str(entry["args"].get("to")),
            state="executed",
            execution_context_id=run_id,
            evidence_ids=(evidence_ref,),
            provenance=Provenance.OBSERVED,
            classification=classification,
        )
        effect_key = f"c4a-effect-{i}"
        ledger.effects[effect_key] = Effect(
            effect_id=effect_key,
            action_id=Fact.unverifiable("Same limitation as the originating action — sink recorded callId='unknown'."),
            effect_type="transfer",
            target_resource=str(entry["args"].get("to")),
            after_state=dict(entry["args"]),
            observed_at=str(entry["ts"]),
            evidence_ids=(evidence_ref,),
            provenance=Provenance.OBSERVED,
            classification=classification,
        )
        ledger.add_event(
            LedgerEvent(
                event_id=f"c4a-evt-{i + 1}-action-executed",
                event_type=EventType.ACTION_EXECUTED,
                test_id=test_id,
                sequence=i + 1,
                timestamp=str(entry["ts"]),
                actor_id=actor_key,
                approval_id=approval_key,
                action_id=action_key,
                effect_id=effect_key,
                after_state=dict(entry["args"]),
                evidence_ids=(evidence_ref,),
                provenance=Provenance.OBSERVED,
                classification=classification,
            )
        )
        ledger.add_event(
            LedgerEvent(
                event_id=f"c4a-evt-{i + 1}-effect-observed",
                event_type=EventType.EFFECT_OBSERVED,
                test_id=test_id,
                sequence=i + 1,
                timestamp=str(entry["ts"]),
                action_id=action_key,
                effect_id=effect_key,
                after_state=dict(entry["args"]),
                evidence_ids=(evidence_ref,),
                provenance=Provenance.OBSERVED,
                classification=classification,
            )
        )

    envelope = derive_envelope(
        ledger,
        envelope_id="c4a-expected-envelope",
        test_id=test_id,
        approval_id=approval_key,
        principal_id=principal_key,
        actor_id=actor_key,
        execution_context_id=run_id,
    )
    # Augment derivation_notes with the run_id provenance caveat (free text,
    # not a structural field — see module docstring on why ExecutionContext's
    # plain identifiers aren't Fact-wrapped).
    envelope = AuthorityEnvelope(
        envelope_id=envelope.envelope_id,
        assessment_id=envelope.assessment_id,
        test_id=envelope.test_id,
        principal_id=envelope.principal_id,
        actor_id=envelope.actor_id,
        execution_context_id=envelope.execution_context_id,
        grant_chain=envelope.grant_chain,
        allowed_actions=envelope.allowed_actions,
        allowed_tools=envelope.allowed_tools,
        allowed_resources=envelope.allowed_resources,
        argument_constraints=envelope.argument_constraints,
        required_approval=envelope.required_approval,
        approval_id=envelope.approval_id,
        approval_scope=envelope.approval_scope,
        valid_from=envelope.valid_from,
        expires_at=envelope.expires_at,
        revocation_state=envelope.revocation_state,
        delegation_constraints=envelope.delegation_constraints,
        terminal_or_consumption_state=envelope.terminal_or_consumption_state,
        source_evidence_ids=envelope.source_evidence_ids,
        derivation_notes=(
            (envelope.derivation_notes or "")
            + " execution_context_id here is this evidence capture's own run_id field (a fresh UUID minted "
            "per harness invocation) — it is not independently confirmed to equal Mastra's internal workflow "
            "runId string that approveToolCall({runId}) actually consumes, which this evidence capture did "
            "not separately record."
        ).strip(),
    )

    diff = compute_diff(ledger, envelope, diff_id="c4a-diff", observed_action_ids=action_keys)

    return ledger, envelope, diff


# ═══════════════════════════════════════════════════════════════════════
# Fixture 3 — EXP-003A H1a (external-framework, no-divergence case):
# argument binding held — a post-approval mutation did not reach execution.
# Source: labs/exp-003-openai-agents-sdk/evidence/h1a_post_approval_mutation.json
# — read as data, not re-run.
# ═══════════════════════════════════════════════════════════════════════


def build_exp003a_h1a_fixture() -> tuple[AuthorityLedger, AuthorityEnvelope, AuthorityDiff]:
    path = _repo_root() / "labs" / "exp-003-openai-agents-sdk" / "evidence" / "h1a_post_approval_mutation.json"
    record: dict[str, Any] = json.loads(path.read_text(encoding="utf-8"))
    evidence_ref = "labs/exp-003-openai-agents-sdk/evidence/h1a_post_approval_mutation.json"

    ledger = AuthorityLedger(assessment_id=None)
    test_id = record["scenario"]
    call_id = record["approval_request"]["call_id"]
    execution = record["execution"][0]

    actor_key = "h1a-actor"
    ledger.actors[actor_key] = Actor(
        actor_type=ActorType.AGENT,
        actor_id=Fact.observed(execution["agent_name"]),
        evidence_ids=(evidence_ref,),
        provenance=Provenance.OBSERVED,
    )

    approval_key = "h1a-approval"
    ledger.approvals[approval_key] = Approval(
        approval_id=Fact.observed(call_id, note="the SDK's call_id is the only correlating key this evidence carries for the approval — no separate approval-record id exists in this framework's evidence shape"),
        requested_by=Fact.unknown("Not evidenced."),
        approved_by=Fact.unknown("approval_decision records decision='approve' but no approver identity — a scripted test decision, not an authenticated human approver."),
        target_action_ids=Fact.observed((call_id,)),
        approval_scope=(
            f"tool_name={record['approval_request']['tool_name']}; "
            f"arguments={record['approval_request']['arguments']} as originally proposed/presented"
        ),
        state="approved",
        evidence_ids=(evidence_ref,),
        provenance=Provenance.OBSERVED,
    )
    ledger.add_event(
        LedgerEvent(
            event_id="h1a-evt-1-approval-granted",
            event_type=EventType.APPROVAL_GRANTED,
            test_id=test_id,
            sequence=1,
            actor_id=actor_key,
            approval_id=approval_key,
            evidence_ids=(evidence_ref,),
            provenance=Provenance.OBSERVED,
        )
    )

    action_key = "h1a-action"
    ledger.actions[action_key] = Action(
        action_id=Fact.observed(execution["call_id"]),
        actor_id=Fact.observed(execution["agent_name"]),
        tool=execution["tool_name"],
        arguments=Fact.observed(dict(execution["arguments"])),
        actual_resource=str(execution["arguments"].get("destination")),
        state="executed",
        evidence_ids=(evidence_ref,),
        provenance=Provenance.OBSERVED,
    )
    effect_key = "h1a-effect"
    ledger.effects[effect_key] = Effect(
        effect_id=effect_key,
        action_id=Fact.observed(execution["call_id"]),
        effect_type="tool_invocation_recorded",
        target_resource=str(execution["arguments"].get("destination")),
        after_state=dict(execution["arguments"]),
        observed_at=str(execution["timestamp"]),
        evidence_ids=(evidence_ref,),
        provenance=Provenance.OBSERVED,
    )
    ledger.add_event(
        LedgerEvent(
            event_id="h1a-evt-2-action-executed",
            event_type=EventType.ACTION_EXECUTED,
            test_id=test_id,
            sequence=2,
            timestamp=str(execution["timestamp"]),
            actor_id=actor_key,
            approval_id=approval_key,
            action_id=action_key,
            effect_id=effect_key,
            after_state=dict(execution["arguments"]),
            evidence_ids=(evidence_ref,),
            provenance=Provenance.OBSERVED,
        )
    )
    ledger.add_event(
        LedgerEvent(
            event_id="h1a-evt-3-effect-observed",
            event_type=EventType.EFFECT_OBSERVED,
            test_id=test_id,
            sequence=3,
            timestamp=str(execution["timestamp"]),
            action_id=action_key,
            effect_id=effect_key,
            after_state=dict(execution["arguments"]),
            evidence_ids=(evidence_ref,),
            provenance=Provenance.OBSERVED,
        )
    )

    envelope = derive_envelope(
        ledger,
        envelope_id="h1a-expected-envelope",
        test_id=test_id,
        approval_id=approval_key,
        actor_id=actor_key,
    )

    diff = compute_diff(ledger, envelope, diff_id="h1a-diff", observed_action_ids=[action_key])

    return ledger, envelope, diff


# ═══════════════════════════════════════════════════════════════════════
# Output generation — writes authority-ledger.json, authority-envelope.json,
# authority-diff.json (raw model objects, spec §22 of the Authority Model
# pass) plus authority-map.txt, authority-diff.txt, authority-view.json
# (the presentation projection, spec §16 of the report-surface pass) per
# fixture. None of these duplicate raw evidence content — every evidence
# reference is an id, resolved back to the real evidence file/record.
# ═══════════════════════════════════════════════════════════════════════

_FIXTURE_BUILDERS = {
    "agentlab-ad02": build_agentlab_ad02_fixture,
    "mastra-candidate-4a": build_mastra_candidate_4a_fixture,
    "exp003a-h1a-argument-binding": build_exp003a_h1a_fixture,
}


def _load_ad02_finding() -> Finding:
    """Re-reads the real AD-02 Finding from the reference engagement — a
    separate read from `build_agentlab_ad02_fixture()`'s own, so that
    function's existing (ledger, envelope, diff) return contract (relied on
    by the prior pass's tests) is untouched."""
    engagement_dir = _repo_root() / "assessment-engagement" / "reference-agentlab-dryrun"
    findings = json.loads((engagement_dir / "findings.json").read_text(encoding="utf-8"))
    raw = next(f for f in findings if f["finding_id"] == "AD-02")
    return Finding.from_dict(raw)


# (workflow label, finding-loader-or-None, outcome_override-or-None). The
# override is passed only where a formal Finding doesn't exist for this
# fixture — see each entry's comment for exactly which already-recorded
# evidence justifies it; `build_authority_view` never invents one on its
# own for a non-NONE divergence. `property_label` (freeze-pass addition):
# resolved from `property_mapping.py` (which mirrors
# docs/methodology/08-exp003-property-id-mapping.md) for any fixture whose
# property has a stable, documented identity — never left to
# `build_authority_view`'s generic "Approval Scope" fallback when a real
# mapping exists, which is exactly the H-number-conflation mistake this
# fixes.
from .property_mapping import resolve_property

_EXP003A_H1 = resolve_property("EXP-003A", "H1")
assert _EXP003A_H1 is not None, "docs/methodology/08-exp003-property-id-mapping.md's EXP-003A H1 entry is missing"

_FIXTURE_VIEW_CONFIG: dict[str, tuple[str, Callable[[], Finding] | None, str | None, str | None]] = {
    "agentlab-ad02": (
        "Reference Agent Lab workflow (get_customer / get_order / add_case_note)",
        _load_ad02_finding,
        None,  # outcome comes from the real Finding.status (VIOLATED, Gate-7-backed — see findings.json)
        None,  # property comes from the real Finding.area.label (no stable PROP-* mapping applies to this Agent Lab area)
    ),
    "mastra-candidate-4a": (
        "Supervisor -> sub-agent delegated transfer approval (Mastra agent-as-tool, Candidate 4a)",
        None,
        "VIOLATED",  # justified by the raw evidence's own evidence_gate_status.g7_security_claim_triage=true
        # (semantic-rulebook/experiments/004-candidate-4a-evidence/candidate-4a.json) — not re-argued here,
        # only surfaced; see that record's own notes for the full G7 reasoning.
        None,  # no EXP-003 mapping applies (Mastra Candidate 4a is not part of the EXP-003 series) — generic fallback is correct here
    ),
    "exp003a-h1a-argument-binding": (
        "OpenAI Agents SDK transfer_funds approval (EXP-003A, H1a post-approval-mutation)",
        None,
        None,  # divergence_type is NONE here -- build_authority_view's own safe default (ENFORCED) applies
        _EXP003A_H1.display_label,  # "Argument binding / drift (PROP-ARG-BINDING)" -- sourced from the mapping, not the bare "H1" label
    ),
}


def build_fixture_view(name: str):
    from .presentation import build_authority_view

    ledger, envelope, diff = _FIXTURE_BUILDERS[name]()
    workflow, finding_loader, outcome_override, property_label_override = _FIXTURE_VIEW_CONFIG[name]
    finding = finding_loader() if finding_loader is not None else None
    view = build_authority_view(
        ledger, envelope, diff,
        view_id=f"{name}-view",
        workflow=workflow,
        finding=finding,
        outcome_override=outcome_override,
        property_label_override=property_label_override,
    )
    return ledger, envelope, diff, view


def write_fixture_outputs(name: str, out_dir: Path) -> None:
    from .render_text import render_authority_map_structural, render_full_text_report

    ledger, envelope, diff, view = build_fixture_view(name)
    out_dir.mkdir(parents=True, exist_ok=True)

    ledger.save(out_dir / "authority-ledger.json")
    (out_dir / "authority-envelope.json").write_text(
        json.dumps(envelope.to_dict(), indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    (out_dir / "authority-diff.json").write_text(
        json.dumps(diff.to_dict(), indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )

    # Presentation outputs (spec §16): the customer-facing report surface.
    (out_dir / "authority-map.txt").write_text(render_authority_map_structural(view), encoding="utf-8")
    (out_dir / "authority-diff.txt").write_text(render_full_text_report(view), encoding="utf-8")
    (out_dir / "authority-view.json").write_text(
        json.dumps(view.to_dict(), indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )


def _update_reference_engagement_authority_map() -> None:
    """Report integration (spec §17 of the report-surface pass): point the
    existing reference-dryrun report's 'Authority Map / Authority Diff'
    section at the real, evidence-backed AD-02 Authority Diff, rendered
    through the presentation projection — never hand-written prose. Only
    `Engagement.authority_map` is set and the report re-rendered; no other
    section, finding, or evidence reference is touched. Run only after
    `reference_dryrun.main()` has already written the engagement (this
    reads, never regenerates, that engagement's findings/evidence)."""
    import dataclasses

    from ..engagement import Engagement
    from ..report import render_report
    from .render_text import render_full_text_report

    engagement_dir = _repo_root() / "assessment-engagement" / "reference-agentlab-dryrun"
    if not (engagement_dir / "engagement.json").exists():
        print(f"Skipped report integration: {engagement_dir} has not been generated yet (run reference_dryrun.main() first).")
        return

    _, _, _, view = build_fixture_view("agentlab-ad02")
    rendered = render_full_text_report(view)
    authority_map_text = (
        "Authority Model for AD-02 (Delegation grant exceeds the stated task's need), derived "
        "deterministically from this engagement's own findings.json/evidence_manifest.json through the "
        "Authority Model presentation projection — see "
        "assessment-engagement/authority-model-fixtures/agentlab-ad02/authority-view.json for the "
        "underlying structured projection this was rendered from.\n\n```\n" + rendered + "```\n"
    )

    engagement = Engagement.load(engagement_dir)
    updated = dataclasses.replace(engagement, authority_map=authority_map_text)
    updated.save(engagement_dir)
    (engagement_dir / "report.md").write_text(render_report(updated), encoding="utf-8")
    print(f"Updated {engagement_dir}'s authority_map from the real AD-02 Authority Diff and re-rendered report.md.")


def main() -> None:
    """`python -m agentlab.assessment.authority_model.fixtures` — writes
    all available fixtures' outputs to
    `assessment-engagement/authority-model-fixtures/<name>/`, then wires
    the AD-02 result into the reference engagement's own report (§17).
    Skips the Mastra fixture gracefully if the sibling semantic-rulebook
    checkout isn't present (its outputs are CRYPTON_PROPRIETARY and this
    script never publishes them anywhere outside Crypton-DI's own tree)."""
    base = _repo_root() / "assessment-engagement" / "authority-model-fixtures"
    for name in _FIXTURE_BUILDERS:
        try:
            write_fixture_outputs(name, base / name)
            print(f"Wrote {name} outputs to {base / name}")
        except MastraEvidenceUnavailable as e:
            print(f"Skipped {name}: {e}")
    _update_reference_engagement_authority_map()


if __name__ == "__main__":
    main()
