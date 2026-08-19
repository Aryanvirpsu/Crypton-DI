# Learning notes: Crypton Agent Lab (v0.1 → v0.2 → v0.3)

This is the "read this to understand exactly how it works" document the
build was asked to produce. It assumes you've read `README.md` for setup
but not necessarily any of the source yet. Sections 1-10 describe v0.1
(the loop, tools, tracing, identity); the v0.2 addendum covers what
changed when Sessions, Tasks, and persistent task state were added; the
v0.3 addendum (at the very end) covers the Agent Registry — read those
last two first if you already know v0.1.

## Architecture

```
 human (WebAuthn login via crypton-id, unchanged)
   │
   │  JWT { sub, username, cred_id, iat, exp }
   ▼
 identity.verify_human_identity(token)
   │  -> HumanIdentity(user_id, username, cred_id)
   ▼
 AgentRunner(agent=AgentIdentity("support-agent"), model, db_conn)
   │
   │  .run(human, task)  ── creates a fresh AgentInstance (run id)
   │                         and a Tracer for this run
   ▼
 ┌─────────────────── agent loop (agent.py) ───────────────────┐
 │                                                              │
 │   for step in 1..max_steps:                                 │
 │                                                              │
 │     model.complete(system, messages, tool_specs) ───────────┼──▶ LLM
 │            │                                                │
 │            ├─ final text ──────────────────────────────▶ return
 │            │
 │            └─ tool_call { name, arguments }
 │                   │
 │                   ▼
 │             1. unknown tool?        -> reject
 │             2. authz.check_authorized(agent_id, tool)
 │                                     -> reject if not allowed
 │             3. Pydantic validate arguments
 │                                     -> reject if malformed
 │             4. tools.<fn>(db_conn, human, instance, args)
 │                   │
 │                   ▼
 │             SQLite, rows scoped by owner_user_id == human.user_id
 │                   │
 │             result or ToolError fed back as the next message
 │                                                              │
 │   every one of the above steps calls tracer.record(...)      │
 └──────────────────────────────────────────────────────────────┘
                        │
                        ▼
        data/traces/<instance_id>.jsonl  (one line per event)
```

## 1. What makes this an agent rather than a chatbot

A chatbot maps `messages -> text`. This system maps
`messages -> (text | tool call) -> tool execution -> messages -> ...`,
repeated until the model stops asking for tools or a step limit is hit.
The defining property is the **loop with an action in the middle**: the
model's output can change what happens in the world (read/write rows in
`db.py` through `tools.py`) before the model gets to respond again. A
chatbot never does anything between "receive input" and "produce output"
except think.

## 2. Where the agent loop lives

`src/agentlab/agent.py`, entirely in `AgentRunner.run()`. It's ~40 lines
of actual control flow (the `for step in range(...)` loop) plus a helper,
`_handle_tool_call`, that runs the four security boundaries in order.
Read that file top to bottom — it's the whole system in miniature; every
other module is a supporting piece it imports.

## 3. What exactly is sent to the model

Three things, every turn, and nothing else:

- **`system`** — a fixed string (`agent.SYSTEM_PROMPT`) telling it it's a
  support agent restricted to the given tools.
- **`messages`** — the conversation so far: the original task, and for
  each prior step either a tool call the model made or the tool's result
  (or error) fed back as the next turn.
- **`tools`** — a JSON-schema block built by
  `models.tool_specs_for_model()`, derived directly from the same
  Pydantic models (`TOOL_ARG_MODELS`) that validate incoming calls, so
  the model can never be told about a tool shape that isn't also
  enforced.

The model is **never** sent: `JWT_SECRET`, `ANTHROPIC_API_KEY`, the
`HumanIdentity.cred_id`, database connection details, or any other
environment/secret value. See section 10 for why that boundary matters.

## 4. How the model chooses a tool

It doesn't — Agent Lab doesn't implement or influence tool selection at
all. The model provider (Claude, via the Anthropic Messages API's native
tool-use / function-calling) decides which tool (if any) best matches the
task, based only on the tool name + description + JSON schema it was
given in `tools`. Agent Lab's job starts *after* that decision: taking
whatever the model asked for and deciding whether to honor it.

## 5. Who actually executes the tool

