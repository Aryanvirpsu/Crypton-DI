"""
crypton_exp003b/authority.py — small SDK-usage helpers shared by scenarios.

Deliberately thin, mirroring crypton_exp003.tools from EXP-003A in spirit
(see README's schema comparison). Every scenario still drives its own
Workflow.run()/checkpoint calls explicitly - the point of this experiment
is to observe those calls directly.
"""

from __future__ import annotations

from typing import Any

import agent_framework as af


def build_single_agent_workflow(
    *,
    agent: af.Agent,
    workflow_name: str,
    checkpoint_storage: af.CheckpointStorage,
) -> af.Workflow:
    """The clean, well-understood pattern confirmed by direct testing:
    wrapping ONE agent as a Workflow's start_executor correctly surfaces
    a pending function_approval_request as a `request_info` WorkflowEvent
    and correctly checkpoints it - unlike the HandoffBuilder multi-agent
    path, where this was NOT confirmed to work cleanly (see README's H2
    notes and the "not fully resolved" observation)."""
    return af.WorkflowBuilder(start_executor=agent, name=workflow_name, checkpoint_storage=checkpoint_storage).build()


def get_pending_approval_requests(result: af.WorkflowRunResult) -> list[Any]:
    """Returns the WorkflowEvents of type 'request_info' whose `.data` is
    a function_approval_request Content - the pending tool approvals."""
    return [ev for ev in result.get_request_info_events() if getattr(ev.data, "type", None) == "function_approval_request"]


async def approve_request(
    workflow: af.Workflow,
    *,
    request_event: Any,
    approved: bool,
    checkpoint_id: str,
    checkpoint_storage: af.CheckpointStorage,
) -> af.WorkflowRunResult:
    """Submits one approval/rejection decision for a pending request and
    resumes the workflow from the given checkpoint."""
    response_content = request_event.data.to_function_approval_response(approved=approved)
    return await workflow.run(
        checkpoint_id=checkpoint_id,
        responses={request_event.request_id: response_content},
        checkpoint_storage=checkpoint_storage,
    )


async def latest_checkpoint_id(storage: af.CheckpointStorage, *, workflow_name: str) -> str:
    latest = await storage.get_latest(workflow_name=workflow_name)
    if latest is None:
        raise RuntimeError(f"no checkpoint found for workflow_name={workflow_name!r}")
    return latest.checkpoint_id
