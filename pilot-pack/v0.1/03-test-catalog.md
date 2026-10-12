# Crypton Test Catalog v0.1

This is a customer-readable description of what Crypton actually tests,
built directly from the working methodology in `docs/methodology/`,
`docs/ASSESSMENT_METHODOLOGY.md`, and `assessment-kit/v1.0/` — not a
marketing gloss over internal experiment names. Every family below states
its real maturity honestly: some are backed by cross-framework evidence
and a working reference implementation; one is explicitly experimental.
We do not present a candidate property as if it were proven.

Every actual engagement scopes a subset of these families against the
customer's specific workflow (Day 1 of `02-founding-assessment.md`) — not
every family applies to every workflow, and "not applicable, with a
reason" is a legitimate scoping outcome, not a gap in coverage.

**The four outcomes, used consistently across every family below**
(`docs/ASSESSMENT_METHODOLOGY.md`, "The four outcomes"):

| Outcome | Meaning |
|---|---|
| **PASS** | Real evidence was collected and it shows the expected control in place and functioning. |
| **FAIL** | Real evidence was collected and it shows a gap — missing, misconfigured, or bypassable. |
| **INCONCLUSIVE** | Sufficient evidence could not be obtained (access withheld, environment unreachable, logs rotated, or nothing to inspect) — always stated with a reason, never a silent default. |
| **NOT_APPLICABLE** | The control genuinely doesn't apply to this workflow's architecture — a sourced reason, never a shrug. |

No PASS or FAIL is ever reported without at least one specific piece of
cited evidence (source excerpt, config, trace, log, or a controlled test
result) — this is a structural rule, not a style preference.

**Structure (revised — see `pilot-pack/v0.1/11-pilot-pack-red-team.md`
for why):** the founding assessment scores **four Core families** in
every engagement. Two further properties exist as **Conditional
Modules** — real, methodology-defined, but scoped in only where the
target architecture actually supports the test, and never presented as
carrying the same evidentiary weight as the Core four. One property is a
**Research Preview** — explicitly not part of the scored assessment at
all. This replaces an earlier v0.1 draft that listed all six plus the
preview as one flat, undifferentiated list, which made the catalog look
more uniformly mature than it actually is.

---

# Core Assessment (scored in every engagement)

## 1. Principal / Session Binding

**Maturity: mature.** Cross-tested across three real agent frameworks
(OpenAI Agents SDK, Microsoft Agent Framework, LangGraph — H6,
`docs/methodology/04-framework-adaptation-guide.md`).

**Question:** when an agent acts, is the action correctly bound to the
specific principal/session that authorized it — not just to *a* valid
credential or session identifier that could belong to someone else?

**Example test:** attempt to resume or inject into an existing agent
session using a session identifier alone (no re-verification of who
created it), and observe whether the system distinguishes the original
principal from a different, separately-authenticated one.

**Evidence collected:** source review of session-resolution logic,
architecture/config showing how session identity is checked, a
controlled staging test attempting cross-principal session access, and
the resulting trace.

**PASS:** the system verifies the authenticated principal on every
request into an existing session, not merely a session identifier — a
different, validly-authenticated principal presenting the same session
ID is rejected or treated as a distinct session.

**FAIL:** a session identifier alone is sufficient to act within that
session, regardless of who currently holds it.

**N/A:** the workflow has no multi-request session concept at all (every
action is a single, independently-authenticated request).

**Boundary conditions:** a system that authenticates every request
independently but doesn't distinguish "sessions" as a concept isn't
failing this property — it doesn't have the surface this property is
about. This exact failure shape (session routed by identifier only, no
principal re-check) is a documented, independently-disclosed pattern in
real MCP server implementations — see `04-evidence-map.md`.

---

## 2. Delegation & Scope Attenuation

**Maturity: mature.** PASS on all three frameworks tested for
delegation-boundary independence (H2); a working reference
implementation of provable multi-hop delegation with scope attenuation
exists (`labs/agent-lab`, v0.6).

**Question:** when authority passes from one principal or agent to
another (parent → child agent, user → delegated tool), does scope only
ever narrow, never silently widen — and does a grant to use one specific
tool/action stay confined to that tool/action?

**Example test:** grant a sub-agent or delegated identity access to one
specific tool, then attempt to reach a different tool or a broader scope
through the same delegated path (a dispatch/execution route that doesn't
re-check the specific per-tool grant it was routed around).

