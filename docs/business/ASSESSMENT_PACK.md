<!-- Canonicalized into the repo 2026-09-02, from Aryan's working copy (Crypton-Assessment-Pack.md). Content unchanged. This is the customer-facing/legal/SOW pack — distinct from docs/ASSESSMENT_METHODOLOGY.md, which is the assessor's internal 8-area technical procedure. See that file's cross-reference note mapping the two. -->

# Crypton — Agent Security Launch Assessment Pack

**Everything needed to sell and deliver the first paid assessments.** Copy sections into a one-pager, SOW, and report as needed.
**Date:** 2 Sep 2026 · Treat price/scope as hypotheses to test, not doctrine.

---

## A. One-Pager (external-facing)

### Agent Authorization & Delegation Assessment
A focused, grey-box review of how authority actually flows through one action-taking AI agent workflow.

**The problem we check for:** When agents take real actions, most teams can't prove which agent did what, under whose authority, whether scope narrowed at each hop, or whether a risky action was stopped *before* it ran. Standard AI pentests check prompt injection and jailbreaks — not this.

**What we assess:**
- Agent identity & ownership — does each agent have its own identity, or is it borrowing a human's / a shared service account?
- Delegation & authority flow — can one agent grant authority to another? Can scope *increase* downstream? Can the chain be reconstructed?
- Authorization boundary — where is the check? Does execution happen before or after validation? What bypasses it?
- Human approval — what requires approval, and is approval bound to the exact request (or can the action be changed after approval)?
- Accountability — can you reconstruct who acted, under whose authority, on what resource, with which permission, when?

**What you get:** an executive summary, a trust-flow map of your agent system, prioritized technical findings with evidence and reproduction, an authorization map, delegation findings, a human-approval analysis, an evidence appendix, and a live remediation walkthrough with your engineers.

**Founding rate:** **$2,500** (design-partner pricing; standard rate $8,000–$10,000).
**Timeline:** ~3 testing days + report + remediation call.

**Optional add-on:** adversarial testing (prompt/tool injection, dangerous-argument, privilege-escalation-via-behavior) — separate scope.

> **CTA:** Book a scoping call.

---

## B. Statement of Work (SOW)

**Engagement:** Agent Authorization & Delegation Assessment (Founding).
**Scope (v1 core):**
- 1 agent workflow
- Up to 3 connected tools
- 1 staging/test environment
- 3 testing days
- 1 written report
- 1 remediation discussion (live walkthrough)

**In scope:** identity, ownership, delegation/authority flow, authorization boundary, human-approval binding, accountability/evidence reconstruction.
**Out of scope (unless add-on purchased):** prompt/tool injection, jailbreak, RAG/model-layer testing, production environment testing, load/DoS, social engineering, physical.
**Deliverable:** report (template §E) within [N] business days of testing completion.
**Fee:** $2,500, [payment terms]. Add-ons quoted separately.
**Assumptions:** customer provides staging access, intake questionnaire (§D) completed before start, a technical point of contact available during testing.
**Not provided:** compliance attestation/audit sign-off; this is a security assessment, not a certified audit.

---

## C. Rules of Engagement + Legal (do this BEFORE any paid test)

**Blocking pre-requisites:**
1. **Entity:** operate through a single-member LLC (liability separation).
2. **Signed Authorization-to-Test** (below) per engagement.
3. **Confirm authority:** customer warrants they own or are authorized to permit testing of the target environment.

**Authorization-to-Test — minimum terms (have a lawyer review once):**
- **Named target:** exact systems, URLs, environments in scope; everything else explicitly excluded.
- **Window:** testing dates/times.
- **Methods:** grey-box review, manual testing, traffic capture, scripted checks; no destructive actions; no production data exfiltration beyond evidence needed.
- **Data handling:** how findings/evidence are stored, for how long, and destruction on request; no retention of customer secrets.
- **Safe harbor:** customer authorizes the activity and won't pursue action for good-faith testing within scope.
- **Liability cap:** limit to fees paid; no consequential damages.
- **Confidentiality / NDA:** mutual.
- **Responsible disclosure:** findings shared only with customer.

**Operating rules during testing:**
- Stay strictly in named scope. If a serious issue appears out of scope, stop and report — don't test it.
- Log every action (command, timestamp) for the evidence appendix.
- No changes to customer systems beyond what's authorized.

---

## D. Intake Questionnaire (send before scoping/testing)

