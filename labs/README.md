# Crypton Agent Lab

**Status: v0.4 built** (v0.1 shipped first — see git history — then
v0.2 added explicit Sessions, Tasks, and persistent task state, then
v0.3 added the Agent Registry — real, owned, persisted Agent
registrations replacing hardcoded agent-identity strings — then v0.4
added Delegation and a deterministic pre-execution authorization
pipeline, replacing v0.1-v0.3's static, name-keyed permission map). See
[`agent-lab/README.md`](agent-lab/README.md) to run it and
[`agent-lab/LEARNING.md`](agent-lab/LEARNING.md) for how it works. The
plan below is what v0.1 actually built; v0.2/v0.3/v0.4's additions are
documented in `agent-lab/LEARNING.md`'s addenda, not re-planned here.

## What this cleanup did NOT build

The previous state of this repo had UI surfaces (`AttackSim.js`, a fake
`/risk/scan` endpoint, a "Risk Intelligence" placeholder page) that implied
some kind of security-testing or agent capability existed. None of it was
real — see the root `README.md`'s "Removed" section for what was taken out
and why. This directory is where a *real* version of that direction starts,
once it's actually built.

## Crypton Agent Lab v0.1 — built (`agent-lab/`)

```
authenticated human
      ↓
AI agent
      ↓
3 tools
      ↓
explicit state
      ↓
full execution traces
```

A human's identity comes from the same JWT `crypton-id` issues via its real
passkey flow; a handwritten agent loop (no framework) verifies it, then
executes a scoped task using exactly three tools (deliberately minimal —
this is a lab for understanding delegation and authorization, not a general
agent framework). Every state transition is explicit and every tool call is
captured in a full execution trace, because the entire point of this lab is
to have something concrete to run agent-identity and agent-security testing
against later. See `agent-lab/LEARNING.md` for how it actually works.

**What it reuses from the existing foundation, as planned:**
- Real user identity + JWT (`crypton-id`) — `agent-lab` verifies the exact
  same JWT locally (same secret, same claims), rather than inventing a
  parallel identity system.
- Request correlation IDs — each agent run mints its own instance/run ID
  (`identity.AgentInstance`), the same correlation-ID concept
  `crypton-gateway`'s `x-request-id` already uses, just scoped to a whole
  agent run instead of one HTTP hop.

**Deliberately not wired up yet** (still future integration, not needed for
v0.1's learning goal): the step-up verification primitive
(`crypton-id/src/routes/actions.rs`, `crypton.actions.sign()`) for
per-action human approval, and the gateway's marked extension point
(`services/crypton-gateway/src/main.rs`) — `agent-lab` runs as a standalone
CLI today, not yet behind the gateway.

**Still explicitly out of scope:** LangChain/LangGraph/CrewAI or similar
orchestration frameworks, MCP, multi-agent orchestration, RAG, long-term
memory, and any actual agent-security test logic (adversarial testing,
prompt injection testing, permission/delegation attack simulation). None of
that exists anywhere in this repo yet — do not build it speculatively ahead
of the next lab phase.
