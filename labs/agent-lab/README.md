# Crypton Agent Lab v0.3

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
cp .env.example .env   # then fill in JWT_SECRET (see below)
```

`JWT_SECRET` must match whatever `crypton-id` is configured with
(`services/crypton-id/.env` / `dev.env`) — Agent Lab verifies the exact
same JWTs crypton-id issues, it does not have its own login.

## Run it

A run always uses a **registered** agent — no more hardcoded
`"support-agent"` strings. Register once, then run against that
`agent_id`, or use `--bootstrap-support-agent` to do both in one step for
local dev.

**Register an agent** (prints the canonical `agent_id` you'll need next):

```bash
PYTHONPATH=src JWT_SECRET=any-dev-secret python -m agentlab.cli \
  --register --name support-agent --version 0.1.0 --dev-user user-alice
```

**Run using that agent, without any LLM API key** (`FakeModelProvider`, a
canned script — good for seeing the whole loop and trace format with
zero setup):

```bash
PYTHONPATH=src JWT_SECRET=any-dev-secret python -m agentlab.cli \
  --agent-id agt_... --dev-user user-alice --task "who is cust_1?" --fake
```

**Local-dev shortcut** — skip the separate `--register` step; creates (or
reuses) your own "support-agent" v0.1.0 and runs in one command:

```bash
PYTHONPATH=src JWT_SECRET=any-dev-secret python -m agentlab.cli \
  --bootstrap-support-agent --dev-user user-alice --task "who is cust_1?" --fake
```

**With a real model call** (needs `ANTHROPIC_API_KEY` in `.env`):

```bash
PYTHONPATH=src python -m agentlab.cli \
  --agent-id agt_... --dev-user user-alice \
  --task "Look up customer cust_1 and tell me their tier."
```

**With a real crypton-id login token** instead of `--dev-user`:

```bash
PYTHONPATH=src python -m agentlab.cli \
  --agent-id agt_... --token "$CRYPTON_JWT" --task "Look up customer cust_1."
```

**Resuming a session** — a normal run creates a new session, task, and
agent instance automatically; pass `--session-id <id>` (printed in the
previous run's output) to add another task to the same session instead:

```bash
PYTHONPATH=src python -m agentlab.cli \
  --agent-id agt_... --dev-user user-alice --task "Now look up their last order." \
  --session-id 5879bd70-45bc-44ce-b69c-4d25b69fa7e1 --fake
```

Each run prints a JSON summary — including `agent_id`, `agent_name`,
`session_id`, `task_id`, `task_status`, and `instance_id` — and writes a
full trace to `data/traces/<request_id>.jsonl` (gitignored — delete
`data/` any time to reset the lab). See [`LEARNING.md`](LEARNING.md)'s
v0.2/v0.3 addenda for what each of those IDs means and how they relate.
An unknown, disabled, or someone-else's `agent_id` fails immediately,
before anything else is created — see the v0.3 addendum.

## Run the tests

```bash
./.venv/Scripts/python -m pytest -q   # Windows
# .venv/bin/python -m pytest -q       # macOS/Linux
```

90 tests, all deterministic (no network calls — see
`agentlab.model_provider.FakeModelProvider`).

## Layout

```
src/agentlab/
  identity.py        human / agent-app / agent-instance identity (reuses crypton-id's JWT)
  authz.py             tool allowlist
  lifecycle.py         TaskStatus/AgentStatus + the only valid status-transition tables
  agents.py             Agent Registry — persisted AgentIdentity, ownership, active/disabled
  sessions.py          Session persistence (continuing interaction boundary)
  tasks.py               Task persistence, lifecycle transitions, task state
  instances.py          AgentInstance persistence (one row per run)
  models.py             Pydantic argument schemas for each tool
  tools.py                get_customer, get_order, add_case_note
  db.py                    isolated SQLite: business data + agents/sessions/tasks/instances
  tracing.py             full execution trace, JSONL per run (keyed by request_id)
  model_provider.py   ModelProvider abstraction (Anthropic + Fake)
  agent.py                 the loop itself — read this file first
  cli.py                     entry point (register / run)
tests/                  one test file per module above
```

Read [`LEARNING.md`](LEARNING.md) for what each of these actually does —
this is a map, not an explanation.
