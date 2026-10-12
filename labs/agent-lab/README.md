# Crypton Agent Lab v0.6.1

The smallest real AI-agent system that lets us learn — and later
security-test — the full chain:

```
human identity -> agent instance -> tool call -> resource/action -> trace
```

This is **not** the agent-security scanner. It's the thing that scanner
will eventually run attacks against. See [`../README.md`](../README.md)
(top of `labs/`) for how this fits into the rest of the cleanup, and
[`LEARNING.md`](LEARNING.md) for how it actually works.

No LangChain/LangGraph/CrewAI, no MCP, no multi-agent orchestration —
just a handwritten loop, three tools, and a real identity boundary
reused from `crypton-id`.

## Setup

```bash
cd labs/agent-lab
python -m venv .venv
./.venv/Scripts/pip install -r requirements.txt   # Windows
# source .venv/bin/activate && pip install -r requirements.txt   # macOS/Linux
cp .env.example .env   # then fill in JWT_PUBLIC_KEY_PEM (see below) — only needed for --token
```

JWTs are asymmetric (Ed25519/EdDSA) since v0.5.1 — see `../../DECISIONS.md`.
`JWT_PUBLIC_KEY_PEM` must be the SAME public key `crypton-id` is
configured with (`services/crypton-id/.env` / `dev.env`'s
`JWT_PUBLIC_KEY_PEM_FILE` — copy that file's contents in). Agent Lab
verifies the exact same JWTs crypton-id issues; it does not have its own
login, and it never holds crypton-id's private signing key. You only need
this set to use `--token <a real crypton-id JWT>`; `--dev-user` (below)
needs no configuration at all — it mints and verifies its own throwaway
keypair per invocation.

## Run it

A run always uses a **registered** agent, acting under an **explicit
Delegation** — no more hardcoded `"support-agent"` strings, and no more
standing, name-keyed permissions (v0.3's `authz.REGISTRY` is gone; see
[`LEARNING.md`](LEARNING.md)'s v0.4 addendum). Register once, grant a
delegation once, then run against that `agent_id` + `delegation_id` — or
use `--bootstrap-support-agent --bootstrap-delegation` to do all three in
one step for local dev.

**Register an agent** (prints the canonical `agent_id` you'll need next):

```bash
PYTHONPATH=src python -m agentlab.cli \
  --register --name support-agent --version 0.1.0 --dev-user user-alice
```

**Grant it a delegation** — explicit, finite-lived authority over
specific tools and resources (prints the canonical `delegation_id` you'll
need next):

```bash
PYTHONPATH=src python -m agentlab.cli \
  --grant-delegation --agent-id agt_... --dev-user user-alice \
  --tools get_customer,get_order,add_case_note --customer-ids cust_1 --order-ids ord_1
```

**Run using that agent + delegation, without any LLM API key**
(`FakeModelProvider`, a canned script — good for seeing the whole loop
and trace format with zero setup):

```bash
PYTHONPATH=src python -m agentlab.cli \
  --agent-id agt_... --delegation-id dlg_... --dev-user user-alice --task "who is cust_1?" --fake
```

**Local-dev shortcut** — skip the separate `--register`/`--grant-delegation`
steps; creates (or reuses) your own "support-agent" v0.1.0, grants it a
fresh delegation scoped to the seeded demo data, and runs, in one command:

```bash
PYTHONPATH=src python -m agentlab.cli \
  --bootstrap-support-agent --bootstrap-delegation --dev-user user-alice --task "who is cust_1?" --fake
```

**With a real model call** (needs `ANTHROPIC_API_KEY` in `.env`):

```bash
PYTHONPATH=src python -m agentlab.cli \
  --agent-id agt_... --delegation-id dlg_... --dev-user user-alice \
  --task "Look up customer cust_1 and tell me their tier."
```

**With a real crypton-id login token** instead of `--dev-user`:

```bash
PYTHONPATH=src python -m agentlab.cli \
  --agent-id agt_... --delegation-id dlg_... --token "$CRYPTON_JWT" --task "Look up customer cust_1."
```

**Resuming a session** — a normal run creates a new session, task, and
agent instance automatically; pass `--session-id <id>` (printed in the
previous run's output) to add another task to the same session instead:

```bash
PYTHONPATH=src python -m agentlab.cli \
  --agent-id agt_... --delegation-id dlg_... --dev-user user-alice --task "Now look up their last order." \
  --session-id 5879bd70-45bc-44ce-b69c-4d25b69fa7e1 --fake
```

Each run prints a JSON summary — including `agent_id`, `delegation_id`,
`agent_name`, `session_id`, `task_id`, `task_status`, and `instance_id`
— and writes a full trace to `data/traces/<request_id>.jsonl` (gitignored
— delete `data/` any time to reset the lab). See [`LEARNING.md`](LEARNING.md)'s
v0.2/v0.3/v0.4 addenda for what each of those IDs means and how they
relate. An unknown, disabled, or someone-else's `agent_id` fails
immediately, before anything else is created (v0.3); a missing, expired,
revoked, or mismatched delegation fails the specific tool call it was
needed for, with the denial persisted as an `AuthorizationDecision`
(v0.4).

## Run the tests

```bash
./.venv/Scripts/python -m pytest -q   # Windows
# .venv/bin/python -m pytest -q       # macOS/Linux
```

All tests are deterministic (no network calls — see
`agentlab.model_provider.FakeModelProvider`). Verified passing in full as
of this cleanup (2026-09-08).

## v0.6: verify a multi-hop delegation chain

```bash
# 1. register three agents (a human operator drives every hop — see LEARNING.md's v0.6 addendum on why)
python -m agentlab.cli --register --name agent-a --dev-user user-alice   # -> agt_A
python -m agentlab.cli --register --name agent-b --dev-user user-alice   # -> agt_B
python -m agentlab.cli --register --name agent-c --dev-user user-alice   # -> agt_C

# 2. give the two agents that will *issue* authority a signing identity (agent-c never issues, doesn't need one)
python -m agentlab.cli --create-signing-key --agent-id agt_A --dev-user user-alice > key_a.json   # save private_key_pem!
python -m agentlab.cli --create-signing-key --agent-id agt_B --dev-user user-alice > key_b.json

# 3. D0: alice -> agent-a (a normal, root grant — same as v0.5)
python -m agentlab.cli --grant-delegation --agent-id agt_A --tools get_customer,get_order,add_case_note \
    --customer-ids cust_1,cust_2 --ttl-seconds 10800 --dev-user user-alice   # -> dlg_D0

# 4. D1: agent-a -> agent-b, narrowed (drops add_case_note)
python -m agentlab.cli --issue-chained-delegation --issuer-agent-id agt_A --issuer-signing-key-id ask_A \
    --issuer-private-key-file priv_a.pem --parent-delegation-id dlg_D0 --subject-agent-id agt_B \
    --tools get_customer,get_order --customer-ids cust_1,cust_2 --ttl-seconds 7200 --dev-user user-alice   # -> dlg_D1

# 5. D2: agent-b -> agent-c, narrowed further (get_customer only, cust_1 only)
python -m agentlab.cli --issue-chained-delegation --issuer-agent-id agt_B --issuer-signing-key-id ask_B \
    --issuer-private-key-file priv_b.pem --parent-delegation-id dlg_D1 --subject-agent-id agt_C \
    --tools get_customer --customer-ids cust_1 --ttl-seconds 3600 --dev-user user-alice   # -> dlg_D2

# 6. prove the chain
python -m agentlab.cli --chain-verify dlg_D2 --requested-tool get_customer \
    --requested-resource-type customer_ids --requested-resource-id cust_1 --dev-user user-alice
```

Prints ROOT PRINCIPAL / ACTING PRINCIPAL / DELEGATION DEPTH / AUTHORITY
PATH / DECISION — `ALLOW` for the request above; try `--requested-tool
add_case_note` (never re-granted past D0) or `--requested-resource-id
cust_2` (narrowed away by D2) to see a `DENY` with its `FAILED HOP` and
`REASON`. Add `--format json` for a machine-readable version of the same
result (a thin serialization of the real `ChainVerificationResult`, useful
as evidence in an assessment engagement's manifest — see below). See
`DECISIONS.md`'s v0.6 section and `docs/CRYPTON_TECHNICAL_SOT_v1.1.md` §11
for the full design record and attack-case matrix.

## v0.6.1: the Agent Authorization & Delegation Assessment delivery kit

This lab's security primitives (delegation, attenuation, approval,
enforcement) are the *evidence source* an assessment cites — they are not
themselves a customer-assessment tool. See
[`docs/ASSESSMENT_METHODOLOGY.md`](../../docs/ASSESSMENT_METHODOLOGY.md)
for the actual eight-area procedure, and
[`agentlab.assessment`](src/agentlab/assessment/__init__.py) for the small
Finding/Evidence/Report package that turns a completed assessment into a
client-deliverable Markdown report:

```bash
# Render an already-built engagement (a directory of scope/workflow/
# evidence/findings files — see docs/ASSESSMENT_METHODOLOGY.md and
# assessment-engagement/TEMPLATE/) to a Markdown report:
python -m agentlab.cli --render-assessment ../../assessment-engagement/reference-agentlab-dryrun --out report.md
```

`--render-assessment` renders data that already exists — it does not
inspect, connect to, or assess anything itself. See
[`assessment-engagement/reference-agentlab-dryrun/`](../../assessment-engagement/reference-agentlab-dryrun/)
for a fully worked, explicitly-labeled **synthetic** example
("Crypton Reference Agent Lab — Methodology Dry Run") — built from this
lab's own real primitive calls, never from a real customer engagement —
demonstrating all four outcome states (PASS/FAIL/INCONCLUSIVE/NOT_APPLICABLE)
across all eight assessment areas. Regenerate it with:

```bash
python -m agentlab.assessment.reference_dryrun
```

## Layout

```
src/agentlab/
  identity.py        human / agent-app / agent-instance identity (reuses crypton-id's JWT)
  lifecycle.py         TaskStatus/AgentStatus/DelegationStatus/SigningKeyStatus + the only valid status-transition tables
  agents.py             Agent Registry — persisted AgentIdentity, ownership, active/disabled
  signing_keys.py    Agent signing identity (Ed25519 keypair per agent that issues authority; v0.6)
  delegation.py       Delegation — explicit, finite-lived authority grant (v0.4), now chainable + signable (v0.6)
  chain.py                 scope-attenuation engine + the one authoritative multi-hop chain verifier (v0.6)
  authorization.py  deterministic pre-execution authorization pipeline + AuthorizationDecision evidence (v0.4)
  approval.py           Approval — exact-argument-bound human confirmation via WebAuthn step-up (v0.5)
  webauthn.py           WebAuthnVerifier interface + FakeWebAuthnVerifier (Lab-only simulation; v0.5)
  sessions.py          Session persistence (continuing interaction boundary)
  tasks.py               Task persistence, lifecycle transitions, task state
  instances.py          AgentInstance persistence (one row per run)
  models.py             Pydantic argument schemas for each tool
  tools.py                get_customer, get_order, add_case_note
  db.py                    isolated SQLite: business data + agents/delegations/agent_signing_keys/authorization_decisions/sessions/tasks/instances
  tracing.py             full execution trace, JSONL per run (keyed by request_id)
  model_provider.py   ModelProvider abstraction (Anthropic + Fake)
  agent.py                 the loop itself — read this file first
  cli.py                     entry point (register / grant-delegation / create-signing-key / issue-chained-delegation / chain-verify / render-assessment / run)
  assessment/            Assessment Delivery Kit (v0.6.1) — Finding/Evidence/Report models + report
                                 renderer; no security logic of its own — see docs/ASSESSMENT_METHODOLOGY.md
tests/                  one test file per module above
```

Read [`LEARNING.md`](LEARNING.md) for what each of these actually does —
this is a map, not an explanation.
