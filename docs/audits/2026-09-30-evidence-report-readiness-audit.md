# Crypton Evidence & Report-Readiness Audit

**Date:** 2026-09-30
**Scope:** Read-only audit. No experiments run, no evidence modified, no canonical documents changed. This file is the only artifact this audit produced.
**Method:** Five parallel read-only inventories (assessment-kit + methodology + business + pilot-pack docs; the three EXP-003 cross-framework experiments; EXP-001/005/007 + validation-benchmark; the agent-lab codebase and test suite; the adjacent semantic-rulebook and mastra-src repos), cross-checked against `DECISIONS.md` and `docs/CRYPTON_TECHNICAL_SOT_v1.1.md`, which were read directly. Every claim below traces to a specific file, test, or an explicit absence found during inventory — nothing here is inferred beyond what the underlying artifacts state.

---

## 1. Bottom Line

**How much evidence exists:** A lot, and most of it is real. Across `labs/agent-lab` (263 passing tests, a real Ed25519-signed multi-hop delegation chain, an append-only decision ledger), the EXP-003 series (three cross-framework experiments with real installed SDKs, real pytest runs, real captured exceptions and JSON evidence files), the Validation Benchmark (two independently-verified CVE cases with before/after JSON evidence and negative controls), and EXP-007 (16 real, locatable tests with determinism proof), Crypton has produced substantially more *real execution evidence* than narrative claims. This is not a case of "we mostly have documentation."

**How much is report-ready:** Less than the volume suggests, for one structural reason that recurs in every area audited: **there are two different things in this repo that look similar but are not interchangeable** —
1. **Crypton's own reference implementation** (`labs/agent-lab`) proving the delegation/authorization/approval *concepts* work, in a lab Crypton fully controls, and
2. **A schema and process for capturing those same facts about an unfamiliar customer's system** (the assessment-kit worksheets, `engagement.json`/`findings.json`/`evidence_manifest.json`).

(1) is mature and heavily tested. (2) is *defined* — the schemas exist, the worksheets exist, the finding library exists — but has been exercised exactly once against a real, unfamiliar, external target (EXP-005/OpenHands), and that one exercise was **static source review only, with no live customer intake, no execution, and no interactive worksheet-filling with an actual customer.** Zero real customer engagements have used this apparatus end-to-end. Every one of the four internal self-readiness documents (`VALIDATION-REPORT.md`, `11-pilot-readiness-report.md`, `12-operational-readiness-review.md`, `pilot-pack/v0.1/11-pilot-pack-red-team.md`) already says a version of this, in four different, non-cross-referenced verdicts — see §5.8.

