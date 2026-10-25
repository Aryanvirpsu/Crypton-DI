"""Control messages and the (deliberately unenforcing) channel that carries them.

The channel accepts anything from any local sender. Whether a message is
honoured is decided only by the policy engine.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from .identity import Session, canonical_bytes, sign


@dataclass(frozen=True)
class ControlMessage:
    controller_pid: str
    session_id: str
    task_id: str
    nonce: str
    action: str
    params: dict = field(default_factory=dict)
    signature: str = ""

    def signed_payload(self) -> bytes:
        return canonical_bytes(
            {
                "controller_pid": self.controller_pid,
                "session_id": self.session_id,
                "task_id": self.task_id,
                "nonce": self.nonce,
                "action": self.action,
                "params": self.params,
            }
        )


def build_signed(
    key: bytes, session: Session, controller_pid: str, action: str, params: dict
) -> ControlMessage:
    unsigned = ControlMessage(
        controller_pid, session.session_id, session.task_id, session.nonce, action, params
    )
    return ControlMessage(
        unsigned.controller_pid,
        unsigned.session_id,
        unsigned.task_id,
        unsigned.nonce,
        unsigned.action,
        unsigned.params,
        sign(key, unsigned.signed_payload()),
    )


def build_unsigned(
    session: Session, claimed_pid: str, action: str, params: dict
) -> ControlMessage:
    """B spoofs every header field (assumed observable) but has no key."""
    return ControlMessage(
        claimed_pid, session.session_id, session.task_id, session.nonce, action, params, ""
    )


def build_forged(
    forge_key: bytes, session: Session, claimed_pid: str, action: str, params: dict
) -> ControlMessage:
    """B signs with a key of its own choosing, not the one registered for the claimed pid."""
    return build_signed(forge_key, session, claimed_pid, action, params)
