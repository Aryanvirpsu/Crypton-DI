# Crypton Strategy Changelog

Append-only record of major strategy pivots in [`CLAUDE.md`](../../CLAUDE.md).
Each entry says what changed and why. Where a pivot replaces prior wedge
language wholesale rather than amending it, the superseded text is
archived below in full, so the reasoning trail survives without living
on as active instruction. Do not delete entries or archived sections —
append new ones instead.

---

## 2026-09-10 — Wedge pivot: agent-authorization assessment → action-integrity testing for autonomous security agents

**What changed:** `CLAUDE.md` §6 (Competitive Reality), §7 (Open Gap →
Core Failure Classes), §8 (Strategy/Wedge), §9 (ICP → Buyer Profile),
§10 (Business Model), §11 (Validation Battery), §12 (near-term plan),
and §13 (Kill Criteria) were rewritten around a new, narrower first
wedge: **Action Integrity Testing for Autonomous Security Agents** —
testing whether privileged defensive/remediation agents can exceed,
mutate, replay, inherit, or continue authority beyond what was actually
authorized. The long-term category widened to **Security of Autonomous
Actions**. Full reasoning: [`2026-09-10-action-integrity-thesis.md`](2026-09-10-action-integrity-thesis.md).

**Why:** The prior wedge (a generic "agent authorization & delegation"
assessment aimed at any team deploying action-taking agents) competed
directly in a market being absorbed by identity platforms (Okta, Cisco,
CrowdStrike, Palo Alto, SailPoint, Snowflake, Cyera), and its core
technical primitive (multi-hop delegation with scope attenuation) was
independently confirmed to be converging on an IETF standard (AAT/
WIMSE) rather than being ownable — see the old §7/§11 RAT-7 verdict
archived below, which already said this in different words. The new
wedge narrows the buyer to teams giving *security/remediation* agents
real consequential permissions: a smaller, sharper population needing
less market education, evaluated against a stricter moat standard
(novel failure classes and hard-to-reproduce datasets, not just
findings volume — see thesis §13–14).

**Evidence:** None yet specific to this new wedge — this is a
directional pivot based on reasoning, the same way the 2026-08 pivot
into agent identity was. The dated kill criteria in the new `CLAUDE.md`
§13 exist specifically to falsify or confirm it with real evidence
between 2026-11-04 and 2027-06-30.

**What did not change:** the existing `labs/agent-lab` code (§3/§4 of
`CLAUDE.md`) and its test suite — the RAT-6A multi-hop delegation result
is still a true fact about what's built; it just stopped being this
wedge's kill gate and became one of the research lab's reference
primitives instead (see thesis §9). Assessment legal gate (LLC +
Authorization-to-Test) and the $2,500 founding-rate framing also carry
forward unchanged.

**Superseded content:** archived verbatim below.

---

## Superseded 2026-09-10 — prior `CLAUDE.md` §6–13 (service-first agent-authorization-assessment wedge)

Kept verbatim for the reasoning trail. This is **no longer active
instruction** — see current `CLAUDE.md` §6–13 for what governs now.

### 6. Competitive Reality — the market is being absorbed

This is the most important section. As of mid-2026 the NHI/agent-identity category consolidated hard:

**Acquired / signed:**
| Startup | Acquirer |
|---|---|
| Astrix | Cisco |
| Entro | SailPoint |
| SGNL | CrowdStrike |
| Natoma | Snowflake |
| Fabrix | Silverfort |
| Permiso | Okta |
| Oasis (largest, ~$1B LOI) | Cyera |
| CyberArk (~$25B) | Palo Alto Networks — relaunched as identity platform w/ agent security as headline |

**Still independent (as of Aug 2026):** Aembit, Clutch, Token Security, P0.

**IdP / auth incumbents shipping agent auth:** Auth0 ("Auth for MCP", GA May 2026), Okta (MCP server + Permiso), Microsoft Entra Agent ID.

**MCP-gateway / runtime players:** Runlayer ($11M seed, 8 unicorn customers in 4 months), Proofpoint, Stacklok, Operant, Strata, LangProtect, Aembit.

**Standards moving:** MCP-I donated to DIF (Mar 2026), renamed KYA-OS; A2A protocol (credential-binding, `TASK_STATE_AUTH_REQUIRED`); NIST AI Agent Standards Initiative (Feb 2026); OWASP AI Agent guidance.

