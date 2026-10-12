"""
Command-line entry point: register an agent, or run one agent task
end-to-end using an already-registered agent.

Usage:
    # register once (prints the canonical agent_id you'll need below)
    python -m agentlab.cli --register --name support-agent --version 0.1.0 --dev-user user-alice

    # grant it explicit, finite-lived authority (prints the delegation_id you'll need below)
    python -m agentlab.cli --grant-delegation --agent-id agt_... --tools get_customer,get_order \
        --customer-ids cust_1 --dev-user user-alice

    # run using that agent_id + delegation_id
    python -m agentlab.cli --agent-id agt_... --delegation-id dlg_... --task "..." --dev-user user-alice --fake

    # local-dev shortcut: create-or-reuse your own "support-agent" + a fresh delegation, run in one step
    python -m agentlab.cli --bootstrap-support-agent --bootstrap-delegation --task "..." --dev-user user-alice --fake

See labs/agent-lab/README.md for the full walkthrough.
"""

from __future__ import annotations

import argparse
import asyncio
import json
import sys
from pathlib import Path
from typing import Any

from dotenv import load_dotenv

from . import agents, chain, delegation, signing_keys
from .agent import AgentRunner
from .agents import AgentAccessError, AgentNotActiveError, AgentValidationError, DuplicateAgentError
from .db import connect, seed
from .delegation import DelegationAccessError, DelegationValidationError
from .identity import AgentInstance, HumanIdentity, IdentityError, generate_dev_keypair, issue_dev_token, verify_human_identity
from .model_provider import AnthropicModelProvider, FakeModelProvider, ModelResponse, ToolCallRequest
from .signing_keys import SigningKeyAccessError

REGISTRY_ERRORS = (AgentAccessError, AgentNotActiveError, AgentValidationError, DuplicateAgentError)
DELEGATION_ERRORS = (DelegationAccessError, DelegationValidationError)
SIGNING_KEY_ERRORS = (SigningKeyAccessError,)

# The seeded demo data's ids (see db.seed) — the default resource scope
# for --bootstrap-delegation, so the --fake walkthrough works out of the
# box without the caller having to already know them.
DEFAULT_DEMO_RESOURCES = {"customer_ids": ["cust_1", "cust_2"], "order_ids": ["ord_1", "ord_2"]}
DEFAULT_DELEGATION_TOOLS = ("get_customer", "get_order", "add_case_note")


def build_human_identity(args: argparse.Namespace) -> HumanIdentity:
    if args.token:
        # A real crypton-id token — verified against JWT_PUBLIC_KEY_PEM
        # (env), never a key this process could also sign with.
        return verify_human_identity(args.token)
    if args.dev_user:
        # Local-dev shortcut: generates a fresh, ephemeral, lab-only
        # keypair, mints a token with its private half, and immediately
        # verifies with its public half — a self-contained round trip that
        # never touches crypton-id's real key material (see
        # identity.generate_dev_keypair). Never a substitute for real login.
        private_key, public_key = generate_dev_keypair()
        token = issue_dev_token(args.dev_user, args.dev_user, "dev-cred", private_key=private_key)
        return verify_human_identity(token, public_key=public_key)
    raise SystemExit("Provide --token <crypton-id JWT>, or --dev-user <id> for local dev")


def build_model_provider(args: argparse.Namespace):
    if args.fake:
        # A tiny canned script so --fake produces a real end-to-end run
        # (tool call -> validation -> authz -> execution -> final answer)
        # without needing an API key.
        return FakeModelProvider(
            script=[
                ModelResponse(
                    tool_call=ToolCallRequest(id="fake-1", name="get_customer", arguments={"customer_id": "cust_1"})
                ),
                ModelResponse(text="Alice Anders is a gold-tier customer (alice@example.com)."),
            ]
        )
    return AnthropicModelProvider()


async def register_async(args: argparse.Namespace) -> int:
    human = build_human_identity(args)
    conn = connect()
    seed(conn)
    agent = agents.register_agent(conn, human, name=args.name, version=args.version)
    print(
        json.dumps(
            {
                "human_user_id": human.user_id,
                "agent_id": agent.agent_id,
                "name": agent.name,
                "version": agent.version,
                "status": agent.status.value,
            },
            indent=2,
        )
    )
    return 0


