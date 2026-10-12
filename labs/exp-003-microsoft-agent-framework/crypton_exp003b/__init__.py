"""
crypton_exp003b — Crypton EXP-003B assessment library.

Target: agent-framework==1.18.0 (import name `agent_framework`), assessed
for authorization/delegation behavior around function-tool approval,
Agent.as_tool()/handoff delegation, WorkflowCheckpoint save/restore, and
standing (ToolApprovalRule) authorization. See ../README.md and
../target.json.

Deliberately isolated from Crypton's production/shared assessment code
and from labs/exp-003-openai-agents-sdk/ (EXP-003A) - no imports cross
those boundaries. Per the EXP-003 operating rule, EXP-003A's
implementation is not imported here; the minimum needed is duplicated
and adapted locally, and compared at the end (see README's schema
comparison).
"""
