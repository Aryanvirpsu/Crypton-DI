# Crypton — Technical Evidence Summary (BIG IDEA 2026)

*Regenerated against code freeze `1b93abe887c9e28f3217cb67e0e707a5a9f618f6` (branch `verification/big-idea-stabilization`, 2026-10-12). Every row links to a file in this package. Items that were not run are labelled.*

## In one sentence
Crypton is a research-stage effort to test whether autonomous AI agents stay inside the authority humans actually granted them, built on a working reference implementation of signed, narrowing delegation chains plus a repeatable test methodology that, in two retrospective cases, tells a known-vulnerable release from its fixed successor.

## Demonstrated today (run at the freeze)
| Claim | Evidence |
|---|---|
| **558 automated tests pass, 0 fail** across Agent Lab (453), EXP-007A (16), three framework experiments (12 + 17 + 12), Rust identity service (38, against an isolated Postgres), JS SDK (10); TypeScript typecheck clean | `evidence/verification-summary.md` §1; `evidence/regression/summary.tsv` |
| Ed25519/EdDSA signed identity tokens; tampered, expired, wrong-key tokens rejected | Rust `jwt` tests in `evidence/regression/rust-crypton-id.txt`; `labs/agent-lab/tests/test_identity.py` |
| Registered agent identities, owners, per-agent Ed25519 signing keys | `test_agents.py`, `test_signing_keys.py` (in `regression/agent-lab.txt`) |
| Scoped, expiring, revocable delegation; pre-execution authorization with zero side effects on denial | `test_delegation.py`, `test_authorization.py`, `test_enforcement_boundary.py` |
| **Multi-hop signed delegation where authority can only narrow** (human → A → B → C): tool amplification, resource escape, broadened issuance and ancestor revocation all denied | `demo/scenario.md`, `demo/raw-output.txt`, `demo/run_demo.py` (**CRYPTON-DEMO-001: ENFORCED**) |
| Approval bound to exact tool arguments, single use (Agent Lab simulation) | `test_approval*.py` |
| Controller-substitution test separating a weak policy from a secure one, deterministically | EXP-007A: `regression/exp-007a-*.txt`; `demo/raw-output.txt` part 2 |
| **External validation, independently reproduced**: MCP Python SDK 1.27.1 → FAIL, 1.27.2 → PASS; `@bytebase/dbhub` 0.22.5 → FAIL, 0.22.6 → PASS. Versions confirmed against GitHub advisories; hashes/lockfiles retained | `evidence/benchmark/`, `regression/bench-*.txt`, `labs/validation-benchmark/v0.1/REPRODUCE.md` |
| Assessment report renders reproducibly from structured evidence | `agentlab.cli --render-assessment` byte-identical re-render; `demo/sample-report.md` |

## Not demonstrated / label carefully
| Item | Status |
|---|---|
| Human passkey (WebAuthn) authentication in `crypton-id` | Code uses `webauthn-rs`; **no automated end-to-end ceremony test**; not verified |
| WebAuthn in Agent Lab approvals | **Simulated** (`FakeWebAuthnVerifier`) |
| Gateway as policy enforcement point | **Does not exist.** `crypton-gateway` is a plain reverse proxy with 0 tests |
| React console, `crypton-main`, deployment, Redis-backed paths | Not run |
| Discovery of an unknown vulnerability in a third-party system | **Not shown.** Both benchmark cases were public before the scripts were written |

## Planned (do not present as existing)
Automated scanner · customer dashboard/SaaS · continuous monitoring · broad framework integrations · production key custody · gateway PEP · real-WebAuthn approval in the agent loop.

## What changed since the first verification run (same day)
Rust: 22 pass + 16 fail → 38 pass (environmental: no database). Benchmark: recorded → independently reproduced. Claims corrected: gateway (was "Built PEP"), WebAuthn scope, stale test-status note. Tree: dirty → committed and clean. Full list: `evidence/CHANGELOG.md`; open items: `evidence/KNOWN-LIMITATIONS-AND-DEFERRED.md`.

## Honest limits
No customer or design partner yet; the sample report is a self-assessment. The strongest finding is "our control works and our test separates known-vulnerable from known-fixed," not "we found a flaw." Core concepts (attenuation, confused deputy, TOCTOU, session binding) are established; the claim is disciplined, repeatable application, not a new primitive. IETF AAT/WIMSE is converging on the delegation problem. Single Windows machine.