async def grant_delegation_async(args: argparse.Namespace) -> int:
    human = build_human_identity(args)
    conn = connect()
    seed(conn)
    if not args.agent_id:
        raise SystemExit("--grant-delegation requires --agent-id <canonical id>")
    tools = [t.strip() for t in args.tools.split(",")] if args.tools else list(DEFAULT_DELEGATION_TOOLS)
    resource_constraints = {}
    if args.customer_ids:
        resource_constraints["customer_ids"] = [c.strip() for c in args.customer_ids.split(",")]
    if args.order_ids:
        resource_constraints["order_ids"] = [o.strip() for o in args.order_ids.split(",")]

    grant = delegation.grant_delegation(
        conn,
        human,
        agent_id=args.agent_id,
        allowed_tools=tools,
        resource_constraints=resource_constraints,
        task_id=args.task_id,
        ttl_seconds=args.ttl_seconds,
    )
    print(
        json.dumps(
            {
                "delegation_id": grant.delegation_id,
                "principal_user_id": grant.principal_user_id,
                "agent_id": grant.agent_id,
                "task_id": grant.task_id,
                "allowed_tools": sorted(grant.allowed_tools),
                "resource_constraints": {k: sorted(v) for k, v in grant.resource_constraints.items()},
                "expires_at": grant.expires_at,
                "status": grant.status.value,
            },
            indent=2,
        )
    )
    return 0


async def run_async(args: argparse.Namespace) -> int:
    human = build_human_identity(args)
    conn = connect()
    seed(conn)

    # An agent_id must be explicit and pre-registered — normal execution
    # never silently creates one. --bootstrap-support-agent is the one,
    # clearly-marked local-dev exception (see agents.bootstrap_support_agent).
    if args.bootstrap_support_agent:
        agent_id = agents.bootstrap_support_agent(conn, human).agent_id
    elif args.agent_id:
        agent_id = args.agent_id
    else:
        raise SystemExit("Provide --agent-id <canonical id> (see --register), or --bootstrap-support-agent for local dev")

    # v0.4: a delegation_id is just as explicit and required as agent_id —
    # this lab never falls back to an implicit or ambient grant of
    # authority. --bootstrap-delegation is the one, clearly-marked
    # local-dev exception (see delegation.bootstrap_delegation).
    if args.bootstrap_delegation:
        delegation_id = delegation.bootstrap_delegation(
            conn, human, agent_id, allowed_tools=DEFAULT_DELEGATION_TOOLS, resource_constraints=DEFAULT_DEMO_RESOURCES
        ).delegation_id
    elif args.delegation_id:
        delegation_id = args.delegation_id
    else:
        raise SystemExit(
            "Provide --delegation-id <canonical id> (see --grant-delegation), or --bootstrap-delegation for local dev"
        )

    model = build_model_provider(args)
    runner = AgentRunner(agent_id=agent_id, delegation_id=delegation_id, model=model, db_conn=conn, max_steps=args.max_steps)
    # Session/Task/AgentInstance are created automatically for a normal
    # run (see agent.py); --session-id lets a second CLI invocation
    # resume the same session instead of starting a new one.
    result = await runner.run(human=human, goal=args.task, session_id=args.session_id)

    print(
        json.dumps(
            {
                "human_user_id": human.user_id,
                "agent_id": agent_id,
                "delegation_id": delegation_id,
                "agent_name": result.agent_name,
                "agent_version": result.agent_version,
                "session_id": result.session_id,
                "task_id": result.task_id,
                "task_status": result.task_status,
                "instance_id": result.instance_id,
                "final_text": result.final_text,
                "steps_used": result.steps_used,
                "stopped_reason": result.stopped_reason,
                "request_id": result.request_id,
                "trace_file": f"data/traces/{result.request_id}.jsonl",
            },
            indent=2,
        )
    )
    return 0 if result.stopped_reason == "final_response" else 1