Never the model. `agent.py`'s `_handle_tool_call` looks up the actual
Python function in `tools.TOOL_FUNCTIONS` — but only after the tool name
has passed the unknown-tool check, the authorization check, and argument
validation. The model can *request* `get_customer(customer_id="cust_1")`;
it cannot make that request execute anything by itself. This is the same
separation `crypton-gateway` draws between "a request arrived" and "the
request is allowed to reach identity" — see
`docs/architecture.md`'s extension-point note.

## 6. Where state lives

*(Updated for v0.2 — see the addendum below for the full breakdown of
Session/Task/state/trace as four separate things.)*

Everything is local to this lab's SQLite file and disposable — not the
real Crypton Postgres database; deleting `data/agentlab.db` and
`data/traces/` resets the lab to a clean slate. Four different kinds of
state live there now, deliberately kept separate (see the v0.2 addendum,
"state boundaries"):

- **Business data** — customers, orders, case notes. Seeded by
  `db.seed()` with two owners (`user-alice`, `user-bob`) so isolation is
  testable.
- **Sessions and Tasks** (`sessions.py`, `tasks.py`) — a Session is a
  continuing interaction boundary; a Task is one specific objective
  within it, with a validated lifecycle status and a small, schema-
  validated `state` blob. New in v0.2 — v0.1 had neither.
- **AgentInstance rows** (`instances.py`) — one row per concrete
  execution/run of a Task, with start/end timestamps and status.
- **`data/traces/<request_id>.jsonl`** — the full execution trace for one
  call to `run()` (see §7 and the v0.2 addendum for what "one run" means
  now that it spans session/task/instance creation, not just tool calls).

There is still no long-term memory or vector store, and the
model-context conversation (`messages` in `agent.py`) still lives only in
a local Python list for the duration of one `run()` call and is discarded
after — it is not persisted anywhere, and is a different thing from Task
state (see the v0.2 addendum).

## 7. Human ID vs. agent ID vs. run ID

Three distinct types in `identity.py`, deliberately not collapsed into
one ID:

| Type | Example | Lifetime | Source |
|---|---|---|---|
| `HumanIdentity` | `user_id="a1b2..."` | one crypton-id login session | crypton-id JWT (`sub`, `username`, `cred_id`) — verified, not invented |
| `AgentIdentity` | `agent_id="support-agent"` | fixed at deploy time | registered in `authz.REGISTRY` |
| `AgentInstance` | `instance_id=<uuid4>` | one `AgentRunner.run()` call | minted fresh every run |

Why keep them separate: each answers a different question that
agent-security work will need to ask independently later — "which human
authorized this?", "which agent application is this?", and "which
specific execution, out of possibly many concurrent ones for the same
human+agent, did this?" Collapsing them (e.g. using one ID for both the
agent app and each run) would make it impossible to tell "the same agent
acting for two different humans at once" apart from "one run gone wrong,"
which is exactly the distinction an audit or an attack investigation
needs.

*(v0.2 extends `AgentInstance` to also reference `session_id`/`task_id`,
and adds `Session`/`Task` as two further, equally-distinct entities — see
the v0.2 addendum's "Session vs. Task vs. AgentInstance" section below.)*

## 8. Where authorization is checked

`agent.py`, `_handle_tool_call`, boundary 2 — `authz.check_authorized(
identity.name, call.name)` — called for **every** tool call, before
arguments are validated or the tool runs. The allowlist itself
(`authz.REGISTRY`) is a plain Python dict keyed by agent *name* (see the
v0.3 addendum, "why names aren't secure identities," for exactly why —
in short: it's static code, and only a stable label can be hardcoded
ahead of a per-owner random `agent_id`, but `identity.name` here is
never caller/model-supplied — it's resolved server-side from the
already-verified registered agent, moments earlier). An unregistered
agent name and a registered-but-not-allowed tool are both rejected the
same way (`AuthorizationError`), traced as `tool_rejected`, and reported
back to the model as a generic `"unauthorized"` error — the model is
told it can't do that, never *why* in enough detail to probe the
boundary.

Data-level authorization (which *rows* a human can see) is a second,
independent check inside `tools.py`: every query filters on
`owner_user_id == human.user_id`. Tool-level authz answers "can this
agent call this tool at all"; row-level authz answers "does this specific
data belong to this specific human." Both have to pass.

## 9. What currently happens if the model is malicious

Worth stating plainly, since this lab exists to make that answerable:

- **Model asks for a tool that doesn't exist** (`delete_customer`,
  `run_shell`, anything not in `models.TOOL_ARG_MODELS`) → rejected
  before any lookup happens, traced as `tool_rejected` /
  `unknown_tool`, loop continues.