**Funding context:** ~$392M in agentic-AI-security funding in the two weeks around RSAC 2026 alone. Median agentic seed ~$9.2M. Elevated M&A likelihood across the category.

#### What this means for Crypton (blunt)
1. **"Agent identity platform" is a feature of platforms customers already own.** You cannot win it head-on. Don't try.
2. **Authentication ("who is this agent") is done** — Okta/Auth0/Entra own it.
3. **Discovery/inventory is a catalog war** — Runlayer/Proofpoint (18,000+ MCP catalogs). Not a solo game.
4. **The absorbing platforms have a stated blind spot** (see old §7). That gap + a service wedge is your only defensible entry.

### 7. The Open Gap (where a defensible angle actually exists)

Every shipped framework verifies *who the agent is* and stops. As of the 2026 research/RSAC snapshot below, delegation chains, scope attenuation, multi-principal attribution, behavioral drift, and offboarding were still largely unsolved **in deployed products** — but this gap is **actively standardizing**, not open turf to claim as a moat (see the 2026-09-02 market scan, `docs/business/MARKET_SCAN.md`, and `docs/CRYPTON_TECHNICAL_SOT_v1.1.md` §12):

- **Recursive delegation chains:** most deployed products still can't cryptographically prove which human authorized which agent to do which action at the 3rd/4th hop.
- **Scope attenuation:** each delegation hop must *narrow*, never widen, permissions — largely custom glue in real deployments.
- **Multi-principal attribution:** which human bears responsibility for a delegated action — still mostly unsolved in production.
- **Behavioral blind spot (RSAC 2026):** frameworks verify identity but don't track *what the agent did* — can't detect an agent rewriting its own policy or delegating without human approval.
- **Offboarding:** confirming a decommissioned agent holds zero credentials — no vendor automates this.

