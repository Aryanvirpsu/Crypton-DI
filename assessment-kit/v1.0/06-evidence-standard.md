# Crypton Evidence Standard v1.0

**Who uses this:** the assessor, writing up every finding (SOP Stage
12); the engagement lead, quality-gating the evidence manifest (SOP
Stage 10) before delivery.
**When:** continuously during Stages 9-10, finalized at Stage 12.
**Why it exists:** defines the canonical evidence package every finding
must carry — the single schema the report renderer, the Finding
Library, and every customer deliverable template all assume.

This extends, and does not replace,
`docs/methodology/05-evidence-reporting-standard.md`'s validated
eleven-link chain (Hypothesis → Threat → Test → Evidence → Observed
Behaviour → Expected Behaviour → Security Property → Risk →
Recommendation → Reproduction → Confidence) and the code-enforced
`assessment/models.py::Finding`/`EvidenceReference` schema. Five fields
below are new in Assessment Kit v1.0 (Architecture Context, Root Cause,
Impact as distinct from Risk, Supporting Artifacts as a first-class
list, Chain of Custody) — each is an **operational packaging
convention**, not a new empirical claim requiring multi-target
validation, so none carries a FUTURE CAPABILITY label; they organize
evidence that was already being collected, into a more complete
customer-facing package.

---

## The Sixteen-Field Canonical Evidence Package

Every finding, PASS or FAIL or INCONCLUSIVE or NOT_APPLICABLE, is
written through all sixteen fields. Where a field is not applicable to
a given outcome (see "Field applicability by outcome" below), state
that explicitly — never omit the field silently.

