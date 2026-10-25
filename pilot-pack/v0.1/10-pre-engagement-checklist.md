# Pre-Engagement Checklist

A fillable version of `06-access-and-safety.md`. Complete before the
scoping call ends; everything here should be resolved before Day 1 is
scheduled.

## Legal gate

- [ ] Engaging entity confirmed (both sides).
- [ ] Authorization-to-Test signed, naming: exact target system(s),
      exact environment (staging/production), exact testing window,
      exact methods in scope.
- [ ] Customer has confirmed ownership/authorization to permit testing of
      the named target.

## Scope

- [ ] One workflow named, specifically (not "our agent platform" broadly).
- [ ] Up to three connected tools named specifically.
- [ ] Engagement mode selected: 1 (customer-controlled, default — see
      `06-access-and-safety.md`), 2 (staging), or 3 (observed live —
      only with explicit, separate authorization).
- [ ] Out-of-scope systems/data explicitly listed, not left implicit.
- [ ] Prompt-injection/jailbreak testing explicitly excluded, unless
      separately scoped as an add-on with its own methodology.

## Access (Mode 2/3 only — Mode 1 needs no credentialed access at all)

- [ ] Staging (or, for Mode 3, live) access provisioned and **confirmed
      working** before Day 1 — not just "granted on paper."
- [ ] Access is scoped to exactly the named workflow/tools — nothing
      broader.
- [ ] Credentials are temporary, created for this engagement, not a
      reused personal or production credential.
- [ ] Test accounts and/or synthetic data are designated — real
      customer/user data is not used unless no staging alternative
      exists and this is explicitly authorized.
- [ ] Technical point of contact named and confirmed available for the
      full testing window.

## Safety

- [ ] Stop conditions understood by both sides (see
      `06-access-and-safety.md`) — who to contact, and that testing
      halts immediately if triggered.
- [ ] No destructive production action will be taken without a separate,
      specific, per-action authorization (Mode 3 only).
- [ ] Confirmed: staging does not share live credentials, state, or a
      production data store with the production system, or if it does,
      that fact is disclosed and accounted for in scope.

## Deliverables & aftercare

- [ ] Report delivery date agreed.
- [ ] Remediation walkthrough date/window agreed.
- [ ] Confirmed: all engagement-specific credentials/accounts will be
      revoked at engagement close, with written confirmation.
- [ ] Founding-partner condition acknowledged (feedback +
      anonymized-learnings permission — see `02-founding-assessment.md`);
      no public naming/testimonial required.

## Intake questionnaire (send ahead of Day 1)

- [ ] Workflow description (what the agent does, end to end).
- [ ] Principal(s) — who/what initiates the workflow.
- [ ] Agent(s) — how many, how provisioned, how identified.
- [ ] Tools the agent can call, and what each does.
- [ ] Credentials/permissions the agent holds, and how they're scoped.
- [ ] Delegated permissions, if any (agent-to-agent, agent-to-sub-agent).
- [ ] Human-approval point(s), if any — what triggers them, what's shown
      to the approver, what happens on approve/reject.
- [ ] Persistent/checkpointed state, if any — what's saved, when it's
      resumed.
- [ ] Downstream resources the workflow's actions ultimately affect.
- [ ] Any safety constraints Crypton must respect beyond the standard
      rules in `06-access-and-safety.md` (e.g., specific accounts/records
      that must never be touched, rate limits, compliance holds).
