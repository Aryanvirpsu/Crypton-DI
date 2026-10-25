# Crypton — Founder Operating Doc (v1.0) — SOURCE OF TRUTH

> **Canonical wedge (2026-09-10):** Crypton is investigating the security of autonomous actions. Our first wedge is action-integrity testing for autonomous security agents: determining whether privileged defensive agents can exceed, mutate, replay, inherit, or continue authority beyond the remediation humans or upstream systems actually intended. We will use research, adversarial testing, and real-world evidence to determine whether this becomes a defensible security company or remains a valuable research direction.
>
> Full reasoning: [`docs/strategy/2026-09-10-action-integrity-thesis.md`](docs/strategy/2026-09-10-action-integrity-thesis.md). Prior wedge language and exactly why it changed: [`docs/strategy/CHANGELOG.md`](docs/strategy/CHANGELOG.md).

**This file is the anchor for every session working in this repository.** Documentation map (one authoritative place per topic): [`docs/README.md`](docs/README.md); repository layout, frontends and branch workflow: [`docs/repository.md`](docs/repository.md). Before adding features, changing positioning, or drafting outreach/pitch material, check it against Section 6 (Competitive Reality), Section 8 (Wedge), and Section 13 (Kill Criteria). Update the Decision Log (Section 15) whenever an assumption is tested with real evidence. **Do not re-argue settled decisions — change them only with new evidence, and log the change.**

Last updated: 10 Sept 2026 · Owner: Aryan Vir · Stage: pre-customer-discovery, working prototype

---

## 0. TL;DR

- **What:** Identity and authorization layer for AI agents — verifiable agent identity + scoped, revocable, human-approvable permissions enforced *before* an action runs.
- **Why now:** Agents are moving from answering to acting. Machine identities outnumber humans ~82:1 and the security model for autonomous action is immature.
- **Hard truth:** The head-on "agent identity platform" market is not just crowded — it is being *absorbed* by Okta, Cisco, CrowdStrike, Palo Alto, SailPoint, Snowflake, Cyera through 2026 acquisitions. A standalone platform play is not venture-defensible for a solo student founder.
- **The play:** Enter through a **service-first wedge** (agent-authorization security assessments) to earn customer evidence, and build toward the **one gap the platforms openly admit is unsolved**: cryptographic proof of the multi-hop delegation chain with scope attenuation. Keep the venture-vs-open-protocol-vs-portfolio decision explicitly open, resolved by TestLab evidence.
- **Immediate goal (TestLab):** Leave able to state — *this specific customer, this repeated problem, how they solve it today, the smallest thing they'd pay for.*

---

## 1. Positioning Statement

> For **teams deploying AI agents that take real actions**, who **cannot reliably prove which agent did what, under whose authority, and whether it should have been allowed**, Crypton is an **agent identity and authorization layer** that **enforces scoped, revocable permission decisions before execution and leaves a verifiable trace** — unlike **treating agents as just another API client or logging actions only after they happen.**

**One-liner (external):** Accountability for autonomous software — verifiable identity and pre-execution authorization for AI agents.

**One-liner (investor/technical):** We make the agent delegation chain provable: who authorized which agent to do what, enforced before the tool call.

---

## 2. The Problem (grounded)

Organizations are handing agents access to real systems using a human's API key, a service account, an OAuth token, or a shared credential. Once the agent holds that credential, the org loses the ability to distinguish:

1. Which specific agent performed an action.
2. Which person/org authorized it.
3. Whether that authority was delegated for *this* resource/action.
4. Whether the permission had expired or been revoked.
5. Whether a sensitive action should have required human approval.

**The core insight:** Logging after the fact is not security. The decision that matters is whether the action should have been permitted *before* it executed.

**Supporting evidence (2026):**

| Signal | Figure | Source |
|---|---|---|
| Machine vs human identity ratio | ~82:1 (some surveys 109:1) | Palo Alto Networks; 2026 identity survey (2,900+ leaders) |
| Orgs with no policy for creating/removing AI identities | 78% | CSA / Oasis, Jan 2026 |
| Orgs with no clear ownership of AI identities | 51% | CSA / Oasis, Jan 2026 |
| Orgs taking >1 day to revoke an exposed credential | 24% | CSA / Oasis, Jan 2026 |
| Orgs that had a successful identity-related breach | ~90% | 2026 identity security survey |
| Predicted enterprise breaches traced to agent abuse by 2028 | 25% | Gartner (widely cited in vendor decks) |

> **Caveat to keep honest:** Several of the sharpest stats are vendor-commissioned (CSA survey commissioned by Oasis). Real, but treat as directional. TestLab exists to replace these borrowed numbers with your own primary evidence.

---

## 3. The Solution

