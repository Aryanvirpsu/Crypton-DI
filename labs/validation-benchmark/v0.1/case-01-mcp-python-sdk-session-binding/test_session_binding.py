"""
Crypton Validation Benchmark v0.1 -- Case 01
Principal / Session Binding vs. the official MCP Python SDK

Independently written by Crypton against the installed `mcp` package's
public API (StreamableHTTPSessionManager, AuthenticatedUser, AccessToken,
Server). Does not import or reuse the SDK's own test suite -- this is a
separately authored probe, run unmodified against two different installed
versions of the same dependency, so any difference in outcome is
attributable to the dependency version, not to the test.

Maps to Crypton test-catalog Family 1 (Principal / Session Binding,
`pilot-pack/v0.1/03-test-catalog.md`) and the external case documented in
`pilot-pack/v0.1/04-evidence-map.md`: CVE-2026-52869 / GHSA-jpw9-pfvf-9f58.
Vulnerable: mcp < 1.27.2. Fixed: mcp >= 1.27.2.

What this script does, deterministically, with no network and no real
credentials:
  1. Starts an in-process StreamableHTTPSessionManager (no HTTP server, no
     socket -- `handle_request` is an ordinary ASGI callable, driven
     directly with a hand-built scope/receive/send, exactly the way
     Starlette itself would drive it).
  2. Opens one session as principal A.
  3. Sends a second request presenting the *same* session ID but a
     *different*, independently-authenticated principal (B).
  4. Records whether that request was served (VULNERABLE -- session
     content/identifier alone was sufficient) or rejected as an unknown
     session (SECURE -- the manager verified the requesting principal).
  5. Runs a negative control: the original principal (A) re-requesting
     its own session must NOT be rejected, on either version -- otherwise
     the "SECURE" result above would just mean the manager rejects
     everything, not that it's actually checking identity.

Run with the SDK version under test already installed in the active
interpreter:
    python test_session_binding.py [--json]

Exit code 0 on a clean, determinate result either way (VULNERABLE or
SECURE) with the negative control intact; exit code 1 if the negative
control itself fails (meaning this probe cannot draw a conclusion on the
installed version, and needs to be re-checked before its result is
trusted).
"""

from __future__ import annotations

import json
import sys
from dataclasses import dataclass, field

import anyio

from mcp.server.auth.middleware.bearer_auth import AuthenticatedUser
from mcp.server.auth.provider import AccessToken
from mcp.server.lowlevel import Server
from mcp.server.streamable_http import MCP_SESSION_ID_HEADER
from mcp.server.streamable_http_manager import StreamableHTTPSessionManager

try:
    from importlib.metadata import version as _pkg_version

    INSTALLED_MCP_VERSION = _pkg_version("mcp")
except Exception:  # pragma: no cover
    INSTALLED_MCP_VERSION = "unknown"


def principal(client_id: str) -> AuthenticatedUser:
    """The scope['user'] value Starlette's AuthenticationMiddleware would set
    for a bearer-token-authenticated caller with this client_id. Only fields
    present in every mcp version back to the vulnerable release are used
    (token, client_id, scopes) -- deliberately avoiding subject/claims, which
    the vulnerable AccessToken does not define, so this exact script runs
    unmodified against both versions under test.
    """
    return AuthenticatedUser(AccessToken(token=f"tok-{client_id}", client_id=client_id, scopes=[]))


def build_scope(*, session_id: str | None, user: AuthenticatedUser | None) -> dict:
    headers = [
        (b"content-type", b"application/json"),
        (b"accept", b"application/json, text/event-stream"),
    ]
    if session_id is not None:
        headers.append((MCP_SESSION_ID_HEADER.lower().encode(), session_id.encode()))
    scope: dict = {"type": "http", "method": "POST", "path": "/mcp", "headers": headers}
    if user is not None:
        scope["user"] = user
    return scope


async def drive_request(manager: StreamableHTTPSessionManager, scope: dict) -> tuple[int | None, dict]:
    """Send one ASGI request through the manager and capture the response
    status and headers, without a real socket or HTTP server."""
    sent: list[dict] = []

    async def receive() -> dict:
        return {"type": "http.request", "body": b"", "more_body": False}

    async def send(message: dict) -> None:
        sent.append(message)

    await manager.handle_request(scope, receive, send)
    start = next((m for m in sent if m["type"] == "http.response.start"), None)
    if start is None:
        return None, {}
    return start["status"], dict(start.get("headers", []))