| # | Field | Definition | Relationship to the validated 11-link chain |
|---|---|---|---|
| 1 | **Hypothesis** | Which universal hypothesis (H1-H10) or operational area (1-8) this finding tests. | = link 1, unchanged |
| 2 | **Threat** | Which of the eleven attacker objectives this defends against. | = link 2, unchanged |
| 3 | **Security Property** | Which pillar(s) this bears on. | = link 7, moved earlier in the package for customer readability (state what's being checked before what was found) |
| 4 | **Architecture Context** *(new)* | Where this finding sits in the customer's actual system — which worksheet(s) it traces to (Identity/Authority/Delegation/Execution/Trust-Boundary/Approval/Persistence/Audit/Tool graph), and the specific component/action affected. | Packages what was previously left implicit in "Test" (link 3) into an explicit, worksheet-traceable field |
| 5 | **Test** | The specific, concrete action taken — controlled staging test, source review, document inspection. | = link 3, unchanged |
| 6 | **Evidence** | Reference(s) to something inspectable, not a claim. | = link 4, unchanged |
| 7 | **Observed Behaviour** | What actually happened, stated as fact. | = link 5, unchanged |
| 8 | **Expected Behaviour** | What the security property under test requires. | = link 6, unchanged |
| 9 | **Root Cause** *(new)* | The specific mechanism that produced the gap (or, for PASS, the specific mechanism that correctly prevents it) — one level more specific than "Observed Behaviour," answering "why does this happen," not just "what happens." | New — the validated corpus already does this informally in prose (e.g. `finding-001d`'s exact quoted `langchain.agents.factory` loop); this field makes it a required, separately-labeled part of every finding, not optional prose. |
| 10 | **Impact** *(new)* | The concrete, real-world consequence if this is exploited or continues unaddressed — stated independent of severity scoring, in language a non-technical stakeholder can read. | New — separates "what could happen" (Impact) from "how bad is that, weighed against the seven factors" (Risk), which the original 11-link chain folded together under "Risk." |
| 11 | **Risk** | FAIL only (including a "candidate finding"-qualified FAIL — see `01-assessor-handbook.md` §9 on why this is not a separate status) — severity, reasoned via the seven-factor rubric. | = link 8, now explicitly the *scoring* half of what Impact + Risk together cover |
| 12 | **Recommendation** | The control to add/fix — never a vendor or a Crypton product pitch. | = link 9, unchanged |
| 13 | **Reproduction** | How to reproduce the result independently — command, file, exact steps. | = link 10, unchanged |
| 14 | **Confidence** | How certain the conclusion is, and why — reasoned statement, per the Confidence Model (`01-assessor-handbook.md` §10). | = link 11, unchanged |
| 15 | **Supporting Artifacts** *(new)* | The literal list of files/captures/screenshots/traces that back this finding, each with a Chain of Custody entry (field 16). | Packages what was previously scattered across "Evidence" references into one enumerated, deliverable-ready list — this is what actually ships in the Evidence Appendix (`09-customer-deliverable-templates.md` §7). |
| 16 | **Chain of Custody** *(new)* | For each Supporting Artifact: who collected it, when, from where, and its integrity hash if practical to compute. | New — required for any artifact that could later be disputed (a customer engineer saying "that's not what our system does") or that contains sensitive material subject to the engagement's data-handling terms (`docs/business/ASSESSMENT_PACK.md` §C). |

### Field applicability by outcome

| Field | PASS | FAIL | INCONCLUSIVE | NOT_APPLICABLE |
|---|---|---|---|---|
| Hypothesis, Threat, Security Property, Architecture Context, Test | Required | Required | Required | Required |
| Evidence | Required (≥1) | Required (≥1) | Optional — evidence *for why it's not applicable/unavailable* still required where available | Required — evidence *for why it's not applicable* (e.g. a source excerpt showing the primitive doesn't exist) |
| Observed / Expected Behaviour | Required | Required | State what was attempted and what happened up to the point evidence ran out | Required — state what was checked to confirm non-applicability |
| Root Cause | Required (what makes it hold) | Required | Optional | N/A |
| Impact | N/A (nothing to size) | Required | N/A | N/A |
| Risk (severity) | N/A | Required | N/A | N/A |
| Recommendation | N/A (or an INFORMATIONAL hardening note) | Required | N/A | N/A |
| Reproduction | Required | Required | State what was attempted | N/A |
| Confidence | Required | Required | Required (why evidence couldn't be obtained) | Required |
| Supporting Artifacts + Chain of Custody | Required if any staging test/trace exists; otherwise cite source location | Required | Whatever partial evidence exists | Required (the artifact showing non-applicability) |

This table is a direct extension of
`docs/methodology/05-evidence-reporting-standard.md`'s "What to do when
a link is genuinely unavailable" section — nothing here loosens that
document's rule that INCONCLUSIVE and NOT_APPLICABLE still require a
stated, specific reason.

---

## Worked Example (all sixteen fields)

Using Crypton's own `finding-001d`/`finding-001e` (merged per
`VALIDATION-REPORT.md` §1/§5). **Note (2026-09-27):** this specific
finding was subsequently closed as `NO VULNERABILITY — EXPECTED
FRAMEWORK BEHAVIOR` and its library entry (formerly FL-01) retired —
see `labs/experiments/001-langgraph-approval-drift/CLOSURE.md`. It is
kept as the worked example below purely because it is the corpus's
most complete sixteen-field evidence package, illustrating what a
complete package *looks like structurally* — not as a standing
vulnerability claim. This is Crypton's own research evidence, never a
customer's.

1. **Hypothesis:** H1 (Argument Binding), extended to a composition
   scenario H1 alone did not originally anticipate.
2. **Threat:** Modify execution / Bypass approval.
3. **Security Property:** Pillar 4 (Human Approval), Pillar 2 (Authority).
4. **Architecture Context:** Approval Graph worksheet — the
   `isolate_endpoint` gated action, in a `create_agent` workflow
   composing `HumanInTheLoopMiddleware` (real approval gate) with
   `HostEscalationMiddleware` (a custom `after_model` hook).
5. **Test:** deterministic scripted-planner A/B (`run_experiment_middleware.py`,
   natural order vs. order-swapped control), then real-model
   confirmation (`run_experiment_llm.py`, `gpt-4o-mini`, two
   temperatures, N=20 each).
6. **Evidence:** `labs/experiments/001-langgraph-approval-drift/finding-001d.md`,
   `finding-001e.md`, `traces/*.jsonl` (gitignored, regenerable), the
   exact quoted `langchain==1.4.0` graph-building loop.
7. **Observed Behaviour:** the human approved `host="workstation-42"`
   twice; the tool executed `host="10.0.0.0/24"` — a value never shown
   to or approved by the human.
8. **Expected Behaviour:** the executed arguments should match the
   arguments the human approved, re-verified at execution time.
9. **Root Cause:** `langchain.agents.factory`'s graph-building code wires
   multiple `after_model` middlewares in **reverse** list order (the
   last middleware in the list runs first, right after the model; the
   first middleware runs last, right before tool execution) — an
   undocumented property. `HostEscalationMiddleware`, listed first
   (intending to run "first"), actually runs last, rewriting the
   already-approved call immediately before execution.
10. **Impact:** a SOC analyst approving containment of one host could,
    with no additional action on their part and no new approval prompt,
    have an entire subnet taken offline — under an incident that, per
    the customer's own audit trail (if it only records "approved:
    yes"), would appear fully authorized.
11. **Risk:** not scored in Crypton's own research corpus (no customer
    engagement produced this finding — see `VALIDATION-REPORT.md` §1);
    a customer engagement finding the same pattern would score against
    the seven-factor rubric using this Impact statement as its starting
    point.
12. **Recommendation:** recompute the approval check against the
    arguments about to actually execute, immediately before execution —
    not against what was proposed when the human was asked. Bind
    approval to a digest of (tool, resource, canonical arguments) and
    re-verify immediately pre-execution, regardless of how many
    `after_model` hooks ran in between or in what order.
13. **Reproduction:** `cd labs/experiments/001-langgraph-approval-drift && python run_experiment_middleware.py --runs 5` (natural order, 5/5 FAIL) `&& python run_experiment_middleware.py --order hitl_first --runs 5` (control, 5/5 PASS).
14. **Confidence:** High. Deterministic mechanism confirmed 5/5 with an
    isolating A/B control; real-model rate (38/40, 95%) independently
    confirms the mechanism fires under ordinary model behavior, not just
    a scripted worst case.
15. **Supporting Artifacts:** `finding-001d.md`, `finding-001e.md`,
    `writeup.md`, JSONL trace samples quoted verbatim in each write-up.
16. **Chain of Custody:** all artifacts generated by the researcher's own
    local, self-hosted harness (`capture.py`/`oracle_judge.py`), no
    external data involved, git-committed write-ups serve as the
    integrity record (commit hash = custody timestamp). For a real
    customer engagement, this field must instead name the specific
    assessor, collection timestamp, and source system for every
    artifact — see "Chain of Custody Rules for Customer Engagements"
    below.

---

## Evidence Quality Requirements

- **Specific over general.** "We reviewed the code" is not evidence;
  the file, function, and exact behavior observed is.
- **Cite the installed version, not a remembered or stated one.** Every
  source-code citation states the exact package/commit version it was
  read against.
- **Quote, don't paraphrase, for a framework/product's own claims.**
  When citing what a third-party framework's documentation or source
  says, quote it verbatim with a source reference — paraphrase risks
  misrepresenting the claim, and every finding in the validated corpus
  that cites third-party behavior does so with a direct quote
  (`finding-001d`'s `langchain` source quote, `finding-001b`'s LangChain
  docs quote, EXP-005's `LLMSecurityAnalyzer.security_risk()` quote).
- **A live observation outranks a static citation.** Where both are
  available, prefer a controlled staging test's actual observed result
  over a source-review inference — see the Confidence Model
  (`01-assessor-handbook.md` §10) for how this affects confidence
  scoring.
- **Never let a Crypton-reference demonstration stand in for customer
  evidence.** A demonstration using Crypton's own Agent Lab reference
  implementation may motivate or illustrate a question during the
  engagement. It is never itself a Supporting Artifact for a finding
  about the customer's system.
- **Redact before collecting, not after.** Any evidence capture
  (screenshot, log excerpt, trace) containing customer secrets, PII, or
  credentials is redacted at collection time — never stored unredacted
  "to redact later."

## Chain of Custody Rules for Customer Engagements

For every Supporting Artifact collected during a real customer
engagement:

1. Record who collected it (assessor name), when (timestamp), and from
   where (the specific system/environment — staging, not production,
   unless explicitly authorized).
2. Compute an integrity hash (`sha256:<hex>`) where practical — for text
   captures and trace files, always; for live screen recordings,
   where tooling supports it.
3. Store evidence only for the retention period specified in the
   Authorization-to-Test's data-handling terms
   (`docs/business/ASSESSMENT_PACK.md` §C); confirm destruction on
   request or at the end of the stated retention window.
4. Never retain customer secrets/credentials beyond what's needed to
   support a specific finding, and never in the report or evidence
   appendix itself — redact and reference by description instead.
5. If an artifact's provenance is later disputed by the customer, the
   chain-of-custody record (collector, timestamp, source, hash) is what
   resolves the dispute — this is why fields 1-2 are not optional for
   any artifact backing a FAIL finding.

## Relationship to `assessment/models.py`

`Finding`/`EvidenceReference` remain the code-enforced schema for real
engagements. This sixteen-field standard is the **reporting-layer**
convention built on top of that schema: fields 1-3, 5-8, 11-14 map
directly onto existing `Finding` fields (`area`, `status`, `observation`,
`expected_control`, `severity`, `risk_impact`, `recommendation`,
`confidence`); fields 4, 9-10, 15-16 are new, currently free-text
additions an assessor writes into the report template
(`09-customer-deliverable-templates.md`) rather than new code-enforced
`Finding` fields — per the Framework Adaptation Guide's "extend, don't
fork" discipline, these become code-enforced fields only if repeated
customer engagements show the need, not preemptively.
