# Crypton Agent Authorization Assessment Methodology — EXP-004

**Status: v0.1, internally derived from real evidence, not yet
field-tested against an external customer or an unfamiliar (non-lab)
target.** This is the document set every future assessment, experiment,
and customer engagement should follow.

Read in this order (the order they were built in, each depending on the ones before it):

1. [Security Property Catalog](01-security-property-catalog.md) — the seven pillars (Identity, Authority, Delegation, Human Approval, Execution, Persistence, Auditability): the universal, framework-independent language for what Crypton checks.
2. [Threat Model](02-threat-model.md) — the eleven attacker objectives, each mapped to a pillar, a `CLAUDE.md` §7 failure class where one exists, and a current evidence status (PASS / partial / coverage gap).
3. [Assessment Methodology v0.1](03-assessment-methodology-v0.1.md) — the 13-stage pipeline (Architecture Review through Remediation) every engagement or experiment follows, and how it relates to the existing `docs/ASSESSMENT_METHODOLOGY.md`.
4. [Framework Adaptation Guide](04-framework-adaptation-guide.md) — how the pipeline's stages 1-8 (framework-independent) meet stage 9 (framework-specific) for OpenAI Agents SDK, Microsoft Agent Framework, and LangGraph, plus the procedure for adapting to a fourth target.
5. [Evidence & Reporting Standard](05-evidence-reporting-standard.md) — the eleven-link chain every finding must satisfy (Hypothesis → Threat → Test → Evidence → Observed → Expected → Security Property → Risk → Recommendation → Reproduction → Confidence), worked through a real example.
6. [Assessment Execution Playbook](06-assessment-execution-playbook.md) — the literal day-by-day sequence, customer and research-lab variants.
7. [Customer Report Template](07-customer-report-template.md) — **HISTORICAL / SUPERSEDED as of 2026-09-30** for report structure (see `assessment-kit/v1.0/09-customer-deliverable-templates.md`, the current canonical report definition); its 11-link evidence-chain vocabulary is still live.
8. [EXP-003 Property ID Mapping](08-exp003-property-id-mapping.md) — added 2026-09-30: stable, framework-independent property IDs for comparing EXP-003A/B/C results, since their local `H1`/`H2`/`H3`... labels do not consistently mean the same property across the three experiments.

## What grounds this document set

Nothing here was written from first principles in a vacuum. Every
claim traces to one of:

- `docs/ASSESSMENT_METHODOLOGY.md` (v0.6.1, 2026-09-02) — the existing, code-enforced 8-area customer-engagement procedure (`labs/agent-lab/src/agentlab/assessment/models.py`).
- `docs/business/ASSESSMENT_PACK.md` (2026-09-02) — the existing SOW/sales/legal pack.
- `CLAUDE.md` §7 / `docs/strategy/2026-09-10-action-integrity-thesis.md` §6 — the settled ten failure classes.
- `labs/experiments/001-langgraph-approval-drift/` — `finding-001`, the one real, confirmed finding predating this methodology set.
- `labs/exp-003-openai-agents-sdk/`, `labs/exp-003-microsoft-agent-framework/`, `labs/exp-003c-langgraph/` — three completed cross-framework research runs, same H1-H6 hypotheses, same evidence discipline, real source-verified results (not simulated).
- `labs/exp-005-openhands-sdk/` — the first end-to-end run of this document set's full 13-stage pipeline against a genuinely unfamiliar real application (not a framework EXP-003 had already studied). Verdict: VALIDATED WITH ADAPTATIONS. See "Known gaps" below for what it changed.

## What this document set is not

- Not a claim that Crypton can automatically assess an arbitrary system — most of the actual procedure is manual review, same as `docs/ASSESSMENT_METHODOLOGY.md` already states.
- Not a replacement for anything listed above — every document here explicitly states its relationship to the existing artifact it extends, never silently duplicates or contradicts it.
- Not proof of product-market fit or customer demand — this is the technical playbook; `CLAUDE.md` §13's kill criteria govern whether the wedge itself is validated, unchanged by this work.

## Known gaps this methodology set itself surfaces

The [Threat Model](02-threat-model.md)'s coverage matrix is the honest
accounting: five of eleven attacker objectives are current coverage
gaps (Acquire authority, Expand authority, Forge authority, Lose
attribution, Persist privilege), and two more are only partially tested
(Confuse authority, Escape delegation). Three new hypotheses (H7 Scope
Expansion, H8 Attribution Durability, H9 Tool Substitution) are defined
but not yet run against any target. This was the concrete backlog this
document set itself identified as needing an EXP-005.

**EXP-005 (2026-09-16, `labs/exp-005-openhands-sdk/`) has now run** —
not by closing the H7-H9 coverage gaps directly (it didn't test scope
expansion or tool substitution either), but by doing the more basic
thing this backlog assumed would happen eventually: running the full
pipeline, stages 1-13, against a real, unfamiliar, non-framework target
for the first time. Result: **VALIDATED WITH ADAPTATIONS**, not
unqualified. Three real gaps in this document set were found and are
proposed for v0.2, not yet merged here:

1. Baseline Verification (Stage 7, `03-assessment-methodology-v0.1.md`)
   assumes live execution is always available and has no defined
   static-review-only substitute.
2. The Threat Model's coverage table conflates two different axes —
   "how much evidence does Crypton have" and "how well-mitigated is the
   target" — that a real customer-facing four-way outcome vocabulary
   forces apart.
3. "Framework-Specific Tests" (Stage 9) and the
   [Framework Adaptation Guide](04-framework-adaptation-guide.md) both
   assume a clean framework/application boundary; EXP-005's
   highest-value finding lived exactly at that seam (a framework
   primitive that was sound, composed with the framework's own
   reference-integration default in a way that silently defeated it).

A new hypothesis (**H10, Signal Independence** — is the signal gating
an authorization decision independent of the actor being gated, or
self-reported by it?) and a Pillar 3 (Delegation) checklist addition
("is the delegated party's own gate actually delivered to a human, or
silently auto-resolved by the delegation mechanism itself?") are
proposed, evidenced by two real findings against OpenHands SDK's public
source. See `labs/exp-005-openhands-sdk/README.md` §7-9 for the full
methodology-quality analysis and exact proposed language — not yet
incorporated into documents 01-07 above, pending review, per the same
"propose, don't unilaterally rewrite" discipline this document set has
followed since EXP-004.

H7-H9 themselves remain open — a candidate EXP-006, per the same
priority ordering laid out at the end of the Threat Model document.