**Evidence collected:** the delegation/grant object or policy definition,
source review of the dispatch path that executes a delegated action,
controlled test attempting to exercise more than the delegated scope.

**PASS:** the delegated path enforces the same per-tool/per-scope grant
at execution time that was defined at delegation time — no route exists
that carries the delegation's identity without carrying its scope.

**FAIL:** a delegated identity can reach a broader tool/resource set than
it was explicitly granted, typically because one dispatch path
re-applies the grant check and another doesn't.

**N/A:** the workflow has no delegation at all — one principal, one
agent, no sub-agents or delegated tool identities.

**Boundary conditions:** a framework that requires the *integrator* to
wire delegation checks (rather than enforcing them itself) is not
automatically FAIL — whether that's a gap depends on whether the
specific deployment actually wired it. This is exactly the
"integrator-owned trust boundary" distinction Crypton's own methodology
already had to make explicit (see `07-why-trust-crypton.md`).

---

## 3. Authorization Boundary & Complete Mediation

**Maturity: mature methodology, one internally-confirmed real finding**
(FL-02 — a documented, application-level checkpoint-mutation pattern;
`assessment-kit/v1.0/07-finding-library.md`).

**Question:** is the authorization check that a system claims to enforce
actually on the path the action takes at execution time — for every
route to that effect, not just the one that was tested?

**Example test:** identify every distinct path that can reach a
protected action (the documented/primary path, and any secondary
route — a retry path, an alternate dispatch mechanism, a direct API call
bypassing a UI-level gate) and confirm the same authorization check
applies to each.

**Evidence collected:** source review of every route to the protected
effect, the authorization check's actual location in each path, a
controlled test exercising the secondary/less-obvious route.

**PASS:** every reachable path to the protected action re-applies the
same authorization check; no route exists that reaches the effect
without passing through it.

**FAIL:** at least one reachable path (often the retry path, a
dispatch/queue route, or an alternate transport) executes the protected
action without the check that the primary path enforces.

**N/A:** the action has exactly one possible path to execution and no
alternate route exists to check.

**Boundary conditions:** this is the family most likely to be confused
with a framework defect when it's actually a configuration or
integration gap, and vice versa — see the triage rule in
`07-why-trust-crypton.md`. It's also the family with the strongest,
most current external corroboration that this failure shape is real and
recurring across independently-built systems — see `04-evidence-map.md`.

---

## 4. Approval-to-Execution Binding

**Maturity: mature methodology and test design; the property is real and
externally corroborated (see below), though Crypton's own prior
candidate finding in this exact family was retracted after threat-model
validation — see "Boundary conditions."**

**Question:** does the action that actually executes remain materially
equivalent to the action a human approved — same tool, same target, same
arguments — not merely the same *label* or tool name?

**Example test:** approve a specific, narrow action (one target, one set
of arguments), then trigger a retry, a recovery path, or any other
post-approval processing step, and compare the arguments that were shown
for approval against the arguments that actually executed.

**Evidence collected:** the proposed action, the approval artifact/record
and what it actually captured, the executed action, the full trace
between approval and execution, and any intervening processing step's
source.

**PASS:** the executed action's tool, target, and arguments match what
was shown to and approved by the human — verified by re-computing a
binding (e.g., a digest of tool + resource + canonical arguments)
immediately before execution, not merely trusting what was proposed at
approval time.

**FAIL:** the executed action diverges from what was approved — a
different target, broadened scope, or materially different
arguments — and nothing between approval and execution caught it.

**N/A:** the workflow has no human-approval step for the in-scope action
at all (nothing to bind).

**Boundary conditions — read this one carefully:** a difference between
approved and executed arguments is **not, by itself,** sufficient to call
something a vulnerability. Before any FAIL is filed here, the assessment
identifies exactly what caused the divergence, what trust level that
actor has, whether it was permitted by the system's own design, what
boundary the system actually claims to enforce, and whether that
boundary was crossed. Crypton previously carried a candidate finding in
this exact family (a LangChain agent-middleware composition case) as
CONFIRMED, then retracted it once this analysis was applied properly —
the divergence was real and reproducible, but the component responsible
was trusted, developer-authored code the framework already permits to
do exactly that; no boundary the framework claims to enforce was
crossed. Full account, including why it doesn't change the family's
validity: `07-why-trust-crypton.md`.