**Context**
1. What does the agent workflow do, in one paragraph?
2. Which agent framework(s)/runtime? (LangChain, AutoGen, custom, etc.)
3. Do you use MCP? If so, which servers/tools?
4. Single agent or multi-agent? If multi, describe the topology.

**Identity & ownership**
5. How does each agent authenticate today? (human's key, service account, OAuth token, per-agent identity?)
6. Who "owns" each agent internally? Who can authorize it?

**Permissions & tools**
7. Which tools/APIs can the agent call? What can each do (read/write/spend/delete)?
8. What data can the agent access?
9. Are any credentials reused across agents or elsewhere?

**Delegation**
10. Can one agent invoke or spawn another? What authority is inherited?
11. Is there any scope-narrowing between agents today, or does a sub-agent get everything?

**Authorization & approval**
12. Where is authorization checked relative to the action executing?
13. Which actions require human approval today? How is approval captured?

**Environment**
14. Staging/test environment available? Any restrictions or systems we must NOT touch?
15. Technical point of contact + availability during the testing window?

---

## E. Report Template

> **HISTORICAL / SUPERSEDED (2026-09-30) for report *structure*.** The
> canonical, current report definition is now
> `assessment-kit/v1.0/09-customer-deliverable-templates.md` (10 sections:
> Decision Sheet, Authority Map/Diff, Action Queue, Findings Summary,
> Finding Cards, Verified Controls, Limitations, Retest Record, Evidence
> Index, Technical Appendix — see `docs/audits/2026-09-30-evidence-report-readiness-audit.md`
> §5.6/§11 for why three overlapping template lineages existed and how
> they were reconciled). This 8-section skeleton is kept here, unedited,
> for provenance — it was the original shape `docs/methodology/07-customer-report-template.md`
> and the kit's own 11-deliverable expansion both extended, and its outcome
> vocabulary (PASS/FAIL/INCONCLUSIVE/NOT_APPLICABLE) is itself superseded
> by the five-outcome model in `docs/ASSESSMENT_METHODOLOGY.md`. Do not
> build a new report against this section — use the kit document instead.

```
# Agent Authorization & Delegation Assessment — [Customer]
Prepared by [Entity] · [date] · Confidential

## 1. Executive Summary
- What we assessed (1 workflow, N tools, staging).
- Top 3 things leadership needs to know.
- Overall authority-flow risk posture (1 paragraph, plain language).

## 2. Trust-Flow Map
Diagram + narrative: how authority currently moves —
Human → identity → agent → credentials → tools → resources.
Mark where authority is ambiguous or over-broad.

## 3. Technical Findings
For each finding:
- Title
- Severity (Critical/High/Medium/Low) + why
- Affected component
- Evidence (capture/screenshot/trace ref → appendix)
- Reproduction steps
- Actual security impact (what an attacker/agent could do)
- Recommended remediation (control-first, tool-agnostic)

## 4. Authorization Map
Table: Agent → identity → credentials → tools → resources → where the
authorization check sits → pre- or post-execution → bypass paths.

## 5. Delegation Findings (if multi-agent)
- The authority path across agents.
- Where scope fails to narrow (or widens).
- Whether the chain is reconstructable / revocation propagates.

## 6. Human-Approval Analysis
- What requires approval today vs what should.
- Is approval bound to the exact request? Can an approved action be modified after?

## 7. Evidence Appendix
HTTP/terminal captures, screenshots, traces, with timestamps.

## 8. Remediation Summary & Priorities
Ranked fixes, effort vs impact. Recommend the control, not a vendor.
```

**Report rules:**
- Recommend the *control*, not Crypton. Assessment credibility is the asset — don't spend it steering findings to your own product.
- No invented findings, no filler. If a section is clean, say so.
- Every claim has evidence in the appendix.

---

## F. Tooling Rule (from ops-plan §11)

Start manual. Allowed: Burp/Caido, terminal capture, screenshots, scripts, Agent Lab, chain verifier, Markdown, templates. **Do not build an automated scanner.** After each assessment ask: *which repeated manual step cost real time?* — automate only what recurs across multiple engagements.

> **Cross-reference, added when reconciling into the repo (2026-09-02):** `labs/agent-lab/src/agentlab/assessment/` (v0.6.1) already follows this rule — it is a Finding/Evidence/Report *model and renderer*, not a scanner; it does not decide PASS/FAIL on a customer's system. See `docs/ASSESSMENT_METHODOLOGY.md`.

---

*Price and scope are hypotheses. The goal of the first engagements is real commitment (money/access/time) + product-discovery evidence, not revenue.*
