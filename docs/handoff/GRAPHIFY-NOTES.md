# Graphify analysis: what the numbers mean and what is wrong with them

Graphify is **not part of this repository**. It is a third-party tool (`graphifyy` 0.9.49,
<https://github.com/Graphify-Labs/graphify>) that the founder runs locally. Its output lives in
`graphify-out/`, which is git-ignored (`.gitignore:59`) and never committed. So nothing here
could be "fixed in Crypton's source": the three problems below are properties of the tool, and
the correct action is the one this file records. No Crypton production source was changed to
improve a graph statistic.

Use the graph as a map for orientation. Do not use an edge from it as evidence of a
dependency, in a security argument, or in a customer report without opening the code.

## What was reported, and what was found

The founder's last full run (491 files, mixed code and documents) reported 3,277 nodes, 7,644
edges, 204 communities, 616 unresolved endpoints, 484 collapsed edges, and three false
`Outcome` links. The checks below were repeated on the current `origin/main`
(`4344036`; 647 files: 467 code, 175 documents, 5 images), structural (AST) layer only.

| Finding | Re-measured on `main` | Cause | Is it a defect in Crypton? |
|---|---|---|---|
| Unresolved edge endpoints | 1,073 of 14,215 raw edges: 880 point to stdlib / third-party modules or crates, 73 to `pyproject`/package dependencies, 120 to names that also match some repo file | The extractor emits an `imports` / `imports_from` edge for every import and keeps it even when the target is outside the corpus. The graph builder then drops those edges | **No.** These are expected for any import graph. The 120 "also matches a repo file" cases are mostly generic names (`agents` is the OpenAI Agents SDK package, `types`, `context`, `decision`) and sibling-module imports in `labs/experiments/001-*`, which use flat `from oracle_judge import ...` style that the extractor does not resolve |
| Collapsed duplicate edges | 564 node pairs carry more than one edge; 828 edges are absorbed when the graph is built as a simple undirected graph. Only 95 of them are exact duplicates (same relation and line); the rest are distinct relations or call sites | `graphify` builds `nx.Graph` by default, which keeps one edge per node pair. Several references to a type such as `String` or `Any` from one function become one edge | **No.** A modelling choice. Pass `--directed` or keep a multigraph if per-site counts matter |
| False `Outcome` links | 3, all `EXTRACTED` (confidence 1.0) | Each of `labs/exp-003-openai-agents-sdk/crypton_exp003/evidence.py`, `labs/exp-003-microsoft-agent-framework/crypton_exp003b/evidence.py` and `labs/exp-003c-langgraph/crypton_exp003c/core.py` defines its **own** local alias `Outcome = Literal[...]` (old `PASS/FAIL/INCONCLUSIVE` vocabulary). The extractor does not create a node for a module-level assignment, so the bare name `Outcome` in `build_evidence()` was resolved to the only class named `Outcome` in the whole corpus: `labs/agent-lab/src/agentlab/assessment/models.py` | **No.** These are three distinct symbols. The agent-lab enum is the five-conclusion type in `assessment/models.py`; the experiments' aliases are frozen evidence code and must not be "unified" |
| `Outcome` as a high-centrality node | 46 edges (43 after removing the three false ones) | Real: every module that builds, validates or renders an assessment finding uses it. 34 of 46 edges are `INFERRED` `uses` edges | **No.** `Outcome` is the central type of `agentlab/assessment/`. Its five conclusions and the documented `migrate_legacy_outcome` behaviour are unchanged |
| Communities split along file and test boundaries | e.g. one `assessment/` package spread over about seven communities | Louvain clustering on a graph where tests, helpers and source files each link to the same few types | **No.** Not an architecture problem. Do not restructure code to change community counts |

A heuristic that flags cross-file `references`/`calls` edges without a matching file-level
import edge produced 606 candidates. It is too noisy to trust (Rust `use crate::...` paths and
Python package imports do not map to file-to-file edges), so only the three `Outcome` edges
are claimed as confirmed false. Treat the rest as unverified, not as correct.

## Required correction (upstream, in the tool)

Report to Graphify, with the `Outcome` case as the reproduction:

1. **Resolve a bare name by scope before resolving it corpus-wide.** A name that is assigned or
   defined in the same file, or imported from a specific module, must bind to that symbol.
   Only fall back to a corpus-wide match when the name is unbound locally, and never label a
   fall-back match `EXTRACTED`.
2. **Create nodes for module-level type aliases and assignments** (`X = Literal[...]`), so
   there is something local to bind to.
3. **Do not carry a name-only match at confidence 1.0.** Mark it `INFERRED` with a lower score.
4. Optionally keep external-module imports as explicit "external" nodes instead of dropping
   them silently, so the unresolved count stops looking like corruption.

## What was done locally

- Rebuilt the **structural** graph for `main` in the cleanup worktree (git-ignored) with the
  three `Outcome` edges removed: 5,083 nodes, 12,403 edges, 229 communities. `Outcome` has 43
  edges in it. The semantic layer (document and image concepts) was **not** regenerated: its
  cache is keyed by absolute path, so it does not carry over to a different checkout, and
  rebuilding it costs about nine sub-agent runs (180 documents and images) for little
  handoff value. The founder's
  existing graph in the primary working copy still has the semantic layer, but predates the
  files added on `main`.
- Left `graphify-out/` ignored. Nothing generated by Graphify was added to Git.

## How to regenerate

```text
/graphify .            # full run (code structurally, documents via the agent)
/graphify . --update   # incremental
```

After a run, open the report's "Surprising Connections" and "Suggested Questions" sections as
leads only. Before relying on any edge that crosses an experiment boundary, check that the
two files actually share a symbol, as the `Outcome` case shows they may not.
