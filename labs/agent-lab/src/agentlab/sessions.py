"""
Session persistence — a continuing human interaction/context boundary
(state boundary 1 of 6; see LEARNING.md). A session may contain multiple
tasks; creating one does not run anything by itself.

Every read is ownership-enforced the same way tools.py enforces it on
customer/order rows: a session_id alone is never sufficient, it's only
ever resolved against the calling HumanIdentity.
"""

from __future__ import annotations

import sqlite3
import time
import uuid
from dataclasses import dataclass

from .identity import HumanIdentity


class SessionAccessError(Exception):
    """Raised when a session doesn't exist, or exists but isn't owned by
    the calling human. Both cases look identical to the caller — never
    reveal that another human's session exists."""


@dataclass(frozen=True)
class Session:
    session_id: str
    human_user_id: str
    created_at: float
    updated_at: float


def create_session(conn: sqlite3.Connection, human: HumanIdentity) -> Session:
    session_id = str(uuid.uuid4())
    now = time.time()
    conn.execute(
        "INSERT INTO sessions (session_id, human_user_id, created_at, updated_at) VALUES (?, ?, ?, ?)",
        (session_id, human.user_id, now, now),
    )
    conn.commit()
    return Session(session_id=session_id, human_user_id=human.user_id, created_at=now, updated_at=now)


def get_session(conn: sqlite3.Connection, human: HumanIdentity, session_id: str) -> Session:
    """Ownership-enforced load — the only way code in this package may
    read a session. Raises SessionAccessError unless `session_id` exists
    AND belongs to `human`."""
    row = conn.execute(
        "SELECT session_id, human_user_id, created_at, updated_at FROM sessions WHERE session_id = ?",
        (session_id,),
    ).fetchone()
    if row is None or row["human_user_id"] != human.user_id:
        raise SessionAccessError(f"session_not_found: {session_id}")
    return Session(**dict(row))


def touch_session(conn: sqlite3.Connection, session: Session) -> Session:
    """Bump `updated_at` — called whenever a task is created within the
    session, so `sessions.updated_at` reflects last activity, not just
    the last direct session operation."""
    now = time.time()
    conn.execute("UPDATE sessions SET updated_at = ? WHERE session_id = ?", (now, session.session_id))
    conn.commit()
    return Session(
        session_id=session.session_id,
        human_user_id=session.human_user_id,
        created_at=session.created_at,
        updated_at=now,
    )
