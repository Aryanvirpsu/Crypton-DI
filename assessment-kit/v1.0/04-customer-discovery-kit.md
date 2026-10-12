# Crypton Customer Discovery Kit v1.0

**Who uses this:** the assessor (or engagement lead), during SOP Stage 1
(Intake), before any testing begins.
**When:** sent to the customer immediately after the Authorization-to-Test
is signed (Stage 0), collected before Day 1 scheduling.
**Why it exists:** this is the raw material for Architecture Review
(SOP Stage 2) — it produces understanding before testing starts, per
`docs/methodology/06-assessment-execution-playbook.md`'s "Day 1 begins
with understanding, not discovery" rule. It expands
`docs/business/ASSESSMENT_PACK.md` §D's 15-question intake into
per-question purpose/expected-answer/red-flag/follow-up detail, plus
three new inventories (Persistence, Risk) and two structured interview
guides that §D did not previously break out separately.

**How to use this:** send the seven written questionnaires as one
document/form. Hold the two interview guides as live conversations —
Engineering with the technical point of contact, Management with
whoever owns the budget/risk decision — typically on the same call
that confirms Day 1 logistics.

---

## A. Architecture Questionnaire

| # | Question | Purpose | Expected answer | Red flags | Follow-up |
|---|---|---|---|---|---|
| A1 | What does the agent workflow do, in one paragraph? | Baseline scope understanding before Day 1. | A specific, concrete description of one workflow and its real-world effect. | Vague ("it automates stuff"); describes multiple unrelated workflows (scope isn't actually 1 workflow yet). | "Which single workflow do you want assessed first?" |
| A2 | Which agent framework(s)/runtime? | Determines whether this is a validated target (`04-framework-adaptation-guide.md`: OpenAI Agents SDK, Microsoft Agent Framework, LangGraph, OpenHands SDK) or requires fresh adaptation. | A named framework + version. | "We built it ourselves" (hand-rolled) — not a red flag per se, but changes effort estimate materially; "not sure" is a red flag. | "What's the exact installed version? (Verify independently in Stage 2 — never trust this answer alone, per SOP standing rule.)" |
| A3 | Do you use MCP? If so, which servers/tools? | MCP tool-permission manifests are a real evidence category for Authority Mapping. | A list of MCP servers/tools, or "no." | Uses MCP but doesn't know which servers are configured — signals weak tool-inventory ownership. | "Who maintains the MCP server configuration?" |
| A4 | Single agent or multi-agent? If multi, describe the topology. | Determines whether Delegation (Pillar 3) checklist items are in scope. | A clear topology description (hierarchical, peer, parent/sub-agent). | "It's complicated, ask the engineers" with no further detail from the person answering. | "Can you sketch or link an architecture diagram?" |
| A5 | What is the deployment environment (cloud, on-prem, which provider)? | Determines staging-access logistics and whether the target overlaps Crypton's validated corpus's environment assumptions. | A specific deployment description. | "We don't have a separate staging environment" — flags a scoping conversation before Day 1, not a Day-1 surprise. | "Is there a staging/test environment we can test against, isolated from production?" |

## B. Identity Questionnaire

| # | Question | Purpose | Expected answer | Red flags | Follow-up |
|---|---|---|---|---|---|
| B1 | How does each agent authenticate today? | Feeds the Identity Graph worksheet directly. | A named mechanism: per-agent service account, OAuth client, API key, shared credential. | "It uses my/a developer's personal API key" — this is itself a likely FAIL on Area 1 before testing even starts; still test to confirm, don't assume. | "Is that credential shared with any other process or person?" |
| B2 | Who "owns" each agent internally? Who can authorize a new one? | Feeds the provisioning-process evidence requirement for PASS on Area 1. | A named role/team + a described process. | "Anyone with repo access can effectively add one" — signals no real provisioning gate. | "Walk me through the last time a new agent/integration was added — who approved it?" |
| B3 | Can an agent's identity be reassigned or reused after it's decommissioned? | Feeds Pillar 1's "does identity survive persistence" question. | "No — new registration required," or "not sure/never happened." | "Yes, we just repoint the credential" — signals a durable-identity FAIL. | "What happens to in-flight or historical records tied to a decommissioned agent's identity?" |
| B4 | Is agent identity distinguished from human identity in your systems (logs, IAM, audit)? | Directly probes Pillar 1's core definition. | A clear "yes, agents have their own identity records" or an honest "no, agents borrow human/service identities." | "We can't easily tell which actions were the agent vs. a human" — a real, common, reportable gap. | "If asked in an incident review, could you show which actions were agent-initiated?" |

## C. Delegation Questionnaire

| # | Question | Purpose | Expected answer | Red flags | Follow-up |
|---|---|---|---|---|---|
| C1 | Can one agent invoke or spawn another? What authority is inherited? | Determines whether Pillar 3 checklist items are in scope, and previews the attenuation question. | A described mechanism, or "no delegation exists." | "The sub-agent basically gets everything the parent has" — a likely Area 3 FAIL signal, still to be tested. | "Is there any code/config that narrows what a sub-agent can do relative to its parent?" |
| C2 | Is there any scope-narrowing between agents today, or does a sub-agent get everything? | Same as C1, phrased as a direct probe. | A specific narrowing mechanism named, or an honest "none." | "We haven't thought about that" — common, not itself disqualifying, but flags this as a likely finding area. | "If a sub-agent were compromised, what's the blast radius compared to the parent?" |
| C3 | If a sub-agent has its own confirmation/approval requirement, what happens when it needs one? | Directly probes the "Delegation Gate Delivery" checklist item (`03-assessment-checklists.md`, EVOLVING) — a real, validated failure pattern (FINDING-005-2) exists here. | A described human-in-the-loop mechanism for sub-agents specifically, or "sub-agents run fully autonomously, no separate approval." | "We're not sure — we didn't configure anything special for sub-agents" — a strong signal this checklist item will find something. | "Has anyone verified that a sub-agent's approval requirement actually reaches a human, end to end?" |
| C4 | Can a parent's delegation grant be revoked while a child is still running? | Feeds the EVOLVING Area-3-adjacent revocation-propagation-to-children question. | A described mechanism, or honest "untested/unknown." | "We've never tried it" — expected for many customers; note as a scoping/testing opportunity, not a customer failing. | "Is this something you'd want tested this engagement, if in scope?" |

## D. Tool Inventory

| # | Question | Purpose | Expected answer | Red flags | Follow-up |
|---|---|---|---|---|---|
| D1 | Which tools/APIs can the agent call? What can each do (read/write/spend/delete)? | Direct input to the Tool Graph worksheet and the SOW's "up to 3 connected tools" scoping. | A concrete list with read/write/spend/delete classification per tool. | A tool list that's "everything in our internal API gateway" — signals scope needs narrowing before Day 1, per the SOW default. | "Which of these tools, if misused, would cause the most damage? Prioritize testing there." |
| D2 | What data can the agent access? | Feeds sensitivity/business-impact reasoning in the severity rubric (factor 3). | A described data classification. | "It has broad read access to our data warehouse" with no scoping — flags a likely high-severity finding if authorization gaps exist elsewhere. | "Is any of this data regulated (PII, PHI, financial)?" |
| D3 | Are any credentials reused across agents or elsewhere? | Directly probes the Identity pillar's "distinct from a single execution instance" property and a common real-world root cause. | "No, each is distinct," or an honest "yes, shared." | "Yes" to shared credentials — likely Area 1/2 FAIL, prioritize testing this. | "If that shared credential were rotated, what would break, and how would you know?" |

## E. Approval Inventory

| # | Question | Purpose | Expected answer | Red flags | Follow-up |
|---|---|---|---|---|---|
| E1 | Which actions require human approval today? | Direct input to the Approval Graph worksheet. | A concrete list of gated actions. | "None — it's fully autonomous" — not disqualifying, but changes the engagement's Area 6 scope to NOT_APPLICABLE and shifts focus to Areas 2/4/5. | "Was that a deliberate decision, or has it just not come up yet?" |
| E2 | How is approval captured (UI click, Slack message, API call, something else)? | Feeds the mechanism-tracing needed for Area 6's binding question. | A concrete described mechanism. | "It's a Slack thumbs-up reaction with no record of exactly what was approved" — a strong signal for testing argument-binding specifically. | "Does the approval record capture the exact arguments, or just that 'the action' was approved?" |
| E3 | Has an approved action ever executed differently than what was shown to the approver — as far as you know? | Directly probes drift awareness, the exact class `finding-001d`/EXP-003C H1c evidenced. | An honest "not that we know of" or a described incident. | "We wouldn't necessarily know if it had" — this is itself informative: it means Area 6's evidence-reconstruction requirement (can drift be detected from records alone) likely fails, independent of whether drift has occurred. | "If it had happened, how would you find out today?" |
| E4 | Is there a "standing" or "always approve" mode, and if so, what's its exact scope? | Directly probes H3 (Standing Approval). | A described scope (tool-only vs. tool+arguments), or "no standing approval exists." | "We turned on always-approve for [tool] and don't remember exactly what that covers" — flags H3 as a priority test. | "Who can turn standing approval on or off?" |

## F. Persistence Inventory

| # | Question | Purpose | Expected answer | Red flags | Follow-up |
|---|---|---|---|---|---|
| F1 | Does the agent/workflow checkpoint or persist state (across restarts, sessions, or retries)? | Determines whether Pillar 6 checklist items (and Baseline C) are in scope. | A described persistence mechanism, or "no, fully stateless / in-memory only." | "Not sure" — flags a Stage 2 architecture-review action item before Day 1. | "What store holds that state (database, file, in-memory cache)?" |
| F2 | Can that persisted state be modified by anything other than the process that originally wrote it? | Directly probes the trust-boundary class EXP-003C's H1c evidenced (`update_state()`-shaped risk). | A described access-control boundary, or an honest "we haven't restricted this." | "Multiple services can write to that state store" — a strong signal for Area 7/Pillar 6 testing priority. | "Is write access to that state store logged/audited separately?" |
| F3 | If the workflow crashes and restarts, does any security-relevant configuration (approval policy, risk settings, credentials) reload correctly, or can it revert to a stale value? | Directly probes the exact class of Crypton's one confirmed real-world historical instance (OpenHands PR #4813, `VALIDATION-REPORT.md` §1). | A described single-source-of-truth mechanism, or an honest "never tested." | "We assume it reloads correctly but haven't verified" — flags this as a near-certain worthwhile test. | "Has this configuration ever silently reverted after a deploy or restart, that you're aware of?" |

## G. Risk Inventory

| # | Question | Purpose | Expected answer | Red flags | Follow-up |
|---|---|---|---|---|---|
| G1 | What's the worst realistic outcome if this workflow's authorization controls failed completely? | Directly feeds severity-rubric factor 3 (sensitivity/business impact) before testing even begins, so the assessor knows where to focus. | A concrete, specific answer (financial loss, safety incident, regulatory exposure, reputational harm). | A vague or minimizing answer ("probably nothing bad") for a workflow that D1/D2 show has write/delete/spend access — a mismatch worth probing directly. | "Has anything close to this happened, even at a smaller scale?" |
| G2 | Who is accountable if this workflow does something it shouldn't have? | Feeds the Auditability pillar's attribution question and the engagement's stakeholder map. | A named role/team. | "Not sure — it would depend" — a real, reportable organizational gap, independent of any technical finding. | "Has this been tested in an incident-response tabletop exercise?" |
| G3 | What have you already tried or built to control this risk? Why isn't it enough (or is it)? | Avoids re-discovering controls the customer already knows about; focuses testing on genuine unknowns. | A described control + an honest self-assessment of its limits. | "We haven't really assessed whether our controls work" — this is, in fact, the exact reason for the engagement; not a red flag, a confirmation of fit. | "What would 'enough' look like to you?" |
| G4 | What would this problem cost you if it went wrong — time, money, incidents? | Calibrates severity-rubric reasoning and the customer's own risk appetite for the report's executive summary. | A concrete estimate or range. | Cannot answer at all — flags that the Management Interview (below) needs to cover this instead. | "Who else in the organization feels this cost most directly?" |

## H. Engineering Interview Guide

Held live with the technical point of contact, typically on the
Day-1-scheduling call. Purpose: fill gaps the written questionnaires
above leave open, and calibrate what to prioritize on Day 1-2.

| # | Question | Purpose | Expected answer | Red flags | Follow-up |
|---|---|---|---|---|---|
| H1 | Walk me through the last time this agent took a consequential action against a real system. | Grounds the entire engagement in a real, recent, concrete instance rather than an abstract description. | A specific, traceable walkthrough. | Cannot recall a specific instance, or the workflow has never actually run against a real system yet — changes engagement scoping (may need to test against a lower-fidelity staging setup). | "Can you show me the trace/log from that specific run?" |
| H2 | How do you know which agent did what, right now, today? Show me. | A live demonstration is stronger evidence than a description — directly probes Auditability. | A live walkthrough of a real log/dashboard. | Cannot actually show this in real time — itself strong evidence for an Area 8 finding, worth noting even before formal testing. | "How far back does this retention go?" |
| H3 | What happens today if the agent does something it shouldn't have? | Probes incident-response maturity and whether any compensating control exists (severity rubric factor 7). | A described process. | "We'd probably just notice it in the output/results" — signals weak detection, relevant to severity reasoning on any FAIL found. | "Is there an alert, or would someone have to notice manually?" |
| H4 | How are you handling this for sub-agents or multi-agent workflows, if any exist? | Directly feeds Delegation checklist prioritization. | A described approach, or "we haven't specifically thought about sub-agents differently." | Same policy/controls assumed to "just apply" to sub-agents with no verification — flags the Delegation Gate Delivery checklist item as high-priority. | "Has anyone actually traced a sub-agent's action back through to a human decision?" |
| H5 | What have you already tried or built to control agent authorization? Why isn't it enough? | Avoids re-testing controls the team already knows are solid; focuses on genuine unknowns; mirrors CLAUDE.md's own Mom Test discipline (asking about their actual experience, not pitching). | A described control + honest self-assessment. | Overconfidence with no specific evidence offered ("it's fine, we built it carefully") — worth gently probing further before accepting at face value. | "What would convince you it's not fine?" |

## I. Management Interview Guide

Held live with whoever owns the budget/risk decision for this
engagement — may be the same person as the technical point of contact
at a small company, or a distinct stakeholder at a larger one.

| # | Question | Purpose | Expected answer | Red flags | Follow-up |
|---|---|---|---|---|---|
| I1 | What decision will this assessment's results inform? | Ensures the report is written for the decision it actually needs to support (a go/no-go on a launch, a budget ask for remediation, a board/compliance conversation). | A specific decision named. | "Just curious, no specific decision pending" — still a valid engagement, but changes how the Executive Summary should be framed. | "Who else needs to see the results to make that decision?" |
| I2 | Who owns this workflow's risk if something goes wrong? | Confirms G2's answer at the management level; surfaces any mismatch between engineering's and management's understanding of accountability. | A named role. | A different answer than Engineering gave to G2 — itself worth reporting as an organizational finding, independent of technical results. | "Is that ownership documented anywhere?" |
| I3 | What's your timeline and appetite for acting on findings? | Sets expectations for the remediation roadmap's framing (Deliverable 9) and the follow-up walkthrough's audience. | A described timeline/appetite. | "We just need the report for a checkbox" — not disqualifying, but should be handled honestly (see Handbook §12: never promise a specific outcome, and this answer doesn't change evidence standards). | "Who would be in the room for the remediation walkthrough?" |
| I4 | Is there anything about this engagement's scope or timing you're uncertain about? | Surfaces scope-creep or timeline risk before Day 1, when it's cheap to resolve. | A specific concern, or none. | Silence/deflection on a direct question about scope — worth a direct follow-up before signing off on Day 1 scheduling. | "Let's confirm the exact scope in writing before we start, then." |

---

## Using the Answers

Every "red flag" answer above is a **prioritization signal for testing
time**, not a conclusion. State clearly in the intake summary handed to
the assessor team which red flags were raised, and confirm in Stage 2
(Architecture Review) that they were followed up rather than only
noted. A red flag that never gets a corresponding checklist result in
the final report is a quality-gate failure per SOP Stage 12.
