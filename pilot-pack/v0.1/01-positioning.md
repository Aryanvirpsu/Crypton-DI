# Crypton — Positioning

**Read this first if you have two minutes. Read `03-test-catalog.md` next if you have ten.**

---

## One sentence

> Crypton independently tests whether an AI agent's real, executed actions
> stay bound to the principal, delegation, approval, scope, and current
> authority state that were supposed to authorize them — before, during,
> and after execution.

## 30 seconds

Companies are giving AI agents real authority: to call internal tools, hit
production APIs, and take consequential actions — isolate a host, issue a
refund, modify a record — sometimes with a human approval step, sometimes
without.

IAM and authentication answer *who this is* and *what they're allowed to
do in principle*. They do this well, and Crypton does not try to replace
them. What they don't automatically answer is what happens to that
authority once it starts moving through an actual agent execution:
delegation to sub-agents, retries after a failure, a human approval that
has to survive to the exact action that executes, state that gets
persisted and resumed, and authority that's supposed to disappear the
moment it's revoked but might not.

Crypton adversarially tests whether the authority relationship your team
*designed* actually survives those transitions in your *specific*
workflow — and reports the result honestly, including when it survives.

## Technical version

**Scope, precisely:** Crypton assesses whether an agent's authority —
identity, delegated scope, human approval, and current authorization
state — remains correctly bound as it moves through one specific,
bounded, action-taking workflow. The unit of assessment is a call path:
`principal → agent → delegation → tool → resource → effect`, at every
hop that path takes, in the code and configuration as actually deployed,
not as documented.

**What this is not, and why it doesn't collapse into it:**

| Category | Its job | Why Crypton isn't a substitute for it, and isn't replaced by it |
|---|---|---|
| Penetration testing | Find exploitable weaknesses in a system's perimeter/network/app surface | A pentest asks "can I get in." Crypton assumes an agent is already inside, already holding some real authority, and asks whether that authority stays where it's supposed to as the agent acts. |
| IAM / identity provider | Establish identities, issue credentials, enforce login/session | Crypton doesn't issue or manage identities — it tests whether the identity and grant your IAM already issued is what's actually checked at every point your agent acts on it. |
| Policy/authorization engines (OPA, Cedar, custom RBAC/ABAC) | Decide, given a request, whether it's allowed | Crypton tests whether the *decision point* is actually on the path the action takes — a correct policy engine that a retry path bypasses is still a real gap, and that's a Crypton-shaped question, not a policy-engine bug. |
| Guardrails / content filters | Constrain model output content, tone, or topic | Orthogonal. A guardrail can be working perfectly while the approval binding underneath it is not — Crypton doesn't evaluate output quality or safety filtering at all. |
| Prompt-injection / jailbreak testing | Probe whether adversarial input manipulates model behavior | Explicitly out of scope for Crypton's lead assessment (see `docs/ASSESSMENT_METHODOLOGY.md`) — it's a real, different discipline, offered only as a separate add-on if a customer wants it, using a different methodology Crypton doesn't claim to own. |
| MCP/agent gateways | Mediate and log tool access at a proxy point | A gateway is a *control*. Crypton tests whether that control's guarantee (e.g., "nothing reaches this tool without passing through here") actually holds for every path the agent can take, including ones that don't cleanly route through the gateway. |
| Model safety evaluations | Assess model behavior in the abstract, off a benchmark | Crypton doesn't evaluate models. It evaluates one specific, deployed system's authority plumbing, model included only as one component in that call path. |

**What Crypton's edge actually is, stated precisely (not oversold):**
session binding, complete mediation, confused-deputy reasoning, and
TOCTOU on an approval are not concepts Crypton invented — they're
classical security theory, and Crypton doesn't claim otherwise (see
`09-competitive-positioning.md` and `11-pilot-pack-red-team.md` for the
full, unhedged version of this point). What Crypton does is **turn those
established principles into a specific, repeatable, agent-shaped
assessment across the full path a real agent workflow takes** —
`principal → agent → delegation → approval → tool → resource → effect` —
built and cross-tested against real agent frameworks before being
pointed at a customer, rather than reinvented ad hoc per engagement. This
is the same relationship a penetration-testing firm has to SQL
injection: nobody claims to have discovered the underlying vulnerability
class; the value is disciplined, specialized, repeated application of it
to a specific target. That's a commercially ordinary, and honest, basis
for a service.

**Why wouldn't a company just test this themselves?**

Sometimes they should, and Crypton says so — see "When Crypton is
redundant" in `09-competitive-positioning.md`. The honest case for an
external assessment, specific to this problem shape:

- **A structured, framework-independent property catalog**, built and
  cross-tested against three different real agent frameworks (OpenAI
  Agents SDK, Microsoft Agent Framework, LangGraph) before any customer
  engagement — not a checklist improvised per engagement
  (`docs/methodology/01-security-property-catalog.md`,
  `docs/methodology/04-framework-adaptation-guide.md`).
- **A standing discipline for telling a real gap from expected framework
  behavior**, applied and demonstrated against Crypton's own prior
  conclusions, not just claimed (see `07-why-trust-crypton.md` — the
  LangChain retraction).
- **An outside view of your own retry/delegation/persistence paths** —
  the paths this problem lives in are exactly the ones a team closest to
  the code is most likely to treat as "obviously fine" because they
  wrote them.
- **A documented, current, cross-vendor pattern of exactly this failure
  shape recurring** in shipped systems in 2026 — not a hypothetical risk
  category (`04-evidence-map.md`).
- **Narrow scope and reproducible evidence**, not a black-box report —
  every finding traces to a specific file, config, or observed trace, so
  your own engineers can independently confirm or dispute it.

**Where internal testing is probably enough, said plainly:** if your team
already runs adversarial authorization testing against agent retry/delegation
paths specifically (not just standard AppSec/pentest scope), if your
workflow has no meaningfully delegated agent authority (a human executes
every consequential step), or if your existing control vendor already
gives you independent, adversarial verification of this exact property
(not just documentation of the control's design) — you likely don't need
this assessment. Said the other way: if nobody on your team can currently
answer "does a retry preserve or exceed what was approved" with evidence,
that's the gap this assessment exists for.