**What Crypton actually built:** WebAuthn / device-based crypto identity, plus (v0.6) an Ed25519-signed, hash-chained, scope-attenuated multi-hop delegation primitive (RAT-6A, proven inside Crypton's own trust/data environment — see the Technical SOT). No zero-knowledge cryptography was built; an earlier draft of this section described the foundation as "WebAuthn / ZK / device-attestation" — that was aspirational language, not a built primitive, and is corrected here (2026-09-02).

**Why this is a credibility asset, not a moat:** the exact mechanism v0.6 implements — offline attenuation, independent chain verification, signed capability tokens — is what the IETF is actively drafting as Attenuating Authorization Tokens (`draft-niyikiza-oauth-attenuating-agent-tokens`), under the broader WIMSE umbrella. A standalone "Crypton delegation passport" is not defensible technology; a working reference implementation of a converging standard is a credibility asset that can seed Path B (old §8b) or back the assessment's technical authority (old §8a), but it is not, by itself, an ownable moat. RAT-7 (old §11) carried a split, per-layer verdict reflecting this.

### 8. Strategy — the Wedge (locked direction, evidence-gated)

**Do not** pitch or build "the next Okta." Enter narrow, earn evidence, then commit to one of three end-states.

#### 8a. Beachhead: service-first
Offer a paid **agent-authorization security assessment**: review a team's agent workflow for identity, permissions, unsafe tool execution, and missing authorization boundaries. Deliver findings + fixes.
- **Why:** Generates revenue + direct customer evidence with near-zero product risk. Turns your prototype into a diagnostic tool, not a platform bet.
- **Output of assessments = your primary research.** Every engagement answers old §11's riskiest assumptions.

**Reposition (2026-09-02):** brand as the **agent authorization & delegation specialist**, not a generic "AI pentest" — that category is already populated (NetSPI, HackerOne, Synack, Software Secured, AppSecure, and others cover prompt injection/jailbreak/RAG poisoning well; see `docs/business/MARKET_SCAN.md` §2). v1 core scope is identity & ownership, delegation & authority flow, the authorization boundary, human-approval binding, and evidence/accountability reconstruction; prompt/tool injection and jailbreak testing are a separate paid add-on, not the core offer (full SOW: `docs/business/ASSESSMENT_PACK.md` §B, §E).

**Blocking legal gate — do not run a paid assessment without this in place:** (1) a single-member LLC for liability separation, (2) a signed Authorization-to-Test agreement per engagement (named target, testing window, methods, data handling, safe harbor, liability cap — template in `docs/business/ASSESSMENT_PACK.md` §C), (3) confirmation the customer owns or can authorize testing of the target environment. This is not optional process — it is what keeps a $2,500 pilot from becoming a real liability problem.

**Full GTM detail** (ICP ranking, target-list template, cold-outreach sequence, intake questionnaire, report template, weekly cadence) lives in `docs/business/` — `MARKET_SCAN.md`, `OUTREACH_AND_ICP.md`, `ASSESSMENT_PACK.md`, `OPERATING_PLAN_UPDATES.md` — canonicalized 2026-09-02. This file stayed the anchor/summary; those were the operational detail.

#### 8b. Three candidate end-states (choose via evidence, not preference)
| Path | What it is | Best if TestLab shows… | Downside |
|---|---|---|---|
| **A. Narrow product** | One control (e.g. pre-execution approval, or delegation-proof) as a focused tool | A specific segment feels acute, repeated pain and will pay | Absorption/acquisition pressure; platform bundling |
| **B. Reference implementer / standards contributor** | Be the recognized reference implementation of, and contributor to, the emerging agent-delegation standard (IETF AAT / WIMSE) — not inventing a rival protocol | Pain is real but buyers consolidate onto platforms, and delegation genuinely standardizes around attenuated capability tokens | Credibility/influence, not revenue by itself; incumbents (Okta, Microsoft, SPIFFE) are in the same standards rooms |
| **C. Portfolio flagship** | Keep as the flagship cryptographic-systems project on your record | No paying buyer emerges | Not a company (but still high-value for you) |

**Path B correction (2026-09-02):** the original framing here — "open protocol contributed toward DIF/A2A" — assumed no such protocol existed yet. It does: `draft-niyikiza-oauth-attenuating-agent-tokens` (AAT, IETF, v01 Jun 2026) describes, in substance, what v0.6 already implements. Inventing a rival protocol would be redundant work with no defensibility payoff. Path B is now concretely: bring v0.6's working-implementation experience to the OAuth WG / WIMSE, maintain a reference implementation and test vectors others verify against, and consider language SDKs or an independent verifier. This was explicitly **a post-Nov-4 unlock, gated on TestLab evidence — not a current activity.**

**Default assumption to disprove:** absent strong evidence, this trends toward **B or C**, not a fundable standalone. TestLab's job was to find whether **A** is real.

### 9. ICP Hypotheses (candidates to test — rank via interviews)

Do **not** assume. Test each. For each: does the pain exist, who owns it, do existing IAM/API controls already cover it?

1. **AI-native startups shipping action-taking agents** — feel the pain first, low process, reachable, fast. Watch-out: small budgets; may grant access easily but not pay.
2. **Agent-infra / dev-tooling companies** — building the plumbing, understand delegation deeply. Watch-out: may want a component/protocol, not an assessment.
3. **Security / platform engineers deploying internal agents** — own the risk, have budget. Watch-out: may prefer to wait for their existing platform to ship it.
4. **Multi-agent / MCP product teams** — multi-hop is their real problem. Watch-out: fewer of them today.

**Revised 2026-09-02** to match `docs/business/OUTREACH_AND_ICP.md`'s ranked list (replacing this section's earlier 5-segment ordering, which included "enterprises" and "regulated verticals" as near-term targets — deprioritized as slower-sales / platform-bundle-prone for a solo-founder first pass). Full outreach mechanics — target-list template, cold-email sequence, weekly cadence — lived in that file, not here.

**Key discriminator question:** *At what point does an agent become risky enough that existing IAM + API security stop being sufficient?* Whoever answers that with a real story is your ICP.

### 10. Business Model & Pricing Hypotheses (to validate, not commit)

| Model | Hypothesis | Test |
|---|---|---|
| **Services (now)** | Fixed-fee assessment ($ low-thousands to low-tens-of-thousands) | Will 3 teams pay for a paid assessment? |
| **Design-partner** | Discounted/paid pilot of the enforcement layer | Will a team run it in a real workflow? |
| **Product (later)** | Usage- or seat-based on authorization decisions/enforced calls | Only after A is proven |
| **Protocol (alt)** | Open core + paid reference impl/support | Only if path B |