- **Model asks for a real tool it isn't allowed to call**
  (`read-only-agent` requesting `add_case_note`) → rejected by
  `authz.check_authorized`, traced as `tool_rejected` / `unauthorized`.
- **Model sends malformed/extra/wrong-typed arguments** (missing
  `customer_id`, an extra field, a customer_id of the wrong type) →
  rejected by Pydantic (`extra="forbid"` catches surprise fields),
  traced as `tool_rejected` / `invalid_arguments`.
- **Model tries to reach another human's data** (asks for a
  `customer_id` that exists but belongs to someone else) → the tool
  itself returns `customer_not_found` — identical to the row not
  existing at all, so the model can't distinguish "wrong id" from
  "someone else's real id."
- **Model just keeps calling tools forever** → `max_steps` (default 6)
  cuts the loop off unconditionally; the run ends with
  `stopped_reason="max_steps"`, no answer, but no runaway execution
  either.
- **Model's response is genuinely garbled / the API call itself fails**
  → caught in `agent.py`'s `try/except` around `model.complete(...)`,
  traced as an `error` event, loop stops cleanly.

What a malicious model **cannot** currently do: name a tool that isn't
registered and have it run; bypass the allowlist by any means (there's no
"admin override" path in `authz.py`); run arbitrary Python, shell, or SQL
(tools.py has three fixed, parameterized functions — nothing takes
free-form code); or read another human's rows through any of the three
tools (row filtering is unconditional, not something the model can
disable by asking).

## 10. What attack surfaces this creates

This is a lab, not a hardened system — recording these honestly is the
point:

- **Argument-injection into `note` text** — `add_case_note`'s `note`
  field accepts up to 2000 characters of free text with no content
  filtering. It's stored and could later be *read back* by a human or
  fed into a future model call; there's no prompt-injection defense on
  data flowing back in. Not exploitable today (nothing re-reads notes
  into a prompt), but the seam exists.
- **The allowlist is static and coarse** — `authz.REGISTRY` grants a
  whole agent identity access to a tool, not "this run" or "this human's
  consent for this run." A compromised or misconfigured agent identity
  has standing access to everything its entry lists, all the time.
- **`JWT_SECRET` is a single shared symmetric key** — Agent Lab and
  crypton-id both need it. Anyone who can read Agent Lab's environment
  can mint tokens for any human (same property the real crypton-id has;
  not new here, but worth naming since this lab now depends on it too).
- **Traces contain full tool arguments and results in plaintext** —
  by design (full tracing was a requirement), but a `data/traces/*.jsonl`
  file is therefore sensitive: it contains customer names, emails, and
  case note contents. No encryption or access control on trace files in
  v0.1.
- **No rate limiting or replay protection** on `AgentRunner.run()` itself
  — nothing stops the same human token from being used to spin up
  unlimited concurrent agent instances.
- **The model is trusted to *ask* even though it can't *execute*
  unilaterally** — the boundaries in section 9 hold today, but they are
  all enforced in this one process, in Python, with no separate policy
  engine. A bug in `agent.py`'s ordering (e.g. validating before
  authorizing, or a future tool added to `TOOL_FUNCTIONS` without a
  matching `authz` entry) would silently remove a boundary. This is
  exactly the class of thing the eventual agent-security scanner should
  test for.

None of the above is fixed in v0.1 on purpose — see `labs/README.md`'s
"do not build yet" list. They're the concrete starting point for the
security work that comes after this lab.

---

## v0.2 addendum: Sessions, Tasks, and persistent state

v0.1's `AgentRunner.run()` did one thing: take a task string, run the
loop, return an answer. Nothing survived past the return statement except
the trace file. v0.2 adds three persisted entities in front of and around
that loop — `Session`, `Task`, and a fuller `AgentInstance` — without
changing anything about how the loop itself validates, authorizes, or
executes a tool call (sections 1-10 above are still accurate for that
part).

```
HumanIdentity
      ↓
   Session
      ↓
     Task
      ↓
AgentInstance
   ↙       ↘
 Model     Tools
      ↓
 Task State
      ↓
   Traces
```

### Session vs. Task vs. AgentInstance

Three more entities, on top of the three identity types from §7
(`HumanIdentity`, `AgentIdentity`, `AgentInstance` itself) — and just as
deliberately kept separate, not collapsed:

| Type | Answers | Lifetime | Module |
|---|---|---|---|
| `Session` | "which continuing interaction is this part of?" | spans multiple tasks; created once, resumed by ID | `sessions.py` |
| `Task` | "what specific objective was requested?" | one `create_task()` call to one terminal status | `tasks.py` |
| `AgentInstance` | "which concrete execution attempted this task?" | one `run()` call's worth of model/tool iterations | `identity.py` + `instances.py` |

A Session can contain many Tasks (a human might ask for several things in
one sitting). In v0.2, a Task has exactly one AgentInstance — `run()`
always creates a new Task and immediately a new AgentInstance for it, it
never resumes a Task. A future retry mechanism could attach a *second*
AgentInstance to the same Task (see "why retries create new
AgentInstances" below), but nothing in v0.2 does that yet — don't read
one-Task-one-instance as a hard 1:1 invariant, just the current reality.

Concretely: `AgentRunner.run(human=alice, goal="...", session_id=None)`
creates a session, a task, and an instance, in that order, before a
single model call happens — see `agent.py`'s `run()`, which is the
authoritative place to read this, not this document.

### Task state vs. model context

These are two different things that happen to both be "what the agent
currently knows," and conflating them is exactly the mistake this split
guards against:

- **Model context** (`messages` in `agent.py`) — the full conversation
  sent to the LLM: the goal, every tool call the model made, every tool
  result (including errors) fed back. Ephemeral, lives only in a Python
  list for one `run()` call, never persisted. This is *input to the
  model*, and it is allowed to contain anything, including a hostile
  model's own prior turns — nothing here is trusted.
- **Task state** (`tasks.Task.state`, schema `tasks.TaskState`) — a
  small, fixed set of fields (`last_tool`, `last_customer_id`,
  `last_order_id`, `last_action`) that our own code chooses to remember
  about a task's progress, persisted to SQLite, schema-validated on every
  write. This is *trusted application state*, and nothing goes into it
  except through `agent.derive_state_update` → `tasks.update_task_state`
  → `TaskState.model_validate`.

The rule the spec calls out — "do not persist arbitrary model output
directly into trusted task state" — is enforced structurally:
`derive_state_update(tool_name, result)` only ever reads `result`, the
tool's own trusted return value (already scoped to the calling human by
a DB query in `tools.py`), never `call.arguments` (the model's raw,
untrusted JSON). Even then, it doesn't copy `result` wholesale — it picks
specific named fields into a dict, which then still has to pass
`TaskState`'s `extra="forbid"` validation before it's written. Three
independent narrowings between "something the model said" and "what's in
`tasks.state`," not one.

### Why persistent state creates a new security boundary

v0.1's only state was request-scoped: a customer/order lookup was
checked against `human.user_id` and forgotten the moment `run()`
returned. v0.2 introduces state that outlives a single call —
`session_id` and `task_id` are values a caller can hand back in on a
*later*, separate `run()` invocation (`--session-id` on the CLI). That
turns "does this belong to this human" from a question answered once per
lookup into a question that has to be answered correctly every time a
persisted ID is presented, indefinitely into the future, potentially by
a different process entirely. This is why `sessions.get_session` and
`tasks.get_task` **always** re-run the ownership check against the
current `HumanIdentity` — there is no "trusted session handle" or cached
authorization anywhere; a `session_id`/`task_id` is just a string until
it's been checked, every single time, exactly like a customer_id in
tools.py.

### Who controls lifecycle transitions

Only `tasks.update_task_status`, and only via `lifecycle.validate_transition`
— a fixed table with terminal statuses (`completed`/`failed`/`cancelled`)
having zero outgoing transitions. The agent loop calls it at exactly four
points (`RUNNING` at task start, then `COMPLETED`/`FAILED` via the single
`_finish` exit path). Nothing else in the codebase writes `tasks.status`
directly — not `tools.py`, not the model, not `cli.py`. The model
influences *when* the loop decides a task is done (by returning final
text instead of a tool call), but it never calls `update_task_status`
itself and has no way to set a status value directly; the loop's control
flow is what decides which of `COMPLETED`/`FAILED` to request, using its
own logic (a clean final response vs. an exception vs. hitting
`max_steps`), never anything read out of the model's output.

