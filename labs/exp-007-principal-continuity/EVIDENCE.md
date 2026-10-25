# Principal Continuity — evidence state

Last updated 2026-09-24.

```
Principal Continuity = Candidate — Testable / 1 Verified External Case
```

- Stable executable test: EXP-007A (frozen, see [FROZEN.md](FROZEN.md))
- Verified independent external evidence families: 1 (CASE-001 / Meta Muse + Amazon)
- Promotion bar (as supplied in the EXP-007 design text, not persisted in the repo's methodology docs; see "Methodology check" below): stable executable test + 2 independent external evidence families (the design text also lists a stable definition and non-redundancy with existing Crypton controls)
- **Not promoted.** A stronger test does not change evidence count.

CASE-001 does not show a vulnerability in Meta's authorization engine. Wardle's `not-a-mused` shows an unprivileged local process redirecting an undocumented control endpoint (local code execution required) so as to influence actions using authority already granted to Muse. Amazon's objection is a separate Resource-Owner Authorization issue.

## Second-family search (2026-09-24, bounded: 5 searches, 3 pages fetched)

Test applied: agent identity valid, delegated capability valid, a *different* controller/instruction source takes over (ideally authenticating as itself), no delegation from the original authority root, and the source documents this. Rejected if the structure is really prompt injection, credential theft, missing authentication, or capability overbreadth.

| Candidate | Primary source | Continuity structure present? | Independent of CASE-001? | Confidence | Verdict |
|---|---|---|---|---|---|
| **ClawJacked** (OpenClaw gateway, fixed 2026.2.13) | [Oasis Security write-up](https://www.oasis.security/blog/openclaw-vulnerability) (fetched); [The Hacker News](https://thehackernews.com/2026/02/clawjacked-flaw-lets-malicious-sites.html) (search snippet only) | Weak. Malicious page brute-forces the gateway *password*, then registers as a device the gateway auto-approves. That is credential guessing plus missing approval; the substitute authenticated with the owner's secret, not honestly as itself. | Yes (different product) | High on facts, low on fit | **Reject** (credential compromise / missing approval) |
| **MCP Ruby SDK session hijack** (CVE-2026-33946; also similar unbound-session CVEs cited in a secondary roundup, unverified) | [GitHub advisory GHSA-qvqr-5cv7-wh35](https://github.com/advisories/GHSA-qvqr-5cv7-wh35) (search snippet only, not fetched) | Weak. Session ID replay lets another party take over an SSE stream. Classic session fixation/hijack; impact is stream interception, not a substitute exercising an agent's delegated action capability. | Yes | Medium | **Reject** (ordinary session hijack; no delegated-capability exercise) |
| **Agent session smuggling** (A2A PoC, Palo Alto Unit 42) | [Unit 42](https://unit42.paloaltonetworks.com/agent-session-smuggling-in-agent2agent-systems/) (fetched) | Partial. Client agent holds trading tools; a remote agent the user authorized only for news retrieval issues instructions beyond that. But the remote agent is a delegated peer, not a controller, and the mechanism is smuggled instructions (prompt-injection class). Source says not observed in the wild; no authentication distinction shown. | Yes | Medium on facts, low on fit | **Reject** (prompt-injection class; PoC only) |
| **Copilot Studio author-credential agents** | Microsoft Learn `configure-enduser-authentication` (search result only, not fetched); Medium posts (secondary) | No. The maker shares the agent, which is a coarse explicit delegation to end users; the resulting oversharing is a documented design choice and capability overbreadth. | Yes | Medium | **Reject** (capability overbreadth / design choice). Useful later as a contrast case for what Principal Continuity is *not*. |

Also seen and dropped without evaluation as clear near-misses: Sentry MCP "agentjacking" (prompt injection), AutoJack (localhost trust, missing authentication; also too close to CASE-001's local-control shape to count as independent without more analysis), mcp-remote CVE-2025-6514 (token theft).

**Result: Outcome B. No second family qualifies.** Weak candidates are recorded above so they are not rediscovered. Caveat: several rows rest on search snippets or vendor write-ups rather than fetched primary documents; that is stated per row, and it is another reason not to accept them.

What a qualifying case would look like: a documented incident or disclosure where a substitute controller *authenticated as itself* (or was otherwise not a credential thief) drove an agent using authority delegated by someone else, and the source itself names the missing delegation or session/controller binding as the flaw.

Next: continue external corroboration search. EXP-007B (replay/approval persistence) remains queued, not started.

---

## Second search pass (2026-09-24): NO QUALIFYING SECOND CASE FOUND

Scope: ~20 candidate families across five search families (local control channels, multi-user/tenant invocation, delegated automation, browser/extension-controlled agents, enterprise agent sharing). 12 primary or vendor pages opened; 8 further candidates seen only in search results and not opened (marked "snippet" below). Status is unchanged: **Candidate — Testable / 1 Verified External Case**.

Evidence dimensions (established / partial / absent). "Auth of B" is informational, not required.

| Candidate | Source (tier) | Identity | Existing authority | Controller substitution | No delegation to B | Action via old authority | Independent of CASE-001 | Verdict |
|---|---|---|---|---|---|---|---|---|
| **Hermes-agent busy-path auth bypass** (#17775, PR #17920 merged 2026-04-30) | GitHub issue + merged PR + tests (T1, fetched) | established | partial (correction 2026-09-24: an earlier summary quoted the issue as saying the text runs with "session credentials and tool access"; the issue body does not say that. It states only that B's text is interrupted into and merged as a next-turn user message in the allowlisted user's active session. Authority is gateway-configured, see adjudication) | established (non-allowlisted Slack user's text enters the authorized user's running session; B authenticated as B on Slack) | established (allowlist is the delegation set; B absent) | **partial**: text reaching the LLM context shown by live reproduction; tool execution *asserted, not demonstrated*; PR tests cover queue/interrupt/steer | yes | **HOLD** (superseded: adjudicated REJECT as CASE-002, see "Hermes adjudication") |
| **BragJack** (Forever Security; Chrome CVE-2026-0628, Edge CVE-2026-55945; Comet, Opera Neon, Claude in Chrome bounty-only) | Researcher blog (T1, fetched) + The Hacker News (T3, fetched) | established | established (full browser authority incl. accounts, files) | established (extension impersonates the trusted vendor origin to command the agent) | established (extension holds no delegation to command the agent) | established (email access, exfiltration, file reads shown) | questionable (different products and mechanism, but same *class* as CASE-001: local component takes over an agent's control path) | **HOLD** |
| OpenClaw sender-allowlist / owner-command advisories (GHSA-534w, GHSA-c28g, GHSA-39mp, others) | GitHub advisories (T1; c28g, 534w fetched; others snippet) | established | partial (c28g says owner privileges for slash commands, "didn't independently grant tool access"; 534w agent authority "determined by group policies") | partial | established | partial to absent | yes | **Reject** (gateway config/authorization checks; authority not clearly U-derived; project docs disclaim hostile multi-user boundary) |
| Langflow CVE-2026-55255 | Vendor DB summary (T2-3; NVD page would not load) | established | partial: source does *not* say whose credentials the executed flow uses | established (authenticated B executes U's flow) | established | **partial/unclear** | yes | **Reject** (IDOR/CWE-639; whose authority is exercised is unstated) |
| CVE-2026-35603 (Claude Code, Cursor, Codex CLI, Gemini CLI config trust) | Cymulate (T2, fetched) | established | partial (victim's OS context, not an agent grant) | partial (planted config, not a controller) | established | established | yes | **Reject** (local privilege escalation via config precedence; source says no agent decision involved) |
| ChatGPT workspace agents, shared connections | OpenAI Help Center (T1; page returned 403, wording from search result only) | established | established | established | **no**: creator publishes the agent, and the vendor documents that anyone who uses it acts "as the creator" | established | yes | **Reject as CASE-002; keep as contrast** (explicit broad delegation; same class as Copilot Studio and Agentforce agent-user design) |
| Cursor MCPoison CVE-2025-54136 | Check Point / Tenable (snippet) | established | established | partial (collaborator swaps an approved config) | established | established | yes | **Reject** (approval bound to name, not content: approval drift, i.e. EXP-007B territory, not controller continuity) |
| MCP OAuth-proxy confused deputy (MCP spec security guidance) | Spec (T1, snippet) | established | established (prior consent) | partial | established | token issuance to attacker | yes | **Reject** (OAuth code interception; ends in token theft) |
| ServiceNow second-order prompt injection (AppOmni); BodySnatcher CVE-2025-12420 | AppOmni; secondary (snippet) | established | established | absent / partial | — | — | yes | **Reject** (prompt-injection class; BodySnatcher is impersonation via hardcoded secret, i.e. authentication bypass) |
| MemoryTrap, Comment-and-Control / GitHub Action agents, Claude Code IDE websocket CVE-2025-52882, generic named-pipe/UNIX-socket issues | secondary/snippet | — | — | — | — | — | — | **Reject / not pursued** (prompt injection; missing authentication; not agent-specific) |

### Why the two HOLDs are not accepted

- **Hermes #17775** is the closest external match to EXP-007A leg d: authenticated B, valid session authority, no delegation, the controller-authorization check missing on one code path. It is held because (1) tool execution under the authorized user's authority is not demonstrated in the source, (2) it is one community-reported instance in one project, and (3) it can be read as a single missing authorization check, which invites the objection that it is ordinary broken access control (see below). Upgrade path: a source showing an unauthorized participant's message driving a tool call under the authorized user's credentials, or a second independent project with the same session-inheritance mechanism. One extra search for that found nothing.
- **BragJack** has the strongest evidence of authority actually exercised, but the researcher describes the root cause as controller authentication resting on origin alone, so it falls under the hard-reject rule for authentication weakness. It also corroborates the same sub-form as CASE-001 (control-path substitution without an authenticated B), so it would not add coverage of the authenticated-substitute form and its independence from CASE-001 is questionable.

### Observation about the search itself

Cases that match leg d's shape (authenticated B, valid authority, missing controller check) read as ordinary broken object-level or sender authorization. Cases that match control-path substitution read as weak controller authentication. Neither pattern has yet produced a source that isolates continuity from both. This does not lower the evidence bar; it is a reason to check, at the promotion review, whether the property is distinct from existing authorization controls rather than a renaming (promotion rule item 4).

### Search notes

Productive terms: "sender allowlist bypass" plus a specific agent gateway name; "shared thread session" plus "authorization"; browser AI assistant extension hijack; agent shared-connection "as the creator". Noise: generic "AI agent hijack IPC", "cross-user credential" (returns platform-wide vulnerability roundups), "confused deputy" (returns explainers). Unresolved avenue: agent gateways where session authority is bound to the first authorized participant (Hermes-style shared-thread sessions in other projects), and desktop assistants with local control sockets that document multi-principal authentication.

Next: continue corroboration work. EXP-007B remains queued, not started.

---

## Hermes adjudication (2026-09-24)

**Final: REJECT as CASE-002.** Retained as a complete-mediation analog (an authority-holding agent's second control path skipped the controller-authorization gate the first path enforces). Distinctness classification: see below. Status unchanged: **Candidate — Testable / 1 Verified External Case**.

### Primary artifacts reviewed

- Issue NousResearch/hermes-agent #17775 (body read in full via API; 0 comments; labels `type/security`, `area/auth`, `P0`; opened and closed 2026-04-30; reporter @syahidfrd; env v2026.4.23)
- PR #17920 by @teknium1, merged 2026-04-30 (merge commit `fbb3775`), salvaging #17816; 2 files, +239/-0 (16-line gate plus `tests/gateway/test_busy_session_auth_bypass.py`)
- Source at the PR base `cc5b9fb` (2026-04-29): `gateway/run.py`, `gateway/platforms/base.py`, `gateway/session.py`, `tools/approval.py`. Line numbers below are at that commit; the issue's line numbers are from v2026.4.23 and differ.

### What the primary artifacts establish

| Claim | Status | Basis |
|---|---|---|
| B is a platform-authenticated user, not allowlisted | established | issue reproduction; `_is_user_authorized` (`run.py:3462`) checks per-platform env allowlist, pairing store, allow-all flags |
| Default `thread_sessions_per_user=False`, so a Slack thread shares one `session_key` | established | `base.py:2384-2387` default; `session.py` `build_session_key` |
| Cold path authorizes, busy path did not | established | cold: `run.py:3802` in `_handle_message`; busy: `_handle_active_session_busy_message` (`run.py:1876`) had no check; PR body says the same |
| B's message interrupted U's running task | **demonstrated live** | issue repro: "⚡ Interrupting current task" posted |
| B's message became input to the next agent turn | **demonstrated live** (bot answered B by name) and **confirmed in code** | `run.py:~12034-12205`: queued event is dequeued, sender attributed as `[user_name] text` (`run.py:4658`), then a recursive `_run_agent` call with the same `session_id`, history and toolsets; this path does not re-enter `_handle_message`, so no re-check |
| Sender identity survives the queue | established | `pending_event.source` carried into `next_source`; `[user_name]` prefix in shared sessions |
| Tool execution re-checks the sender | **no** | no `_is_user_authorized` on the run path; `user_id` is passed to the agent and hooks as metadata, and toolsets come from platform config (`enabled_toolsets`), not the sender |
| A tool call or action caused by B was observed | **not demonstrated** | no log, screenshot or test in the issue, PR or tests shows a tool call driven by B's text |
| Whose authority does the session carry? | gateway/operator configuration, not U-derived | the allowlist is gateway-wide; every allowlisted user has the same control and the same toolsets; no per-user credential or per-user tool grant found |
| Do approvals bind to the sender? | no, to the session | `tools/approval.py` state is keyed by `session_key` (`_session_approved`, `_session_yolo`); `/approve` itself goes through the authorized `_handle_message` path, so an unauthorized user cannot grant one, but approvals already granted for the session would apply to a later turn driven by B. This is a code-reading inference, not a demonstrated exploit, and it only matters after the ingress gap. |

### The fix, read semantically

- Condition added: `if not self._is_user_authorized(event.source): log; return True` (dropped).
- Location: first statement of the busy handler, i.e. **control-path ingress**, before draining, queueing, interrupt or steer logic.
- Branch that lacked it: the "active session exists" branch; the cold branch already had it.
- Tests (`test_busy_session_auth_bypass.py`) use mocks and assert only: no queue, no `interrupt()`, no acknowledgment, no `steer()`, still true during drain; authorized user still queued. **They stop message ingestion and control actions; none asserts on tool execution.** The PR body reports an end-to-end reproduction and fix, described the same way ("no queue, no interrupt, no ack").

### Why REJECT and not HOLD

1. The source's own root cause and fix are a missing authorization check at ingress on one path (complete mediation), not a failure to keep an authority bound to its grantor.
2. Hermes has no U-derived authority to be continuous with. Authority is gateway-configured and shared by the whole allowlist. B's offense is being outside the controller set, and once inside that set nothing distinguishes U's control from B2's.
3. The decisive edge (B's instruction causes an action under existing authority) is structurally reachable but undemonstrated.
4. The alternative explanation was tested and holds: with `_is_user_authorized(B)` on the busy path (which is the merged fix), the security problem in the source disappears entirely. Nothing about time, control transitions or grant provenance is left over.

Authority graph as evidenced: `operator/gateway config -> A(active session, toolsets)`; allowlist = controller set; `B -> A` edge existed via the busy path without membership. The further edge `A -> tool/action because of B` is code-reachable only.

### Muse vs Hermes (evidence only; Muse cells from the CASE-001 record, not re-verified in this task)

| Dimension | Muse (CASE-001) | Hermes |
|---|---|---|
| Legitimate agent identity survives | yes | yes |
| Existing action authority survives | yes (access granted to Muse; Sentinel still mediates) | yes, but gateway-wide, not U-derived |
| Controller changes | yes (control endpoint redirected by a local process) | yes (a non-allowlisted participant's input enters the session) |
| Substitute authenticated as itself | not established | yes (platform identity; Hermes sees `user_id`/`user_name`) |
| Explicit delegation to substitute | none | none |
| Action under previous authority demonstrated | influence asserted in the PoC repo; scope limited per CASE-001 caveats | not demonstrated (interrupt demonstrated; tool execution code-reachable) |
| Root issue looks like authentication | partly (local control-endpoint integrity/trust; requires local code execution) | no (platform auth is sound) |
| Root issue looks like authorization | unclear | yes (missing check on a second path) |
| Complete mediation failure | partly (upstream control path unprotected; downstream mediation intact) | yes, by the source's own framing |
| Confused-deputy structure | partly | partly (deputy holds standing authority driven by a non-authorized requester) |

Reading: two different ordinary bug classes (control-endpoint integrity; missing authorization on an alternate control path) that EXP-007A models identically. They share one abstraction (an authority-holding agent's control plane admits a requester outside the authorized controller set), not one underlying failure mechanism. That is a weaker basis for "independent manifestations of a new invariant" than for "two instances of complete mediation on the control plane."

## Distinctness note

**Classification: DERIVED / SPECIALIZED AUTHORIZATION INVARIANT** (analytical judgment, medium confidence; not promotion).

Decomposing EXP-007A's secure policy: `capability_valid` (function-level authorization of the action) AND `controller_authenticated` (requester authentication) AND `controller in authorized_controllers` (caller authorization / delegation-chain membership) AND `session bound` (session binding; replay deferred to 007B). Each conjunct is a standard control, and the check is per message: **EXP-007A is stateless, so nothing in the frozen test exercises "continuity across time".**

Smallest statement: an authorization to exercise delegated authority must be enforced for the actual requester on every control path, and not inherited from an earlier requester. The final clause is the only place state appears, and "do not reuse an earlier authorization decision for a different requester" is complete mediation (no cached decisions). Ambient authority induced by an unauthorized caller is the confused-deputy problem. Membership in the grant's chain is delegation validation. Nothing remains that these three do not already cover.

What is agent-specific is the surface, not the invariant: many control paths (chat gateways, local endpoints, browser channels), shared sessions, natural-language control, controllers changing mid-session. That justifies naming the obligation in an agent assessment catalog (for example, "enumerate every path that can direct the agent and verify each enforces the same controller check") as a derived invariant.

What would overturn this: a source where every request is individually authenticated and authorized as a general user of the system, yet authority state created under one principal (an approval, session grant, checkpoint) is honored for another principal's request, and the failure is not attributable to a missing per-request check. That is the stateful inheritance form, EXP-007B territory (approval and session-state binding), and it is currently unevidenced. Note Crypton's own catalog already contains adjacent obligations: Pillar 3 ("never ambient"), Pillar 4 (approval binding), Pillar 6 (U-PERSIST-2: the set of principals who may mutate pending state is a defined trust boundary), Pillar 7 (attribution).

Implication for EXP-007A (unchanged, no renames): if derived, it is a reference test for controller/delegation complete mediation within agent authorization. Terminology could later change from "Principal Continuity" to something describing control-path controller authorization; recommendation only.

## Methodology check

- Repo wording found: a **2-target usefulness bar** for promoting shared abstractions (`docs/methodology/04-framework-adaptation-guide.md` step 7; `assessment-kit/v1.0/VALIDATION-REPORT.md` §2: "2-target promotion bar"). This tests recurrence and unmodified usefulness across targets.
- **Not found in the repo:** the "2 independent external evidence families" rule, and any criterion requiring a property to be conceptually distinct from existing controls or from established security principles. Those appear only in the EXP-007 design text supplied in-session and in this file.
- Methodology gap: the persisted promotion rules test evidence recurrence, not conceptual non-redundancy. Recommended future amendment (not made): add an explicit non-redundancy check against Crypton's own pillars and against named prior principles (complete mediation, confused deputy, object-level authorization) before a candidate property is promoted.

Next: methodology review before additional experiments (distinctness is unresolved at the program level even though this analysis leans derived). EXP-007B remains queued, not started — a minimal specification (scope: revocation-between-approval-and-resume only; replay/nonce consumption still deferred) is written at [`EXP-007B-SPEC.md`](EXP-007B-SPEC.md), 2026-09-27.
