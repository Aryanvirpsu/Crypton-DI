"""Mock downstream resource. Enforces nothing; only the agent/policy path gates it."""

from __future__ import annotations


class MockResource:
    def __init__(self) -> None:
        self.sent: list[dict] = []

    def send_message(self, params: dict) -> str:
        self.sent.append(dict(params))
        return f"SENT to={params.get('to')} body={params.get('body')!r}"