**Is the main problem missing testing or poor organization?** Mostly organization and vocabulary consolidation, not missing testing, with two real exceptions:
- **Real experiment gap:** nothing tests forged/compromised authority, concurrent/multi-agent race conditions, multi-tenant isolation, or supply-chain integrity anywhere in the corpus (confirmed independently by `assessment-kit/v1.0/13-gap-analysis.md` and by this audit's own read of the code).
- **Real observability gap:** the customer-facing evidence pipeline has never captured a runtime trace against an external target — every third-party artifact in the corpus is either static-source-only (EXP-005) or a single before/after JSON snapshot (Validation Benchmark), never a multi-step execution trace of the kind `agent-lab`'s own `tracing.py` produces for itself.

Everything else — the outcome-vocabulary fork, the thin `engagement.json` schema, the never-populated chain-of-custody fields, the uncommitted semantic-rulebook harness results — is **organization/metadata work**, fixable without new research.

**Is another experiment required before producing the current sanitized sample?** No. The sanitized sample (`pilot-pack/v0.1/05-sample-assessment.md`) is explicitly fictional and doesn't need new evidence to exist as a sample. But if the next deliverable is a *real* customer report rather than another sample, the honest answer is: **not another lab experiment, but one real, live, end-to-end run of the customer-facing apparatus** (worksheets → evidence capture → findings.json → report) is the one thing this corpus has never done and cannot substitute for with more lab work.

---

## 2. Existing Evidence Inventory (condensed — see §9 area notes for full per-file detail already captured during this audit's inventory pass)

### 2a. Methodology / reporting apparatus (`assessment-kit/v1.0/`, `docs/methodology/`, `docs/ASSESSMENT_METHODOLOGY.md`, `docs/business/`, `pilot-pack/v0.1/`, `assessment-engagement/`)

| Artifact | Path | Canonical? | What it actually proves |
|---|---|---|---|
| `docs/ASSESSMENT_METHODOLOGY.md` | repo root/docs | **Most canonical doc in the tree** — tied to enforced code (`Finding.__post_init__`) | The frozen 4-outcome vocabulary (PASS/FAIL/INCONCLUSIVE/NOT_APPLICABLE), 8 assessment areas, evidence categories, severity rubric |
| `assessment-kit/v1.0/VALIDATION-REPORT.md` | assessment-kit | Canonical gate | Independent re-review of every EXP-001–005 finding; source of every reclassification (§5.7) |
| `assessment-kit/v1.0/07-finding-library.md` | assessment-kit | Canonical, gated by VALIDATION-REPORT.md | 3 active findings (FL-02/03/04), 1 retired (FL-01), a Known-Safe-Patterns appendix |
| `assessment-engagement/TEMPLATE/*.json,*.md` | repo root | Canonical schema definitions | `engagement.json`'s metadata fields; `findings.json`'s finding schema; `evidence_manifest.json`'s E-### evidence-ID scheme |
| `assessment-engagement/reference-agentlab-dryrun/*` | repo root | Instance, explicitly labeled non-customer | The only populated example of the schema above — proves the pipeline *renders*, not that it's been used on a real target |
| `assessment-kit/v1.0/05-assessment-worksheets.md` | assessment-kit | Canonical template, never field-tested | 10 worksheets (Identity/Authority/Delegation/Execution/Trust-Boundary/Approval/Persistence/Audit/Tool/Data-Flow) — the actual customer-facing authority-ledger capture mechanism; worked examples cite `labs/` research, not customer data |
| `docs/methodology/01–07-*.md` (EXP-004) | docs/methodology | Canonical vocabulary/pipeline source, **not fully reconciled with the kit** | 7-pillar catalog, 11-objective threat model, 13-stage pipeline, evidence-chain standard — uses a *different*, richer outcome vocabulary than the frozen 4-value one (see §5.3) |
| `pilot-pack/v0.1/03-test-catalog.md` | pilot-pack | Customer-facing, self-rated by maturity | 4 Core families + 2 Conditional + 1 Research Preview; two families (Revocation, Accountability) explicitly marked untested/immature in the doc's own words |
| `pilot-pack/v0.1/11-pilot-pack-red-team.md` | pilot-pack | Canonical readiness gate | Defines the "pre-customer credibility gate" (2 benchmark cases + 1 external review); states the benchmark half is done, the external-review half is not |

### 2b. Agent Lab (`labs/agent-lab/`) — Crypton's own reference implementation

| Artifact | Type | What it proves |
|---|---|---|
| `src/agentlab/delegation.py`, `chain.py`, `authorization.py`, `approval.py` | Real, persisted, SQL-backed | The full authority-ledger data model: principal, agent (canonical `agent_id`), delegated_from/to, scope (tools+resources), expiry, revocation, approval binding by argument hash — see field table in §5.2 |
| `src/agentlab/tracing.py` | Real JSONL file, append-on-write | Full run reconstruction, but **explicitly ephemeral/gitignored** per the README — not part of any committed audit trail |
| `db.py`'s `authorization_decisions` table | Real SQLite table, append-only, never updated | Durable decision ledger, but stores only `resource` (compact string) and `reason_code` — not full arguments or results |
| `tests/test_delegation_chain.py`, `test_enforcement_boundary.py`, `test_approval_pipeline.py`, `test_attenuation.py` (263 tests total as of v0.6) | Real, substantive, with negative/control cases | Proves zero-side-effect on every denial path (`SELECT COUNT(*) FROM case_notes == 0`), tamper detection via direct SQL mutation, TOCTOU-style approval-mismatch rejection |
| `src/agentlab/assessment/reference_dryrun.py` | Real, reproducible pipeline (only `uuid`/`time` monkeypatched for determinism) | The `assessment-engagement/reference-agentlab-dryrun/*` JSON is genuinely generated output, verified byte-for-byte against the generator — not fabricated prose |

### 2c. Cross-framework research (EXP-003A/B/C)

| Experiment | Target | Real execution? | Key finding |
|---|---|---|---|
| EXP-003A | OpenAI Agents SDK 0.22.2 | Yes — 17 tests, evidence JSON, `.pytest_cache` corroboration, installed package matches pin | All H1–H5 PASS with real execution evidence; H3's "namespace collision" sub-case honestly INCONCLUSIVE, not skipped |
| EXP-003B | Microsoft Agent Framework 1.18.0 | Yes — 12 tests | H3's stale-approval replay is the strongest single result in the corpus: one human decision produced **two real executions** of `transfer_funds`, reproduced twice. H2's full-cycle claim is honestly INCONCLUSIVE (harness bug, not observed framework behavior). **No TOCTOU/policy-recheck test exists in B at all — a silent, unflagged gap.** |
| EXP-003C | LangGraph 1.2.11 | Yes — 12 tests, but **no pinned dependency manifest exists** (no requirements.txt/lockfile) | H3 verified by static source introspection, not runtime execution (correctly self-labeled); H6b's result contradicted the author's own initial assumption and was honestly corrected |

**Critical cross-cutting finding (not previously surfaced anywhere in Crypton's own docs):** the three experiments' H-numbers do **not** consistently label the same hypothesis. B's "H5" = standing approval; A's/C's "H5" = policy TOCTOU. B's "H3" = checkpoint replay; A's/C's "H3" = standing approval. B has no H5-equivalent at all. Only H1 (argument binding) and H2 (delegation boundary) are genuinely apples-to-apples across all three. Anyone citing "H3 held across all three frameworks" without checking the underlying test would be comparing three different properties.

### 2d. EXP-001 (LangGraph approval drift) — full lifecycle

Original `finding-001`: real captured traces, but a **hand-authored specimen** (`# THE BUG` in the source's own comment) — later reclassified **SUPERSEDED**, excluded from the customer-facing library. `finding-001b/c`: real negative results against LangGraph's documented HITL patterns — survive unchanged, moved to a "Known-Safe Patterns" appendix. `finding-001d/e`: real framework (`langchain==1.4.0`), real model calls (gpt-4o-mini), deterministic 5/5 and 38/40 reproduction, structured JSONL traces with `run_id`+`timestamp` — the single best-instrumented evidence in the corpus. **But on 2026-09-27, `CLOSURE.md` reclassified this as `NO VULNERABILITY — EXPECTED FRAMEWORK BEHAVIOR`**: the mutating middleware is trusted, same-trust-domain developer code, explicitly permitted by the framework's own contract, and no framework-claimed boundary was crossed. **This is Crypton's own documented example of passing all six evidence-sufficiency gates (§4) while still reaching the wrong classification**, because evidence sufficiency and trust-boundary triage are different questions — Crypton caught this itself and wrote a new triage rule (`01-assessor-handbook.md` §9a) as a result. That is evidence of a maturing discipline, not a weakness to hide.

### 2e. EXP-005 (OpenHands SDK) — the one real-unfamiliar-target field test

Static source review only (`target.json`: `"test_methods_not_used": ["controlled staging test against a live running instance", "LLM-driven scripted execution"]`). Two findings survive as CONFIRMED (self-report risk gating; silently-auto-approved delegated confirmations); a third was downgraded to QUALITY ISSUE and excluded by the *later* `VALIDATION-REPORT.md` (not by EXP-005's own `findings.md`, which is an attribution detail worth getting right in any customer-facing citation). Zero execution evidence anywhere in this experiment — real, verifiable source citations, but Gate 3 (controlled execution) and Gate 4 (effect confirmation) are not met by design, and EXP-005's own docs say so honestly.

### 2f. Validation Benchmark (VB-001, VB-002)

The strongest third-party-target evidence in the whole corpus: real before/after JSON, matching a real CVE, with an explicit negative control in each case, and a single unmodified script run against two pinned versions. No commit/tag pin (PyPI/npm version pin only), no run_id/timestamp/environment manifest in the JSON itself (only in README prose), and each case is run once per version rather than repeated — a minor Gate-6 weakness relative to EXP-001e's or EXP-007's repetition discipline.

### 2g. EXP-007 (Principal Continuity)

16 real, locatable pytest tests matching the "16 passed" claim exactly (verified by direct count). A stale `.pytest_cache` referencing 6 non-existent test names was found and should not be mistaken for evidence about the current suite. Determinism proven across 10 in-process repeats and 4 `PYTHONHASHSEED` variants. EXP-007B (the natural next step — revocation between approval and resume) is **explicitly unimplemented**, by its own spec's own words: "No code for EXP-007B exists yet."

### 2h. semantic-rulebook (adjacent repo) and mastra-src (adjacent repo)

**semantic-rulebook**: EXP-002 (binding-drift) is genuinely committed, JSON-backed evidence (`experiments/002-binding-drift/results/reference_run.json`) — this is the one thing in that repo actually cited by Crypton's docs, and the citation is accurate. But the newer, more headline-worthy work (`RESULTS.md`'s Candidate 4a/2 findings against Mastra) is **real, non-stub, executable TypeScript that drives the actual Mastra SDK — but has zero committed output, zero git history, and zero captured run logs.** The result tables in `RESULTS.md` are hand-typed prose describing what the author reports observing, not machine-generated evidence. Candidate 2 is also explicitly **not novel** — it matches a public, withdrawn GitHub issue (#22575).

**mastra-src's `templates/template-security-incident-triage`**: confirmed, via git remote/blob-hash comparison against the real upstream `mastra-ai/mastra` repository, to be **unmodified stock open-source template content with zero Crypton connection** and zero citation anywhere in Crypton-DI. Not relevant evidence; should not appear in any customer-facing inventory.

---

## 3. Coverage by Required Data Domain

- **Assessment metadata:** thin. `engagement.json` has 4 real fields (title, label, date, assessor) plus 6 file pointers; it has no `assessment_id`, no structured client identifier, no framework/version field, no commit-hash field, and no structured excluded-scope field. See §5.1.
- **Authority map:** strong *inside Crypton's own lab* (agent-lab's data model covers nearly every field the spec asks for — see §5.2), essentially untested *as a customer-facing capture process* (the worksheets that would do this for a real target have never been run against one).
- **Property tests:** the strongest area overall — real execution evidence dominates (§2c, §2d, §2f, §2g) — undermined only by the outcome-vocabulary fork (§5.3) and inconsistent H-number semantics across EXP-003A/B/C.
- **Evidence:** the E-### ID scheme is defined and has one working populated instance, but every chain-of-custody field (`timestamp`, `reference`, `integrity_hash`) in that instance is `null` — meaning the "hard" evidentiary metadata fields have literally never been populated with a real value anywhere in this repository.
- **Findings:** the pattern library (`07-finding-library.md`) is rich in prose (typical causes, detection guidance, remediation) but lacks discrete `test_id`/`invariant`/`preconditions`/`reproduction-count` fields; the separate engagement-level `findings.json` schema has more of these structurally but still no `invariant` or `preconditions` field.
- **Remediation:** prose guidance exists per finding-library entry; no structured "customer owner" or "retest gate" field is ever populated (appropriately — per the audit's own rule, this is legitimately a customer-context gap, not a Crypton gap).
- **Retest:** conceptually present in the SOP and scoring model; no artifact anywhere shows an actual VIOLATED → REMEDIATED → ENFORCED-equivalent lifecycle having been run.

---

## 4. Report-Readiness Matrix

Mapped against the report structure this audit was asked to check readiness for. Crypton's own existing 11-deliverable structure (`assessment-kit/v1.0/09-customer-deliverable-templates.md`) does not use identical section names, so each row below states the mapping.

| Report section | Ready now? | Evidence available | Missing information | Work required |
|---|---|---|---|---|
| Decision Sheet | **PARTIAL** | Severity/maturity ratings exist (`08-scoring-model.md`); Executive Summary and Assessment Certificate templates exist | No single-page "decision sheet" artifact has ever been assembled from these pieces | Assemble a template from existing scoring + exec-summary content; no new evidence needed |
| Authority Map / Authority Diff | **PARTIAL** | Topology concept exists (Authorization Map in `ASSESSMENT_PACK.md` §E; Authority worksheet in `05-assessment-worksheets.md`); agent-lab's own chain trace (`root_principal`, `acting_principal`, `delegation_depth`, `authority_path`, `chain_hash`) proves the *concept* is capturable | An "expected vs. observed" **diff** framing does not exist as its own artifact anywhere; never populated against an external target with both sides evidenced | Design the diff template (schema work, not research); first real population will need a live engagement |
| Action Queue | **PARTIAL** | Remediation Roadmap deliverable + severity/retest concepts exist | No stateful, owner-assigned, retest-gated "queue" artifact exists; customer-owner fields are correctly left blank pending a real customer (this part is fine as-is) | Template assembly only |
| Findings Summary | **READY** | Risk Summary + Finding Detail deliverables (kit docs 4–5) match this closely | — | None |
| Detailed Finding Cards | **READY** | Finding Detail template (kit doc 5) matches; `findings.json` schema backs it | Field completeness depends on which fields a real engagement actually populates | None structural |
| Verified Controls | **PARTIAL** | "Known-Safe Patterns" appendix in `07-finding-library.md` covers the concept (finding-001b/c, EXP-003A/B/C's INCONCLUSIVE-but-safe results) | Not packaged as its own per-engagement deliverable | Template assembly only |
| Limitations | **READY — a genuine strength** | Every experiment in the corpus states its own limitations explicitly (static-only, INCONCLUSIVE-not-guessed, harness bugs disclosed); `limitations.md` template exists | — | None |
| Retest Record | **MISSING** | Concept exists in SOP/scoring docs; zero populated instances anywhere | No artifact demonstrates a VIOLATED→REMEDIATED→ENFORCED cycle | Needs a real retest cycle from an actual engagement; cannot be manufactured |
| Evidence Index | **PARTIAL** | E-### scheme + Evidence Standard (16-field) exist and have one working instance | Chain-of-custody fields (`timestamp`, `reference`, `integrity_hash`) are `null` in the only populated example — never demonstrated populated | Populate these fields the next time real evidence is captured; process discipline, not research |
| Technical Appendix | **READY** | Engineering Appendix deliverable (kit doc 8) matches | — | None |

**Do not overstate the Authority Map row:** the topology drawing itself is well-supported by agent-lab's chain-verification code; what's *not* supported is proving, for a real customer's system, who the approver actually was — that depends entirely on what a real engagement's worksheets capture, which has never happened.

---

## 5. Evidence Sufficiency Review

### 5.1 Assessment metadata schema — `engagement.json`

Present fields: `title`, `label`, `date` (single, ambiguous — start? delivery? report date?), `assessor` (one free-text string, no lead/reviewer distinction), plus 6 file pointers (`scope_file`, `workflow_file`, `evidence_manifest_file`, `findings_file`, `limitations_file`, `executive_summary_file`).

Absent as structured fields (present, if at all, only as unstructured prose inside a pointed-to file): `assessment_id`, structured client/customer identifier, `assessment type`, `tested workflow`, `environment` (staging/prod/sandbox), `framework/product` + version, `commit hash`, a distinct testing-window start/end, `explicitly excluded scope` as its own field. **UNCLEAR** (per the source audit) whether this thinness is a deliberate "engagement.json is a router, not a record" design choice or an unaddressed gap — no document states the rationale either way.

### 5.2 Authority-ledger fields — agent-lab's own model (for calibration/demo purposes, not customer capture)

| Field | Status | Note |
|---|---|---|
| Principal / agent identity | PRESENT | `identity.py`, `agents.py` — canonical `agent_id`, never a shared name, since the v0.4 fix |
| delegated_from / delegated_to | PRESENT | `delegation.py` |
| Delegation depth | DERIVABLE | Computed by walking `parent_delegation_id`, not a stored column |
| Scope (tools + resources) | PRESENT | Exact allow-lists, no wildcards |
| Expiry / revocation | PRESENT | Checked live on every call, never cached |
| Approval binding | **PARTIAL** | Bound via `argument_hash` (sha256) — the *raw* approved arguments are never persisted server-side; only a hash. A human reconstructing "what exactly was approved" after the fact needs the caller to have separately logged it |
| run_id / session_id / task_id / instance_id | PRESENT | Three intentionally distinct IDs (`identity.py`'s own documented rationale) |
| Executed action (tool+args, result) | PRESENT, but **ephemeral** | Only in `tracing.py`'s JSONL file, which the README calls gitignored/disposable — not the durable `authorization_decisions` table, which stores only `resource` + `reason_code` |
| Evidence / trace location | PARTIAL | Two real mechanisms exist (durable-but-thin SQL ledger; rich-but-ephemeral JSONL trace) — no single durable, complete record |
| Commit/version pin tying evidence to a specific `agent-lab` commit | **MISSING** | No artifact anywhere embeds a git SHA; linkage today is informal (manual date correlation with `git log`) |

### 5.3 Outcome vocabulary — a real, unresolved fork

**The vocabulary this audit's own instructions assumed (ENFORCED/VIOLATED/UNREPRESENTABLE/UNVERIFIABLE/N/A) does not exist anywhere in this repository.** Stating this plainly rather than silently substituting Crypton's real vocabulary: Crypton made its own, different, deliberate, code-enforced decision.

- **The frozen, canonical, code-enforced vocabulary**: `PASS / FAIL / INCONCLUSIVE / NOT_APPLICABLE` (`docs/ASSESSMENT_METHODOLOGY.md`, enforced by `assessment/models.py::Finding.__post_init__`, which raises `ValueError` on an unevidenced PASS/FAIL). This is reaffirmed as frozen in `VALIDATION-REPORT.md` §7 and `01-assessor-handbook.md` §9, and used consistently in every JSON schema and the customer-facing test catalog.
- **A second, looser, additive vocabulary** lives in the `docs/methodology/` (EXP-004) research documents: `COVERAGE GAP`, `SPLIT`, `PARTIAL PASS`, `CANDIDATE_FINDING_APPLICATION_LEVEL`, `PASS_WITH_CAVEAT`. The reconciliation rule exists (`01-assessor-handbook.md` §9: file any extended-vocabulary result as `FAIL` with a prose qualifier in a real report) — but `02-threat-model.md` and `04-framework-adaptation-guide.md` themselves have **not been edited** to reflect this; the fork is acknowledged in one place and still live in two others.
- **A minor code-level instance of the same fork**: EXP-003B's own declared `Outcome` type (`Literal["PASS","FAIL","INCONCLUSIVE","BLOCKED_PROVIDER"]`) does not include `"CANDIDATE_FINDING"` — yet that exact string is written into both test code and evidence JSON for the corpus's single strongest finding (the stale-approval replay). Harmless at runtime (Python doesn't enforce `Literal`), but it means there is no automated gate that would have caught an undeclared outcome value.

**This matters for the audit specifically because §3 of the original brief asked to classify outcomes as ENFORCED/VIOLATED/UNREPRESENTABLE/UNVERIFIABLE/N/A "and never collapse into PASS/FAIL."** Crypton's actual, frozen, intentional system *is* a PASS/FAIL-shaped vocabulary (with two extra values for incompleteness). Reporting this honestly matters more than forcing a fit to a vocabulary Crypton doesn't use.

### 5.4 Evidence-ID scheme — defined, used once, chain-of-custody never populated

`E-###` is real and populated in the one existing instance (`reference-agentlab-dryrun/evidence_manifest.json`, 8 entries, cross-referenced correctly in the rendered report). But `timestamp`, `reference`, and `integrity_hash` are `null` in every one of those 8 entries — the "hard" custody fields defined in `06-evidence-standard.md` field 16 have **never** been populated with a real value anywhere in this repository. Crypton's own Internal QA review already flagged this exact gap (`10-internal-qa-review.md`: the standard says to compute an integrity hash "where practical" without saying how).

### 5.5 Finding Library structure vs. the spec's requested fields

`07-finding-library.md`'s entries (FL-02/03/04) have rich prose (typical causes, detection guidance, remediation, confidence, related findings) but lack discrete `test_id`, `invariant`, `preconditions`, and `reproduction-count` fields. The separate, engagement-level `findings.json` schema is more structured (`finding_id`, `area`, `status`, `observation`, `expected_control`, `evidence_ids`, `severity`, `confidence`) but still has no dedicated `invariant` or `preconditions` field — the closest equivalents are free-text `observation`/`expected_control`, which is a real, not cosmetic, gap if a future customer report needs to state a formal invariant.

### 5.6 Report-template proliferation

Three separate documents each call themselves "the customer report template," in a stated lineage (each extends, not replaces, the last) but with genuinely different section counts: `docs/business/ASSESSMENT_PACK.md` §E (8 sections) → `docs/methodology/07-customer-report-template.md` (adds Appendix A) → `assessment-kit/v1.0/09-customer-deliverable-templates.md` (expands into 11 separate deliverables: adds Risk Summary, Finding Detail, Engineering Appendix, Leadership Summary, Assessment Certificate, Management Presentation). No single document is "the" template; the kit's own README says the 8-section skeleton's rules "still apply" underneath the 11-deliverable expansion — this is navigable but not obvious to a new assessor without cross-referencing three files.

### 5.7 VALIDATION-REPORT.md's reclassifications (complete list, for the record)

1. EXP-001 original `finding-001`: CONFIRMED-shaped claim → **SUPERSEDED** (synthetic, author-induced specimen; no reusable diagnostic value). Excluded from the customer-facing library.
2. `finding-001b`/`001c`: stay CONFIRMED, moved into a Known-Safe-Patterns appendix.
3. `finding-001d`+`001e` (merged as FL-01): CONFIRMED → **retired, 2026-09-27, as `NO VULNERABILITY — EXPECTED FRAMEWORK BEHAVIOR`** (see §2d) — the corpus's single largest reclassification.
4. EXP-003A H3, EXP-003C H1c/H6: substance unchanged, relabeled into the formal taxonomy (`EXPECTED DESIGN`, `APPLICATION DESIGN CHOICE`).
5. EXP-005 FINDING-005-1: stays CONFIRMED, `DOCUMENTATION GAP` tag added.
6. EXP-005 FINDING-005-2: stays CONFIRMED, confidence raised high → very high on independent re-derivation.
7. EXP-005 FINDING-005-3: informal "informational" → formal **QUALITY ISSUE**, **excluded** from the customer-facing library (a maintainer-facing code-hygiene note, not a reusable pattern).
8. EXP-005's PR #4813 context note: confirmed N/A — never presented as a Crypton finding.

**No finding was found to be a false positive across this entire re-review pass.** That is a genuine, citable strength.

### 5.8 Four self-readiness verdicts that don't cross-reference each other

- `VALIDATION-REPORT.md` §8: **"READY WITH LIMITATIONS."**
- `11-pilot-readiness-report.md`: **"GO — with explicit, disclosed scope limits, not a blanket GO,"** conditioned on 4 stated conditions.
- `12-operational-readiness-review.md`: **"READY TO OPERATE,"** function-by-function (pricing explicitly still "a hypothesis, not validated").
- `pilot-pack/v0.1/11-pilot-pack-red-team.md`: **"READY WITH CONDITIONS, not READY FOR FIRST DESIGN PARTNER"** — because its own "pre-customer credibility gate" (2 independent Validation Benchmark cases + 1 external methodology review) is only half-satisfied: the benchmark half is done (VB-001+VB-002), the external-review half is not (`EXTERNAL-REVIEW-BRIEF.md` exists but has explicitly never been sent to anyone).

All four agree on the same underlying facts (zero completed customer engagements, narrow research corpus, unvalidated pricing) but use four different labels and never cite each other. The most conservative and most recently-dated of the four (`pilot-pack`'s red-team) should be treated as controlling until the external review it names actually happens — sending `EXTERNAL-REVIEW-BRIEF.md` is explicitly a founder action, not something further repository work can substitute for.

---

## 6. Authority Diff Readiness

Crypton can construct a **topologically correct** Authority Map for its own lab today — agent-lab's chain-verification trace (`chain_verified` event: `allowed`, `root_principal`, `acting_principal`, `delegation_depth`, `authority_path`, `chain_hash`, `failed_hop`, `denial_reason`) is real, structured, and sufficient to answer "who is accountable, who acted, how deep, where did it fail," entirely from the trace, for Crypton's own reference system.

For an **external target**, evidence sufficient to populate both sides of an Expected-vs-Observed diff exists in exactly one place: the Validation Benchmark (VB-001/VB-002), and only for a narrow, single-hop authorization-boundary property (session binding, read-only enforcement) — not a multi-hop delegation chain, because neither CVE target has one. EXP-005's findings are prose assertions about what OpenHands' source *would* do, not a captured expected-vs-observed pair with both sides evidenced. The semantic-rulebook Candidate 4a work is the closest thing to a real multi-hop-adjacent diff against a real framework (Mastra) — but per §2h, it has no captured run artifact, so it cannot currently back a diff node/edge with evidence rather than assertion.

**Conclusion:** the Authority Diff *concept* is sound and the *schema* to hold it could be assembled from existing pieces (worksheets + chain-trace field names), but **no artifact today shows a populated Authority Diff, with both sides evidenced, against an external target.** This is not an evidence-organization problem — it is an un-run experiment, but a cheap one: the semantic-rulebook harness already does the hard technical work; it just needs its output captured and committed (see §11, item 2).

---

## 7. Gap Classification

**Organization gaps** (evidence exists, just not indexed/packaged):
- The three customer-report-template documents (§5.6) — consolidate into one canonical doc with the others marked historical.
- The finding-library's prose fields could be lightly restructured into the missing `invariant`/`preconditions` fields without new research — the underlying facts already exist in the source experiment write-ups.
- semantic-rulebook's harness code and `RESULTS.md` are real and substantive but sit uncommitted with no git provenance — a `git add`/commit and one real captured run away from being usable evidence (see Metadata gap below too).

**Metadata gaps** (evidence exists, key metadata about it doesn't):
- Chain-of-custody fields (`timestamp`, `reference`, `integrity_hash`) in the Evidence Standard — defined, never populated, anywhere.
- No commit/version pin ties any agent-lab evidence artifact to the specific commit it was generated from.
- EXP-003C has no pinned dependency manifest (no requirements.txt/lockfile) — the only Q5-failure among the three EXP-003 experiments.
- Validation Benchmark JSON lacks run_id/timestamp/environment fields (present only as README prose).

**Observability gaps** (the target likely has the information; Crypton didn't capture it):
- semantic-rulebook's Candidate 4a/2 runs against Mastra — the harness clearly *can* observe (it drives a real ledger), but no run was captured to a file.
- EXP-005 — deliberately static-only; live confirmation of either surviving finding has never been attempted (this is disclosed, not hidden, in EXP-005's own docs).

**Representation gaps** (the system doesn't structurally capture the fact at all):
- Agent-lab's `approvals` table stores an argument *hash*, not the raw approved arguments — by design, but it means the DB alone cannot reconstruct "what was approved" without the caller separately logging it.
- No `role`/entitlement concept exists in agent-lab beyond the tool allow-list.

**Experiment gaps** (never tested, no control/calibration exists):
- Forged/compromised authority, concurrent/multi-agent races, multi-tenant isolation, supply-chain integrity — confirmed absent by `13-gap-analysis.md` and this audit's own review.
- EXP-007B (revocation between approval and resume) — explicitly specified, explicitly not built.
- No experiment anywhere tests replay/nonce consumption for approvals.

**Customer-context gaps** (can only be supplied by a real customer — correctly left blank, not a defect):
- Every "customer owner" / "risk acceptance" / "production criticality" field across every template.
- Pricing validation ($2,500 explicitly labeled a hypothesis).

---

## 8. Coverage Numbers

Three separate measurements, kept separate per the audit's own instruction not to produce one blended score.

**A. Directly Captured Coverage — Assessment Metadata schema (`engagement.json`, the field set a real customer engagement would actually populate)**
- Numerator (PRESENT as a structured field): `title`, `label`, `date`, `assessor` = **4**
- Denominator (fields the spec asks for that are applicable to an engagement record): 15 (assessment_id, client identifier, assessment type, assessment scope, tested workflow, tested layer(s), environment, framework/product, framework/product version, commit/hash, test date/time, assessor, configuration, excluded scope, limitations)
- **4 / 15 ≈ 27%.**
- What counts: only fields with a dedicated, named key in `engagement.json` itself — a pointer to a freeform prose file does not count as the field being captured, since nothing forces its content.

**B. Recoverable Coverage — same field set, PRESENT + safely DERIVABLE**
- Adding fields recoverable *without guessing* from the file pointers, once those files are actually filled in per their own instructions: `assessment scope` (from `scope_file`), `limitations` (from `limitations_file`) — these are DERIVABLE, not PRESENT, because the schema doesn't force their content, but a filled-in file would answer the question without inference. `tested workflow` and `environment` are **not** counted here — they have no dedicated file at all, only prose that may or may not appear inside `workflow.md`/`scope.md`.
- Numerator: 4 (A) + 2 (derivable) = **6**
- **6 / 15 = 40%.**
- Biggest reason it isn't higher: `assessment_id`, structured client identifier, `framework/product`+version, and `commit hash` have no field *and* no pointer to recover them from — they are simply absent from the schema today.

**C. Report-Ready Coverage — proposed 10-section report structure (§4)**
- Numerator (sections rated READY): Findings Summary, Detailed Finding Cards, Limitations, Technical Appendix = **4**
- Denominator: 10 proposed sections
- **4 / 10 = 40%.**
- The other 6 are PARTIAL (Decision Sheet, Authority Map/Diff, Action Queue, Verified Controls, Evidence Index) or MISSING (Retest Record) — none are un-buildable, but each needs either a template-assembly pass (cheap) or, for Authority Diff and Retest Record specifically, a real engagement's data (not cheap, not fabricatable).

These three numbers deliberately measure different things and should not be averaged: (A) measures schema thinness, (B) measures how much of that thinness is fixable by just filling in existing files, (C) measures deliverable-template readiness independent of the underlying schema. All three point the same direction: the apparatus is more built than filled-in.

---

## 9. What NOT to Re-Test

- **RAT-6A (multi-hop delegation with scope attenuation)** — 263 tests, including deliberate adversarial cases (tampered rows via raw SQL, cycle/depth guards, ancestor revocation/expiry). Re-running or re-litigating this is unnecessary; the evidence is thorough and the milestone's own document already states exactly what it does *not* claim (RAT-6B, gateway enforcement, real WebAuthn wiring).
- **EXP-003A/B/C's H1 (argument binding) and H2 (delegation boundary) results** — genuinely comparable, real-execution-backed across all three frameworks. Safe to cite as "held across three independent frameworks" without further work.
- **EXP-001b/c (LangGraph's two documented safe HITL patterns)** — real, sourced, unchanged through two independent review passes. Safe to reuse as-is in a "Known-Safe Patterns" section.
- **VB-001 and VB-002** — real, dated, version-pinned, negative-controlled. No reason to re-run; if anything, a third case would extend the benchmark, not re-validate the first two.
- **EXP-005's static findings, as static findings** — the underlying source citations are real and independently re-checkable by anyone with the OpenHands repo at the pinned commit. Re-reviewing the *citations* is unnecessary; what's missing is *live* confirmation, which is a different, new activity, not a re-test.
- **EXP-007A's 16 tests and determinism proof** — solid as far as it goes (a stateless, single-message test). Don't re-run it to answer a continuity-across-time question it was never designed to answer — that's EXP-007B's job, not a re-test of 007A.

---

## 10. What Must Be Captured Going Forward — Minimum Mandatory Evidence Schema

**MUST CAPTURE** (justified by: the customer report cannot be defensibly assembled without it, or a repeated gap in this audit demands it):
1. **`assessment_id`** on every engagement record — without it, two engagements against the same customer over time cannot be distinguished or linked in a retest record.
2. **A structured `target_commit`/`target_version` field** on every finding or evidence item that references a specific piece of external code — this exact gap (no commit pin anywhere in agent-lab's own evidence, no dependency manifest in EXP-003C) recurred in three of the five areas audited.
3. **`run_id` + `timestamp` on every evidence item that comes from an actual execution** — EXP-001's traces already do this correctly; it should be the baseline everywhere, not the exception (Validation Benchmark and EXP-007 both fall short of it today).
4. **An explicit `evidence_gate_status` per finding** (did it meet controlled-execution and effect-confirmation, or is it static/source-only) — this is the single distinction (REAL EXECUTION vs. MIXED vs. NARRATIVE-ONLY) every inventory pass in this audit had to reconstruct by hand from prose; making it a required field would prevent a future overclaim by construction, the same way `Finding.__post_init__` already prevents an unevidenced PASS/FAIL.

**SHOULD CAPTURE:**
5. Environment manifest (OS, language runtime version, key dependency versions) as a structured object, not README prose — closes the Validation Benchmark's and EXP-007's gaps cheaply.
6. `invariant` and `preconditions` as discrete fields on finding-library entries — the underlying content already exists in prose in every surviving entry; this is a restructuring task, not new research.
7. Chain-of-custody fields (`integrity_hash` at minimum) actually populated the next time real evidence is captured, closing the one gap Crypton's own QA review already flagged.

**OPTIONAL:**
8. A distinct lead-assessor vs. reviewer field on `assessor` — only matters once Crypton is not a single-assessor operation (currently explicitly true per `12-operational-readiness-review.md`).
9. A numeric reproduction count field — most findings already state this in prose (5/5, 38/40, 10x/4x); making it structured is a nice-to-have, not blocking.

---

## 11. Highest-Leverage Next Actions

Ordered by: (1) improves report defensibility, (2) unlocks Authority Diff, (3) reduces future manual work, (4) requires no speculative platform-building.

1. **Reconcile the outcome vocabulary** (§5.3): either edit `docs/methodology/02-threat-model.md` and `04-framework-adaptation-guide.md` to footnote/defer to the frozen 4-value vocabulary, or formally document the two-tier model (research vocabulary vs. customer-facing frozen vocabulary) in one place all three link to. This is a doc-editing pass, not research — and it's the single most likely thing to cause an inconsistent customer report if skipped.
2. **Capture and commit a real run of the semantic-rulebook Candidate 4a/2 harness** (§2h, §6): the code is real and already does the hard work; running it once and committing the JSON output turns "narrative-only" into "real execution evidence" and is the cheapest way to strengthen the Authority Diff story against a real framework (Mastra) rather than only a synthetic one (agent-lab).
3. **Populate the chain-of-custody fields** (`integrity_hash`, `reference`, `timestamp`) the next time any real evidence is captured — a process fix Crypton's own QA review already identified; no new tooling required.
4. **Consolidate the three customer-report-template documents** (§5.6) into one canonical document, with the other two marked explicitly historical/superseded — prevents a future assessor from working off a stale 8-section skeleton by accident.
5. **Add a `target_commit`/`target_version` structured field** to the evidence and engagement schemas (§10.2) — closes a gap that independently recurred in agent-lab, EXP-003C, and the Validation Benchmark.
6. **Restructure `07-finding-library.md`'s entries to add `invariant`/`preconditions` fields** using content that already exists in prose in each entry's source write-up — a packaging task, not new research.
7. **Send `pilot-pack/v0.1/EXTERNAL-REVIEW-BRIEF.md`** to an outside technical reviewer — this is the one action every internal self-readiness document agrees is the actual remaining gate, and it is explicitly a founder action, not something further repository work can substitute for.
8. **Run one worksheet-based intake against a real, cooperative, even informal target** (a friendly design partner's staging system, or a second self-directed target chosen for unfamiliarity like EXP-005 was, but this time live/interactive rather than static) — this is the only way to test the customer-facing authority-ledger capture process (§3, §6) rather than continuing to validate it only in the abstract.
9. **Fix EXP-003C's missing dependency manifest** — add a `requirements.txt`/lockfile so the one experiment lacking reproducibility parity with A and B gets it; a 10-minute fix relative to its research value.
10. **Correct the EXP-003B `Outcome` type** to include `"CANDIDATE_FINDING"` (§5.3) so the schema that's supposed to gate outcomes actually reflects the value already being written into it — a one-line fix that closes a real, if minor, rigor gap in the corpus's strongest single finding.
