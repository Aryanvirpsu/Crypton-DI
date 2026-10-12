# EXP-007 / Scenario 007-A — Controller Substitution

**Status: FROZEN (2026-09-24). See [FROZEN.md](FROZEN.md).** Deterministic, stdlib-only, local mock. No network, no external crypto (HMAC-SHA256 stands in for signatures). Scenarios 007-B/C/D are intentionally not built; a minimal spec for 007-B (authority validity across state change) is written at [EXP-007B-SPEC.md](EXP-007B-SPEC.md), not yet implemented.

```
python -m exp007.scenario_007a [--evidence | --json]
python -m pytest
```

Exit code is non-zero if the distinction collapses.

## What it measures

Principal continuity: whether a grant held by Agent A stays valid when the party *controlling* A is no longer inside the authority chain rooted at the grant's owner U. The core invariant:

```
identity valid + capability valid + controller authenticated
!= controller authorized to exercise that delegated authority
```

| Leg | Controller | Controller authenticates? | U -> controller delegation? | Secure expectation |
|---|---|---|---|---|
| a | U, signed, in-session | yes | (is the root) | ALLOW (`CONTINUITY_VERIFIED`) |
| b | B, unsigned, spoofing U's pid | no | no | STEP_UP `CONTINUITY_UNVERIFIED` (UNSIGNED) |
| c | B, signature under a key of its own choosing | no | no | STEP_UP `CONTINUITY_UNVERIFIED` (BAD_SIGNATURE) |
| d | B, honestly signed as B with B's registered key | **yes** | **no** | STEP_UP `CONTROLLER_NOT_AUTHORIZED` |
| cal-no-capability | U, signed | yes | (root) | DENY `NO_CAPABILITY` |

Leg d is the pure case: A is authenticated, B is authenticated, the U -> A capability is valid, unexpired and unrevoked, no signature is malformed. The only change is that the effective controller is B, to whom U delegated nothing. `substitute_preconditions()` asserts each of those facts directly and the gate fails if any is untrue, so leg d cannot pass by accident of an unrelated failure.

Reason codes are kept apart on purpose: `CONTINUITY_UNVERIFIED` (controller cannot be authenticated), `CONTROLLER_NOT_AUTHORIZED` (authenticated, no delegation), `NO_CAPABILITY` (ordinary authorization). The gate requires four distinct values across legs a, b, d and the calibration probe.

| Policy | Rule | Result |
|---|---|---|
| `vulnerable_policy` | A holds a valid capability -> allow | FAIL (executes b, c, d) |
| `secure_policy` | valid capability + authenticated controller + controller in authority chain + session/task/nonce match | PASS |
| `authenticated_only_policy` | **deliberately broken control**: authenticates + checks capability, ignores authority chain | must be rejected by the gate (FAIL on leg d) |

## Evidence

Every step records the 10 required fields, `controller_changed`, plus `agent_authenticated`, `controller_authenticated`, `capability_valid`, `expected_authority_root`, `controller_authorized_for_authority_chain`. `None` means the policy never evaluated that question (the vulnerable policy records B as the controller as claimed and does not relabel it U). `reconstruction_identifies_true_chain` is true only if the trace names the real controller and shows it differs from the authority root.

## Limits

- **Replay protection is deferred to EXP-007B.** The session nonce is compared to the session, never consumed; this is not replay protection and no replay claim is made.
- If B obtains U's key, this scenario says nothing; that is key compromise, a different property.
- Delegation is a static set (`authorized_controllers`); no delegation issuance, expiry or revocation of U -> X delegations is modelled.
- STEP_UP is a verdict only; no step-up flow is implemented.
- Keys are derived from fixed labels for reproducibility, not a key-management model.
- `N/A` is defined but never emitted: the local mock always has a comparable concept.
- This does not promote Principal Continuity to the core catalog.
