"""Evidence recording: the 10 required fields plus controller_changed.

Time and per-step nonces are derived from a step counter, so identical runs
produce byte-identical evidence.
"""

from __future__ import annotations

import hashlib
from dataclasses import asdict, dataclass, replace

from .authorization import Decision


@dataclass(frozen=True)
class EvidenceRecord:
    leg: str
    policy: str
    step: int
    timestamp: int
    step_nonce: str
    principal_identity: str
    agent_identity: str
    controller_identity: str
    agent_authenticated: bool
    controller_authenticated: bool | None
    capability_valid: bool
    expected_authority_root: str
    controller_authorized_for_authority_chain: bool | None
    original_grant: dict
    approval_object: str
    action_params: dict
    resource_decision: str
    execution_result: str
    audit_note: str
    reconstruction_identifies_true_chain: bool | None
    controller_changed: bool


class Recorder:
    def __init__(self, run_label: str, start: int = 1_700_000_000) -> None:
        self._label = run_label
        self._start = start
        self.records: list[EvidenceRecord] = []

    def record(
        self, *, leg: str, policy: str, principal: str, agent: str, grant: dict,
        authority_root: str, params: dict, decision: Decision, result: str,
    ) -> int:
        step = len(self.records)
        ts = self._start + step
        nonce = hashlib.sha256(f"{self._label}|{step}|{ts}".encode()).hexdigest()[:16]
        executed = result != "NOT_EXECUTED"
        self.records.append(
            EvidenceRecord(
                leg=leg, policy=policy, step=step, timestamp=ts, step_nonce=nonce,
                principal_identity=principal, agent_identity=agent,
                controller_identity=decision.controller_attribution,
                agent_authenticated=decision.agent_authenticated,
                controller_authenticated=decision.controller_authenticated,
                capability_valid=decision.capability_valid,
                expected_authority_root=authority_root,
                controller_authorized_for_authority_chain=decision.controller_authorized,
                original_grant=grant, approval_object="none (not in scope for 007-A)",
                action_params=params, resource_decision=decision.render(),
                execution_result=result,
                audit_note=f"{'executed' if executed else 'not executed'} via {agent}; "
                f"authority root {authority_root}; "
                f"controller recorded as {decision.controller_attribution}; {decision.detail}",
                reconstruction_identifies_true_chain=None,
                controller_changed=decision.controller_changed,
            )
        )
        return step

    def finalize(self, step: int, true_actor: str) -> EvidenceRecord:
        """Harness-only: compare what the trace lets a reviewer conclude to ground truth."""
        rec = self.records[step]
        attributed = rec.controller_identity
        if attributed.startswith("UNVERIFIED("):
            # Trace flags a non-authenticated origin; correct iff the actor really wasn't U.
            ok = rec.controller_changed and true_actor != rec.principal_identity
        else:
            # Must name the actual controller AND show it differs from the authority root.
            ok = attributed == true_actor and (true_actor == rec.principal_identity or rec.controller_changed)
        self.records[step] = replace(rec, reconstruction_identifies_true_chain=ok)
        return self.records[step]

    def dump(self) -> list[dict]:
        return [asdict(r) for r in self.records]
