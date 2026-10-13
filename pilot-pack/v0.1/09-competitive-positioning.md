# Competitive Positioning

`01-positioning.md`'s technical version already distinguishes Crypton
from adjacent categories by *job*. This document goes one level further:
where the overlap actually is, and — more importantly — where Crypton is
honestly redundant and you shouldn't buy it.

## By primary job

| Category | Primary job |
|---|---|
| IAM / agent identity platform | Establish identities, issue credentials, manage access |
| Authorization engine (OPA, Cedar, custom RBAC/ABAC) | Decide, given a request, whether it's allowed |
| Agent / MCP gateway | Mediate and log tool access at a proxy point |
| Generic penetration test | Find exploitable weaknesses in a system's exposed surface |
| AI red teaming | Probe model/system behavior under adversarial input |
| **Crypton** | **Verify that authority/delegation/approval properties actually survive real agent execution — retries, delegation, persistence, revocation** |

## Where the overlap actually is

- **IAM / agent identity:** genuinely adjacent. IAM establishes *what a
  principal is allowed to do in principle*. Crypton tests whether that
  permission is *what actually gets checked* at every point an agent
  acts on it — including paths (a retry, a secondary dispatch route) the
  IAM system's own policy definition never sees, because nothing routed
  the request through it. A correctly-configured IAM system can still
  sit behind a dispatch path that doesn't call it.
- **Authorization engines:** same shape of overlap, one layer down. A
  policy engine returning the correct decision is not the same claim as
  "every code path that executes the protected action actually calls the
  policy engine before doing so" — Family 3 in `03-test-catalog.md` is
  precisely the second question, not the first.
- **Agent / MCP gateways:** a gateway is itself a *control* Crypton would
  test, not a substitute for testing it. The honest overlap: if a
  gateway's own vendor already adversarially tests "does every real path
  to a protected tool actually route through us," that specific claim
  may already be covered — ask them directly whether they do, and how
  they evidence it, before assuming a gap exists.
- **Generic pentest:** true overlap is thin. A pentest's target is the
  system's exposed perimeter; Crypton's target is authority that's
  already, legitimately, inside the system (an agent that's supposed to
  have some access) and asks whether it stays where it's supposed to.
  Some pentest firms are starting to offer agent-specific modules — where
  that module specifically covers retry/delegation/approval-binding
  testing with the same evidence standard, it may already cover this
  ground; most current generic-pentest scopes don't.
- **AI red teaming:** near-zero overlap in method, real overlap in
  subject. Red teaming asks whether adversarial input changes model
  behavior; Crypton assumes the model's behavior (including a naive,
  non-adversarial retry) as a given and asks whether the *authority
  plumbing around it* holds. The two are complementary, not competing —
  an engagement that wants both should scope both, separately, because
  they're different disciplines with different evidence standards.

## When Crypton is genuinely redundant

Said plainly, because an honest "when you don't need this" section is
part of what makes the rest of this pack credible:

- **You already run equivalent adversarial authorization testing**
  against your own agent retry/delegation/approval paths specifically —
  not just standard AppSec/pentest scope, but this exact class of
  question, with comparable evidence standards. If your team can already
  produce `03-test-catalog.md`'s PASS/FAIL evidence for your own
  workflow, an external assessment adds confirmation, not new
  information.
- **Your workflow has no meaningfully delegated agent authority** — a
  human executes every consequential action, and the agent only drafts,
  suggests, or reads. There's no authority-in-motion for this
  methodology to test.
- **All actions in scope are manually executed**, with the agent as an
  assistant rather than an actor — same reasoning as above, different
  phrasing.
- **Your control vendor (IAM, gateway, policy engine) already supplies
  independent, adversarial verification of exactly this property** — not
  just documentation of the control's intended design, but evidence
  someone actually tried to defeat it the way Crypton would. Ask for
  that evidence before assuming you need a separate assessment.

## When Crypton is a genuine fit

The inverse of the above: an agent that takes real, consequential
actions, with some delegated authority and/or a human-approval step, in
a workflow with retry/recovery logic or persisted state — where nobody
on your team can currently produce evidence, one way or the other, for
whether a retry preserves or exceeds what was approved. See "Ideal pilot
target" in `02-founding-assessment.md`.