async def create_signing_key_async(args: argparse.Namespace) -> int:
    """v0.6: give a registered agent an Ed25519 signing identity — a
    separate, explicit step from --register, the same way --grant-delegation
    is separate from it (see signing_keys.py's module docstring). Prints
    the private key exactly once, in PEM form, to stdout — this process
    never persists it. Save it (e.g. redirect to a file) if you intend to
    use --issue-chained-delegation with it later; there is no way to
    recover it afterward, only to revoke the key and create a new one."""
    human = build_human_identity(args)
    conn = connect()
    seed(conn)
    if not args.agent_id:
        raise SystemExit("--create-signing-key requires --agent-id <canonical id>")
    key, private_key_pem = signing_keys.create_signing_key(conn, human, args.agent_id)
    print(
        json.dumps(
            {
                "key_id": key.key_id,
                "agent_id": key.agent_id,
                "algorithm": key.algorithm,
                "public_key_pem": key.public_key_pem,
                "private_key_pem": private_key_pem,
                "warning": "private_key_pem is shown once and never stored — save it now",
            },
            indent=2,
        )
    )
    return 0


async def issue_chained_delegation_async(args: argparse.Namespace) -> int:
    """v0.6: issue a non-root delegation — `--issuer-agent-id` (which must
    be the exact subject of `--parent-delegation-id`) delegates a narrowed
    subset of its own authority to `--subject-agent-id`. See
    delegation.grant_chained_delegation for the full validation this runs
    at issuance time."""
    human = build_human_identity(args)
    conn = connect()
    seed(conn)
    missing = [
        f"--{flag}"
        for flag, val in [
            ("issuer-agent-id", args.issuer_agent_id),
            ("issuer-signing-key-id", args.issuer_signing_key_id),
            ("issuer-private-key-file", args.issuer_private_key_file),
            ("parent-delegation-id", args.parent_delegation_id),
            ("subject-agent-id", args.subject_agent_id),
        ]
        if not val
    ]
    if missing:
        raise SystemExit(f"--issue-chained-delegation requires: {', '.join(missing)}")

    issuer_private_key_pem = Path(args.issuer_private_key_file).read_text(encoding="utf-8")
    tools = [t.strip() for t in args.tools.split(",")] if args.tools else list(DEFAULT_DELEGATION_TOOLS)
    resource_constraints = {}
    if args.customer_ids:
        resource_constraints["customer_ids"] = [c.strip() for c in args.customer_ids.split(",")]
    if args.order_ids:
        resource_constraints["order_ids"] = [o.strip() for o in args.order_ids.split(",")]

    grant = delegation.grant_chained_delegation(
        conn,
        human,
        issuer_agent_id=args.issuer_agent_id,
        issuer_private_key_pem=issuer_private_key_pem,
        issuer_signing_key_id=args.issuer_signing_key_id,
        parent_delegation_id=args.parent_delegation_id,
        subject_agent_id=args.subject_agent_id,
        allowed_tools=tools,
        resource_constraints=resource_constraints,
        ttl_seconds=args.ttl_seconds,
    )
    print(
        json.dumps(
            {
                "delegation_id": grant.delegation_id,
                "issuer_type": grant.issuer_type,
                "issuer_id": grant.issuer_id,
                "subject_agent_id": grant.agent_id,
                "parent_delegation_id": grant.parent_delegation_id,
                "parent_hash": grant.parent_hash,
                "allowed_tools": sorted(grant.allowed_tools),
                "resource_constraints": {k: sorted(v) for k, v in grant.resource_constraints.items()},
                "expires_at": grant.expires_at,
                "signing_key_id": grant.signing_key_id,
            },
            indent=2,
        )
    )
    return 0


