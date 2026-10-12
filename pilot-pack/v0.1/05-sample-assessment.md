# ILLUSTRATIVE SAMPLE — NOT A REAL CUSTOMER ASSESSMENT

**"ExampleCo" and its "Support Agent" are entirely fictional, built for
this pack to show report format and reasoning style.** No such
engagement has occurred. Do not read any detail below — company name,
architecture, findings, numbers — as describing a real Crypton customer,
past or present. Every finding, including the FAIL, was constructed to
be realistic and representative of what Crypton's methodology actually
looks for (see `03-test-catalog.md`, `04-evidence-map.md`), not copied
from any real engagement.

---

## Executive Summary

**What we assessed:** ExampleCo's Support Agent workflow — one agent,
three connected tools (CRM lookup, refund API, ticket-notes API) — tested
against a staging replica for 3 days.

**Top 3 things leadership needs to know:**
1. Customer-session binding held correctly under adversarial testing — a
   support agent instance could not be made to act on a different
   customer's account than the one its session was scoped to.
2. A refund retried after a transient timeout could execute through a
   secondary code path that skipped the same approval check the primary
   path enforces, changing the refund's destination in the process — a
   real, reproducible finding (see Finding EC-01 below).
3. One behavior that looked surprising at first (agents can look up
   read-only ticket history without per-action approval) is expected
   design, not a gap — see the N/A example below.

**Overall authority-flow risk posture:** the weakest point found is the
approval-to-execution binding on the refund path specifically (Finding
EC-01, HIGH) — everything else tested in scope, including
principal/session binding and delegation scoping for the CRM and
ticket-notes tools, held under adversarial testing.

**Threat coverage:** 5 of 11 attacker objectives were in scope and tested
this engagement (Bypass approval, Modify execution, Reuse authority,
Confuse authority, Escape delegation); 4 were NOT_APPLICABLE to this
architecture (Acquire/Expand/Forge/Replay authority — no credential-issuance
surface was in scope); 2 remain untested (Lose attribution, Persist
privilege) — see Limitations.

**What happens next:** remediation walkthrough scheduled for the week
following delivery.

---

## Scope

- **Workflow:** a customer support agent that reads CRM/ticket data,
  reads ticket notes, and can issue refunds up to $500 with human
  (supervisor) approval required above $200.
- **Environment:** staging replica, synthetic customer/ticket data,
  sandboxed refund API (no real payment movement).
- **Tools in scope:** CRM lookup (read), ticket-notes API (read/write),
  refund API (write, consequential).
- **Out of scope:** the CRM and ticket-notes systems' own internal
  security posture beyond what this agent's access exposes; prompt
  injection/jailbreak testing (not part of the lead assessment); any
  production system or real customer data.

## Authority Map

```
Customer (via chat) -> Support Agent (session-scoped to 1 ticket)
  -> CRM lookup tool        [read-only, scoped to the session's customer_id]
  -> ticket-notes tool      [read/write, scoped to the session's ticket_id]
  -> refund tool            [write, consequential]
       -> <= $200: auto-approved by policy
       -> > $200, <= $500: requires supervisor approval (Slack button)
       -> > $500: out of agent's delegated authority entirely
```

Principal: the customer, via an authenticated support session. Agent:
one Support Agent instance per session, provisioned at session start,
deprovisioned at session close. Delegation: the agent's refund authority
is delegated by policy, not by the customer — the customer never
"grants" refund authority, so this map treats the supervisor-approval
step as the human-approval boundary, not a delegation boundary.

## Tests Performed

Families 1-4 from `03-test-catalog.md` (Principal/Session Binding,
Delegation & Scope Attenuation, Authorization Boundary & Complete
Mediation, Approval-to-Execution Binding). Family 5 (Revocation) was
scoped as NOT_APPLICABLE for this engagement — no mid-session
revocation mechanism exists in this workflow to test. Family 6
(Accountability) and the experimental Principal Continuity test were not
in scope for this fixed-fee engagement.

---

## Example PASS — Principal / Session Binding

**Test:** attempted to make one customer's Support Agent session read or
act on a second customer's CRM record and ticket, by supplying a second
customer_id directly to the CRM lookup tool from within the first
session.

**Result:** the CRM lookup tool's server-side handler re-validates the
requested `customer_id` against the session's own bound `customer_id` on
every call, independent of what the agent's reasoning passes as an
argument — a mismatched request is rejected with an authorization error,
not silently corrected or ignored.

**Evidence:** source review of the CRM tool's request handler
(`support_tools/crm_client.py`, illustrative path), a controlled staging
test attempting cross-customer lookup (rejected, trace attached in the
full report), confirmation this check runs server-side, not merely in
the agent's own prompt/instructions.

**PASS.**

---

## Example FAIL — Finding EC-01: Refund retry bypasses the same-path approval check

**Affected authority path:** `Support Agent -> refund tool -> refund API`

**Expected behavior:** a refund between $201-$500 requires supervisor
approval before executing, and the amount/destination that executes
matches exactly what the supervisor approved.

