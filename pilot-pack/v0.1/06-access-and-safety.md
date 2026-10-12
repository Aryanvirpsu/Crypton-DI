# Pilot Access & Safety

This document exists to make one thing concrete: **what, exactly, are you
being asked to give Crypton access to, and what won't Crypton do with
it.** Read this before the intake call — it should make the access
question smaller, not bigger.

---

## Three engagement modes

Only offered where the methodology can actually support it — Crypton
does not offer a mode it can't realistically deliver against.

### Mode 1 — Customer-Controlled Assessment (the default first engagement)

Crypton asks for **no credentials, no production access, and no source
repository.** The Day 1 authority-mapping call produces the same Authority
Map as any other mode; from it, Crypton and your technical point of
contact jointly define the specific tests for your workflow (drawn from
`03-test-catalog.md`'s Core four). **You execute them** — either
independently and hand over the resulting traces/evidence, or, more
commonly, over a shared screen so Crypton can guide the exact steps in
real time without ever touching your systems directly. Crypton then
applies its classification pipeline —
`architecture → authority claim → test → evidence → classification` — to
what you produced, and writes the report from that.

- Crypton never holds a credential, a session, or a line of your source
  outside the shared screen.
- This changes the actual question a security lead has to answer from
  *"why would I let an unproven outside party operate inside my agent
  infrastructure"* to *"am I willing to spend part of a day showing this
  person one authorization workflow, on my own screen, under my own
  control"* — a categorically smaller ask.
- Best fit for a first engagement with any new assessor, not only
  Crypton, and for environments with compliance, air-gapping, or data
  sensitivity constraints that make external access impractical.
- Trade-off: slower iteration than Crypton driving tests directly, and
  chasing an unexpected result live depends on your point of contact's
  availability during the window.
- If something genuinely needs direct testing to resolve — a result that
  only reproduces with timing or state your side can't easily replicate
  live — Crypton requests **temporary, scoped, staging-only access
  (Mode 2)** for that specific gap, not as a silent default.

### Mode 2 — Staging Assessment (when direct access adds real value)

Crypton receives **temporary, scoped, staging-only** access to the
in-scope workflow. No production access, ever, under this mode.

- Access is provisioned for the engagement window only (typically the 3
  testing days plus a short buffer), on test accounts and staging
  credentials created for this purpose — never your own personal or
  production credentials.
- Scope is exactly the workflow and up to three connected tools agreed
  in intake — not broader system access "while we're in there."
- Available from the start of an engagement if a customer prefers it over
  Mode 1, or invoked mid-engagement for a specific gap Mode 1 couldn't
  resolve.

### Mode 3 — Observed Live Workflow (only if explicitly authorized)

Only used when a customer specifically wants a specific action observed
in a live (not staging) context, and explicitly authorizes it in the
Authorization-to-Test.

- A customer operator remains present and in control for the entire
  observed session.
- No action is taken without that operator's explicit, in-the-moment
  authorization — Crypton does not act unilaterally against a live
  system under any mode.
- No destructive or consequential action is taken under this mode
  without the operator's explicit authorization for that specific
  action, in addition to the general Authorization-to-Test.
- This mode is rare in a first, founding-rate engagement — Mode 1 is
  the default and Mode 2 the fallback; Mode 3 should not be assumed available
  without a specific reason it's needed.

---

## Rules that apply across every mode

- **Least privilege, always.** Access is scoped to exactly the in-scope
  workflow and its connected tools — never broader "just in case."
- **Temporary credentials only.** Nothing standing, nothing permanent.
  Credentials are created for the engagement window and revoked at its
  close — see "After the engagement," below.
- **Customer-controlled execution preferred (Mode 1); staging only under Mode 2, production only under Mode 3, with explicit,
  per-action authorization.**
- **No credential retention.** Crypton does not keep copies of any
  credential, key, or token after the engagement closes.
- **Agreed test accounts and resources only.** Testing uses accounts and
  data created or designated for this purpose (synthetic data preferred)
  — not real customer/user data unless there is no staging alternative
  and the customer explicitly authorizes it.
- **No production destructive actions**, under any mode, without
  explicit, specific, per-action authorization.
- **No persistence.** Crypton does not install standing agents, cron
  jobs, backdoors, monitoring hooks, or anything that continues to run
  or collect data after the engagement window closes.
- **No lateral exploration outside scope.** If testing surfaces something
  interesting outside the agreed workflow/tools, it's reported as a
  scoping note for a future conversation — not pursued unilaterally.
- **Immediate stop conditions.** Testing stops immediately, and the
  customer's technical point of contact is notified immediately, if: a
  test produces an unexpected effect on anything outside the declared
  scope; a staging environment turns out to share state or credentials
  with production in a way not previously disclosed; or the customer
  asks for a pause, for any reason, at any time.
- **Evidence handling.** Evidence collected (traces, logs, config
  excerpts, screenshots) is used only to produce the report and is
  shared only with the customer — responsible disclosure, no third
  party, no public use, ever, of a customer's specific findings.
  Anonymized, generalized *patterns* (not customer data or specific
  findings) may be used to improve Crypton's methodology only with the
  customer's separate agreement (see the founding-partner condition in
  `02-founding-assessment.md`).
- **Credential revocation after engagement.** Every temporary credential,
  account, and access grant created for the engagement is revoked or
  deleted at its close, confirmed in writing to the customer.

## Legal gate (before any of the above)

No testing begins — under any mode — before: (1) the engaging entity is
confirmed, (2) a signed Authorization-to-Test naming the exact target
systems, environment, testing window, and methods is on file, and (3)
the customer has confirmed they own or are authorized to permit testing
of the named target. This is a hard block, not a formality to route
around under time pressure.

## Pre-engagement checklist

See `10-pre-engagement-checklist.md` for the concrete, fillable version
of everything above.
