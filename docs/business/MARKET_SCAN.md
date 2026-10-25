<!-- Canonicalized into the repo 2026-09-02, from Aryan's working copy (Crypton-Market-Scan.md). Content unchanged. See CLAUDE.md §7/§11/§14/§15 and DECISIONS.md for how this scan's findings were reconciled into the rest of the repo's docs. -->

# Crypton — Market Scan (Current Wedge)

**Date:** 2 Sep 2026 · **Scope:** Scanned against the *current* wedge only — the service-first **agent-authorization assessment** and the **multi-hop delegation + attenuation** primitive (RAT-6A). Not the broad platform (already known to be absorbed).
**Status:** Canonical reference. Refresh only on material RAT-7 / positioning change.

---

## Headline

The two wedge pieces point in opposite strategic directions:

- **Service wedge (assessment):** *more* defensible than it looks — the AI-security-testing market is crowded but leaves the authorization/delegation lane under-owned.
- **Technical wedge (delegation primitive):** *less* defensible than assumed — it's actively standardizing, not an open gap. This is a **Path B trigger** by the plan's own criteria.

---

## 1. Demand — real and rising

The problem the assessment sells against is now showing up as incidents, not just vendor surveys.

- **CSA/Okta (Mar 2026)** documented the core failure: by the 3rd delegation hop there's no cryptographic link back to the initiating agent/user, letting a malicious agent forge delegation claims.
- **Real incidents:** a May 2026 case where an agent received expanded capability via a side channel and forwarded it down a chain with no scope check at the boundary → multi-billion-token drain. Vercel disclosed (Apr 2026) a breach that pivoted through a compromised third-party AI tool's OAuth app.
- Machine identities outnumber humans ~82:1 (Palo Alto). ~90% of orgs reported an identity-related breach (2026 survey). Gartner (widely cited): 25% of enterprise breaches traced to agent abuse by 2028.

**Read:** demand for "audit how authority flows through our agents" is genuine and growing.

---

## 2. Supply A — the assessment competitive set

AI penetration testing is a **populated, maturing category** in 2026.

**Players:** NetSPI, HackerOne, Synack, Cobalt, Bugcrowd (established); Software Secured, AppSecure, SecureLayer7, Escape, xBow, Simbian, Penligent, Invicti, Penetrify (AI-native); plus OSS agents (PentestGPT, PentAGI) and cloud (AWS Security Agent).

**Their coverage centers on the model layer:** prompt injection, jailbreaking, RAG poisoning, model inversion, "agentic exploitation" (chaining actions), MCP endpoint vulns — mapped to OWASP LLM Top 10, MITRE ATLAS, NIST AI RMF.

**The gap = your lane:** even AWS's autonomous pentester is noted as *not* covering bespoke authorization/IAM control-plane testing — that still needs independent human assessment.