def _chain_verify_output(result: "chain.ChainVerificationResult", *, requested_tool: str, requested_resource: str | None) -> str:
    """Render a `ChainVerificationResult` the same way §39 of the v0.6
    spec asks for — human-readable, and built entirely from the real
    verifier's output, never a second, presentation-layer copy of
    authorization logic."""
    lines: list[str] = []
    if result.authority_path:
        lines.append("ROOT PRINCIPAL")
        lines.append(f"Human {result.root_principal}")
        lines.append("")
        lines.append("ACTING PRINCIPAL")
        lines.append(f"Agent {result.acting_principal}")
        lines.append("")
        lines.append("DELEGATION DEPTH")
        lines.append(str(result.delegation_depth))
        lines.append("")
        lines.append("AUTHORITY PATH")
        for i, (kind, pid) in enumerate(result.authority_path):
            label = f"Human {pid}" if kind == "human" else f"Agent {pid}"
            # Plain ASCII "->" rather than "→": stdout on a default-codepage
            # Windows console (cp1252) can't encode U+2192 and raises
            # UnicodeEncodeError on print() — matches this codebase's own
            # existing "a -> b" convention in comments/docstrings anyway.
            lines.append(label if i == 0 else f"  -> {label}")
        lines.append("")
    lines.append("REQUESTED TOOL")
    lines.append(requested_tool)
    lines.append("")
    lines.append("REQUESTED RESOURCE")
    lines.append(requested_resource or "(none)")
    lines.append("")
    lines.append("DECISION")
    lines.append("ALLOW" if result.allowed else "DENY")
    if not result.allowed:
        lines.append("")
        lines.append("FAILED HOP")
        lines.append(result.failed_hop or "(leaf)")
        lines.append("")
        lines.append("REASON")
        lines.append(result.denial_reason or result.reason_code)
    return "\n".join(lines)


def _chain_result_to_dict(result: "chain.ChainVerificationResult") -> dict:
    """v0.6.1: a thin, direct serialization of a real ChainVerificationResult
    — no re-derivation, no reformatting of the underlying decision. Used by
    `--chain-verify --format json`, for embedding a chain-verification
    result as evidence in an assessment engagement (see
    docs/ASSESSMENT_METHODOLOGY.md's "controlled staging test" evidence
    category)."""
    return {
        "allowed": result.allowed,
        "reason_code": result.reason_code,
        "root_principal": result.root_principal,
        "acting_principal": result.acting_principal,
        "delegation_depth": result.delegation_depth,
        "authority_path": [f"{kind}:{pid}" for kind, pid in result.authority_path],
        "effective_tools": sorted(result.effective_tools),
        "effective_resources": {k: sorted(v) for k, v in result.effective_resources.items()},
        "effective_expiry": result.effective_expiry,
        "chain_hash": result.chain_hash,
        "failed_hop": result.failed_hop,
        "denial_reason": result.denial_reason,
        "decision_id": result.decision.decision_id,
    }


async def chain_verify_async(args: argparse.Namespace) -> int:
    """v0.6 developer-facing chain inspection (§39). Calls the real
    verifier (chain.verify_delegation_chain) — this function only formats
    its output; it does not re-implement or shortcut any check."""
    human = build_human_identity(args)
    conn = connect()
    seed(conn)
    leaf_delegation_id = args.chain_verify
    leaf = delegation.get_delegation_unchecked(conn, leaf_delegation_id)
    if leaf is None:
        raise SystemExit(f"no such delegation: {leaf_delegation_id}")
    if not args.requested_tool:
        raise SystemExit("--chain-verify requires --requested-tool")

    resource_type = args.requested_resource_type
    resource_id = args.requested_resource_id
    requested_resource = f"{resource_type}:{resource_id}" if resource_type else None

    # A standalone inspection has no real, in-flight AgentRunner
    # instance/task — construct the minimal AgentInstance the leaf-level
    # check needs to compare against (agent_id/human_user_id must match
    # what the delegation was actually issued for), the same convenience
    # constructor identity.AgentInstance.new() already documents itself as
    # existing for ("tests and tools that only care about instance_id").
    instance = AgentInstance.new()
    instance = AgentInstance(
        instance_id=instance.instance_id,
        started_at=instance.started_at,
        agent_id=leaf.agent_id,
        human_user_id=human.user_id,
        session_id="",
        task_id=leaf.task_id or "",
    )

    result = chain.verify_delegation_chain(
        conn,
        principal=human,
        agent_id=leaf.agent_id,
        instance=instance,
        task_id=leaf.task_id or "",
        delegation_id=leaf_delegation_id,
        tool_name=args.requested_tool,
        resource_type=resource_type,
        resource_id=resource_id,
    )
    if args.format == "json":
        print(json.dumps(_chain_result_to_dict(result), indent=2))
    else:
        print(_chain_verify_output(result, requested_tool=args.requested_tool, requested_resource=requested_resource))
    return 0 if result.allowed else 1


