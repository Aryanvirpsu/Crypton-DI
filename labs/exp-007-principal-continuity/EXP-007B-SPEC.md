# EXP-007B — Minimal Specification (not implemented)

Date: 2026-09-27. This is a specification only. **No code for EXP-007B
exists yet and none should be written from this document without a
separate decision to start it.** EXP-007A (`scenario_007a.py`) remains
frozen and untouched — see [`FROZEN.md`](FROZEN.md).

## Target property

**Approval / authority validity across state change.** A human approval,
or any other standing authorization decision, must be re-validated
against the authority state that exists at the moment of resume/replay —
not treated as a bearer capability that remains good regardless of what
changed underneath it after it was granted.

## Why this is a distinct property from EXP-007A

EXP-007A is stateless: every one of its legs (a-d, plus the calibration
probe) is a single-message decision, evaluated once, with no time
dimension. Its own Distinctness note (`EVIDENCE.md`, "Distinctness
note") says this explicitly: "nothing in the frozen test exercises
continuity across time." EXP-007B is the deliberate complement — same
underlying question (is this authority actually still good, right now,
for this actor?), but asked **after a real gap in time during which the
authority state itself changed**, not just after a different controller
shows up in the same instant.

## Core sequence

```
valid proposal
  -> human approval
    -> authority/controller/state changes  (the mutation under test)
      -> resume/replay/execution attempt
        -> system must recompute whether authority is still valid
```

The system under test fails this property if it executes the
resumed/replayed action using the *original* approval decision, without
recomputing validity against the *current* authority state. It passes if
it recomputes and correctly denies/step-ups when the authority state no
longer supports the action.

## Scope decision: pick ONE mutation for the first build

The task list of candidate mutations (revocation, controller/principal
change, delegation expiration, scope reduction, resource-owner denial,
resumed persisted workflow) is not all being built. Per the smallest
deterministic scenario that isolates the property, **EXP-007B v1 should
implement exactly one mutation: revocation between approval and
resume/execution.** Rationale:

- It is the cleanest binary state change (authority present -> authority
  revoked), with no ambiguity about partial validity, unlike scope
  reduction (needs a "does the resumed action still fit the narrowed
  scope" sub-check) or delegation expiration (needs a clock/TTL model).
- It maps directly onto the OpenAI incident input (see below):
  "credential possession != continuing authority" and "previous
  capability != permission after explicit denial" are both revocation-
  shaped statements, not scope- or controller-shaped ones.
- It reuses EXP-007A's existing primitives with minimal new surface:
  `identity.py`/`authorization.py` already model an authority grant that
  can be checked; EXP-007B only needs to add a revoke-after-approval step
  and a resume/replay path, not a new controller or delegation model.

Controller/principal change, delegation expiration, scope reduction, and
resource-owner denial remain explicitly candidate mutations for a later
EXP-007B iteration (not EXP-007C — this stays inside 007B's numbering
per the task's scope) if v1 demonstrates the harness pattern works.
Resumed persisted workflow (i.e. running the revocation-across-resume
case through an actual checkpoint/persistence layer, not just an
in-memory replay) is the natural v1-to-v1.1 extension, not v1 itself.

## What v1 must demonstrate (acceptance shape, not a full spec)

1. A proposal is approved while the actor's authority is valid.
2. Between approval and resume, the authority is revoked (a real state
   change to the authorization store/model, not a flag flipped for the
   test's convenience).
3. A vulnerable/reference policy that treats the approval as a bearer
   token executes the resumed action anyway.
4. A secure policy recomputes authority validity at resume time and
   denies, with a reason code distinct from EXP-007A's
   `CONTINUITY_UNVERIFIED`/`CONTROLLER_NOT_AUTHORIZED`/`NO_CAPABILITY`
   (e.g. `AUTHORITY_REVOKED_SINCE_APPROVAL` or similar — exact naming is
   an implementation decision, not fixed here).
5. Deterministic, stdlib-only, local mock — same engineering standard as
   EXP-007A (no network, no real crypto beyond what EXP-007A already
   uses).
6. A negative control equivalent to EXP-007A's `authenticated_only_policy`
   — a policy that looks like it revalidates but actually doesn't (e.g.
   checks authentication/capability but not revocation status) — must be
   caught by the harness's own gate, not just pass silently.

## Explicitly out of scope for EXP-007B (do not implement)

- **Replay/nonce consumption** — remains deferred from EXP-007A per its
  own documented limitation ("Replay protection is deferred to
  EXP-007B... this is not replay protection and no replay claim is
  made"). EXP-007B v1, as scoped above (revocation only), still does not
  implement nonce consumption or replay-attack resistance. If a future
  EXP-007B iteration adds a replay-specific leg, it must say so
  explicitly and not silently claim replay resistance as a side effect
  of the revocation check.
- Controller/principal change, delegation expiration, scope reduction,
  resource-owner denial, and resumed-persisted-workflow mutations — all
  named as candidates in the task input, all deferred past v1 (see
  "Scope decision" above).
- EXP-008, or any experiment beyond 007B — not started, not scoped here.
- Promoting any EXP-007B result to a core Crypton property — that
  question doesn't arise until EXP-007B actually runs and produces
  evidence, exactly as EXP-007A was not promoted merely because its
  harness worked (`EVIDENCE.md`).

## Relationship to EXP-007A's promotion status

EXP-007A remains `Candidate — Testable / 1 Verified External Case`, not
promoted (`EVIDENCE.md`). EXP-007B is not a precondition graded against
that promotion bar directly, but per `EVIDENCE.md`'s own promotion rule,
a future promotion review should check EXP-007B's result for
non-redundancy against EXP-007A (i.e. "is 'authority validity across
state change' really a distinct invariant from 'principal continuity,'
or the same invariant viewed at two different moments in time?") before
either is promoted to a core catalog property.

## OpenAI incident mapping (context, not a new property)

The September 2026 OpenAI standing-GitHub-credential / explicit-human-denial
incident is methodology inspiration for exactly the mutation EXP-007B v1
chooses (revocation/denial between grant and execution), not grounds for
inventing a new Crypton property. It maps into existing categories:

- **Authorization Boundary / Revocation:** "credential possession !=
  continuing authority" is precisely the boundary EXP-007B v1 tests —
  holding a credential is not the same as the authority behind it still
  being valid at execution time.
- **Approval Binding / Denial Binding:** "previous capability != permission
  after explicit denial" extends Pillar 4's approval-binding definition
  to the negative case — a denial must bind going forward the same way an
  approval binds to what it approved, and a system that keeps honoring
  pre-denial capability has the same shape of failure as one that lets a
  pre-approval capability outlive a later revocation.
- **Authority Provenance:** the incident is evidence that "this agent once
  had a grant" is not the same durable fact as "this agent currently has
  a grant" — provenance records history, not current validity.
- **Accountability / Evidence:** whether a system can even show *when*
  authority was revoked/denied relative to when the action executed is
  itself an auditability question (Pillar 7), independent of whether the
  authority check itself was correct.

No claim is made here that this incident proves Crypton's thesis, that it
is technically identical to the revocation mutation EXP-007B v1 will
build (it is used as motivation, not as a source of test fixtures), or
that EXP-007B's eventual result will resemble it in mechanism.