**Rule:** No pricing page, no platform build-out, until old §11 RAT-1 and RAT-2 pass.

**Pricing update (2026-09-02):** the concrete founding-assessment rate is **$2,500**, explicitly framed as *design-partner pricing (standard rate $8,000–$10,000)* — not stated as a bare number. The 2026 AI/agent-pentest market floor is ~$9,500 (general pentests run $10K–$30K), so $2,500 is intentionally ~4x under-market to secure a real, low-friction paid RAT-2 commitment, not a permanent price point. Full pricing comparison table: `docs/business/MARKET_SCAN.md` §4; SOW: `docs/business/ASSESSMENT_PACK.md` §A–B.

### 11. Validation Battery — "all tests possible"

Structured as **Riskiest Assumption Tests (RATs)** across Desirability / Viability / Feasibility. Each has an explicit **pass threshold** and **kill signal**. Run them in order; a failed RAT stops downstream work.

#### Desirability (do they want it?)
- **RAT-1 — Problem exists & is acute.**
  Test: 15+ Mom-Test interviews with agent-deploying teams.
  Pass: ≥8 describe a *specific past incident or active workaround* around agent permissions unprompted.
  Kill: pain is hypothetical/"nice to have" across the board.
- **RAT-2 — Willingness to act/pay.**
  Test: offer paid assessment to interviewees.
  Pass: ≥3 agree to a paid assessment OR a design-partner pilot.
  Kill: interest but zero commitment of money/time/access.
- **RAT-3 — Existing tools insufficient.**
  Test: ask exactly how they handle agent creds/permissions today.
  Pass: ≥half describe a real gap their current IAM/API/IdP does not cover.
  Kill: "our Okta/CyberArk/platform already does this."

#### Viability (is there a business?)
- **RAT-4 — Owner exists.**
  Pass: a nameable role/budget owns "agent authorization" in ≥3 orgs.
  Kill: nobody owns it / it's diffuse.
- **RAT-5 — Not immediately bundled away.**
  Pass: a segment where the platform incumbent demonstrably *doesn't* solve the specific control.
  Kill: every buyer says "we'll wait for [Okta/Palo Alto/Cisco] to ship it."

#### Feasibility (can you build/defend it?)
- **RAT-6A — Technical wedge is real (in-environment).**
  Test: prototype the delegation-passport / scope-attenuation proof on the existing base.
  Pass: working demo verifying a ≥3-hop chain with narrowing scope, inside Crypton's own trust/data environment.
  Kill: reduces to what OAuth token-exchange already does.
  **Status (2026-09-02): PASSED.** Built in Agent Lab v0.6 — Ed25519-signed, hash-chained, scope-attenuated 3-hop delegation with a full attack-case matrix (unauthorized tool, resource escape, revoked ancestor, tampered delegation → all DENY). See `docs/CRYPTON_TECHNICAL_SOT_v1.1.md` §11.
- **RAT-6B — Technical wedge is portable (no prior coordination).**
  Test: the same proof, independently verifiable by a third party with no prior integration and no access to Crypton's own database.
  Pass: a genuinely portable, standards-aligned verification path.
  Kill: not attempted, or reduces to reimplementing a standard that's already being drafted.
  **Status (2026-09-02): HOLD, not built.** This is close to what the IETF AAT draft (`draft-niyikiza-oauth-attenuating-agent-tokens`) already specifies — building a proprietary version would be redundant. Only build if customer evidence or a Path B decision requires it, and if so, align to AAT rather than inventing a rival encoding. See `docs/CRYPTON_TECHNICAL_SOT_v1.1.md` §12.
- **RAT-7 — Defensibility (per-layer verdict, revised 2026-09-02).**

  | Layer | Verdict | Basis |
  |---|---|---|
  | Delegation primitive (token/attenuation tech) | **Trending FAIL as a moat** | Actively standardizing (IETF AAT draft, DeepMind Delegation Capability Tokens, academic Invocation-Bound Capability Tokens, CSA/Okta best practice) — a feature arriving via standards, not an ownable edge. |
  | Assessment specialization (authorization/delegation audit) | **PASS (provisional)** | AI-pentest market centers on prompt injection/model-layer; authorization/delegation is under-owned as a category — though not uncontested, see `docs/business/MARKET_SCAN.md`'s iC Consult / Article 11 caveat. |

  Kill (primitive layer only): it's on a *shipped* incumbent roadmap — treat as a confirmed Path B signal, not a reason to kill the service wedge, which was evaluated independently via RAT-1 through RAT-5 (old §13).