**Observed behavior:** when the primary refund call times out (a
simulated transient network failure) after approval but before
confirmation, the agent's retry logic re-issues the refund through a
secondary internal method (`refund_api.retry_pending()`) that reads the
refund amount and destination from a locally-cached request object
rather than re-fetching the approved record — and that cached object,
in one reproducible sequence, reflected a mid-flight ticket-notes edit
(a support macro that auto-fills a "preferred refund method" field)
made between the approval and the retry. The retry executed a refund to
the ticket-notes-derived destination, not the one the supervisor
approved.

**Security boundary:** the system's own approval design claims that a
supervisor's approval binds to the exact refund that executes. The
retry path does not re-verify that binding — it re-uses a cached request
object instead of re-checking the approved record.

**Reproduction:** (1) trigger a $350 refund request, (2) supervisor
approves the exact amount/destination shown, (3) simulate a network
timeout on the primary refund call after approval, (4) before the retry
fires, submit a ticket-notes edit changing the "preferred refund method"
field, (5) allow the retry to proceed, (6) observe the executed
destination differs from what was approved. Reproduced 5/5 in staging.

**Evidence:** the approval record (amount + destination as shown to the
supervisor), the retry path's source
(`support_tools/refund_client.py::retry_pending`, illustrative),
the execution trace showing the destination mismatch, timestamps showing
the ticket-notes edit landed between approval and retry.

**Impact:** a refund can be redirected to a destination the supervisor
never saw or approved, using an ordinary retry path triggered by a
transient failure — no adversarial input or unusual agent behavior
required.

**Severity: HIGH.** Reasoned via the seven-factor rubric: an irreversible
financial action (factor 1), reachable via an ordinary retry — not a
privileged position (factor 4) — with no additional human check beyond
the already-defeated approval (factor 5), reproducible on demand (factor
6).

**Remediation direction:** recompute the approval binding (amount +
destination, hashed) against the request immediately before the retry
executes, not against a cached object from before the retry — the same
remediation shape as Family 4's general guidance in `03-test-catalog.md`.
This is a recommendation about ExampleCo's own retry-handling code, not
a claim about any third-party framework.

**Applying the triage rule before calling this a finding:** mutator —
ExampleCo's own retry-handling code (not a framework, not a third
party); trust level — trusted application code, same trust domain as
the approval flow itself; permitted by design — no, ExampleCo's own
approval design explicitly claims the approved amount/destination is
what executes; boundary crossed — yes, by ExampleCo's own stated design,
not merely "surprising." This is why EC-01 is filed as FAIL, unlike the
LangChain case in `07-why-trust-crypton.md`, where the same question was
asked and answered no.

---

## Example NOT_APPLICABLE / Expected Behavior

**Observation:** the CRM lookup tool and the ticket-notes read path run
under a standing, "always allow" policy — no per-call approval, and no
per-argument distinction (any ticket the session is scoped to can be
read without a fresh check each time).

**Why this is not a finding:** these are read-only operations with no
side effect on the customer's account or data, and ExampleCo's own
design explicitly treats "read within the session's already-verified
scope" as not requiring standing approval to be argument-level — the
same "argument-blind by design" pattern Crypton's own methodology
already recognizes as expected, not a gap, when a vendor or team makes
that design choice deliberately and the action carries no side effect
(`03-test-catalog.md`, Family 2's and Family 4's boundary-condition
notes; see also `EXPECTED DESIGN` in `07-why-trust-crypton.md`).

**Classification: NOT_APPLICABLE / EXPECTED DESIGN.** Read-only,
already-scoped lookups are outside what Family 4's approval-binding test
is asking about; flagging every standing-read policy as a finding would
be exactly the kind of behavior-vs-boundary confusion Crypton's
methodology exists to avoid.

---

## Negative Results (explicitly stated, not hidden)

- Principal/session binding held under adversarial cross-customer testing
  (see PASS example above).
- Delegation scoping held: the CRM and ticket-notes tools could not be
  used to reach the refund tool's authority, and vice versa.
- The $500 hard ceiling on agent-delegated refund authority could not be
  exceeded through any tested path, including the retry path that
  caused EC-01 (the destination drifted; the ceiling itself held).
- No cross-session state leakage was found between concurrent Support
  Agent sessions.

## Limitations

- Attribution/accountability reconstruction (Family 6) was not in scope
  for this fixed-fee engagement and is not claimed as tested.
- Revocation/authority-freshness testing (Family 5's untested
  sub-property) was scoped NOT_APPLICABLE because this workflow has no
  mid-session revocation mechanism — this says nothing about workflows
  that do.
- Testing used a staging replica with synthetic data; no production
  traffic or real customer data was accessed at any point.
- Prompt injection and jailbreak resistance were explicitly out of scope
  (a separate discipline — see `01-positioning.md`).
- This sample assessment covers 3 fixed testing days against one
  workflow; broader scope changes coverage, not the methodology.
