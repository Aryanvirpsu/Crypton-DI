<!-- Canonicalized into the repo 2026-09-02, from Aryan's working copy (Crypton-Operating-Plan-Update-v1.1.md). Content unchanged except this note.

This change-set's "§N" references are to a fuller working Operating Plan
that Aryan maintains outside this repo — CLAUDE.md (the in-repo founder
doc) uses a shorter 0-16 section numbering and does not have §17-23. Where
a change below had a clear equivalent in CLAUDE.md, it was applied there
directly (see CLAUDE.md §7, §8b, §9, §10, §11, §13, §14, §15 and this
same date's Decision Log entries). Where it didn't (the detailed legal/
compliance gate, the weekly cadence, the outreach mechanics), the source
material lives in this docs/business/ directory instead of being force-fit
into CLAUDE.md's structure. -->

# Crypton — Operating Plan Update (v1.1 Change Set)

**Purpose:** Slots into your existing Operating Plan (Sep–Nov 2026). These are the only things that change after today's market scan. Apply each change to the referenced section; everything else in the plan stands.
**Date:** 2 Sep 2026 · **Trigger:** Market scan of the current wedge (see `Crypton-Market-Scan.md` / `docs/business/MARKET_SCAN.md`)

---

## Change 1 — Plain-language framing of the three paths (add to §14 intro)

The end-state is one of three. You don't pick now; TestLab evidence picks.

- **Path A — Sell a product.** Companies pay for a Crypton tool. (Makes money.)
- **Path B — Make your tech the shared industry standard and be its recognized expert.** You don't sell the delegation core; you help make it the standard way agents delegate authority, and you become the reference implementation everyone checks against. (Makes reputation/influence, not direct revenue.)
- **Path C — Portfolio/research flagship.** No company. Proof you build hard things. Move on.

---

## Change 2 — Redefine Path B (replaces §15 framing)

**Old Path B:** "invent an open delegation protocol / delegation passport."
**New Path B:** "**be the reference implementer of and contributor to the emerging agent-delegation standard.**"

**Why the change:** The scan found the delegation-passport primitive is already being written as an IETF Internet-Draft (Attenuating Authorization Tokens, `draft-niyikiza-oauth-attenuating-agent-tokens`) plus related OAuth/WIMSE work. Inventing a rival protocol is redundant. Your v0.6 is a *working implementation of what they're still drafting* — that's the leverage.

**Concrete Path B activities (only if evidence unlocks it):**
- Bring v0.6 implementation experience to the OAuth WG / WIMSE (running code carries weight in IETF).
- Maintain the reference implementation + test vectors others verify against.
- Language SDKs, independent verifier.

**Honest caveats (do not over-read this):**
- Reference-implementer = credibility/influence, **not revenue**. It's a Path B/C asset, not a business by itself.
- Big players (Okta, Microsoft, SPIFFE) are in these rooms too; your edge is speed and focus, not staying power.
- Only matters if agent delegation actually standardizes around attenuated capability tokens.
- **Do not start this now.** It's a post-Nov-4 unlock, not a current activity. Current mandate = customer evidence.

---

## Change 3 — Revised RAT-7 (replaces §13 RAT-7 assessment)

**RAT-7 — Is the wedge defensible / off incumbents' shipped roadmap?**

| Layer | Verdict | Basis |
|---|---|---|
| **Delegation primitive (the token/attenuation tech)** | **Trending FAIL as a moat** | Actively standardizing (AAT draft, DeepMind DCT, IBCT, CSA/Okta best-practice). It's a feature arriving via standards, not an ownable edge. |
| **Assessment specialization (authorization/delegation audit)** | **PASS (provisional)** | The AI-pentest market centers on prompt injection / model-layer; the *authorization & delegation* dimension is under-owned. No named specialist. |

**Implication:** RAT-7 on the primitive being weak is, by your own §15, a **Path B signal** — but the *service wedge* remains defensible on specialization. So: sell the assessment (Path A test), hold the primitive as credibility + Path B option. Do not build the primitive into a product moat.

---

## Change 4 — Reposition the assessment (updates §8, §9, §21)

**From:** "agent-authorization security assessment" (generic AI security review).
**To:** **"The agent authorization & delegation assessment"** — own the lane generalist AI red-teamers skip.

**Why:** HackerOne, Synack, NetSPI, Software Secured, AppSecure et al. already sell AI pentesting (prompt injection, jailbreak, RAG poisoning). You lose that fight on brand/scale. But their coverage under-serves identity/delegation/attenuation/approval-binding/revocation — the exact things your v0.6 implements. Position as the specialist, not another AI pentest.

**Messaging update (§21 external line, revised):**
> We assess how authority actually flows through your AI agents — who authorized which agent to do what, whether scope narrows at each hop, and whether a bad action is stopped before it executes, not logged after.

**Positioning guardrail:** In findings, recommend the *control*, not Crypton. Assessment credibility is the asset; don't spend it steering every finding toward your own product.

---

## Change 5 — Tighten assessment scope (updates §9)

Your §9 scope (identity + ownership + permissions + delegation + revocation + approval + tool-execution + prompt/tool-injection + evidence) is too broad for 3 days / solo / first engagement. Split it:

**v1 core engagement (what you sell now):**
- Agent identity & ownership
- Delegation & authority flow (lineage, scope attenuation, revocation propagation)
- Authorization boundary (where the check sits; pre- vs post-execution; bypass paths)
- Human-approval analysis (what requires approval; is approval bound to the exact request)
- Evidence/accountability reconstruction

**Paid add-on (separate scope, later):**
- Prompt/tool injection, jailbreak, dangerous-argument, privilege-escalation-via-behavior testing.

Reason: injection testing is a different discipline the crowded firms already do well; leading with it puts you head-to-head with them. Lead with the authority/delegation lane you own; offer injection as an add-on once the base engagement is repeatable.

---

## Change 6 — Reframe the $2,500 price (updates §8)

Market floor for an AI/agent pentest is ~$9,500; typical AI pentests run $9.5K–$75K; general pentests $10K–$30K. $2,500 sits ~4x under the floor (~$800/day) — an easy yes, but it can read as "not a serious assessment" and anchors you low.

**Fix — keep the number, change the frame:**
> Founding assessment rate: **$2,500** (design-partner pricing; standard rate $8,000–$10,000).

You still get the real paid commitment RAT-2 needs, without permanently anchoring at the floor.

---

## Change 7 — Gate first paid assessment on legal setup (new item in §18 Sep 2–7)

**Blocking pre-req before any paid assessment:**
- Form a single-member LLC (liability separation).
- Signed **Authorization-to-Test** agreement per engagement (scope, environments, out-of-scope, safe-harbor). Template in `Crypton-Assessment-Pack.md` / `docs/business/ASSESSMENT_PACK.md`.
- Confirm customer owns/can authorize the target environment.

Do not run Burp/injection/priv-esc testing against a customer environment as an unincorporated individual with no signed authorization. This is the one omission that turns a $2,500 pilot into a real problem.

---

## Change 8 — Add Market Scan as a standing reference (new §)

`Crypton-Market-Scan.md` (`docs/business/MARKET_SCAN.md`) is now a canonical reference alongside the Founder SOT. Refresh it only when a competitor or standards development materially changes RAT-7 or the assessment positioning — not weekly. Classify changes per §23.

---

## Decision Log entries (append to §15 log)

| Date | Decision | Rationale | Evidence |
|---|---|---|---|
| 2026-09-02 | Delegation primitive reclassified: credibility asset, not moat | Actively standardizing (AAT/WIMSE) | Market scan §Supply-B |
| 2026-09-02 | Path B redefined: reference implementer/contributor, not new protocol | Rival protocol redundant vs AAT draft | Market scan |
| 2026-09-02 | Assessment repositioned to authorization/delegation specialist | AI-pentest market under-serves this lane | Market scan §Supply-A |
| 2026-09-02 | Assessment scope split: v1 core (authority) + injection add-on | Too broad for 3-day solo engagement | Plan review |
| 2026-09-02 | $2,500 reframed as founding/design-partner rate | ~4x under AI-pentest floor | Pricing scan |
| 2026-09-02 | Legal entity + authorization-to-test gate first paid assessment | Solo-founder liability on grey-box testing | Plan review |

---

*Everything not listed here is unchanged. This change set keeps the plan honest after the scan.*