**On max-step termination specifically**: the spec asks for "an explicit
terminal lifecycle state," not a new status value. v0.2 maps it to
`failed` — the task didn't reach a completed goal, so `failed` is
accurate — and records *why* separately, as `reason="max_steps_exceeded"`
on the `task_failed` trace event, rather than inventing a sixth
`TaskStatus` member. This keeps the lifecycle vocabulary exactly the five
values the spec lists ("do not rely on arbitrary status strings") while
still making the specific cause reconstructable from the trace.

### Why ownership must not come from model output

Three independent reasons this holds, not one:

1. **The tool argument schemas don't have ownership fields.** `GetCustomerArgs`/
   `GetOrderArgs`/`AddCaseNoteArgs` have no `human_user_id`, `session_id`,
   or `task_id` field, and `extra="forbid"` means a model that tries to
   add one gets `invalid_arguments`, not a silently-ignored extra key.
2. **Tool execution never reads identity out of `call.arguments`.**
   `_handle_tool_call` passes the real `human`/`instance` objects — sourced
   from the verified JWT and from `run()`'s own session/task creation —
   into every tool function. There is no code path where a tool receives
   an identity value derived from the model's JSON.
3. **State updates only ever come from `derive_state_update`, never from
   the model directly.** Even if a model's *tool result* somehow echoed
   back something identity-shaped (it can't, today — see tools.py — but
   hypothetically), `derive_state_update` only extracts `last_tool` /
   `last_customer_id` / `last_order_id` / `last_action` by name; there is
   no field in `TaskState` an ownership value could land in even by
   accident.

`tests/test_agent.py::test_model_cannot_override_ownership` exercises
this directly: a scripted tool call with `human_user_id`/`session_id`
fields in its arguments gets rejected by schema validation, and
regardless, the task's real ownership (checked via `tasks.get_task`) is
unaffected either way.

### Why retries should create new AgentInstances

A `Task` is "what was asked for and how it's going"; an `AgentInstance`
is "one attempt at it." If a retry reused the same `AgentInstance` (e.g.
by resetting its status back to `running`), the *evidence* of the first,
failed attempt — its trace, its start/end timestamps, whatever tool
calls it made before failing — would either be overwritten or become
ambiguous about which attempt produced which trace lines. Attaching a
new `AgentInstance` to the same `Task` instead (not built in v0.2 — the
loop always creates one Task with one instance — but the shape is
already there for it) means: the Task's identity and accumulated `state`
persist across attempts, but each attempt's execution history stays
intact and separately attributable via its own `instance_id`. This is
the same reasoning as `TERMINAL_STATUSES` having no outgoing
transitions — history should accumulate, not be silently overwritten.

### What new attacks become possible when state persists

v0.1's LEARNING.md §10 already named several attack surfaces that are
still true. v0.2 adds these, specific to persistence:

- **Session/task ID enumeration or guessing** — `session_id`/`task_id`
  are UUIDv4s (not sequential), so guessing one is impractical, but the
  *design* now depends on that being true; if a future change made these
  IDs predictable, the ownership check (`get_session`/`get_task`) is the
  only thing standing between "guess an ID" and "read someone else's
  session/task metadata" (not their customer/order data, which has its
  own independent check in `tools.py`, but the task's `goal` text and
  accumulated `state` would be exposed).
- **Stale-state confusion across a long-lived session** — a session can
  now span many tasks over an arbitrary amount of real time. Nothing in
  v0.2 expires a session or re-verifies the human is still "present" for
  a *later* task the way `crypton-id`'s step-up verification does for a
  specific sensitive action — a session_id, once obtained, is usable
  indefinitely by anyone who can present it plus a valid JWT for the same
  `human.user_id`.
- **State-update logic bugs become persistent, not transient.** In v0.1,
  a bug in a tool's return value only mattered for the single response it
  fed back to the model. In v0.2, `derive_state_update` writing something
  wrong doesn't just affect one turn — it's written to `Task.state` and
  will be read back on every subsequent turn *and* would be visible to
  anything that later inspects that task's row directly. `TaskState`'s
  schema validation limits how wrong it can be, but a logic bug within
  the allowed schema (e.g. attributing the wrong `last_customer_id`) is
  now a persistence-layer bug, not just a display bug.
