# EXP-003 Property ID Mapping (added 2026-09-30, remediation pass)

**Why this document exists:** the 2026-09-30 evidence-readiness audit found that
EXP-003A (OpenAI Agents SDK), EXP-003B (Microsoft Agent Framework), and
EXP-003C (LangGraph) do **not** use their `H1`/`H2`/`H3`... labels to mean the
same property across experiments. For example, EXP-003B's "H5" is standing/
always-approve scope, while EXP-003A's and EXP-003C's "H5" is a policy/
guardrail TOCTOU check — a genuinely different property. Citing "H3 held
across all three frameworks" without checking the underlying test would
silently compare three different things in two of the three cases.

**What this document does and does not do:** it defines stable,
framework-independent property IDs and maps each experiment's local H-number
to the correct one. It does **not** rename, renumber, or edit anything inside
`labs/exp-003-openai-agents-sdk/`, `labs/exp-003-microsoft-agent-framework/`,
or `labs/exp-003c-langgraph/` — every test file, README, and evidence JSON in
those three directories keeps its original H-numbering exactly as recorded,
for historical reproducibility. This mapping is the thing to cite when
comparing results *across* experiments or in a customer-facing report; the
original H-numbers remain the thing to cite when reproducing or re-reading
one specific experiment's own files.

## Stable property IDs

| Property ID | Name | EXP-003A | EXP-003B | EXP-003C | Cross-framework comparable? |
|---|---|---|---|---|---|
| `PROP-ARG-BINDING` | Argument binding / drift — is approval bound to the exact arguments presented, or can materially different arguments reach execution under the same authorization event? | H1 | H1 | H1 | **Yes** — directly comparable, same test shape in all three. |
| `PROP-DELEGATION-BOUNDARY` | Nested delegation / handoff boundary — approval at one boundary must not silently become authorization for a distinct downstream authority | H2 | H2 | H2 | **Yes** — directly comparable. (EXP-003B's full-cycle sub-case is honestly INCONCLUSIVE due to a harness limitation, not a framework-behavior result — see EXP-003B's own README.) |
| `PROP-STANDING-APPROVAL` | Standing / always-approve scope — does an "always approve" grant authority narrower or broader than expected across repeated or nested calls? | H3 | **H5** | H3 | **Yes, by concept** — same property, different local numbers. Do not cite "H3" alone as cross-framework; cite `PROP-STANDING-APPROVAL`. |
| `PROP-CHECKPOINT-STALE-REPLAY` | Checkpoint restore / replay of a stale approval response after the underlying request has already completed | — (not present) | **H3** | — (not present as a dedicated hypothesis) | **No** — this is Microsoft Agent Framework's own checkpoint/workflow-identity surface (`crypton_exp003b/evidence.py`'s own docstring: "Microsoft Agent Framework exposes workflow/checkpoint identity as first-class concepts EXP-003A had no equivalent for"). Neither OpenAI Agents SDK nor LangGraph's tested surface has an equivalent dedicated test. This is the corpus's single strongest result (two real executions from one human decision) — report it as an EXP-003B-specific finding, not as a cross-framework one. |
| `PROP-MULTI-PENDING-APPROVALS` | Multiple simultaneous pending approvals — approving one does not cross-correlate with a different pending approval | sub-case inside H4 (unlabeled) | **H4** | — (not a standalone test) | **Partial** — EXP-003A covers this as an unlabeled sub-case of its H4, EXP-003B has a dedicated H4 for it, EXP-003C does not test it at all. |
| `PROP-PERSISTENCE-EXACTLY-ONCE` | Persistence / resume exactly-once — a completed thread/state cannot be resumed a second time to re-trigger execution | H4 | only informally via baseline B3 (no dedicated H) | H4 | **Partial** — A and C are directly comparable (both dedicated `H4`); B only covers this informally through a baseline test, not a numbered hypothesis. |
| `PROP-POLICY-TOCTOU` | Policy / guardrail TOCTOU — a policy limit changed between approval and resume must be re-checked, not trusted from approval time | H5 | **absent entirely — no test exists** | H5 | **Partial** — A and C are directly comparable; B has **no test for this property at all**, and B's own README does not flag this as a gap (a silent, not an acknowledged, absence — worth correcting in a future EXP-003B pass, not in this remediation). |
| `PROP-IDENTITY-ORIGIN-BINDING` | Identity / origin binding — a response/approval issued for one agent, session, or thread must not be accepted by a different one | unlabeled sub-case inside H2 ("same tool name, different nesting level") | **H6** | **H6** | **Partial** — B and C are directly comparable (both dedicated `H6`); A only has an unlabeled, narrower sub-case of this property inside its H2 test, not a dedicated hypothesis. |

## How to use this table

- **In a customer-facing report or any cross-framework claim**, cite the
  `PROP-*` ID, and name which experiment(s) actually support it (e.g. "the
  argument-binding property (`PROP-ARG-BINDING`) held across all three
  frameworks tested: EXP-003A `H1`, EXP-003B `H1`, EXP-003C `H1`").
- **When reproducing or re-reading one specific experiment**, use that
  experiment's own original H-number and file paths — this table does not
  change what's inside `labs/exp-003-*/`.
- **Do not claim "N/A" where this table says "Partial" or "No".** A property
  that was only tested in one or two of the three frameworks is exactly that
  — tested in one or two, not three. Report the gap, don't collapse it.
- This table is itself subject to the same discipline as any other finding
  in this corpus: if a future EXP-003D (a fourth framework) is run, extend
  this table rather than renumbering it, and if B's `PROP-POLICY-TOCTOU` gap
  is ever closed, update the "absent entirely" cell to point at the new test
  rather than silently removing the historical note that it was once absent.

Source: 2026-09-30 evidence-readiness audit's EXP-003 cross-framework
inventory (`docs/audits/2026-09-30-evidence-report-readiness-audit.md` §2c),
cross-checked directly against each experiment's own `tests/*.py` and
`README.md` during this remediation pass.