---

---

# Conditional Modules (scoped in only where the architecture supports the test)

These are real, methodology-defined properties — not aspirational — but
maturity and applicability vary enough by target that they are scoped in
per engagement rather than scored by default, and reported with the
reduced-confidence framing each one states below.

## 5. Revocation & Authority Freshness

**Maturity: split — one sub-property is mature, the other is explicitly
not yet tested.** Be precise about which is which:

- **Exactly-once execution across persistence/resume** (a saved,
  checkpointed action cannot be re-triggered to execute twice by
  reloading/resuming the workflow) is **mature**: PASS across all three
  frameworks tested (H4).
- **Authority revalidation after revocation** (does a resumed or replayed
  action recompute whether the authority behind it is *still* valid, not
  just whether it *was* valid at approval time) is **not yet tested
  against any target** — a minimal specification exists
  (`labs/exp-007-principal-continuity/EXP-007B-SPEC.md`) but no test has
  been run. Do not treat this as a mature, sellable test today; it is
  disclosed here as planned scope, not as a delivered capability.

**Question (revocation sub-property, as designed, not yet delivered):**
if authority is revoked, expired, or denied between approval and
execution, does the system recompute validity before acting, or does it
treat the earlier approval as a standing, bearer-style capability?

**Evidence collected (where tested — exactly-once sub-property):**
checkpoint/persistence mechanism source, a controlled test reloading a
completed or in-flight workflow from a fresh process/connection,
confirmation the action did not re-execute.

**PASS (exactly-once sub-property):** resuming a completed or
in-flight persisted workflow from a genuinely fresh process/connection
does not cause the protected action to execute again.

**FAIL (exactly-once sub-property):** reload/resume causes duplicate
execution of an already-completed protected action.

**N/A:** the workflow has no persistence/checkpoint mechanism to resume.

**Boundary conditions:** do not accept a vendor or assessor claim of
"revocation is handled" without a specific, reproduced test — this is
exactly the sub-property Crypton itself has not yet built a test for,
and it is the kind of gap that's easy to assume away without evidence.

---

## 6. Accountability & Authority Provenance

**Maturity: methodology defined, real coverage gap acknowledged in
Crypton's own threat model** (`assessment-kit/v1.0/01-assessor-handbook.md`
§7 lists "Lose attribution" as a stated coverage gap, not a solved
property).

**Question:** after an action executes, can you reconstruct — from
durable records, without trusting the agent's own self-report — which
specific agent acted, under whose authority, and when?

**Example test:** review the durable audit/trace record for a
consequential action and confirm it independently identifies the actor,
the authorizing principal, and the authority basis, without relying on
data the agent itself supplied about its own actions.

**Evidence collected:** audit/log records, the mechanism that writes
them, and confirmation of what's independently verifiable in them versus
what's self-reported by the agent.

**PASS:** the record independently establishes actor, authorizing
principal, and authority basis from sources the agent doesn't control.

**FAIL:** reconstruction depends on the agent's own self-reported
identity/authority claims, with no independent corroboration.

**N/A:** the action has no consequential effect requiring
reconstruction (rare — most in-scope actions by definition have one).

**Boundary conditions:** this family is honestly the least mature in
this catalog today. An engagement scoping it should expect Crypton to
say so plainly rather than presenting thin coverage as complete.

---

---

# Research Preview (not part of the scored assessment)

## Principal Continuity

**Not offered as a standard test family in the $2,500 assessment today.**
Included here for transparency, since it's real, ongoing Crypton
research: whether an authority grant held by an agent stays valid when
the party *controlling* that agent changes to someone the original
grantor never delegated to — distinct from ordinary authentication
failure (`labs/exp-007-principal-continuity/`).

Current status, stated exactly as Crypton's own internal record states
it: **Candidate — Testable / 1 Verified External Case.** A deterministic
test harness exists and passes; one independent external incident
(Meta's Muse assistant + a local-control-path takeover, reported by
Patrick Wardle) shows a related but not identical failure shape. Two
bounded external searches found no second independent case meeting the
test's own bar. **Not promoted to a core property**, and not sold as a
delivered test today. If a customer's architecture and interest align
with this specific question, it can be discussed as explicitly
experimental, add-on scope — never bundled into the standard assessment
as if it carried the same evidentiary weight as families 1-4 above.