- **The `agent_instances`/`tasks`/`sessions` tables are new, unencrypted,
  un-access-controlled SQLite state**, same caveat as v0.1's trace files:
  anyone with filesystem access to `data/agentlab.db` can read every
  human's session/task history directly, bypassing the ownership checks
  entirely (those checks are enforced in `sessions.py`/`tasks.py`'s
  Python code, not by the database itself — there's no row-level
  security in SQLite here). This was already true of `data/traces/`
  in v0.1; it now also applies to the new tables.

None of this is fixed in v0.2 on purpose, same as v0.1 — see
`labs/README.md`'s "do not build yet" list. They're what the next round
of security work has to account for now that state outlives a request.

---

## v0.3 addendum: the Agent Registry

v0.1/v0.2 let any caller construct `AgentIdentity(agent_id="support-agent")`
by hand and hand it to `AgentRunner` — the string was never checked
against anything. v0.3 replaces that with real, persisted registrations
(`agents.py`, table `agents`): an `agent_id` now always refers to a row
someone actually registered, owns, and can disable.

```
HumanIdentity
      ↓ owns
AgentIdentity
      ↓ creates
AgentInstance
      ↓ performs
Task
      ↓
Model + Tools
```

### Agent vs. model

These are not the same thing, and this addendum's single most important
point is that nothing in this codebase is allowed to conflate them:

```
AgentIdentity
    support-agent v0.1.0        (agt_3f9a... — registered, owned by alice)
        ↓ uses
    Claude / another model       (execution-time choice, made in cli.py /
                                   model_provider.py — carries no identity)
```

`AgentIdentity` is *the registered software application* — a name,
version, and owner, persisted once at registration time. The model
(Claude, or whatever `ModelProvider` a run is configured with) is
*infrastructure the agent happens to use for one execution* — configured
per `AgentRunner` construction (`build_model_provider` in `cli.py`), not
persisted anywhere as part of the agent's identity. **Swapping the model
provider behind an agent does not create a new AgentIdentity, and does
not require re-registering anything** — `test_model_provider_metadata_is_not_agent_identity`
proves this directly: two runs against the identical registered
`agent_id`, using two different `FakeModelProvider` scripts (standing in
for "different providers"), resolve to the exact same `agent_name`/
`agent_version`/`agent_id` every time. The only thing that *would*
create a new `AgentIdentity` is a new `register_agent(...)` call — e.g.
if the agent *application itself* changes enough that its owner decides
it's now "support-agent v0.2.0," that's a deliberate, explicit
registration, not something that happens automatically because a model
was swapped.

### AgentIdentity vs. AgentInstance (recap, now with a registry underneath)

Same distinction as v0.1 §7, sharpened: `AgentIdentity` (the row in
`agents`) is the *application* — one registration, reused across many
runs. `AgentInstance` (the row in `agent_instances`) is *one execution*
of it — a fresh one every `AgentRunner.run()` call, always referencing a
real, currently-active, correctly-owned `AgentIdentity` (never a
dangling or foreign one — see "why disabled agents cannot instantiate"
below). Multiple `AgentInstance` rows may, and normally will, reference
the same `AgentIdentity` over that agent's lifetime.

### AgentIdentity vs. model (why this needs its own heading)

To state the negative property explicitly, since it's one of the
security requirements this addendum has to satisfy: a model's name
(`"claude-sonnet-5"`) or provider (`"anthropic"`) is never read, checked,
or stored anywhere `agent_id` is used for identity or authorization.
`ModelProvider`/`AnthropicModelProvider`/`FakeModelProvider`
(`model_provider.py`) were not touched by this change at all — they have
no awareness that an Agent Registry exists, and nothing in `agents.py`
or `agent.py`'s identity-resolution path (`require_active_agent`) takes
a model or provider as input. There is no code path where "the model
says its name is X" could influence which `AgentIdentity` a run uses.

### Why names aren't secure identities

The one non-obvious design choice in this addendum: `authz.REGISTRY`
(the static tool-permission table from v0.1) is keyed by agent *name*
(`"support-agent"`, `"read-only-agent"`), not by canonical `agent_id`.
This looks, at first glance, like exactly the thing this addendum is
supposed to prevent — so it's worth being precise about why it's safe.

