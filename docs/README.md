# Documentation map

One authoritative place per topic. If two documents disagree, the one listed here wins and
the other is stale (fix it or mark it historical). Historical records are never rewritten:
they are superseded by a newer entry, not edited.

| Topic | Authoritative location | Notes |
|---|---|---|
| **Engineering handoff: what exists, how to build and test it, risks, priorities, ownership** | [`handoff/CTO-HANDOFF.md`](handoff/CTO-HANDOFF.md) | Fresh test results: [`handoff/HANDOFF-VERIFICATION-REPORT.md`](handoff/HANDOFF-VERIFICATION-REPORT.md); starting state: [`handoff/PRE_CLEANUP_BASELINE.md`](handoff/PRE_CLEANUP_BASELINE.md); Graphify caveats: [`handoff/GRAPHIFY-NOTES.md`](handoff/GRAPHIFY-NOTES.md) |
| Agent / developer instructions, founder strategy, **security and strategic Decision Log** | [`/AGENTS.md`](../AGENTS.md) (`CLAUDE.md` is a one-line pointer to it) | Append-only Decision Log is §15 |
| Technical decision log (Agent Lab / identity design decisions, invariants) | [`/DECISIONS.md`](../DECISIONS.md) | Append-only; different scope from AGENTS.md §15 |
| System architecture, request flow, real-vs-experimental | [`architecture.md`](architecture.md) | |
| Identity and gateway contracts (token, claims, public routes, nonce, audit) | [`../services/crypton-gateway/README.md`](../services/crypton-gateway/README.md) | Token contract section is the contract; `services/crypton-id/src/jwt.rs` is the issuer |
| Deployment topology, trust boundary, security-signal producers/consumers, non-atomic behaviour | [`deployment-topology.md`](deployment-topology.md) | |
| Test instructions and what each layer proves | [`testing.md`](testing.md) | Runtime suite: `tests/runtime/` |
| Dependency vulnerabilities: findings, remediation, deliberate exceptions | [`dependency-audit.md`](dependency-audit.md) | Rust gate: `scripts/rust-audit.sh` + `scripts/rust-audit-ignore.txt` |
| Known limitations | per-document "limitations"/"gaps" sections: [`deployment-topology.md`](deployment-topology.md) (runtime), gateway README ("Remaining gaps"), [`../crypton-bigidea-2026/evidence/KNOWN-LIMITATIONS-AND-DEFERRED.md`](../crypton-bigidea-2026/evidence/KNOWN-LIMITATIONS-AND-DEFERRED.md) (point-in-time, 2026-10-12) | AGENTS.md §14 lists strategic risks |
| Assessment methodology (what is tested, how, outcome semantics) | [`methodology/`](methodology/README.md) (property catalog, threat model, pipeline) and the operating manual [`../assessment-kit/v1.0/`](../assessment-kit/v1.0/README.md) | [`ASSESSMENT_METHODOLOGY.md`](ASSESSMENT_METHODOLOGY.md) is the original 8-area procedure that the code in `labs/agent-lab/src/agentlab/assessment/` implements; it stays because the code cites it, but the property taxonomy and outcome vocabulary in `methodology/` and the kit govern |
| Repository structure, frontends, generated artifacts, branches | [`repository.md`](repository.md) | |
| Lost-device recovery flow | [`recovery-flow.md`](recovery-flow.md) | |
| Technical source of truth (v1.2, RAT-6 delegation record) | [`CRYPTON_TECHNICAL_SOT_v1.1.md`](CRYPTON_TECHNICAL_SOT_v1.1.md) | Historical record of the v0.6 design; §-numbers are cited elsewhere |
| Strategy | [`strategy/`](strategy/) (`2026-09-10-action-integrity-thesis.md`, `CHANGELOG.md`) | Prior wedge language is archived in `strategy/CHANGELOG.md` |
| Go-to-market / pilot material | [`business/`](business/), [`../pilot-pack/v0.1/`](../pilot-pack/v0.1/) | Scope language in `business/` predates the 2026-09-10 wedge change (see AGENTS.md §8c) |
| Audits and point-in-time evidence | [`audits/`](audits/), [`../crypton-bigidea-2026/`](../crypton-bigidea-2026/) | Evidence is preserved as captured; do not edit |
| Service-specific READMEs | `services/*/README.md`, `libs/crypton-sdk/README.md`, `risk-engine/README.md`, `policy-engine/README.md`, `labs/**/README.md` | Describe one component; link here for cross-cutting topics |
