"""
The three tools the agent may call (requirement 2).

Every function here:
  - receives a *validated* Pydantic args object, never raw model JSON
    (models.py / agent.py do validation before this is ever called)
  - receives the calling HumanIdentity and enforces per-user data
    isolation against it — a customer/order/note row is only visible to
    the human who owns it
  - returns a plain JSON-serializable dict, or raises ToolError
  - never touches the filesystem, network, or a shell, and never
    executes model-generated code (see LEARNING.md sections 9-10)
"""

from __future__ import annotations

import sqlite3
import time
from typing import Any

from .identity import AgentInstance, HumanIdentity
from .models import AddCaseNoteArgs, GetCustomerArgs, GetOrderArgs


class ToolError(Exception):
    """Domain-level tool failure (not found, not owned by this human,
    etc). Distinct from AuthorizationError (agent isn't allowed to call
    the tool at all) and ValidationError (arguments are malformed) — this
    is the tool doing its job and reporting it couldn't complete."""


def get_customer(
    conn: sqlite3.Connection,
    human: HumanIdentity,
    instance: AgentInstance,
    args: GetCustomerArgs,
) -> dict[str, Any]:
    row = conn.execute(
        "SELECT customer_id, owner_user_id, name, email, tier FROM customers WHERE customer_id = ?",
        (args.customer_id,),
    ).fetchone()
    if row is None or row["owner_user_id"] != human.user_id:
        # Identical error whether the row doesn't exist or belongs to
        # someone else — never reveal that another human's record exists.
        raise ToolError(f"customer_not_found: {args.customer_id}")
    return {
        "customer_id": row["customer_id"],
        "name": row["name"],
        "email": row["email"],
        "tier": row["tier"],
    }


def get_order(
    conn: sqlite3.Connection,
    human: HumanIdentity,
    instance: AgentInstance,
    args: GetOrderArgs,
) -> dict[str, Any]:
    row = conn.execute(
        "SELECT order_id, owner_user_id, customer_id, status, total_cents, items FROM orders WHERE order_id = ?",
        (args.order_id,),
    ).fetchone()
    if row is None or row["owner_user_id"] != human.user_id:
        raise ToolError(f"order_not_found: {args.order_id}")
    return {
        "order_id": row["order_id"],
        "customer_id": row["customer_id"],
        "status": row["status"],
        "total_cents": row["total_cents"],
        "items": row["items"],
    }


def add_case_note(
    conn: sqlite3.Connection,
    human: HumanIdentity,
    instance: AgentInstance,
    args: AddCaseNoteArgs,
) -> dict[str, Any]:
    owner = conn.execute(
        "SELECT owner_user_id FROM customers WHERE customer_id = ?",
        (args.customer_id,),
    ).fetchone()
    if owner is None or owner["owner_user_id"] != human.user_id:
        raise ToolError(f"customer_not_found: {args.customer_id}")
    conn.execute(
        "INSERT INTO case_notes (customer_id, owner_user_id, note, created_at, created_by_agent_instance) "
        "VALUES (?, ?, ?, ?, ?)",
        (args.customer_id, human.user_id, args.note, time.time(), instance.instance_id),
    )
    conn.commit()
    return {"customer_id": args.customer_id, "status": "note_added"}


# Single source of truth: tool name -> implementation. agent.py looks up
# by name here only *after* authz.check_authorized has passed — this dict
# itself is not an authorization boundary, just a dispatch table.
TOOL_FUNCTIONS = {
    "get_customer": get_customer,
    "get_order": get_order,
    "add_case_note": add_case_note,
}