async def open_session(manager: StreamableHTTPSessionManager, user: AuthenticatedUser) -> str:
    # Deliberately send no JSON-RPC body -- this probe never needs a real MCP
    # handshake to succeed. Even an invalid/empty body still causes the
    # transport to assign and return a session id before it gets far enough
    # to reject the body's content, on every mcp version tested here.
    _status, headers = await drive_request(manager, build_scope(session_id=None, user=user))
    session_id = headers.get(MCP_SESSION_ID_HEADER.lower().encode())
    assert session_id is not None, "session-open response carried no session id header"
    return session_id.decode()


@dataclass
class ProbeResult:
    installed_mcp_version: str
    control_status: int | None = None
    cross_principal_status: int | None = None
    negative_control_ok: bool = False
    verdict: str = "INCONCLUSIVE"
    notes: list[str] = field(default_factory=list)


async def run_probe() -> ProbeResult:
    result = ProbeResult(installed_mcp_version=INSTALLED_MCP_VERSION)

    app = Server("crypton-validation-benchmark-case-01")
    manager = StreamableHTTPSessionManager(app=app)

    async with manager.run():
        principal_a = principal("crypton-principal-A")
        principal_b = principal("crypton-principal-B")

        session_id = await open_session(manager, principal_a)

        # Baseline / negative control: the ORIGINAL principal re-requesting
        # its own session. No real JSON-RPC body is sent, so this is NOT
        # expected to succeed end-to-end -- it's expected to reach the
        # transport and fail there (a parse/validation error), which is a
        # different thing from being rejected by the MANAGER as an unknown
        # session (404) before the transport is ever reached. That
        # distinction -- manager-level rejection vs. transport-level
        # handling -- is exactly what this probe measures.
        result.control_status, _ = await drive_request(
            manager, build_scope(session_id=session_id, user=principal_a)
        )
        result.negative_control_ok = result.control_status != 404
        if not result.negative_control_ok:
            result.verdict = "INCONCLUSIVE -- negative control failed, see notes"
            result.notes.append(
                f"principal A's own follow-up request was rejected as unknown (status={result.control_status}); "
                "the manager isn't distinguishing 'no session' from 'my own session' at all here, "
                "so the cross-principal comparison below cannot be trusted."
            )
            return result

        # The actual probe: a DIFFERENT, independently-authenticated
        # principal presents the SAME session id. Compare its status
        # against the baseline just established above, not against an
        # absolute code -- the transport's own generic error for a bad body
        # is allowed to be anything, as long as it's THE SAME for a
        # legitimate re-request and a mismatched-principal request only
        # when the manager fails to tell them apart.
        result.cross_principal_status, _ = await drive_request(
            manager, build_scope(session_id=session_id, user=principal_b)
        )

        if result.cross_principal_status == 404 and result.control_status != 404:
            result.verdict = "PASS -- SECURE (manager rejected the mismatched principal as an unknown session, before ever reaching the transport)"
        elif result.cross_principal_status == result.control_status:
            result.verdict = (
                "FAIL -- VULNERABLE (a different, independently-authenticated principal reached the transport "
                "exactly as the session's own creator did; the manager never checked who was asking)"
            )
        else:
            result.verdict = (
                f"INCONCLUSIVE -- cross-principal status ({result.cross_principal_status}) "
                f"neither matches the baseline ({result.control_status}) nor is a clean 404 rejection"
            )

    return result


def main() -> int:
    as_json = "--json" in sys.argv
    result = anyio.run(run_probe)

    if as_json:
        print(json.dumps(result.__dict__, indent=2))
    else:
        print("Crypton Validation Benchmark v0.1 -- Case 01: Principal / Session Binding")
        print(f"Installed mcp version: {result.installed_mcp_version}")
        print(f"Baseline status (creator re-requests own session): {result.control_status}")
        print(f"Negative control (baseline is not itself a 404): "
              f"{'ok' if result.negative_control_ok else 'FAILED -- see notes'}")
        print(f"Cross-principal request status: {result.cross_principal_status}")
        print(f"VERDICT: {result.verdict}")
        for note in result.notes:
            print(f"  note: {note}")

    if not result.negative_control_ok:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
