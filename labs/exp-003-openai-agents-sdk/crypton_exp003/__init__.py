"""
crypton_exp003 — Crypton EXP-003A assessment library.

Target: openai-agents==0.22.2 (the `agents` package), assessed for
authorization/delegation behavior around function-tool approval,
Agent.as_tool() delegation, RunState serialization/resume, and
approval/guardrail ordering. See ../README.md and ../target.json.

This package is deliberately isolated from Crypton's production/shared
assessment code (labs/agent-lab/src/agentlab/assessment/) — nothing here
is imported by, or imports, that code. Promotion of any abstraction here
into shared Crypton code only happens after a scenario proves it's
actually reusable (see README "What Crypton may add").
"""
