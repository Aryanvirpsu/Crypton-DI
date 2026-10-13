# Evidence Map — External Corroboration for Crypton's Test Catalog

**What this document is:** a mapping between Crypton's test-catalog
properties (`03-test-catalog.md`) and independently disclosed,
publicly-documented failures in real, unrelated, production agent/MCP
systems, current as of September 2026.

**What this document is not, and this matters:** Crypton did not
discover, report, or verify any of the third-party cases below — they
are independent security disclosures by other researchers and vendors,
found and read directly for this pack (primary advisories, CVE records,
and vendor security bulletins, cited below). **No external incident
independently validates Crypton specifically** — what they validate is
that the *failure classes* Crypton's methodology targets are real,
current, and recur across systems Crypton has never touched, not that
Crypton's specific findings about any customer are correct. Every row
below is kept separate from "Crypton-produced evidence" (Crypton's own
internal research corpus, listed at the bottom), which is the only
evidence that actually demonstrates Crypton's own testing process works.

---

## External cases, mapped to test-catalog families

| Case | Family | Observed external evidence | Crypton interpretation |
|---|---|---|---|
| **MCP Python SDK** — CVE-2026-52869 (CVSS 7.1) | 1. Principal / Session Binding | The official MCP Python SDK's SSE and stateful Streamable HTTP transports routed requests to an existing session using only the session identifier, without verifying the authenticated principal that created it — a different bearer-token-authenticated client presenting the same session ID could inject messages into it. Fixed in 1.27.2, which now records and checks the authenticated principal per session. [GitHub Advisory GHSA-jpw9-pfvf-9f58](https://github.com/advisories/GHSA-jpw9-pfvf-9f58) | This is precisely the failure shape Family 1's test asks about — session-identifier-only routing with no principal re-check — found and fixed in the reference MCP SDK itself, not an obscure third-party clone. It shows this is not a hypothetical Crypton invented; it shipped, by default, in widely-used infrastructure. **Independently reproduced, not just read about:** Crypton wrote its own probe against this exact code path and ran it, unmodified, against both the vulnerable (1.27.1) and fixed (1.27.2) installed package — FAIL on the former, PASS on the latter. This is the strongest evidence in this whole document, because it's the one case that isn't just citation — see `labs/validation-benchmark/v0.1/case-01-mcp-python-sdk-session-binding/`. |
| **MCP Rust SDK (rmcp)** — CVE-2026-63127 (CVSS 8.2) | 1. Principal / Session Binding (resource-identity variant) | The Rust SDK's OAuth resource-discovery path omitted the RFC 9728 resource field and didn't confirm a returned resource identifier matched the configured MCP server, letting a malicious server capture a legitimate token via a confused discovery flow. Fixed in 2.0.0. [OffSeq / OSV](https://osv.dev/vulnerability/CVE-2026-63127) | A binding failure one level removed from session identity — the *resource* a token is scoped to wasn't verified, not just the session. Related enough to Family 1 to note; not identical to the session-hijack shape above. |
| **mcp-atlassian (sooperset)** — CVE-2026-77243 (CVSS 8.8) | 3. Authorization Boundary & Complete Mediation | `ENABLED_TOOLS`/`TOOLSETS` restrictions were applied when tools were *listed* but not rechecked when a `tools/call` request was actually *dispatched* — a client that knew a hidden tool's name could invoke excluded read/write/delete tools directly. Fixed in 0.22.0. [OffSeq](https://radar.offseq.com/threat/cve-2026-77243-cwe-862-missing-authorization-in-sooperset-mcp-atlassian-c314b02cb2a26f89) | This is Family 3's exact question — "is the check on the path the action actually takes" — answered no, on a real, actively-used MCP server, because the enforcement point checked at list-time, not call-time. A close real-world analog to the reasoning EXP-003C's `update_state()` finding (FL-02) already used internally. |
| **mcp-atlassian** — CVE-2026-77244, GHSA AIKIDO-2026-670729 | 3. Authorization Boundary & Complete Mediation | HTTP/SSE transport accepted requests with no verified user identity; with global credential fallback enabled (a common config), any network caller reaching the endpoint could invoke Atlassian actions as the operator's own account. [OffSeq](https://radar.offseq.com/threat/cve-2026-77244-cwe-287-improper-authentication-in-sooperset-mcp-atlassian-57f4964ca891f817), [Aikido](https://intel.aikido.dev/cve/AIKIDO-2026-670729) | A missing-mediation failure at the transport layer rather than the dispatch layer — same family, different hop in the same call path. |
| **Casdoor** — CVE-2026-15630 (CVSS 9.9) | 3. Authorization Boundary & Complete Mediation | A non-global org admin could act on arbitrary other organizations because the authorization check read the target from `?id=` in the query string while the actual mutating action read its target from the request body — the two disagreed, and the check didn't govern the action it was meant to gate. [Strix](https://www.strix.ai/cve/CVE-2026-15630) | A textbook "check one object, act on a different one" desynchronization — exactly the complete-mediation question Family 3 asks, in a real identity/authorization product, not an edge-case toy example. |
| **Casdoor** — CVE-2026-91998 | 2. Delegation & Scope Attenuation | A single application's `clientId`/`clientSecret` on the `/api/mcp` endpoint granted unrestricted user administration across *all* organizations, not just that application's own scope. [Vulners](https://vulners.com/cvelist/CVELIST:CVE-2026-91998) | Authority that should have attenuated to one application's scope instead inflated to global admin — exactly Family 2's failure shape. |
| **MaxKB** — CVE-2026-77521 (CVSS 10.0) | 4. Approval-to-Execution Binding | Assistants using a shell/MCP/skill tool were, by construction, excluded from the `interrupt_on` approval list — meaning the human-approval gate the product advertises was never actually invoked for this tool, not bypassed after the fact but never wired in for it at all. [GHSA-f36j-f34j-h3rx](https://gbhackers.com/critical-maxkb-ai-agent-flaw/) | The most literal real-world match in this whole map to Family 4's question — a specific tool executing without ever reaching the approval boundary the product claims to have. Distinct from Crypton's own retracted LangChain case (see `07-why-trust-crypton.md`): here the vendor's own advisory frames it as a real gap and ships a fix, not as expected, permitted behavior. |
| **MaxKB** — CVE-2026-77516 | 2. Delegation & Scope Attenuation | A workspace member explicitly denied access to a specific tool could still bind that tool's identifier through the agent/workflow dispatch path, because the dispatch path never re-applied the per-tool grant check that the dedicated tool routes enforced. Fixed in 2.10.5-lts. | The same "checked on one path, not on another" shape as the mcp-atlassian and Casdoor cases above, here specifically at the delegation/scope layer rather than the top-level authorization layer. |
| **Bifrost (Maxim AI gateway)** — CVE-2026-90898 (CVSS 9.8) | 3. Authorization Boundary & Complete Mediation | An unauthenticated single HTTP request to the management API could register a new MCP client, and the gateway started the specified process immediately — before any handshake or authentication — as the gateway's own user, with access to every provider's stored API keys. Fixed in 2.1.0. [The Hacker News](https://thehackernews.com/2026/09/critical-bifrost-ai-gateway-flaw-lets.html), [vendor advisory](https://www.getmaxim.ai/bifrost/blog/security-update-cve-2026-90898-and-cve-2026-86242-are-fixed-in-bifrost-v2-1-0) | A gateway is exactly the kind of control `01-positioning.md` says Crypton tests the guarantee *of*, not replaces — this case shows a real gateway's own management surface skipping the check it exists to provide, by default. |
| **`@bytebase/dbhub`** — CVE-2026-61788 (CVSS 7.4) | 3. Authorization Boundary & Complete Mediation | Setting `readonly = true` on the SQL-execution tool did not actually make the connection read-only — the code path that was supposed to enforce it was gated on a config value that was never populated (originally reported against the PostgreSQL connector), on an unauthenticated-by-default HTTP transport. | A claimed boundary (readonly) that was never actually wired to anything — the inverse failure of Crypton's own §9a triage rule: here the boundary really was claimed and really was crossed, unlike the LangChain case Crypton retracted. **Independently reproduced (VB-002), not just read about:** this session's own diff of the published `0.22.5`/`0.22.6` packages found the identical pattern separately present, and separately fixed, in the SQLite connector too — Crypton wrote its own probe against the SQLite connector's public API and confirmed FAIL on `0.22.5`, PASS on `0.22.6`. See `labs/validation-benchmark/v0.1/case-02-dbhub-sqlite-readonly-boundary/`. |
| **Google MCP Toolbox for Databases** — CVE-2026-11720 (CVSS 9.3) | 2. Delegation & Scope Attenuation | The HTTP tool's URL builder let a caller-supplied path parameter contain directory-traversal sequences that survived URL resolution, letting a request escape the operator-configured path scope and reach unintended endpoints on the same host while still carrying the toolbox's own credentials. Fixed in 1.3.0. [Vulners/OSV](https://vulners.com/osv/OSV:GO-2026-6411) | Scope that was supposed to be bounded to one configured path instead reached arbitrary endpoints with the same credential — a scope-attenuation failure at the network/path layer rather than the tool-permission layer. |
| **Plugin4Shell** (cross-vendor: Claude Code, Codex, Copilot, Gemini CLI) | related to 4, not a clean fit for any single family | A SHA-pin meant to lock a reviewed plugin/skill to a specific commit could be defeated by crafting a branch name that mimicked a commit hash, letting an attacker swap in different code while the agent reported the original, reviewed hash as installed. [Help Net Security](https://www.helpnetsecurity.com/2026/09/18/plugin4shell-ai-coding-agents-vulnerability/), [air.security](https://www.air.security/blog-posts/plugin4shell) | Not a delegation/approval-argument binding case — the thing that drifted was the *artifact's identity itself* (what code is actually running vs. what was reviewed and pinned), a related but distinct binding question from anything in the current catalog. Recorded here honestly as adjacent, not force-fit into an existing family. |
| **ReSA** (academic research, not a disclosed incident) | 4. Approval-to-Execution Binding | A September 2026 paper formalizes exactly the general question Family 4 asks — that the object shown for approval is often not the object ultimately consumed at execution, because reload, transcript projection, argument rebinding, or durable-state lookup can reconstruct it in between — and shows that checking the right fields isn't sufficient if the checked *version* doesn't provably reach the execution sink. [arXiv:2609.26529](https://arxiv.org/abs/2609.26529) | This is academic corroboration, not an incident — cited as evidence the underlying question is a recognized open problem in the research literature independent of Crypton, not as a disclosed vulnerability. |

---

## The Crypton Validation Benchmark — breaking the circularity problem

Everything in the table above, except Case 01, is Crypton *reading and
mapping* someone else's disclosure — real, but still one step removed
from evidence that Crypton's own testing process works. A methodology
that only ever tests itself (designs a test, runs the test, the test
passes) proves nothing about whether the test measures anything real.

The **Crypton Validation Benchmark** (`labs/validation-benchmark/v0.1/`)
exists to close that gap without needing a customer: take an
independently disclosed, independently *patched* vulnerability, and show
that one unmodified Crypton test script produces the correct verdict on
both sides — `FAIL` against the known-vulnerable version, `PASS` against
the patched version. v0.1 ships **two** cases, deliberately chosen to be
independent of each other — different vendor, different language, and a
different test-catalog property family each:

- **VB-001** — official MCP Python SDK / CVE-2026-52869 (Principal /
  Session Binding), detailed above.
- **VB-002** — `@bytebase/dbhub`'s SQLite connector / CVE-2026-61788
  (Authorization Boundary & Complete Mediation), detailed above.

A third, authorization-boundary case on a different target
(`mcp-atlassian`) was investigated and deliberately set aside rather than
rushed — see the benchmark's own README for why. This does not mean
Crypton discovered anything in either case — it means Crypton's tests for
two different properties can each tell a documented failure from its
documented fix, on two unrelated real systems, which is a real, checkable
claim no prior version of this pack could make.

**Why two, not one:** one case only answers "is this entirely
self-referential." Two independent cases, in different languages, against
different vendors, testing different properties, start to answer a
different, harder question: does the methodology *transfer*, or does it
only work on the one system it happened to be checked against.

## Crypton-produced evidence (internal, distinct from the above)

Everything above was found by someone else, about a system Crypton did
not build or touch. The following is what Crypton's own research has
actually produced, against Crypton's own reference implementation and
three real open-source agent frameworks — kept separate because it's a
different kind of claim (evidence Crypton's own methodology works, not
evidence the problem is real):

- **Delegation-boundary independence PASS across three frameworks**
  (OpenAI Agents SDK, Microsoft Agent Framework, LangGraph) — H2,
  `docs/methodology/04-framework-adaptation-guide.md`.
- **A real, internally-confirmed application-level finding** (checkpoint
  state mutation between approval and resume in LangGraph, source-verified
  against the installed package) — FL-02,
  `assessment-kit/v1.0/07-finding-library.md`.
- **A retracted candidate finding**, kept and cited specifically because
  the retraction itself is evidence of process discipline — see
  `07-why-trust-crypton.md`.
- **Principal Continuity**, one independent external case (Meta's Muse
  assistant, reported by security researcher Patrick Wardle) —
  deliberately *not* claimed as strong corroboration; see
  `labs/exp-007-principal-continuity/EVIDENCE.md` for the full, hedged
  account of why a second search found no qualifying second case.

**Rule this document follows throughout:** an external case shows the
*failure class* is real. It is never cited as if it were a finding about
any Crypton customer, and it is never cited as proof that Crypton's own
methodology is correct — that claim rests only on the internal evidence
above and, ultimately, on delivered engagements.
