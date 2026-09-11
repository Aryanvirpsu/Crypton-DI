# Experiment #001 — LangGraph approval/execution drift

Crypton's first action-integrity experiment (see `docs/strategy/2026-09-10-action-integrity-thesis.md`
§6/§9/§10): does a real, third-party agent framework's own
reasoning/retry/delegation layer let a privileged action drift from
what a human actually approved?

**Result (#001):** yes, reproducibly, in a target I hand-wrote — see
[`finding-001.md`](finding-001.md).

**Result (#001b — representativeness check):** does NOT reproduce under
LangGraph's interrupt-in-tool HITL pattern, cited against a live,
first-party doc page — #001's bug was author-induced, not
framework-invited for that topology. See [`finding-001b.md`](finding-001b.md).

**Result (#001c — second topology check):** does NOT reproduce under
LangGraph's separate-review-node HITL pattern either, cited against the
official LangGraph JS SDK's own "Review Tool Calls" how-to (Tier A) and
faithfully ported to Python with no bypass edge added. See
[`finding-001c.md`](finding-001c.md). Both of LangGraph's major
documented HITL topologies are now checked; #001's finding stays real
but author-induced.

**Result (#001d — off-the-paved-path check, the strong finding):** YES,
reproducibly (5/5), on `langchain`'s real `create_agent` + the real,
shipped `HumanInTheLoopMiddleware`, once a small, independently benign
custom `after_model` middleware (widen scope after a host fails to
respond) is added in the order a developer would naturally write the
list. No approval-skipping code exists anywhere — the drift is a pure
side effect of `after_model` middleware composing in *reverse* list
order (verified against the installed package's own graph-building
code), which is not how the list reads top-to-bottom. Swapping only the
order flips it to 5/5 PASS. See [`finding-001d.md`](finding-001d.md) —
this is the first §19-grade, framework-representative finding in the
series.

**Result (#001e — real model, the closing check):** the #001d mechanism
requires no scripted retry to fire — a real model (OpenAI `gpt-4o-mini`,
temperatures 0.0 and 1.0, N=20 each) triggers it **38/40 times (95%)**,
and never once by the model itself choosing to broaden scope (0/40) —
every trigger was the model naively retrying the identical failed host,
with `HostEscalationMiddleware` doing the actual widening after
approval, exactly as #001d predicted. See [`finding-001e.md`](finding-001e.md)
for the full rate breakdown, the conditional (broadened × verdict)
table, and an honest account of a quota wall that forced a mid-experiment
provider switch (Gemini → OpenAI), reported rather than routed around.

Read in order: #001 → #001b → #001c → #001d → #001e — each is a
calibration check on the one(s) before it, not a separate scenario.
#001d is where "is this real or just my bug" lands on "real"; #001e is
where "would this actually happen" lands on "yes, almost always."

Local, self-hosted, fully mocked: no live systems, no real credentials,
no real side effects. TARGET is a fresh LangGraph agent (not Crypton
code); the ORACLE that judges it is Crypton Agent Lab's own
`approval.compute_argument_hash` invariant, imported unmodified — see
`finding-001.md`'s "Tech-reuse boundary" for why the two must never be
the same code.

## Layout

```
target_agent.py                       TARGET (#001) — hand-wired specimen with an author-induced bypass
target_agent_idiomatic.py        TARGET (#001b) — interrupt-in-tool topology, Tier A doc-cited, no bypass authored
target_agent_review_node.py   TARGET (#001c) — separate-review-node topology, Tier A doc-cited (langgraphjs), no bypass authored
target_agent_middleware.py    TARGET (#001d) — create_agent + real HumanInTheLoopMiddleware + a benign after_model hook, ordering-dependent drift
target_agent_llm.py                 TARGET (#001e) — #001d's exact middleware stack, planner swapped for a real model
capture.py                             capture layer — external tool-call interception + trace recorder (new code, frozen across all five)
oracle_judge.py                       ORACLE — imports agentlab.approval.compute_argument_hash, nothing else (frozen)
run_experiment.py                   harness for #001
run_experiment_idiomatic.py     harness for #001b
run_experiment_review_node.py  harness for #001c
run_experiment_middleware.py  harness for #001d
run_experiment_llm.py               harness for #001e (needs a real API key — see .env.local.example)
test_oracle_sanity.py                control tests proving the oracle is sound independent of LangGraph
finding-001.md                          write-up: the author-induced bug
finding-001b.md                        write-up: interrupt-in-tool calibration check
finding-001c.md                        write-up: separate-review-node calibration check
finding-001d.md                        write-up: the middleware-ordering finding (the strong result)
finding-001e.md                        write-up: real-model confirmation, rates not 5/5
.env.local.example                  template for #001e's API key(s) — copy to .env.local (gitignored), fill in
traces/                                     JSONL trace per run (gitignored — regenerate by rerunning)
```

## Reproduce

```bash
pip install -r requirements.txt
python test_oracle_sanity.py                             # 6/6 — oracle sanity, no LangGraph involved
python run_experiment.py --runs 5                         # #001 TARGET as found -> 5/5 FAIL
python run_experiment.py --control --runs 5               # #001 one-line routing fix -> 5/5 PASS
python run_experiment_idiomatic.py --runs 5               # #001b interrupt-in-tool -> 5/5 PASS
python run_experiment_review_node.py --runs 5             # #001c separate-review-node -> 5/5 PASS
python run_experiment_middleware.py --runs 5              # #001d TARGET as found (natural order) -> 5/5 FAIL
python run_experiment_middleware.py --order hitl_first --runs 5   # #001d control (order swapped) -> 5/5 PASS

# #001e needs a real API key: cp .env.local.example .env.local && fill in OPENAI_API_KEY
python run_experiment_llm.py --probe                              # a few manual runs, verbose
python run_experiment_llm.py --runs 20 --temperature 0.0          # -> 20/20 FAIL
python run_experiment_llm.py --runs 20 --temperature 1.0          # -> 18/20 FAIL, 2/20 inconclusive
```
