<!-- Canonicalized into the repo 2026-09-02, from Aryan's working copy (Crypton-Outreach-and-ICP.md). Content unchanged. -->

# Crypton — Outreach & Customer Discovery Pack

**Everything to fill the interview pipeline and run discovery.** Goal: 50+ qualified targets → 10 interviews booked before Oct 14.
**Date:** 2 Sep 2026 · Primary KPI: qualified conversations (not impressions).

---

## 1. ICP — who to target (ranked)

| Rank | Segment | Why | Watch-out |
|---|---|---|---|
| 1 | **AI-native startups shipping action-taking agents** | Feel the pain first, low process, reachable, fast | Small budgets; grant access easily but may not pay |
| 2 | **Agent-infra / dev-tooling companies** | Building the plumbing; understand delegation deeply | May want a component/protocol, not an assessment |
| 3 | **Security/platform engineers deploying internal agents** | Own the risk, have budget | May prefer to wait for their existing platform to ship it |
| 4 | **Multi-agent / MCP product teams** | Multi-hop is their real problem | Fewer of them today |

**Discriminator question (find your ICP with this):** *At what point does an agent become risky enough that your existing IAM / API security stops being sufficient?* Whoever answers with a real story is your ICP.

---

## 2. Target-List Template (fill to 50+)

| Company | Person | Role | Agent use-case | Tool access | Why relevant | Warm/Cold | Source | Outreach date | Response | Interview? | Pain signal (0–3) | Assessment signal (0–3) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|

**Rules:** every target has a *reason* (no mass blasting). Prefer warm intros. Quality over volume, but don't stop at 25 — booking 10 from 25 assumes an unrealistic ~40% rate; cold security outreach converts low single digits, so aim 50+.

---

## 3. Where to source names

- **Warm first:** Penn State network, LaunchBox cohort, professors, prior internship/research contacts, anyone building agents you already know.
- YC/accelerator batch pages (filter for AI agents).
- GitHub: contributors/maintainers of agent frameworks, MCP servers, agent-auth repos.
- LinkedIn: "AI engineer / platform engineer / security engineer" at companies shipping agents.
- Communities: agent/LLM Discords, MCP community, relevant subreddits, security Slacks.
- Conference/meetup speaker lists on agent security.

---

## 4. Cold Outreach Sequence (interview ask — NOT selling)

**Email 1 — the ask**
> Subject: 15 min — how your team handles AI agent permissions
>
> Hi [name] — I'm researching how teams deploying AI agents control what those agents are allowed to *do* (I'm not selling anything). You're clearly hands-on with [specific thing they build]. Could I borrow 15 minutes to hear how you handle it today? Happy to share what I'm learning across other teams in return.
> — Aryan

**Follow-up 1 (+3 days)**
> Quick nudge in case this slipped by — 15 minutes on how you handle agent permissions/authority today would genuinely help my research. Any time [day] or [day] work?

**Follow-up 2 (+4 days, then stop)**
> Last one from me — if now's not the time, no worries. If it's ever useful to compare notes on agent authorization, I'm around.

**LinkedIn variant (shorter)**
> Hi [name] — researching how teams control what their AI agents are allowed to do (not a pitch). Would love 15 min to hear how you handle it. Open to it?

**Rules:** personalize line 1 every time. Never pitch in the ask. One thread, three touches, then stop.

---

## 5. Interview Script (Mom Test — ask about their life, not your idea)

1. Walk me through the last time an agent you deployed took an action against a real system.
2. How do you know which agent did what right now? Show me.
3. What happens today if an agent does something it shouldn't have?
4. Last time a credential/permission needed revoking — what did that take?
5. Who's responsible when an agent acts on someone's behalf?
6. What have you already tried or built to control this? Why isn't it enough?
7. How are you handling this for sub-agents / multi-agent workflows?
8. What does it cost you when this goes wrong — time, money, incidents?
9. Who else feels this? Who owns it internally?
10. **Never ask "would you buy."** Instead, after a relevant interview, offer the assessment (§6) and watch what they actually do.

**Discipline:** don't pitch during the Mom Test portion. Don't redesign Crypton after one conversation — look for *repetition* across interviews.

---

## 6. The Offer Test (after a good interview — Week 2+)

> Based on what you described, we can run a scoped agent-authorization & delegation review of that workflow and return findings + remediation. Would it be useful to scope that?

Then watch behavior. Commitment = money, meaningful engineering access, or real time. Compliments don't count.

---

## 7. Score every interview against the RATs

Quick tally after each conversation:

| Interview | RAT-1 acute? (recent+consequential+unsolved) | RAT-2 will commit? | RAT-3 tools insufficient? | RAT-4 named owner? | RAT-5 not bundled away? | Notes / exact words |
|---|---|---|---|---|---|---|

**Targets by Nov 4:** RAT-1 ≥8/15 · RAT-2 ≥3 paid/pilot · RAT-3 ≥50% real gap · RAT-4 ≥3 orgs with an owner · RAT-5 ≥1 clear segment.

**Capture verbatim language** — the phrases buyers actually use become your messaging (replaces borrowed security jargon).

---

## 8. Weekly cadence (Sep → Nov 4)

- **Sep 2–7:** finalize this pack + assessment pack; entity/authorization-to-test underway.
- **Sep 8–20:** build the list to 50+; start warm outreach.
- **Sep 21–Oct 13:** book 10, complete a few; document objections; refine script from real language.
- **Oct 14–Nov 4 (TestLab):** wk1 problem discovery → wk2 offer test → wk3 run first assessment → wk4 fill RAT scorecard, decide A/B/C, write Decision Log entry.

**Primary metric until Nov 4:** qualified interviews booked/completed, repeated pain stories, named owners, assessments offered/accepted/paid. **Not** commits, tests, or LOC.

---

*No lead disappears because you forgot to reply. Track every conversation.*