def _reconfigure_stdout_utf8_best_effort(stream: Any = None) -> None:
    """v0.6.1: `--render-assessment` prints Markdown that legitimately
    contains em-dashes and other non-ASCII punctuation (report prose, not
    code — unlike the `--chain-verify` text formatter, which sidesteps this
    entirely by never using "→"; see test_cli_chain_verify_output.py). A
    legacy Windows console codepage raises `UnicodeEncodeError` on
    `print()` for those characters — specifically the OEM codepages
    (cp437/cp850) `cmd.exe`'s console traditionally defaults to, which lack
    an em-dash glyph entirely (not to be confused with the cp1252 *ANSI*
    codepage referenced elsewhere in this file for "→", which is a
    different, narrower failure — cp1252 does encode an em-dash). Called
    only immediately before that one print, not globally in `main()`, so
    every other command's stdout (register/run/grant-delegation/chain-
    verify) is left untouched — this is the only path that actually needs
    it. Best-effort and non-fatal: some stream types don't expose
    `.reconfigure` at all (`AttributeError`) or reject it in their current
    state (`ValueError`) — in either case we fall back to the stream's
    existing encoding rather than crash a command whose whole point is to
    hand someone a readable report; writing to a file via `--out` is
    unaffected either way, since that path always uses an explicit
    `encoding="utf-8"` file write, never this function."""
    target = stream if stream is not None else sys.stdout
    try:
        target.reconfigure(encoding="utf-8", errors="replace")
    except (AttributeError, ValueError):
        pass


async def render_assessment_async(args: argparse.Namespace) -> int:
    """v0.6.1: render an existing, already-built assessment engagement
    (a directory of scope/workflow/evidence/findings files — see
    docs/ASSESSMENT_METHODOLOGY.md and assessment/engagement.py) to a
    Markdown report. This renders data that already exists; it does not
    assess anything itself, and it does not accept an arbitrary customer
    system as input — only a pre-built Engagement directory."""
    from .assessment.engagement import Engagement, EngagementLoadError
    from .assessment.report import render_report

    try:
        engagement = Engagement.load(args.render_assessment)
    except EngagementLoadError as e:
        raise SystemExit(f"could not load engagement: {e}")

    report = render_report(engagement)
    if args.out:
        Path(args.out).write_text(report, encoding="utf-8")
        print(f"Wrote report to {args.out}")
    else:
        _reconfigure_stdout_utf8_best_effort()
        print(report)
    return 0


