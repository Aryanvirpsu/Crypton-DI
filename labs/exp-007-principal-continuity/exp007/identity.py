"""Principals, bound sessions, key directory, and HMAC-SHA256 sign/verify.

HMAC-SHA256 stands in for a real signature scheme. Keys are derived from
fixed labels so every run is reproducible; this is a lab convenience, not
key management guidance.
"""

from __future__ import annotations

import hashlib
import hmac
import json
from dataclasses import dataclass


@dataclass(frozen=True)
class Principal:
    pid: str
    label: str


@dataclass(frozen=True)
class Session:
    """The session U's grant is bound to."""

    session_id: str
    principal_pid: str
    task_id: str
    nonce: str


def derive_key(label: str) -> bytes:
    return hashlib.sha256(b"exp007-lab-key:" + label.encode()).digest()


class KeyDirectory:
    """pid -> registered verification key. B is never registered."""

    def __init__(self) -> None:
        self._keys: dict[str, bytes] = {}

    def register(self, pid: str, key: bytes) -> None:
        self._keys[pid] = key

    def key_for(self, pid: str) -> bytes | None:
        return self._keys.get(pid)


def canonical_bytes(fields: dict) -> bytes:
    return json.dumps(fields, sort_keys=True, separators=(",", ":")).encode()


def sign(key: bytes, payload: bytes) -> str:
    return hmac.new(key, payload, hashlib.sha256).hexdigest()


def verify(key: bytes, payload: bytes, signature: str) -> bool:
    return hmac.compare_digest(sign(key, payload), signature)