#### The Mom Test — interview question bank (use verbatim)
Ask about their life, not your idea. Never pitch.
1. Walk me through the last time an agent you deployed took an action against a real system.
2. How do you know which agent did what right now? Show me.
3. What happens today if an agent does something it shouldn't have?
4. Last time a credential/permission needed revoking — what did that take?
5. Who's responsible when an agent acts on someone's behalf?
6. What have you already tried or built to control this? Why isn't it enough?
7. How are you handling this for sub-agents / multi-agent workflows?
8. What does this problem cost you when it goes wrong — time, money, incidents?
9. Who else on the team feels this? Who owns it?
10. (Never ask "would you buy" — instead) → offer the paid assessment and watch what they actually do.

(This question bank was carried forward unchanged into the new `CLAUDE.md` §11 — it wasn't specific to the old wedge.)

#### Experiment scorecard (as of 2026-09-02, frozen here)
| RAT | Threshold | Result | Verdict |
|---|---|---|---|
| RAT-1 Problem acute | ≥8/15 | | |
| RAT-2 Willingness | ≥3 paid/pilot | | |
| RAT-3 Tools insufficient | ≥50% real gap | | |
| RAT-4 Owner exists | ≥3 orgs | | |
| RAT-5 Not bundled | ≥1 clear segment | | |
| RAT-6A Wedge works (in-environment) | 3-hop demo | PASSED | **GREEN** (2026-09-02) |
| RAT-6B Wedge portable (no coordination) | independent verification | HOLD, not attempted | — |
| RAT-7 Defensible (primitive) | off-roadmap | On IETF standards track | **Trending FAIL** (2026-09-02) |
| RAT-7 Defensible (assessment) | under-owned segment | — | PASS provisional |

### 12. TestLab 4-Week Plan (Oct 14 – Nov 4)

| Week | Focus | Milestone | Gate |
|---|---|---|---|
| Pre-work | Build interview list (target 25 names), finalize Mom-Test script | 25 outreach targets, 10 booked | — |
| Wk1 (Oct 14) | Interviews + problem discovery | 5–8 interviews done | RAT-1 signal |
| Wk2 (Oct 21) | More interviews + offer paid assessment | 12+ total, ≥1 assessment agreed | RAT-2/3 signal |
| Wk3 (Oct 28) | Run first assessment(s), synthesize ICP | 1 assessment in progress, ICP narrowed | RAT-4/5 signal |
| Wk4 (Nov 4) | Decide end-state (A/B/C), define smallest product | Written go/no-go + smallest-viable-offering | Full scorecard |

**Success statement to earn:** *"This is the specific customer, the problem they repeatedly hit, how they solve it today, and the smallest Crypton offering they'd pay for."*

(The Oct 14 – Nov 4 window itself carried forward into the new `CLAUDE.md` §12 — only its goal changed, from booking interviews for this assessment to finding teams matching the new buyer profile.)

### 13. Kill Criteria (decide in advance, honor them)
- If **RAT-1 fails** → problem isn't acute → pivot problem or move to path C.
- If **RAT-1 passes but RAT-2 fails** → real pain, no willingness → likely path B (protocol) or C.
- If **RAT-3 or RAT-5 fails** → incumbents already cover it → path B/C, do not build a platform.
- If **RAT-6A fails** → no working technical primitive at all → path C, keep as portfolio.
- **RAT-7's primitive-layer verdict trending FAIL (2026-09-02) is expected and non-fatal on its own** — it does not kill the service wedge, which is validated independently via RAT-1 through RAT-5. It does confirm Path B (not Path A) as the primitive's likely long-run home. Only kill the whole venture on primitive-defensibility grounds if the *assessment* RATs (1–5) also fail.
- **Green light for path A** requires RAT-1,2,3,4,5 all pass.

> Write the verdict down in the Decision Log even if it's "no." A clean "no" now beats months building the wrong thing.