| Dimension | Well-covered by market | **Under-owned (Crypton's lane)** |
|---|---|---|
| Focus | Prompt injection, jailbreak, RAG poisoning, model behavior, MCP vulns | Agent **identity, delegation lineage, scope attenuation, approval-binding, revocation propagation** |
| Framing | OWASP LLM Top 10 / MITRE ATLAS | "Who authorized which agent to do what — and where does that break?" |
| Named specialist | 10+ firms | **None** |

**Read:** Don't brand as "AI pentest" (lose to HackerOne/Synack on scale). Brand as **the agent authorization & delegation specialist**. v0.6 is the credibility proof — you assess against attenuation/lineage best practices because you've built them.

> **Caveat, added when reconciling this scan into the repo (2026-09-02):** "no named specialist" above is a category-market read, not a competitor-by-competitor exhaustive search. In a direct conversation the same day this scan was reconciled, specific firms — **iC Consult** and **Article 11** — were named as already overlapping materially with this exact lane. Treat "under-owned" as "not yet dominated by a category leader," not "no one else is here." Crypton's differentiation in this lane is open and unproven, earned through delivered engagements, not asserted from this scan. See `CLAUDE.md` §6/§7 and `DECISIONS.md`'s v0.6.1 strategy-correction section.

---

## 3. Supply B — the delegation primitive is standardizing (the uncomfortable part)

The multi-hop-delegation-with-attenuation primitive (your RAT-6A) is **converging into a standard now**:

- **IETF draft `draft-niyikiza-oauth-attenuating-agent-tokens`** (v01, Jun 2026) — Attenuating Authorization Tokens (AATs): asymmetric proof-of-possession, structured tool-level capability claims, typed constraints, **offline attenuation, independent chain verification without network calls, signed-JWT encoding**. ≈ a description of your v0.6.
- **Related active drafts:** cryptographically verifiable actor chains for OAuth token exchange; OAuth identity chaining across domains; SPIFFE/WIMSE per-agent workload identity.
- **Research:** Google DeepMind's 2026 "Intelligent AI Delegation" → Delegation Capability Tokens (on macaroons); academic Invocation-Bound Capability Tokens with Rust+Python reference impls.
- **Best practice already written:** CSA/Okta's append-only attenuated caveat chain.
- **Prior primitives:** Macaroons (2014), Biscuit, UCAN, SPKI/SDSI (RFC 2693, 1999).

**Read against the framework:**
- **RAT-6A (internal primitive built):** GREEN — real and correct.
- **RAT-6B (portable/no-coordination passport):** exactly what AAT specifies → building a proprietary version is **redundant**. If unlocked, *implement/contribute to AAT*, don't invent a rival.
- **RAT-7 (primitive defensibility):** effectively **FAIL** — feature arriving via standards, not a moat. → Path B signal.

---

## 4. Pricing reality

| Engagement (2026) | Price |
|---|---|
| General pentest, typical | $10K–$30K (avg ~$18K) |
| **AI/agent pentest, entry floor** | **~$9,500** (single chatbot) |
| AI/agent pentest, enterprise multi-agent | up to $75K |
| Simbian AI Pentest Agent | $4K standard / $8K complex |
| AWS Security Agent | $50/task-hour |
| Continuous AI pentest platforms | $100–$1,700/mo |
| **Crypton pilot (current)** | **$2,500** |

$2,500 is ~4x under the AI-pentest floor (~$800/day). Easy yes, but risks signaling "not serious" and anchors low.
**Fix:** keep the number, frame as **founding/design-partner rate — "standard $8–10K."** Preserves the RAT-2 paid commitment without anchoring at the floor.

---

## 5. Strategic implications (what changes)

1. **Reposition the assessment** → "agent authorization & delegation specialist." Own the under-served lane; don't fight AI-pentest incumbents.
2. **Demote the primitive** from moat → credibility asset + possible standards contribution. Update Tech SOT + RAT-7.
3. **Pre-load Path B** as the live alternative (RAT-7 primitive already trending fail). Path B = reference implementer/contributor to AAT/WIMSE, *not* invent a protocol.
4. **Reframe $2,500** as a discounted founding rate.

---

## Sources (paraphrased; for refresh/verification)

Market/consolidation & demand: CSA/Okta "Control the Chain" (Mar 2026); Palo Alto / 2026 identity survey; Gartner agent-abuse projection; incident write-ups (Bankrbot token drain, Vercel/Context.ai). Assessment market: SecureLayer7, Software Secured, AppSecure, Penligent, CodeAnt roundups (2026); AWS Security Agent analysis (Stingrai). Delegation standards: IETF `draft-niyikiza-oauth-attenuating-agent-tokens` v00/v01; WorkOS multi-hop delegation write-up; OAuth WG mailing list (delegation-chain splicing); DeepMind DCT; IBCT/AIP academic work; arXiv 2510.25819. Pricing: Simbian, Invicti, Synack, DeepStrike, Penetrify, pentesttesting.com (2026 pricing guides).

*Figures from analyst/vendor reports vary widely and several are vendor-commissioned — treat as directional. Replace with your own primary evidence from TestLab interviews where possible.*