`authz.py` is unchanged from v0.1/v0.2 — it still just does
`REGISTRY.get(x)`. What changed is *what `x` is allowed to be* at the one
call site, in `agent.py`'s `_handle_tool_call`. It is never a value a
caller or model supplied directly. The only way to reach that call site
at all is: present a canonical `agent_id` → `agents.require_active_agent`
verifies it exists, is owned by the calling human, and is active →
*only then* is `identity.name` read off that already-verified database
row and used to pick a policy bucket. A model cannot "claim" a name to
get support-agent's permissions; it would need to have caused a real,
owned, active `agt_...` row to exist first, and that row's *name* is
whatever its owner registered it as, not whatever string shows up later
in a tool call. This is why `test_agent_name_cannot_substitute_for_agent_id`
constructs `AgentRunner(agent_id="support-agent", ...)` (the bare name,
not a real ID) and asserts it raises `AgentAccessError` — a name is
never, on its own, a valid way to select or authenticate as an agent.

The pragmatic reason `REGISTRY` has to be name-keyed rather than
`agent_id`-keyed: `REGISTRY` is static code, written by a developer ahead
of time, but registration generates a fresh random `agent_id` *per
owner* — there is no single canonical ID a static dict could reference
for "the support-agent policy" across every human who registers their
own copy of it. Name is the only stable thing available to write in code
ahead of time. A consequence worth being honest about: a newly
registered agent whose name isn't `"support-agent"` or `"read-only-agent"`
gets **zero** tool permissions by default (fail-closed) until a developer
adds a `REGISTRY` entry for that name — this is a real limitation of
keeping authorization static in v0.3 (explicitly permitted by the spec:
"current authorization may still be static"), not an oversight. A
dynamic, per-agent permission system is future work, not this chunk.

### Why canonical IDs are generated by Crypton

`agents.register_agent` has no `agent_id` parameter at all —
`_new_agent_id()` (`f"agt_{uuid.uuid4().hex}"`) is the only place an ID
is produced, and it runs entirely inside `register_agent`, after
validating `name`/`version`, with no way for a caller to influence the
result. This matters because an ID a caller (or, transitively, a model
whose output somehow reached a registration call — it doesn't, in this
codebase) could choose would be an ID a caller could *predict or
collide with*. A random, server-generated ID means presenting a valid
`agent_id` is only possible by having actually registered it (or been
told it by whoever did) — the ID itself carries no information an
attacker could compute in advance.

### Why disabled agents cannot instantiate

`AgentRunner.run()` calls `agents.require_active_agent` as literally its
first action after minting the trace correlator — before any
Session/Task/AgentInstance is created. This ordering is deliberate: an
agent that can't run shouldn't leave behind a half-started Task sitting
in `running` forever, or a Session created for a launch that was always
going to fail. `require_active_agent` raises (`AgentNotActiveError` for
disabled, `AgentAccessError` for unknown/foreign) and `run()` lets that
exception propagate — no `AgentResult` is returned, no `AgentInstance`
row is ever written for a rejected launch. `test_disabled_agent_cannot_run`
and `test_unknown_agent_cannot_run` both confirm zero `sessions` rows get
created for a rejected attempt. Disabling an agent (`agents.disable_agent`)
is therefore an *immediate, unconditional* kill switch on new executions
— it does not stop an `AgentInstance` already mid-run (there is no such
thing as a currently-running instance in this synchronous, one-loop-at-
a-time lab), but it guarantees no *new* one can start.

### Why persistent registration matters for audit/accountability

Before v0.3, "which agent did this" was answered by an arbitrary string
nobody could verify — `"support-agent"` might have meant anything,
supplied by whoever constructed the `AgentIdentity`. After v0.3, every
`Task.agent_id` and `AgentInstance.agent_id` resolves to a real row with
a real `owner_user_id`, a registration timestamp, and a status history
(`agent_registered`/`agent_disabled`/`agent_enabled` trace events, when a
tracer is passed to those functions). This is what makes "who is
accountable for this agent's actions" answerable at all: not just "a
human was authenticated" (that was already true) but "a specific human
*registered this specific agent application*, and it has been active
this whole time" — the missing link between an authenticated human and
an actually-trustworthy claim about which software was running on their
behalf. `test_task_references_canonical_agent_id` and
`test_agent_instance_references_task_session_human` both verify this
link is real (a genuine foreign-key-shaped reference to a row that
exists), not just a string that happens to look like an ID.

None of this is fixed further in v0.3 on purpose — delegation (one human
authorizing another human's agent, or an agent acting for someone other
than its registrant) is explicitly out of scope; see the task spec's
"do not build" list. An `Agent`'s `owner_user_id` is fixed at
registration and nothing in v0.3 changes it after the fact.
