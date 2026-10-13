# Case 01 — Principal / Session Binding vs. the official MCP Python SDK

**Target:** `mcp` (PyPI), the official Model Context Protocol Python SDK
— `mcp.server.streamable_http_manager.StreamableHTTPSessionManager`.
**Advisory:** [CVE-2026-52869 / GHSA-jpw9-pfvf-9f58](https://github.com/advisories/GHSA-jpw9-pfvf-9f58).
**Vulnerable:** `mcp < 1.27.2` (tested at `1.27.1`). **Fixed:** `mcp >= 1.27.2`
(tested at `1.27.2`). **Crypton test-catalog family:**
Principal / Session Binding (`pilot-pack/v0.1/03-test-catalog.md`, Family 1).

## What the advisory says

Prior to 1.27.2, the SSE and stateful Streamable HTTP transports routed a
request to an existing session using only the `Mcp-Session-Id` header —
never checking whether the authenticated caller presenting that header
was the same principal who created the session. A different,
independently bearer-token-authenticated client that learned or guessed a
valid session ID could inject JSON-RPC messages into someone else's
session. 1.27.2 adds an `_session_owners` map keyed by session ID,
recording the `AuthorizationContext` (client ID, issuer, subject) that
created each session, and rejects a mismatched requester with the same
404 response as an unknown session — chosen deliberately so a session's
existence isn't itself leaked to a caller who doesn't own it.

## What Crypton's test does

`test_session_binding.py`, run unmodified against both versions:

1. Opens a session as principal A (an `AuthenticatedUser` built from a
   locally-constructed `AccessToken` — no real OAuth flow, no network).
2. Records the status code when principal A makes a second request for
   its own session (the **baseline** — whatever the transport does with
   this probe's deliberately empty request body, on this installed
   version).
3. Records the status code when principal B — a different, independently
   constructed, equally "authenticated" principal — requests the *same*
   session ID.
4. **Negative control:** if the baseline itself were `404`, the test
   would be meaningless (the manager would be rejecting everyone, proving
   nothing about identity checking) — the script checks this and exits
   with code `1`, refusing to report a verdict, if it happens. It did not
   happen on either version tested.
5. Compares B's status to the baseline: identical status → the manager
   never distinguished B from A (**FAIL**); a clean `404` where the
   baseline wasn't `404` → the manager rejected B before ever reaching
   the transport (**PASS**).

This works without ever completing a real MCP handshake — the point
being measured is *which principal is allowed to reach the transport at
all* for an existing session, not what a valid MCP conversation looks
like. Comparing against the baseline (rather than a fixed HTTP status)
means the test doesn't need to know or assume what the transport's
"generic invalid request" response looks like on any given version — it
only needs the two runs' *comparison* to be meaningful, which the
negative control checks directly.

## Actual results (this session)

```
$ .venv-vuln/Scripts/python test_session_binding.py
Installed mcp version: 1.27.1
Baseline status (creator re-requests own session): 400
Negative control (baseline is not itself a 404): ok
Cross-principal request status: 400
VERDICT: FAIL -- VULNERABLE (a different, independently-authenticated principal
reached the transport exactly as the session's own creator did; the manager
never checked who was asking)

$ .venv-fixed/Scripts/python test_session_binding.py
Rejecting request for session <id>: credential does not match the one that
created the session
Installed mcp version: 1.27.2
Baseline status (creator re-requests own session): 400
Negative control (baseline is not itself a 404): ok
Cross-principal request status: 404
VERDICT: PASS -- SECURE (manager rejected the mismatched principal as an
unknown session, before ever reaching the transport)
```

Full JSON evidence: [`evidence/vulnerable_1.27.1.json`](evidence/vulnerable_1.27.1.json),
[`evidence/fixed_1.27.2.json`](evidence/fixed_1.27.2.json).

Exit code was `0` on both runs (negative control held on both versions,
so both verdicts are trustworthy, not just "the test happened to run").

## Independence from the vendor's own fix

This script was written by reading the public advisory and the
`streamable_http_manager.py` source diff between the two published
releases, then independently authoring a probe against the package's
public API (`StreamableHTTPSessionManager`, `AuthenticatedUser`,
`AccessToken`, `Server`) — it does not import, adapt, or reuse the SDK's
own test suite. This matters for what the result is evidence *of*: a
vendor's own regression test passing against its own fix is expected and
proves little; an independently-authored probe, built from the public
description of the problem, reaching the same verdict is evidence the
*methodology* — not just the vendor's own test — can tell the two states
apart.

## Limitations

- This is retrospective — the vulnerability and its fix were already
  public before this case was built. It shows Crypton's test *recognizes*
  a known failure/fix pair, not that Crypton would have *found* it first.
- Only the client_id component of `AuthorizationContext` is exercised
  (the vulnerable version's `AccessToken` has no `subject`/`claims`
  fields at all, so this script deliberately never constructs them, to
  keep one script valid against both versions). The fixed version's
  additional subject/issuer distinctions are not separately probed here.
- No real network transport, OAuth flow, or live MCP client/server pair
  was used — `handle_request` was driven directly as an ASGI callable, a
  standard, realistic way to exercise this code (the same approach the
  SDK's own maintainers use in their test suite), but it is still an
  in-process test, not an end-to-end network reproduction.
