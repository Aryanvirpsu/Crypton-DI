# Crypton Founding Agent Authorization Assessment

**$2,500 · 3 testing days · founding/design-partner rate**
(Standard rate for this class of engagement runs $8,000-$10,000; this is
an intentionally under-market rate to secure real first engagements, not
a permanent price.)

---

## One-page offer

**We test** whether your agent's real, executed actions stay bound to the
principal, delegation, approval, and authorization state that were
supposed to govern them — for one specific, bounded, action-taking
workflow. Four Core properties are scored in every engagement (Principal/
Session Binding, Delegation & Scope Attenuation, Authorization Boundary &
Complete Mediation, Approval-to-Execution Binding); two Conditional
Modules (Revocation & Authority Freshness, Accountability & Provenance)
are scoped in only where your architecture supports the test. See
`03-test-catalog.md` for the exact properties, and
`labs/validation-benchmark/v0.1/` for a real, reproducible demonstration
that the Principal/Session Binding test correctly distinguishes a known
vulnerable dependency version from its patched successor.

**You provide:**
- One named workflow with a technical point of contact available for the
  testing window.
- No credentials, production access, or source repository required by
  default — the default engagement mode is Customer-Controlled
  Assessment (see `06-access-and-safety.md` Mode 1): you run the agreed
  tests, on your own screen or independently, and hand over the
  resulting evidence. Temporary, scoped staging access (Mode 2) is
  requested only if something specific needs it.
- Architecture/config documentation for the in-scope workflow (or a
  30-45 minute walkthrough if it isn't written down).
- A signed Authorization-to-Test naming the exact systems, environment,
  and window (a hard legal gate before any testing — see
  `06-access-and-safety.md`).

**You receive:**
- A trust-flow/authority map of the in-scope workflow (delivered end of
  Day 1, so Day 2's testing doesn't start from a disputed premise).
- A written technical report: executive summary, findings (with
  reproduction steps and evidence), negative results (what we tested and
  did *not* find broken), explicit threat-coverage and limitations
  statement.
- A live remediation walkthrough call.

**We don't need:**
- Production access, or access beyond the one named staging workflow.
- Source code for unrelated systems.
- Standing/permanent credentials — temporary, scoped-to-window access
  only, revoked at engagement close.
- Broad network access — one workflow, up to three connected tools is
  the default scope; anything broader is a separate scoping conversation,
  not something silently absorbed into the fixed fee.

**Ideal pilot target:** an agent that acts on behalf of real users, calls
real tools/APIs (even in staging), has some form of delegated permission
or human approval gate, and performs at least one consequential state
change (not just read/retrieval). It doesn't need to be your most
critical system — a bounded, representative workflow is the point.

**Not a good fit right now:**
- A chatbot that only produces text with no tool-calling/action surface.
- A request focused on model-output quality or content safety.
- A generic prompt-injection/jailbreak engagement (a real, different
  discipline — Crypton doesn't offer it as the lead assessment; see
  `01-positioning.md`).
- A traditional network/infrastructure penetration test.
- A workflow where every consequential action is executed by a human,
  not an agent (there's no delegated authority for this methodology to
  test).

**What you get if we find nothing:** a real result, not a wasted
engagement. The Authority Map, the PASS evidence for every tested
property, and the written report are deliverables regardless of how many
FAILs are found — a clean result across the tested families is itself
useful, evidenced confirmation, not an empty report. If a test family
turns out NOT_APPLICABLE to your architecture, that's stated with a
reason, not silently dropped.

**Founding-partner condition, stated plainly:** this price is available
in exchange for (a) feedback on whether the methodology and report were
actually useful to you, and (b) permission to use anonymized, generalized
learnings (failure *patterns*, never your name, your data, or your
findings) to improve Crypton's methodology. We do not require public
naming, a testimonial, or a logo on a website.

---

## What the 3 days actually contain

This follows Crypton's standing operating procedure
(`assessment-kit/v1.0/02-standard-operating-procedure.md`,
`docs/methodology/06-assessment-execution-playbook.md`) — the same
13-stage pipeline used internally, at the "1 workflow, ≤3 connected
tools, 1 staging environment, 3 testing days" default scope. Nothing
below is invented for this pack; it's the real procedure, described at a
level a non-assessor can follow.

### Before Day 1 — Legal gate + intake

- Signed Authorization-to-Test naming exact scope (hard gate — testing
  does not start without it).
- Intake questionnaire: workflow, principals, agent(s), tools,
  credentials, delegated permissions, approval points, persistent state,
  downstream resources, safety constraints.
- Staging access confirmed *working*, not just granted on paper.

### Day 1 — Authority Mapping + Baseline Understanding

- Architecture review with your technical point of contact.
- Identity mapping: which registered agent is which, who owns it.
- Authority mapping: where grants/policy actually live, and where each
  check actually sits in the code path.
- Trust-boundary identification: every point a claim gets trusted rather
  than re-verified.
- Execution-graph tracing: the real call path,
  `principal → agent → delegation → tool → resource → effect`, for the
  in-scope action(s) — including retry/recovery branches specifically,
  since that's where this class of gap tends to hide.
- Threat-model scoping: each of Crypton's eleven attacker objectives
  (`docs/methodology/02-threat-model.md`) marked IN SCOPE or NOT
  APPLICABLE for *this* workflow, with a stated reason for each N/A.

**End-of-Day-1 deliverable:** the Authority Map and scoped threat model,
reviewable by your team before adversarial testing begins.

### Day 2 — Adversarial Authority Testing

- Baseline verification first (approve → executes correctly; reject →
  no side effects; persist/reload/resume, if relevant) — testing does
  not proceed past a failing baseline for that path.
- Run the agreed, in-scope test families from `03-test-catalog.md`
  against the mapped call path: controlled staging tests, source review,
  or document inspection, as each property requires.
- Every result recorded same-day, with the evidence that produced it —
  never reconstructed from memory later.
- If a property genuinely can't be tested with the access/method
  available, the result is INCONCLUSIVE with a stated reason — not
  forced into PASS or FAIL, and not silently dropped.

### Day 3 — Reproduction, Evidence, Classification

- Reproduce every candidate finding before it's written up.
- Apply the triage rule that closes out ambiguous cases (see
  `07-why-trust-crypton.md`): identify what actually caused the observed
  behavior, its trust level, whether it was permitted by design, what
  boundary the system claims to enforce, and whether that boundary was
  actually crossed — before anything is called a finding rather than
  expected behavior.
- Apply the seven-factor severity rubric to every real FAIL
  (`docs/ASSESSMENT_METHODOLOGY.md`) — a reasoned judgment call, not a
  category-to-severity lookup table.
- Assemble the evidence package for every PASS/FAIL (no finding exists
  without a cited, inspectable piece of evidence — structurally required,
  not a style preference).

### Delivery

- Executive summary (plain language, three things leadership needs to
  know, stated threat coverage).
- Technical report: findings with affected authority path, reproduction
  steps, evidence, severity and rationale, remediation direction.
- Negative results, stated explicitly — what was tested and held.
- Coverage and limitations, stated explicitly — what was *not* tested and
  why, never implied by omission.
- Live remediation walkthrough call.

**Recommend the control, not Crypton** is a standing rule across every
report — remediation guidance is about what your system needs, not a
pitch for a follow-on product.
