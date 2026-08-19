"""
Model provider abstraction (requirement 6).

Keeps the agent loop independent of any specific LLM vendor: the loop in
agent.py only ever calls `ModelProvider.complete(...)`. One real provider
(Anthropic) is implemented; FakeModelProvider drives the deterministic
unit tests without any network access. Swapping providers later means
writing one more class that satisfies this interface — nothing in
agent.py changes.

Configuration comes from environment variables only (ANTHROPIC_API_KEY,
AGENT_LAB_MODEL). Never commit a real key — see .env.example. The model
never receives environment variables, secrets, or anything beyond the
system prompt, conversation, and tool schemas built in agent.py.
"""

from __future__ import annotations

import json
import os
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Any


@dataclass
class ToolCallRequest:
    """A structured tool call the model asked for."""

    id: str
    name: str
    arguments: dict[str, Any]


@dataclass
class ModelResponse:
    """What the model returned for one turn: either a final text answer
    or a tool call — never both. agent.py branches on which one it got,
    explicitly, rather than guessing from free text."""

    text: str | None = None
    tool_call: ToolCallRequest | None = None
    raw: Any = None


class ModelProvider(ABC):
    """Implement this to add a new provider. The agent loop depends only
    on this interface, never on a vendor SDK directly."""

    @abstractmethod
    async def complete(
        self, *, system: str, messages: list[dict], tools: list[dict]
    ) -> ModelResponse: ...


class AnthropicModelProvider(ModelProvider):
    """The one supported real provider for v0.1."""

    def __init__(self, *, api_key: str | None = None, model: str | None = None):
        api_key = api_key or os.environ.get("ANTHROPIC_API_KEY")
        if not api_key:
            raise RuntimeError("ANTHROPIC_API_KEY is not set")
        self.model = model or os.environ.get("AGENT_LAB_MODEL", "claude-sonnet-5")
        # Imported lazily so the rest of the package (and the fake-model
        # tests) work without the `anthropic` package installed.
        import anthropic

        self._client = anthropic.AsyncAnthropic(api_key=api_key)

    async def complete(
        self, *, system: str, messages: list[dict], tools: list[dict]
    ) -> ModelResponse:
        resp = await self._client.messages.create(
            model=self.model,
            max_tokens=1024,
            system=system,
            messages=_to_anthropic_messages(messages),
            tools=tools,
        )
        tool_use = next((b for b in resp.content if b.type == "tool_use"), None)
        if tool_use is not None:
            return ModelResponse(
                tool_call=ToolCallRequest(id=tool_use.id, name=tool_use.name, arguments=tool_use.input),
                raw=resp,
            )
        text_blocks = [b.text for b in resp.content if b.type == "text"]
        return ModelResponse(text="\n".join(text_blocks), raw=resp)


def _to_anthropic_messages(messages: list[dict]) -> list[dict]:
    """Translate agent.py's minimal, provider-agnostic message log into
    the Anthropic Messages API's content-block format. Kept private to
    this provider — nothing else in the codebase needs to know Anthropic's
    wire format."""
    out: list[dict] = []
    for m in messages:
        role = m["role"]
        if role == "user" and "tool_call" not in m:
            out.append({"role": "user", "content": m["content"]})
        elif role == "assistant" and m.get("tool_call") is not None:
            call = m["tool_call"]
            out.append(
                {
                    "role": "assistant",
                    "content": [{"type": "tool_use", "id": call.id, "name": call.name, "input": call.arguments}],
                }
            )
        elif role == "tool_result":
            out.append(
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "tool_result",
                            "tool_use_id": m["tool_use_id"],
                            "content": json.dumps(m["content"]),
                            "is_error": m.get("is_error", False),
                        }
                    ],
                }
            )
        else:
            out.append({"role": role, "content": m.get("content")})
    return out


@dataclass
class FakeModelProvider(ModelProvider):
    """Deterministic provider for tests: replays a fixed script of
    responses in order, regardless of what it's sent. Never makes a
    network call — this is what makes the test suite fast and
    deterministic (requirement 8: "prefer deterministic/fake model
    responses for unit tests")."""

    script: list[ModelResponse] = field(default_factory=list)
    calls: list[dict] = field(default_factory=list)

    async def complete(
        self, *, system: str, messages: list[dict], tools: list[dict]
    ) -> ModelResponse:
        self.calls.append({"system": system, "messages": list(messages), "tools": tools})
        if not self.script:
            return ModelResponse(text="(fake model ran out of scripted responses)")
        return self.script.pop(0)
