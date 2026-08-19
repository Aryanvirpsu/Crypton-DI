"""
Structured schemas for tool arguments (requirement 7: schema-validate
every tool argument).

Every tool the agent can call has exactly one Pydantic model here. This
is the boundary between "the model emitted some JSON" and "Python code
receives typed, bounded, validated input" — see LEARNING.md section 8.
`extra="forbid"` means an unexpected field is a hard rejection, not a
silently-ignored surprise.
"""

from __future__ import annotations

from pydantic import BaseModel, ConfigDict, Field


class GetCustomerArgs(BaseModel):
    model_config = ConfigDict(extra="forbid")
    customer_id: str = Field(min_length=1, max_length=64)


class GetOrderArgs(BaseModel):
    model_config = ConfigDict(extra="forbid")
    order_id: str = Field(min_length=1, max_length=64)


class AddCaseNoteArgs(BaseModel):
    model_config = ConfigDict(extra="forbid")
    customer_id: str = Field(min_length=1, max_length=64)
    note: str = Field(min_length=1, max_length=2000)


# Single source of truth: tool name -> its argument schema. The agent
# loop (agent.py) uses this dict both to validate incoming tool calls and
# to build the JSON-schema block sent to the model — the model is never
# told about a tool that isn't also enforced here.
TOOL_ARG_MODELS: dict[str, type[BaseModel]] = {
    "get_customer": GetCustomerArgs,
    "get_order": GetOrderArgs,
    "add_case_note": AddCaseNoteArgs,
}

TOOL_DESCRIPTIONS: dict[str, str] = {
    "get_customer": "Look up a customer's profile by customer_id. Returns name, email, and tier.",
    "get_order": "Look up an order by order_id. Returns status, total, and items.",
    "add_case_note": "Append a support case note to a customer's record.",
}


def tool_specs_for_model() -> list[dict]:
    """Build the tool-use schema block sent to the model provider — the
    *only* thing the model is told about what tools exist. Derived
    directly from TOOL_ARG_MODELS so the schema the model sees can never
    drift from the schema that's actually enforced."""
    return [
        {
            "name": name,
            "description": TOOL_DESCRIPTIONS[name],
            "input_schema": model.model_json_schema(),
        }
        for name, model in TOOL_ARG_MODELS.items()
    ]
