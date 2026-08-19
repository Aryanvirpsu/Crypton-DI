"""
Command-line entry point: register an agent, or run one agent task
end-to-end using an already-registered agent.

Usage:
    # register once (prints the canonical agent_id you'll need below)
    python -m agentlab.cli --register --name support-agent --version 0.1.0 --dev-user user-alice

    # run using that agent_id
    python -m agentlab.cli --agent-id agt_... --task "..." --dev-user user-alice --fake

    # local-dev shortcut: create-or-reuse your own "support-agent" and run in one step
    python -m agentlab.cli --bootstrap-support-agent --task "..." --dev-user user-alice --fake

See labs/agent-lab/README.md for the full walkthrough.
"""

from __future__ import annotations

import argparse
import asyncio
import json
import sys

from dotenv import load_dotenv

from . import agents
from .agent import AgentRunner
from .agents import AgentAccessError, AgentNotActiveError, AgentValidationError, DuplicateAgentError
from .db import connect, seed
from .identity import HumanIdentity, IdentityError, issue_dev_token, verify_human_identity
from .model_provider import AnthropicModelProvider, FakeModelProvider, ModelResponse, ToolCallRequest

REGISTRY_ERRORS = (AgentAccessError, AgentNotActiveError, AgentValidationError, DuplicateAgentError)


def build_human_identity(args: argparse.Namespace) -> HumanIdentity:
    if args.token:
        return verify_human_identity(args.token)
    if args.dev_user:
        # Local-dev shortcut: mints + immediately verifies a token in the
        # exact shape crypton-id issues, so the same verification code
        # path runs regardless. Never a substitute for real login.
        token = issue_dev_token(args.dev_user, args.dev_user, "dev-cred")
        return verify_human_identity(token)
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

    model = build_model_provider(args)
    runner = AgentRunner(agent_id=agent_id, model=model, db_conn=conn, max_steps=args.max_steps)
    # Session/Task/AgentInstance are created automatically for a normal
    # run (see agent.py); --session-id lets a second CLI invocation
    # resume the same session instead of starting a new one.
    result = await runner.run(human=human, goal=args.task, session_id=args.session_id)

    print(
        json.dumps(
            {
                "human_user_id": human.user_id,
                "agent_id": agent_id,
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


def main() -> None:
    load_dotenv()
    parser = argparse.ArgumentParser(description="Register or run a Crypton Agent Lab agent.")
    parser.add_argument("--register", action="store_true", help="register a new agent instead of running a task")
    parser.add_argument("--name", help="agent name (required with --register)")
    parser.add_argument("--version", default="0.1.0", help="agent version (used with --register; default 0.1.0)")
    parser.add_argument("--agent-id", help="canonical registered agent_id to run as (see --register's output)")
    parser.add_argument(
        "--bootstrap-support-agent",
        action="store_true",
        help="local-dev only: create-or-reuse your own 'support-agent' v0.1.0, then run with it",
    )
    parser.add_argument("--task", help="the task to hand the agent (required unless --register)")
    parser.add_argument("--token", help="crypton-id JWT (same as Authorization: Bearer <token>)")
    parser.add_argument("--dev-user", help="local-dev shortcut: user id to mint a dev token for")
    parser.add_argument("--session-id", help="resume an existing session instead of creating a new one")
    parser.add_argument("--max-steps", type=int, default=6)
    parser.add_argument("--fake", action="store_true", help="use FakeModelProvider, no LLM API key required")
    args = parser.parse_args()

    if args.register and not args.name:
        raise SystemExit("--register requires --name")
    if not args.register and not args.task:
        raise SystemExit("--task is required for a normal run (or pass --register to register an agent instead)")

    try:
        if args.register:
            sys.exit(asyncio.run(register_async(args)))
        else:
            sys.exit(asyncio.run(run_async(args)))
    except IdentityError as e:
        print(f"identity error: {e}", file=sys.stderr)
        sys.exit(2)
    except REGISTRY_ERRORS as e:
        print(f"agent registry error: {e}", file=sys.stderr)
        sys.exit(3)


if __name__ == "__main__":
    main()