Crypton gives each agent a **verifiable identity** tied to an owner/org, then lets permissions be **delegated with defined scopes, expiration, revocation, and human approval** for sensitive actions — with an **enforcement point that checks authorization before tool execution** (implemented in Agent Lab's agent loop; the gateway does not check agent delegation) and a **persistent, verifiable trace**.

**Prototype already implements:**
- Registered agent identities + ownership
- Individual agent instances
- Scoped delegation of authority
- Resource-level authorization
- Permission expiration + revocation
- Authorization checks before tool execution
- Persistent identity/authorization traces
- Human-approval boundaries for higher-risk actions
- Supporting identity infrastructure + automated tests

---

## 4. What's Built (Tech Snapshot)

| Layer | Choice | Status |
|---|---|---|
| Backend | Rust (Axum) | Built |
| Data | Postgres + Redis | Built |
| Frontend | React dashboard | Built |
| Enforcement (agent authorization) | In-process pre-execution check in Agent Lab (`authorization.py`, `chain.py`, `approval.py`) | Built (lab only; 453 tests pass) |
| Gateway | `crypton-gateway` (on `dev`; `main` still has the plain proxy): the stack's single external entry point. Verifies `crypton-id` EdDSA tokens (public key only), mirrors revocation fail-closed, exact-match public auth routes, principal-scoped single-use nonces, rate limits, risk-engine policy, a durable allow/deny decision trail in `audit_logs`, atomic-ish audit ingest, and forwards `x-gateway-secret` so identity can refuse requests that bypassed it. Identity produces the security signals (failed/successful login, account lock enforced at `login/start`, revocation, recovery, device-added) and writes step-up completion only after server-side WebAuthn verification. **Demonstrated at runtime** against the real `crypton-id` and gateway images (Linux containers, real Postgres/Redis, software WebAuthn authenticator): `tests/runtime/run.sh`, 88/88 checks, including network-isolation bypass attempts and Redis/Postgres outages; plus 32 gateway unit tests, 37 router/E2E tests, 41 identity unit tests (Linux). **Not demonstrated / open:** any production platform's private networking; `GATEWAY_SHARED_SECRET` is opt-in; no producers for `actions_executed_5m`, `registrations_from_ip_10m`, `active_session_count`; step-up is not bound to the named action; no key rotation, `iss`/`aud`, or agent-delegation checks; ingest commit window is non-atomic (documented, tested). See `docs/deployment-topology.md`. | Containerized topology and real-service enforcement demonstrated; **deployment readiness not claimed** (platform isolation unverified) |
| Identity base | Human passkeys via `webauthn-rs` in `crypton-id`, Ed25519 (EdDSA) JWTs. No automated end-to-end passkey-ceremony test (needs a real authenticator). Agent Lab uses `FakeWebAuthnVerifier`, a simulation. | Built (crypton-id); simulated in Agent Lab |
| Core features | register, device mgmt, recovery, audit logs, RBAC, admin panel | Built |
| Deployment target | Vercel + Railway | Planned |

**Architectural honesty:** Today's *agent-authorization* enforcement point is Agent Lab's in-process check, not the gateway (the gateway enforces human-session/risk controls only; it now verifies real identity tokens but has no agent-delegation checks). A proxy-PEP model, if built, would only see traffic that reaches it — endpoint discovery and enforcement coverage must be proven, not assumed. This is a known structural limitation of every gateway approach.

---

## 5. Market Size (with analyst variance flagged)

Nascent-market sizing varies wildly by analyst. Use the **AI Agent IAM** figure as the tightest ICP-relevant number; the broader ones are context.

| Market (segment) | 2025/26 value | Forecast | CAGR | Source |
|---|---|---|---|---|
| AI Agent IAM (tightest fit) | $300.9M (2025) | $9.2B (2035) | 40.8% | Astute Analytica |
| Non-Human Identity Mgmt | $4.5B (2026) | $29.4B (2035) | 23.2% | Globe Market Research |
| NHI Security | $8.22B (2026) | $22.94B (2031) | 22.78% | Mordor Intelligence |
| Agentic AI Security | $1.9B (2026) | $52.3B (2035) | 44.5% | Globe Market Research |
| AI Agent Security (broad) | $18.72B (2025) | $507B (2035) | 39.1% | SNS Insider (likely inflated) |
| AI Cybersecurity (Gartner) | ~$26B (2025) | $172B (2029) | 73.9% | Gartner 4Q25 forecast |

**TAM / SAM / SOM (your framing, defensible):**
- **TAM:** AI agent IAM + agentic security tooling — multi-billion by 2030, high-CAGR but analyst-noisy.
- **SAM:** Orgs deploying action-taking agents that need pre-execution authorization (not just discovery/inventory). Materially smaller today.
- **SOM (realistic first 18 mo):** A handful of design-partner teams paying for **assessments/services**, not platform seats. Think low tens of thousands of dollars, not ARR at scale. **Your SOM is measured in learning + design partners, not revenue.**

---

## 6. Competitive Reality — the moat question, not the platform map (revised 2026-09-10)

The wedge changed on 2026-09-10 (§15) from a head-on "agent identity platform" play to **action-integrity testing for autonomous security agents** — a narrower, adjacent problem. The old competitive map (Okta/Cisco/CrowdStrike/Palo Alto/SailPoint/Snowflake/Cyera absorbing the NHI/agent-identity category through 2026 acquisitions) is still true background — it's *why* we don't compete on agent identity/discovery — but it's no longer the battlefield that decides this wedge. Full old map, archived verbatim: `docs/strategy/CHANGELOG.md`.

**The competitors that matter now** are whoever could plausibly ship "test whether our own security agent's remediation stayed inside its authorized scope" without us: CrowdStrike, Palo Alto Networks, Microsoft, Wiz, and any AI-pentest shop that decides to specialize into this lane (see `docs/business/MARKET_SCAN.md` for the adjacent AI-pentest supply side).

**The hard moat test — run this at every stage:** What do we possess that one of those companies could not reproduce within six months by assigning a capable team? Weak answers: "we have experience," "we have ten customer reports," "we know agent authorization." Potentially strong answers: a novel, formalized attack class; a unique cross-framework failure dataset; a detection/test engine that catches something others don't model; a benchmark that becomes the standard way of evaluating the problem. **A findings corpus alone is not a moat** — it only becomes one if the patterns it surfaces generalize, materially improve future detection, and are expensive to independently reproduce. Full framing: `docs/strategy/2026-09-10-action-integrity-thesis.md` §13–14.

Crypton currently has no unfair advantage — only a plausible right-to-win path (existing Agent Lab implementation + hands-on delegation/authorization/approval work + offensive-security background + academic-research access + ability to publish before having customer logos). Whether that path produces a real moat is exactly what §13's kill criteria are designed to test.

---

## 7. Core Failure Classes — what we're actually testing for (revised 2026-09-10)

The old "Open Gap" (recursive delegation chains, scope attenuation, multi-principal attribution, behavioral drift, offboarding — full original text archived in `docs/strategy/CHANGELOG.md`) is understood as *converging on a standard* (IETF AAT/WIMSE), not a claimable moat — that finding stands and isn't relitigated here.

The current research targets a narrower, concrete question instead: **when a privileged autonomous security/remediation agent acts, does the exact executed action stay inside the authority and intent that was actually granted?** Ten concrete failure classes to test for (full definitions: `docs/strategy/2026-09-10-action-integrity-thesis.md` §6):

1. **Scope escalation** — authorized to act on X, executes on X+Y+Z.
2. **Argument drift** — authorized `disable user123`, executes `disable entire group`.
3. **Tool substitution** — replans through a different, less-constrained interface to the same effective action.
4. **Semantic replay** — one human approval gets re-executed via retry/replanning as if freshly authorized.
5. **Delegation escalation** — a child agent receives or discovers more privilege than the parent delegated.
6. **Revocation failure** — authority is revoked but an already-running descendant keeps acting.
7. **Approval/execution drift** — the approved action changes before execution without a fresh approval.
8. **Cross-agent confused deputy** — one agent causes a more-privileged agent to act outside the first agent's effective authority.
9. **Credential inheritance** — a sub-agent inherits credentials more powerful than its delegated task needs.
10. **Retry-induced overreach** — error recovery/retry logic changes scope, arguments, target, or execution count.

**What Crypton's existing code is for now:** `labs/agent-lab`'s primitives (`agents.py`, `identity.py`, `delegation.py`, `chain.py`, `authorization.py`, `approval.py`, `tracing.py`) are a controlled research lab for producing and testing these failure classes — not customer-facing infrastructure customers are expected to adopt. §3/§4 (what's actually built) haven't changed; only their role has.

---

## 8. Strategy — the Wedge (revised 2026-09-10)

**Permanent strategic filter:** prefer problems where the security need gets *stronger* as agents get more capable, autonomous, interconnected, and economically important — never build the company's survival on a problem likely to shrink as models/frameworks mature.

### 8a. Long-term category
**Security of Autonomous Actions.** The central question: when autonomous agents cause consequential real-world effects, can we determine what they're effectively capable of, whether the exact executed action stayed inside the authority/intent granted, and what actually happened if something goes wrong.

### 8b. Medium-term architecture (candidate end-states, not a build spec)
Four connected capabilities that may eventually matter — investigate in this order, gated on evidence, not built speculatively ahead of it:

| Capability | Question it answers |
|---|---|
| **Effective Authority** | What can an agent ultimately cause to happen? |
| **Action / Intent Integrity** | Did the exact executed action stay inside what was actually authorized? |
| **Evidence / Forensics** | Can we reconstruct and prove what happened? |
| **Containment** | Can a compromised/malfunctioning agent be stopped without the failure propagating? |

### 8c. Near-term wedge (the active mandate)
**Action Integrity Testing for Autonomous Security Agents.** Research/commercial question: can an autonomous security agent take a consequential remediation or enforcement action that exceeds, mutates, replays, inherits, or continues beyond the authority or intent that originally justified it? (Full framing: `docs/strategy/2026-09-10-action-integrity-thesis.md` §4–8.)

**Initial offer:** an **Autonomous Security Agent Action-Integrity Assessment** on one narrow security-agent workflow — not a platform, not "agent identity," no certification promise. Deliverables: authority-flow map, tested scenarios (the §7 failure classes), reproducible findings, expected-vs-observed behavior, affected action, consequence, evidence, remediation, retest. The legal gate is unchanged from the prior wedge: LLC + signed Authorization-to-Test per engagement before any paid work (`docs/business/ASSESSMENT_PACK.md` §C).

**Operating principle:** find real users → observe real actions → discover real failures → formalize the failure → publish/validate → test repeatability → earn access → earn payment → accumulate hard-to-replicate knowledge → build only what repeated evidence demands. Do not build the 8b architecture in advance of that evidence.

**What would change this wedge (it is not sacred):** autonomous security agents aren't arriving soon enough; teams keep consequential actions human-gated; another vendor solves action integrity sufficiently; buyers care far more about forensic reconstruction than integrity testing; effective-authority analysis produces much stronger demand; a different research finding exposes a more urgent problem. The long-term category (§8a) can stay valid while this wedge changes.

**Prior wedge (service-first agent-authorization & delegation assessment, GTM detail in `docs/business/`) is superseded, not deleted** — full text archived in `docs/strategy/CHANGELOG.md`. The `docs/business/` files (`MARKET_SCAN.md`, `OUTREACH_AND_ICP.md`, `ASSESSMENT_PACK.md`, `OPERATING_PLAN_UPDATES.md`) still hold real operational detail (legal-gate templates, pricing comparisons) but describe the *old* assessment's scope/positioning — treat their scope language as superseded by §8c/§9/§10 until they're revised.

---

## 9. Buyer Profile (was: ICP Hypotheses; revised 2026-09-10)

The first buyer is **not** "every company using AI agents." It's:

**Security engineering, security-platform, SOC-automation, or security-product teams experimenting with agents capable of real defensive/remediation actions.**

Priority targets are teams where the phrase *"our agent can automatically…"* ends with something consequential: isolate, revoke, rotate, block, patch, disable, terminate, reconfigure. These teams already understand blast radius, least privilege, approval, rollback, and incident response — which cuts the market-education cost that the old, broader ICP carried.

**Key discriminator question:** *Has this team given (or is it about to give) a security agent a permission that could itself become a privileged attack or failure surface?* Whoever answers yes with a real story is the buyer. Full buyer reasoning: `docs/strategy/2026-09-10-action-integrity-thesis.md` §7.

The prior 4-segment ICP list (AI-native startups / agent-infra companies / security-platform engineers / multi-agent product teams — archived in `docs/strategy/CHANGELOG.md`) is superseded by this narrower profile; it was too broad for the §7 failure classes to land on unprompted.

---

## 10. Business Model & Pricing Hypotheses (to validate, not commit; revised 2026-09-10)

Offering renamed to match the new wedge: **Autonomous Security Agent Action-Integrity Assessment** (was: "agent authorization & delegation assessment"). Deliverables and legal gate are in §8c. Pricing mechanics are unchanged from the prior wedge — the wedge narrowed, the commercial motion did not:

| Model | Hypothesis | Test |
|---|---|---|
| **Services (now)** | Fixed-fee assessment ($ low-thousands to low-tens-of-thousands) | Will 3+ teams pay for a scoped action-integrity assessment? |
| **Design-partner** | Discounted/paid pilot testing a real (staging) autonomous security-agent workflow | Will a team grant access to a non-production workflow? |
| **Product (later)** | Usage-/seat-based, or a reusable test harness/methodology license | Only after repeated engagements show a reusable pattern (§13) |
| **Research (parallel)** | No direct revenue — credibility, distribution, technical differentiation | Publish findings; track collaborator/design-partner pull generated |

**Rule (unchanged):** no pricing page, no platform build-out, until real buyer evidence (§13) says otherwise.

**Pricing (unchanged from 2026-09-02):** founding/design-partner rate **$2,500**, explicitly framed against a standard rate of $8,000–$10,000 — the 2026 AI/agent-pentest market floor is ~$9,500, so this is intentionally ~4x under-market to secure a real, low-friction first paid engagement, not a permanent price point. Full comparison: `docs/business/MARKET_SCAN.md` §4; SOW: `docs/business/ASSESSMENT_PACK.md` §A–B (the SOW's scope language still describes the old assessment — update it to §8c's scope before using it on a real engagement; not yet done as of 2026-09-10).

---

## 11. Validation Battery (revised 2026-09-10)

The RAT framework below is retired in favor of the dated milestones in §13 — those *are* the validation battery now; this section holds the metrics to track against them and the interview instrument. The full original RAT-1…RAT-7 scorecard (including the PASSED/HOLD/Trending-FAIL verdicts on the old delegation-primitive wedge) is archived in `docs/strategy/CHANGELOG.md` — do not re-run those RATs, they answered the old wedge's question, not this one. RAT-6A's result (working 3-hop delegation demo, Agent Lab v0.6) still stands as a fact about what's built (§3/§4/§7) — it just isn't this wedge's kill gate anymore.

### What to track (not commits, LOC, features, or dashboards)

**Market:** qualified teams found · real consequential autonomous actions identified · buyer conversations · design partners · willingness to grant access · willingness to pay.

**Technical:** meaningful failure classes discovered (§7) · reproducible attacks · frameworks tested · false positive/negative rate · generalization across systems.

**Research:** novel hypotheses · experimental results · expert reviews · responsible disclosures · publications · citations/adoption where applicable.

**Defensibility:** reusable test assertions · unique datasets · specialized instrumentation · patterns that repeat across engagements · external adoption of the methodology · improvements driven by accumulated data.

### Interview instrument (Mom Test — unchanged, aim it at the buyer in §9)
Ask about their life, not the idea. Never pitch.
1. Walk me through the last time an agent you deployed took an action against a real system.
2. How do you know which agent did what right now? Show me.
3. What happens today if an agent does something it shouldn't have?
4. Last time a credential/permission needed revoking — what did that take?
5. Who's responsible when an agent acts on someone's behalf?
6. What have you already tried or built to control this? Why isn't it enough?
7. How are you handling this for sub-agents / multi-agent workflows?
8. What does this problem cost you when it goes wrong — time, money, incidents?
9. Who else on the team feels this? Who owns it?
10. (Never ask "would you buy" — instead) → offer the assessment (§8c) and watch what they actually do.

---

## 12. Immediate Priorities (revised 2026-09-10, replaces the old TestLab week-by-week plan)

The Oct 14 – Nov 4 sprint window is unchanged — the first kill-criteria checkpoint (§13) still lands 2026-11-04 — but the goal for that window changed from "book interviews for a delegation assessment" to "find real teams building/deploying agents with consequential remediation authority." Work in this order:

1. **Market discovery** — find actual security teams using agents that take real actions.
2. **Literature / competitive research** — map current autonomous-security-agent products and research (feeds §6).
3. **Threat model** — define the §7 failure classes precisely against a real target workflow.
4. **Agent Lab experiments** — use `labs/agent-lab` as the controlled environment to produce real failures and controls.
5. **External authorized experimentation** — move into an open-source or collaborator system, with permission.
6. **Technical publication** — turn valid discoveries into public evidence.
7. **Design partner** — only after sufficient credibility from 1–6.
8. **Paid assessment** — only after demonstrated value from 1–7.

The old interview-quota-driven weekly plan (aimed at the prior wedge's buyer) is archived in `docs/strategy/CHANGELOG.md`.

---

## 13. Kill Criteria (revised 2026-09-10 — dated, not RAT-numbered)

- **By 4 Nov 2026:** identify multiple credible teams actively building/deploying agents with real security/remediation capability. Kill/downgrade: if almost none exist, the commercial wedge is probably too early — continue research if technically valuable, downgrade commercial expectations.
- **By 15 Dec 2026:** at least 5 serious technical conversations with teams working on autonomous or semi-autonomous security actions. Kill/investigate: if targeted outreach can't get these, check whether the buyer definition, wedge timing, problem language, or an access/credibility barrier is wrong.
- **By 31 Jan 2027:** demonstrate at least one genuinely non-trivial action-integrity failure — in the Agent Lab, an authorized open-source target, or a collaborating external environment. Kill signal: if every test just rediscovers obvious IAM misconfiguration, technical differentiation hasn't been shown.
- **By 31 Mar 2027:** at least one external team willing to let us test a staging/non-production workflow. Kill signal: if credible public work still gets no access, the trust/access problem may make the commercial wedge impractical.
- **By 30 Jun 2027:** after technical evidence + public research + a coherent methodology + ≥1 design partner, test willingness to pay. Kill signal: if nobody will purchase a scoped engagement, the commercial thesis is weak even if the research stays interesting.

> Write the verdict down in the Decision Log (§15) even if it's "no." A clean "no" now beats months building the wrong thing.

Prior RAT-numbered kill criteria (tied to the old delegation-assessment wedge) are archived in `docs/strategy/CHANGELOG.md`.

---

## 14. Risks & Honest Weaknesses
- **Absorption risk:** the whole category is being acquired; standalone renewals are shortening. Build assuming platforms bundle your feature.
- **Solo-founder / distribution:** buyers consolidate onto trusted platforms; a student solo founder has near-zero enterprise distribution.
- **Gateway coverage gap:** isolation and signal flow are demonstrated for the repo's container stack (`docs/deployment-topology.md`), but nothing here can verify a production platform's private networking, `GATEWAY_SHARED_SECRET` is opt-in, and the gateway only acts on traffic routed through it.
- **Standards race:** DIF/KYA-OS, A2A, NIST, and specifically the IETF `draft-niyikiza-oauth-attenuating-agent-tokens` (AAT) draft under the WIMSE umbrella are already on the delegation problem — v0.6's primitive is a working implementation of what AAT is still drafting, not a novel idea. Being technically right, or even first-to-implement, ≠ winning adoption or owning a moat (see §7, §11 RAT-7).
- **Vendor-stat dependence:** the problem stats you're leaning on are partly vendor-commissioned; replace with your own.
- **First-principles trap (recurring):** past reasoning kept landing in crowded spaces and overstating uniqueness. Counter it with customer evidence, not logic.

---

## 15. Decision Log (append-only — the anti-going-in-circles record)
| Date | Decision | Rationale | Evidence |
|---|---|---|---|
| 2026-08 | Pivot from human passwordless → agent identity/authorization | Human passwordless saturated | Market analysis |
| 2026-08 | Do NOT pursue standalone platform head-on | Category being absorbed by incumbents | §6 consolidation map |
| 2026-08 | Enter via service-first assessment wedge | Earns revenue + evidence, low risk | §8a |
| 2026-08 | Keep A/B/C end-state open, gate on TestLab | Avoid premature commitment | §8b |
| 2026-09-02 | Delegation primitive reclassified: credibility asset, not moat | Actively standardizing (IETF AAT draft / WIMSE) | Market scan §Supply-B; `docs/CRYPTON_TECHNICAL_SOT_v1.1.md` §12 |
| 2026-09-02 | Path B redefined: reference implementer/contributor to AAT/WIMSE, not a new rival protocol | Inventing a protocol AAT already drafts is redundant | Market scan; §8b |
| 2026-09-02 | Assessment repositioned to "agent authorization & delegation specialist" (not generic AI pentest) | AI-pentest market is crowded but under-serves this lane | Market scan §Supply-A; `docs/business/ASSESSMENT_PACK.md` |
| 2026-09-02 | Assessment scope split: v1 core (authority/delegation) + injection/jailbreak as a separate paid add-on | Injection testing is a different, already-crowded discipline; too broad for a first solo 3-day engagement | `docs/business/OPERATING_PLAN_UPDATES.md` Change 5 |
| 2026-09-02 | $2,500 reframed explicitly as founding/design-partner rate (standard $8K–$10K) | ~4x under the AI-pentest market floor (~$9,500); avoid signaling "not serious" while keeping the ask low-friction | Market scan §4; `docs/business/ASSESSMENT_PACK.md` §A |
| 2026-09-02 | Legal entity (LLC) + signed Authorization-to-Test now a hard gate before any paid assessment | Solo-founder liability exposure on grey-box testing of a customer's real environment | `docs/business/ASSESSMENT_PACK.md` §C |
| 2026-09-02 | Competitive-differentiation language corrected: never claim the delegation primitive is something competitors "don't have"; iC Consult and Article 11 are named real overlaps in the assessment lane | Prior internal draft language overclaimed uniqueness; differentiation must be earned via delivered engagements, not asserted | `DECISIONS.md` v0.6.1 strategy-correction section; `docs/business/MARKET_SCAN.md` §2 caveat |
| 2026-09-10 | Wedge changed: generic "agent authorization & delegation assessment" → "Action Integrity Testing for Autonomous Security Agents"; long-term category widened to "Security of Autonomous Actions" | Old wedge's core primitive (multi-hop delegation w/ attenuation) confirmed converging on the IETF AAT/WIMSE standard, not ownable (already flagged in the 2026-09-02 RAT-7 verdict); new wedge narrows the buyer to security/remediation-agent teams and ties defensibility to a stricter moat test (§6) instead of assessment volume | `docs/strategy/2026-09-10-action-integrity-thesis.md`; `docs/strategy/CHANGELOG.md` |
| 2026-09-15 (provisional, pending demo) | Within the §8c action-integrity wedge, committed to a specific technical bet: adversarially validating that a synthesized per-request invariant actually matches request intent (property → LLM-proposed counterexample → independent non-LLM oracle confirms → refine), not raw NL→policy synthesis-and-enforce on its own. First demo target is the `finding-001` binding-drift class (approval bound to proposed args, never re-verified against executed args), not stateful aggregate/postcondition checks — those are named out of v1 scope, not claimed solved. | An adversarial PC-style review found the synthesize+enforce framing already covered by prior art the team had in full (RefineAct, PAuth) and adjacent-but-distinct work (ShieldAgent, FORGE) — RefineAct and PAuth explicitly punt on validating that the synthesized spec matches intent, which is exactly the gap `finding-001` demonstrates matters in practice (an interpretation/binding failure a pure enforce-what-you-synthesized system structurally cannot catch). Marked provisional because no demo has run yet and no external party has seen it — treat as a working hypothesis until `002-binding-drift` runs, not a settled kill-criteria result. | This session's adversarial review transcript; `labs/experiments/001-langgraph-approval-drift/writeup.md` (`finding-001` class) |
| 2026-09-16 | Formalized three completed cross-framework research runs (EXP-003A OpenAI Agents SDK, EXP-003B Microsoft Agent Framework, EXP-003C LangGraph — same H1-H6 hypotheses, independently evidenced on each) into a durable, framework-independent methodology set (EXP-004, `docs/methodology/`): a seven-pillar Security Property Catalog, an eleven-objective Threat Model, a 13-stage assessment pipeline, a Framework Adaptation Guide, an eleven-link Evidence & Reporting Standard, an Execution Playbook, and a Customer Report Template. This is the shared vocabulary and procedure every future assessment/experiment/engagement should now follow — not a new technical bet, a synthesis of ones already evidenced. Explicitly not yet field-tested against an external customer or an unfamiliar target; explicitly surfaces its own coverage gaps (5 of 11 threat objectives untested, 3 new hypotheses H7-H9 defined but unrun) rather than implying completeness. | The existing 8-area `docs/ASSESSMENT_METHODOLOGY.md` and `docs/business/ASSESSMENT_PACK.md` already had real, code-enforced structure (`assessment/models.py`) but no framework-independent vocabulary connecting them to the EXP-003 series' cross-framework evidence; EXP-004 closes that gap using only evidence already collected, not new speculation. | `docs/methodology/` (all seven documents + README); `labs/exp-003-openai-agents-sdk/`, `labs/exp-003-microsoft-agent-framework/`, `labs/exp-003c-langgraph/` (the underlying evidence) |
| 2026-09-16 | EXP-005 field-tested EXP-004's methodology (`docs/methodology/`) end-to-end, stages 1-13, against a genuinely unfamiliar real application for the first time (OpenHands Software Agent SDK, commit `97dbce5c...`, public source only, static review, no execution) — not another framework research run like EXP-003A/B/C. Verdict: **VALIDATED WITH ADAPTATIONS**, not unqualified — three real methodology gaps surfaced (Baseline Verification assumes live execution and has no static-only substitute; the Threat Model's outcome vocabulary conflates "Crypton's evidence coverage" with "the target's mitigation strength"; "Framework-Specific Tests" assumes a clean framework/application boundary that doesn't hold). Produced two real, source-verified, non-trivial findings (risk-gated confirmation satisfiable by an LLM's own self-reported risk label when `LLMSecurityAnalyzer` is used alone; delegated sub-agents' confirmation pauses are silently auto-approved by default unless an integrator manually wires a callback the reference example never shows) plus one independent, third-party, dated corroboration that the "Persist privilege" threat class is a real engineering failure mode (a merged upstream PR fixing exactly that bug shape in this same codebase). A new hypothesis (H10, Signal Independence) and a Pillar-3 checklist addition are proposed for v0.2. | Directly answers the coverage-gap/field-testing backlog EXP-004 itself flagged as its top open item; the methodology needed to prove it works on something Crypton didn't build the test harness for, not just on well-documented frameworks it had already studied | `labs/exp-005-openhands-sdk/README.md` (full verdict, Phase 5-6 methodology-quality analysis, v0.2 recommendations), `labs/exp-005-openhands-sdk/findings.md` (eleven-link findings), `labs/exp-005-openhands-sdk/customer-report-simulation.md` (Phase 7 artifact) |
| 2026-09-16 | EXP-006 (a) independently re-reviewed every EXP-001–005 finding and methodology adaptation with the explicit goal of disproving them (`assessment-kit/v1.0/VALIDATION-REPORT.md`), and (b) operationalized everything that survived into **Crypton Assessment Kit v1.0** (`assessment-kit/v1.0/`) — the first internal operating manual a new assessor could use, with no prior exposure to Crypton, to run a complete customer engagement. Re-review outcome: EXP-001's original finding downgraded to SUPERSEDED and dropped from the reusable library (a synthetic, author-induced specimen — the project's own later work, finding-001b/c/d/e, already showed this, this pass made the supersession and exclusion formal); finding-001d+e merged into one library entry; EXP-005's FINDING-005-3 reclassified QUALITY ISSUE and excluded from the customer-facing library; every other finding (EXP-003A/B/C's H1-H6 results, EXP-003C's H1c, EXP-005's FINDING-005-1/2) survived re-review substantively unchanged, now under the mission's formal taxonomy (CONFIRMED / EXPECTED DESIGN / APPLICATION DESIGN CHOICE / etc.) instead of ad hoc labels. **No finding was found to be a false positive.** Of EXP-004/005's proposed methodology adaptations, only two (Pillars 5/6, the outcome-vocabulary split) met Crypton's own 2-target promotion bar; H10, the delegation-gate-delivery checklist item, and the Stage-9 rename remain explicitly EVOLVING, not promoted to core. Operational readiness verdict: **READY WITH LIMITATIONS** — a first paid engagement is a considered **GO**, conditioned on scoping it toward the validated corpus's shape (Python, one of the four studied frameworks) and disclosing kit maturity to the customer, not an unconditional green light. | The mission was explicit that research (EXP-001–005) was complete and no further experiments should be run — the actual remaining gap was operationalization, and operationalizing five experiments' worth of ad hoc findings without first re-testing whether they'd survive adversarial scrutiny would have risked baking an unvalidated conclusion into a document a future assessor would treat as authoritative without ever re-checking it themselves | `assessment-kit/v1.0/VALIDATION-REPORT.md` (the re-review); `assessment-kit/v1.0/README.md` (kit index, 15 documents); `assessment-kit/v1.0/11-pilot-readiness-report.md` (GO recommendation) |
| 2026-09-27 | Closed the LangChain `after_model`-composition case (`finding-001d`/`finding-001e`) as `NO VULNERABILITY — EXPECTED FRAMEWORK BEHAVIOR`: composition order is intentional/documented, the mutating middleware is trusted developer code in the same trust domain as the approval gate, permitted by the framework's own contract, and no framework-claimed boundary was crossed. Retired that finding's library entry (formerly `FL-01`) from `assessment-kit/v1.0/07-finding-library.md`, downgraded its `VALIDATION-REPORT.md` classification from CONFIRMED to SUPERSEDED, and added a new triage rule (§9a, `01-assessor-handbook.md`) requiring every candidate FAIL to identify the mutator, its trust level, whether it's permitted by design, the framework's actual claimed boundary, and whether that boundary was crossed, before it can be classified as a security failure. Separately confirmed EXP-007A (Principal Continuity, controller-substitution scenario) already satisfies its full authenticated-substitute acceptance criteria as previously frozen (2026-09-24) — 16/16 tests pass, no code changes needed — and wrote a minimal, unimplemented EXP-007B specification (revocation-between-approval-and-resume only; replay/nonce consumption still deferred) informed by the September 2026 OpenAI standing-credential/explicit-denial incident, mapped into existing categories (Authorization Boundary/Revocation, Approval/Denial Binding, Authority Provenance, Accountability) rather than as a new property. | The original `finding-001d`/`finding-001e` classification (CONFIRMED, re-validated as recently as EXP-006's 2026-09-16 independent re-review) had never asked who caused the observed mutation, what trust level they held, or what boundary the framework actually claims to enforce — only whether approved and executed arguments differed. That gap is exactly the failure mode Crypton exists to catch in *other people's* systems, so leaving it uncorrected in Crypton's own flagship finding would have been a live credibility and methodology risk, not just a paperwork discrepancy. | `labs/experiments/001-langgraph-approval-drift/CLOSURE.md` (full reasoning); `assessment-kit/v1.0/01-assessor-handbook.md` §9a (new triage rule); `assessment-kit/v1.0/VALIDATION-REPORT.md` §1/§5 (reclassification); `assessment-kit/v1.0/07-finding-library.md` (FL-01 retirement); `labs/exp-007-principal-continuity/FROZEN.md`, `EVIDENCE.md` (EXP-007A status, unchanged); `labs/exp-007-principal-continuity/EXP-007B-SPEC.md` (new, unimplemented) |
| 2026-09-28 | Built and shipped `pilot-pack/v0.1/` (12 documents) — the external-facing package answering "why should a company give Crypton one bounded workflow to assess," covering positioning, the $2,500/3-day founding offer, a 6-property test catalog, an evidence map, a sanitized sample assessment, access/safety rules, a trust argument, MVP definition, competitive positioning, a pre-engagement checklist, and a 5-persona red-team of the pack itself. Same-day, in response to red-teaming that pack, restructured and hardened it: (1) built **Crypton Validation Benchmark v0.1** (`labs/validation-benchmark/v0.1/`) — one real case (official MCP Python SDK, CVE-2026-52869) where a single, unmodified, independently-authored Crypton test script returns FAIL against the documented vulnerable version (1.27.1) and PASS against the documented fixed version (1.27.2), including a negative control; (2) split the test catalog into 4 scored Core families, 2 Conditional Modules, and 1 Research Preview (Principal Continuity), rather than presenting six uneven families as equally mature; (3) made Customer-Controlled Assessment (no credentials, no production access, no source repo — the customer runs agreed tests and hands over evidence) the *default* first engagement mode, with staging access as a fallback requested only where it adds real value, not the default; (4) made explicit, in the positioning doc itself, that the underlying security concepts (complete mediation, TOCTOU, confused deputy, session binding) are not novel — Crypton's claim is disciplined, repeatable application of them across the full agent authority path, the same relationship a pentest firm has to SQL injection. EXP-007B stays on hold; a second Validation Benchmark case (`mcp-atlassian`, CVE-2026-77243) was investigated and deliberately not forced through under time pressure — the fix spanned a multi-release refactor that couldn't be cleanly isolated in the available time, and one rigorous case was judged better than two rushed ones. | The pilot pack's own internal red-team surfaced that every prior piece of evidence for "Crypton's methodology works" was self-referential (designed the test, ran the test, it passed) — a real gap a customer engagement alone doesn't fix, since that's still in the future. The Validation Benchmark answers a narrower, honest, immediately answerable version of the same question (does Crypton's test tell a documented vulnerable state from a documented fixed one) without needing a customer or fabricating a track record. The catalog restructuring and default-mode change directly answer two of the red-team's five objections (catalog looked more mature than it is; the access ask was the single biggest source of first-pilot friction) without changing any claim that wasn't already true. | `pilot-pack/v0.1/` (all 12 files, especially `03-test-catalog.md`, `04-evidence-map.md`, `06-access-and-safety.md`, `07-why-trust-crypton.md`, `11-pilot-pack-red-team.md`); `labs/validation-benchmark/v0.1/` (README + case-01, including run output and JSON evidence) |
| 2026-09-28 (later) | Extended the Crypton Validation Benchmark to a **second, independent** case (VB-002: `@bytebase/dbhub`'s SQLite connector, CVE-2026-61788, Authorization Boundary & Complete Mediation — FAIL on `0.22.5`, PASS on `0.22.6`, same unmodified script) — deliberately a different vendor, language (TypeScript/Node vs. VB-001's Python), and test-catalog property than VB-001, not a second MCP-SDK case. Formalized a **pre-customer credibility gate**: two independent Validation Benchmark cases + one substantive external methodology review = the bar for calling status READY FOR FIRST DESIGN PARTNER instead of READY WITH CONDITIONS; recorded in `pilot-pack/v0.1/11-pilot-pack-red-team.md`. Wrote `pilot-pack/v0.1/EXTERNAL-REVIEW-BRIEF.md` — a condensed brief with three pointed, reviewer-specific attack questions (are these legitimate boundaries; does the evidence justify the verdicts; is "agent-specific" overclaimed) — ready to send to outside technical reviewers, not yet sent to anyone. **The benchmark half of the gate is now satisfied; the external-review half is not** — status remains READY WITH CONDITIONS until an actual outside review happens, which is a founder action this repository cannot perform on its own. | One benchmark case only answers "is this self-referential"; it doesn't answer "does it generalize." A second, deliberately independent case (different vendor, language, property) is the cheapest available evidence toward generalization without needing a customer, and formalizing the two-part gate (benchmark diversity + outside scrutiny) turns "we feel more confident" into a checkable bar that doesn't move just because more work got done. | `labs/validation-benchmark/v0.1/` (README + case-02, including run output and JSON evidence); `pilot-pack/v0.1/04-evidence-map.md`, `07-why-trust-crypton.md`, `11-pilot-pack-red-team.md` (gate definition), `EXTERNAL-REVIEW-BRIEF.md` (new) |
| 2026-10-12 | Corrected overstated claims found during BIG IDEA verification: `crypton-gateway` is a plain reverse proxy (CORS, request-ID propagation), not a Policy Enforcement Point, and has 0 tests; Agent Lab's pre-execution enforcement is in-process; Agent Lab approvals use `FakeWebAuthnVerifier` (simulation), while `crypton-id` uses `webauthn-rs` with no automated end-to-end ceremony test. Rust identity service's 16 previously failing tests were environmental (no Postgres): 38/38 pass against an isolated Postgres 16, no code defect. | CLAUDE.md §4 previously listed the gateway as a built PEP; the gateway's own source and README say otherwise. Claims must match what a test or a source file demonstrates. | `services/crypton-gateway/src/main.rs` header, `services/crypton-gateway/README.md`; `crypton-bigidea-2026/evidence/` |
| 2026-10-13 | Supersedes the gateway characterization in the 2026-10-12 entry **for branch `dev` only** (that entry remains accurate for `main` and for the code it examined). After merging `feat/risk-engine`, `feat/gateway-hardening` and `feat/policy-engine` into `dev`, the gateway is no longer a plain proxy: auth, nonce, rate-limit, device/step-up and risk-engine policy middleware are wired into the router, and request-ID propagation + JSON logging were restored. Router-level tests (18, throwaway Redis+Postgres) demonstrate those controls enforce at the request boundary for HS256 tokens; disabling the nonce, rate-limit, JWT and policy layers each makes the relevant tests fail. They also surfaced defects that keep it from being called a working PEP: token scheme mismatch with `crypton-id` (EdDSA, no `/verify`), public auth routes unreachable, audit ingest targets a nonexistent table, no audit trail of gateway decisions. Docs now state "wired and router-tested, not proven end to end". `AGENTS.md` is now the canonical agent/project instruction file; `CLAUDE.md` is a pointer to it (this reverses the earlier 'keep CLAUDE.md, AGENTS as pointer' choice at the founder's direction). | Wired middleware is not demonstrated enforcement; claims must follow what tests prove and what source shows. | `services/crypton-gateway/src/gateway_tests.rs`; `services/crypton-gateway/README.md`; dev merge commits `e91eca6`, `252f963` |
| 2026-10-13 (later) | Identity/gateway integration and P0/P1 repair on `dev`. Gateway now verifies `crypton-id`'s actual EdDSA tokens (public key only; algorithm/`kid`/claims pinned; HS256 and the nonexistent `/verify` fallback removed), mirrors identity's revocation (Redis denylist + credential-active) failing closed, and reads admin role from `users.role` (tokens carry none). Auth bootstrap routes are an exact method+path allowlist; everything else fails closed. Audit: ingest and the new decision trail write to the existing `audit_logs` (no `audit_events`), ingest is transactional with one atomic Redis effect, allow is recorded before forwarding and refused if unrecordable. Nonces are scoped per authenticated user (anonymous: ip+method+path); cross-user reuse is allowed by design but replaying a captured request stays in the victim's scope and is rejected; Redis outage fails closed. `@crypton/sdk` now sends `x-nonce`/`x-timestamp` (without them no real client could pass the gateway). 34 router/E2E tests pass; six deliberate mutations each fail the matching tests. Identity contract left unchanged on purpose: adding `aud` would break `labs/agent-lab`'s `pyjwt.decode`. The full `crypton-id` binary could not be built on this host (openssl-sys), so E2E uses its real token/auth source via `tests-support/identity-src`; a CI run against the real binary is still owed. Supersedes the 2026-10-13 row's list of gateway defects (all five fixed or reclassified in the README) and leaves the new gaps recorded in the gateway README. | Make the claim "a real identity token authorizes a request through the gateway and invalid ones fail closed" true and testable, before any structural cleanup. | `services/crypton-gateway/src/gateway_tests.rs`, `services/crypton-gateway/tests-support/identity-src`, `services/crypton-gateway/README.md` |
| 2026-10-13 (runtime) | Runtime security integration on `dev`. One documented external entry point (`infra/docker-compose.stack.yml`): only the gateway is published; identity, Postgres, Redis sit on an `internal: true` network; the gateway's own internal listener stays loopback-only. Added opt-in `GATEWAY_SHARED_SECRET` defence in depth (identity refuses requests lacking the gateway's `x-gateway-secret`). Identity (the authentication authority) now produces the signals the gateway consumes, writing them to the shared Redis, and enforces the account lock at `login/start`/`login/finish`; step-up completion is written by identity only after `finish_passkey_authentication` succeeds, bound to user + session `iat`, and the gateway downgrades only a Challenge to Allow for that token. Replaced the misleading policy-block metric with per-verdict counters; bounded Redis timeouts in identity and the gateway after the runtime run exposed a hang under Redis outage; fixed the gateway Dockerfile (it could not build from its own directory after the risk-engine path dependency) and bound the dev Postgres/Redis to 127.0.0.1. Verified against the REAL services built in Linux containers: 82/82 runtime checks (`tests/runtime/run.sh`). Not verified: any production platform's network configuration, which this repository cannot see. Supersedes the 'identity reachable directly / no signal producer' gaps recorded 2026-10-13 for the containerized stack only. | Closing the bypass and missing-signal gaps with evidence from the real services rather than harnesses. | `tests/runtime/e2e.mjs`, `docs/deployment-topology.md`, `infra/docker-compose.stack.yml`, `services/crypton-id/src/signals.rs` |
| 2026-10-13 (consolidation) | Repository consolidation on `dev`. Kept both frontends: `crypton-console` is the product app (SDK -> gateway -> identity), `crypton-main` the marketing site with uncommitted work in progress, so the three byte-identical boilerplate files are cheaper to carry than a shared package; documented in `docs/repository.md`. Removed only proven-dead files (unreferenced `crypton-console` `App.css`/`logo.svg`, an empty `infra/infra.txt`, redundant `.gitkeep`s), untracked generated/tool output (`risk-engine/risk_playground.log`, a stray `.claude/worktrees` gitlink), and added `.gitattributes` (LF for scripts and container inputs). `graphify-out/` was already untracked on `dev`; it remains tracked only on `main` until `dev` is merged. Added a documentation map and rewrote the stale `docs/testing.md`. Production-intended services now refuse to start when `REQUIRE_GATEWAY_SECRET=true` and `GATEWAY_SHARED_SECRET` is missing or under 16 characters (identity and gateway; the reference stack sets it), verified by 6 added runtime checks (88/88 total). Created branch `lab`; every other local and remote branch is an ancestor of `dev` except two Dependabot lockfile branches (kept, recommended for regeneration). No remote branch deleted, nothing pushed, `main` untouched. Not changed: `crypton-main` (uncommitted work there), `policy-engine` (unwired but unique), `labs/` and `services/` paths. | Less duplication and a clearer structure without breaking paths cited by evidence, experiments or deployment config. | `docs/README.md`, `docs/repository.md`, `tests/runtime/e2e.mjs` section 10 |
| 2026-10-13 (release candidate) | Release-candidate verification of `dev` before any promotion to `main`. Both frontends now build with `CI=true`: the failure was exactly eleven lint warnings in `crypton-main/Landing.js` (4 never-referenced components, 2 hook-dependency, 1 anchor-without-href, 3 literal `//` text nodes (one inside a removed component), 1 duplicate style key); the unused components were removed and the rest fixed without changing rendered output, and the CI `CI: false` carve-out is gone. CI now also runs the gateway integration tests, the real-stack runtime suite and `npm ci`; it is validated locally only and has not run on hosted runners. `cargo fmt --check` (already required by CI) failed for the gateway and identity and was fixed. Dependabot: both open branches are obsolete lockfile bumps; the only live advisory-relevant drift (`follow-redirects`, `brace-expansion`) was fixed in `services/crypton-main/package-lock.json` and the root lock already had them. `npm audit` still reports 96 findings, all through four direct dev/build dependencies (`react-scripts` 5, `jest` and its types) with no non-breaking fix; none are in the shipped browser bundle. Not claimed: hosted-CI success, production isolation, Rust dependency auditing (`cargo audit` not run). | Make promotion a decision backed by one reproducible gate. | `.github/workflows/ci.yml`, `docs/testing.md`, `services/crypton-main/src/Landing.js` |
| 2026-10-13 (rust deps) | Rust dependency remediation on `dev`, separate from the baseline so each step is reversible. Lockfile-only semver-compatible updates (`rustls` 0.23.45, `rustls-webpki` 0.103.15, `aws-lc-sys` 0.46.0, `h2` 0.4.20, `quinn-proto` 0.11.19, others) and one manifest change, `crypton-id` `sqlx` 0.7 -> 0.8 (the only way to drop `rustls-webpki` 0.101.7, which has no patch). Vulnerabilities per lockfile 29 -> 4 (17 distinct RustSec IDs -> 2); the per-lockfile findings compiled into a shipped binary 17 -> 0. Remaining: `rsa` RUSTSEC-2023-0071 (no fix; lockfile-only, not in any binary's graph) and `maxminddb` RUSTSEC-2025-0132 (optional `geoip` feature off; code uses `open_readfile`, not the affected `open_mmap`), each listed with a justification and reopen trigger in `scripts/rust-audit-ignore.txt`. CI gained a blocking `rust-audit` job and a weekly run; it fails on any unlisted advisory, so new findings cannot be added silently. Verified after each group: identity 41 (Linux), gateway 32+37, risk-engine 309, policy-engine 35, runtime 88/88. The 96 npm findings are unchanged (CRA/Jest tooling; migration is a separate workstream). Details: `docs/dependency-audit.md`. | Measurable risk reduction, no behaviour change. | `docs/dependency-audit.md`, `scripts/rust-audit.sh` |
| _next_ | _(send `pilot-pack/v0.1/EXTERNAL-REVIEW-BRIEF.md` to outside technical reviewers and record what changed as a result — this is the one remaining action inside the pre-customer credibility gate above that a coding session cannot perform on its own — or fill after the first real customer engagement using `assessment-kit/v1.0/`, or after the 2026-11-04 kill-criteria checkpoint §13, whichever lands first)_ | | |

---

## 16. Appendix — Reusable Assets

### 16a. Cold outreach (interview ask)
> Subject: 15 min — how your team handles AI agent permissions
> Hi [name] — I'm researching how teams deploying AI agents control what those agents are allowed to do (not selling anything). You're clearly hands-on with [their agent work]. Could I get 15 minutes to hear how you handle it today? Happy to share what I learn across other teams in return.

### 16b. Assessment offer (after a good interview)
> Based on what you described, I can do a focused review of your agent workflow — identity, permissions, unsafe tool execution, and missing authorization boundaries — and give you a written findings + fixes report. [Fixed fee / design-partner terms]. Worth a scoping call?

### 16c. Competitive one-liners (when asked "isn't this just X?")
- **vs Okta/Auth0/Entra:** they answer *who the agent is*; we answer *what it's allowed to do right now and whether this action should run* — and prove the delegation chain.
- **vs Oasis/Astrix (NHI platforms):** they discover and inventory identities; we enforce scoped authorization at the point of action, with verifiable multi-hop delegation.
- **vs MCP gateways (Runlayer etc.):** a gateway sees routed traffic; our differentiator is the cryptographic delegation passport that travels with the call and attenuates scope per hop.

### 16d. Brand
- Logo: segmented ring forming a "C" — eight arc segments, white outline.
- Tagline: *"Identity Redefined."*
- Asset: Gemini Veo prompt kit — 60-sec dark cinematic brand film (seven scenes, no voiceover, music-driven).

---

*End of v1.0. Update the Decision Log and Scorecard as evidence arrives. Don't reopen settled decisions without new evidence.*