def main() -> None:
    load_dotenv()
    parser = argparse.ArgumentParser(description="Register or run a Crypton Agent Lab agent.")
    parser.add_argument("--register", action="store_true", help="register a new agent instead of running a task")
    parser.add_argument(
        "--grant-delegation", action="store_true", help="grant a delegation instead of registering or running"
    )
    parser.add_argument("--name", help="agent name (required with --register)")
    parser.add_argument("--version", default="0.1.0", help="agent version (used with --register; default 0.1.0)")
    parser.add_argument("--agent-id", help="canonical registered agent_id to run as (see --register's output)")
    parser.add_argument(
        "--bootstrap-support-agent",
        action="store_true",
        help="local-dev only: create-or-reuse your own 'support-agent' v0.1.0, then run with it",
    )
    parser.add_argument("--delegation-id", help="canonical delegation_id to run under (see --grant-delegation's output)")
    parser.add_argument(
        "--bootstrap-delegation",
        action="store_true",
        help="local-dev only: grant a fresh delegation scoped to the seeded demo data, then run with it",
    )
    parser.add_argument("--tools", help="comma-separated tool names to grant (used with --grant-delegation)")
    parser.add_argument("--customer-ids", help="comma-separated customer_ids to grant (used with --grant-delegation)")
    parser.add_argument("--order-ids", help="comma-separated order_ids to grant (used with --grant-delegation)")
    parser.add_argument("--task-id", help="task-bind the delegation to this exact task_id (used with --grant-delegation)")
    parser.add_argument("--ttl-seconds", type=int, default=3600, help="delegation lifetime (used with --grant-delegation)")
    # v0.6: signing keys + chained delegation + chain verification.
    parser.add_argument(
        "--create-signing-key",
        action="store_true",
        help="v0.6: mint an Ed25519 signing identity for --agent-id (required before that agent can issue a chained delegation)",
    )
    parser.add_argument(
        "--issue-chained-delegation",
        action="store_true",
        help="v0.6: issue a non-root delegation from --issuer-agent-id (must be --parent-delegation-id's subject) to --subject-agent-id",
    )
    parser.add_argument("--issuer-agent-id", help="v0.6: the agent issuing a chained delegation (see --issue-chained-delegation)")
    parser.add_argument("--issuer-signing-key-id", help="v0.6: the issuer's signing key id (see --create-signing-key's output)")
    parser.add_argument(
        "--issuer-private-key-file",
        help="v0.6: path to the issuer's private key PEM (see --create-signing-key's output; never pass a private key inline)",
    )
    parser.add_argument("--parent-delegation-id", help="v0.6: the delegation this chained grant narrows from")
    parser.add_argument("--subject-agent-id", help="v0.6: the agent the new chained delegation grants authority to")
    parser.add_argument(
        "--chain-verify", metavar="LEAF_DELEGATION_ID", help="v0.6: verify + print the full delegation chain for a leaf delegation_id"
    )
    parser.add_argument("--requested-tool", help="v0.6: the tool to check (used with --chain-verify)")
    parser.add_argument("--requested-resource-type", help="v0.6: e.g. customer_ids (used with --chain-verify)")
    parser.add_argument("--requested-resource-id", help="v0.6: e.g. cust_1 (used with --chain-verify)")
    parser.add_argument(
        "--format", choices=["text", "json"], default="text", help="v0.6.1: output format for --chain-verify (default: text)"
    )
    parser.add_argument(
        "--render-assessment",
        metavar="ENGAGEMENT_DIR",
        help="v0.6.1: render an existing assessment engagement directory to a Markdown report (see docs/ASSESSMENT_METHODOLOGY.md)",
    )
    parser.add_argument("--out", help="v0.6.1: write --render-assessment's report to this file instead of stdout")
    parser.add_argument("--task", help="the task to hand the agent (required unless --register/--grant-delegation)")
    parser.add_argument("--token", help="crypton-id JWT (same as Authorization: Bearer <token>)")
    parser.add_argument("--dev-user", help="local-dev shortcut: user id to mint a dev token for")
    parser.add_argument("--session-id", help="resume an existing session instead of creating a new one")
    parser.add_argument("--max-steps", type=int, default=6)
    parser.add_argument("--fake", action="store_true", help="use FakeModelProvider, no LLM API key required")
    args = parser.parse_args()

    v06_verb = args.create_signing_key or args.issue_chained_delegation or bool(args.chain_verify)
    v061_verb = bool(args.render_assessment)
    if args.register and not args.name:
        raise SystemExit("--register requires --name")
    if args.grant_delegation and not args.agent_id:
        raise SystemExit("--grant-delegation requires --agent-id")
    if args.create_signing_key and not args.agent_id:
        raise SystemExit("--create-signing-key requires --agent-id")
    if not args.register and not args.grant_delegation and not v06_verb and not v061_verb and not args.task:
        raise SystemExit(
            "--task is required for a normal run (or pass --register/--grant-delegation/--create-signing-key/"
            "--issue-chained-delegation/--chain-verify/--render-assessment to do one of those instead)"
        )

    try:
        if args.register:
            sys.exit(asyncio.run(register_async(args)))
        elif args.grant_delegation:
            sys.exit(asyncio.run(grant_delegation_async(args)))
        elif args.create_signing_key:
            sys.exit(asyncio.run(create_signing_key_async(args)))
        elif args.issue_chained_delegation:
            sys.exit(asyncio.run(issue_chained_delegation_async(args)))
        elif args.chain_verify:
            sys.exit(asyncio.run(chain_verify_async(args)))
        elif args.render_assessment:
            sys.exit(asyncio.run(render_assessment_async(args)))
        else:
            sys.exit(asyncio.run(run_async(args)))
    except IdentityError as e:
        print(f"identity error: {e}", file=sys.stderr)
        sys.exit(2)
    except REGISTRY_ERRORS as e:
        print(f"agent registry error: {e}", file=sys.stderr)
        sys.exit(3)
    except DELEGATION_ERRORS as e:
        print(f"delegation error: {e}", file=sys.stderr)
        sys.exit(4)
    except SIGNING_KEY_ERRORS as e:
        print(f"signing key error: {e}", file=sys.stderr)
        sys.exit(5)


if __name__ == "__main__":
    main()
