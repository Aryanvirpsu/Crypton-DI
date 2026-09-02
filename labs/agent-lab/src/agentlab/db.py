"""
Tiny isolated datastore for Agent Lab.

Deliberately NOT the real Crypton Postgres database — Agent Lab's tools
operate on their own seed data (a local SQLite file) so the lab can be
run and reset without touching crypton-id's real user data. Every row
carries an `owner_user_id`, the human identity that owns it, so tools can
enforce per-user data isolation (requirement: "separate users' data
remains isolated"). See tools.py for where that's enforced.
"""

from __future__ import annotations

import sqlite3
import threading
from pathlib import Path

DEFAULT_DB_PATH = Path(__file__).resolve().parents[2] / "data" / "agentlab.db"

_SCHEMA = """
CREATE TABLE IF NOT EXISTS customers (
    customer_id TEXT PRIMARY KEY,
    owner_user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    email TEXT NOT NULL,
    tier TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS orders (
    order_id TEXT PRIMARY KEY,
    customer_id TEXT NOT NULL,
    owner_user_id TEXT NOT NULL,
    status TEXT NOT NULL,
    total_cents INTEGER NOT NULL,
    items TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS case_notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    customer_id TEXT NOT NULL,
    owner_user_id TEXT NOT NULL,
    note TEXT NOT NULL,
    created_at REAL NOT NULL,
    created_by_agent_instance TEXT NOT NULL
);

-- v0.3: the Agent Registry. See agents.py — the only module allowed to
-- write this table. (owner_user_id, name, version) is unique: the same
-- human can't register the same name+version twice, but different
-- humans may each register their own agent under the same name.

CREATE TABLE IF NOT EXISTS agents (
    agent_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    owner_user_id TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at REAL NOT NULL,
    updated_at REAL NOT NULL,
    UNIQUE(owner_user_id, name, version)
);
CREATE INDEX IF NOT EXISTS idx_agents_owner ON agents(owner_user_id);

-- v0.2: Session / Task / AgentInstance. See sessions.py, tasks.py,
-- instances.py — each table's rows are only ever read/written through
-- those modules' ownership-enforced functions, never raw SQL elsewhere.

CREATE TABLE IF NOT EXISTS sessions (
    session_id TEXT PRIMARY KEY,
    human_user_id TEXT NOT NULL,
    created_at REAL NOT NULL,
    updated_at REAL NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_sessions_human ON sessions(human_user_id);

CREATE TABLE IF NOT EXISTS tasks (
    task_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    human_user_id TEXT NOT NULL,
    agent_id TEXT NOT NULL REFERENCES agents(agent_id),  -- v0.3: always a canonical registered id
    goal TEXT NOT NULL,
    status TEXT NOT NULL,
    state TEXT NOT NULL,           -- JSON-encoded tasks.TaskState
    created_at REAL NOT NULL,
    updated_at REAL NOT NULL,
    completed_at REAL
);
CREATE INDEX IF NOT EXISTS idx_tasks_session ON tasks(session_id);
CREATE INDEX IF NOT EXISTS idx_tasks_human ON tasks(human_user_id);

CREATE TABLE IF NOT EXISTS agent_instances (
    instance_id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL REFERENCES agents(agent_id),  -- v0.3: always a canonical registered id
    human_user_id TEXT NOT NULL,
    session_id TEXT NOT NULL REFERENCES sessions(session_id),
    task_id TEXT NOT NULL REFERENCES tasks(task_id),
    started_at REAL NOT NULL,
    ended_at REAL,
    status TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_instances_task ON agent_instances(task_id);

-- v0.4: Delegation — explicit authority a human principal grants to a
-- canonical agent_id (never a name; see delegation.py). Rows are never
-- deleted, only transitioned active -> revoked, so a delegation remains
-- as evidence/history after it stops being usable.

CREATE TABLE IF NOT EXISTS delegations (
    delegation_id TEXT PRIMARY KEY,
    principal_user_id TEXT NOT NULL,
    agent_id TEXT NOT NULL REFERENCES agents(agent_id),
    task_id TEXT,                          -- NULL means not task-bound
    allowed_tools TEXT NOT NULL,           -- JSON list of tool names
    resource_constraints TEXT NOT NULL,    -- JSON object: resource_type -> [ids]
    issued_at REAL NOT NULL,
    expires_at REAL NOT NULL,
    status TEXT NOT NULL,
    revoked_at REAL
);
CREATE INDEX IF NOT EXISTS idx_delegations_agent ON delegations(agent_id);
CREATE INDEX IF NOT EXISTS idx_delegations_principal ON delegations(principal_user_id);

-- v0.4: AuthorizationDecision — deterministic evidence for every
-- attempted consequential tool call, ALLOW or DENY. Append-only, never
-- updated or deleted; see authorization.py. Not a security boundary by
-- itself (it's a record of one), but every DENY here must correspond to
-- zero tool side effects.

CREATE TABLE IF NOT EXISTS authorization_decisions (
    decision_id TEXT PRIMARY KEY,
    decision TEXT NOT NULL,                -- 'ALLOW' | 'DENY'
    reason_code TEXT NOT NULL,
    principal_user_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    instance_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    delegation_id TEXT,
    tool_name TEXT NOT NULL,
    resource TEXT,
    timestamp REAL NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_authz_decisions_instance ON authorization_decisions(instance_id);

-- v0.5: Approval — a human principal's exact-argument-bound confirmation
-- of one specific consequential tool call, on top of (never instead of)
-- Delegation. `instance_id`/`task_id` are nullable the same way
-- `delegations.task_id` is: NULL means "not bound to that exact one",
-- set means an exact-match requirement — see approval.py. Never deleted,
-- only transitioned (pending -> approved -> consumed, or -> cancelled/
-- expired), so denied/expired/replayed attempts remain in history.

-- v0.6: Agent signing identity — one active Ed25519 keypair per agent,
-- public half only (see signing_keys.py). This is what makes a Delegation
-- issued *by* an agent (not a human) cryptographically attributable: the
-- delegation's `signing_key_id` names the row here whose public key must
-- verify its signature. Private keys are never persisted — created,
-- returned once to the caller, and held only by the caller (CLI output,
-- test fixtures). Revocation is one-way (active -> revoked, see
-- lifecycle.SigningKeyStatus) and permanent for every delegation the key
-- ever signed — see DECISIONS.md's v0.6 key-lifecycle policy.

CREATE TABLE IF NOT EXISTS agent_signing_keys (
    key_id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL REFERENCES agents(agent_id),
    public_key_pem TEXT NOT NULL,
    algorithm TEXT NOT NULL,               -- "Ed25519" — the only algorithm v0.6 supports
    created_at REAL NOT NULL,
    revoked_at REAL,
    status TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_signing_keys_agent ON agent_signing_keys(agent_id);

CREATE TABLE IF NOT EXISTS approvals (
    approval_id TEXT PRIMARY KEY,
    principal_user_id TEXT NOT NULL,
    agent_id TEXT NOT NULL REFERENCES agents(agent_id),
    instance_id TEXT,                      -- NULL means not instance-bound
    task_id TEXT,                          -- NULL means not task-bound
    delegation_id TEXT NOT NULL REFERENCES delegations(delegation_id),
    tool_name TEXT NOT NULL,
    resource TEXT,
    argument_hash TEXT NOT NULL,           -- sha256 of the canonical (tool, resource, validated args)
    issued_at REAL NOT NULL,
    expires_at REAL NOT NULL,
    status TEXT NOT NULL,
    approved_at REAL,
    consumed_at REAL,
    webauthn_reference TEXT
);
CREATE INDEX IF NOT EXISTS idx_approvals_principal ON approvals(principal_user_id);
CREATE INDEX IF NOT EXISTS idx_approvals_agent ON approvals(agent_id);
"""

_lock = threading.Lock()

# v0.6: additive columns on the pre-existing `delegations` table, applied
# via idempotent ALTER TABLE rather than added to `_SCHEMA`'s CREATE TABLE
# above — one code path handles both a brand-new database and one created
# before v0.6, instead of the schema and the migration disagreeing about
# where these columns come from. Never a destructive drop/recreate, per
# DECISIONS.md's migration-compatibility rule. Each entry is
# (column, declaration); NULL-able with no default, so every pre-v0.6 row
# reads back as a valid *root* delegation (issuer_type/issuer_id/etc. all
# NULL) — the degenerate, backward-compatible case the v0.6 chain verifier
# already treats a parent-less delegation as. See delegation.py's
# `Delegation.parent_delegation_id is None` == "this is a root".
_DELEGATION_V06_COLUMNS: list[tuple[str, str]] = [
    ("format_version", "INTEGER"),
    ("issuer_type", "TEXT"),  # 'human' | 'agent' — NULL on pre-v0.6 rows, treated as 'human'
    ("issuer_id", "TEXT"),  # human_user_id or agent_id of the immediate issuer
    ("parent_delegation_id", "TEXT REFERENCES delegations(delegation_id)"),
    ("parent_hash", "TEXT"),  # sha256 of the parent's canonical bytes at issuance time
    ("signing_key_id", "TEXT REFERENCES agent_signing_keys(key_id)"),
    ("signature", "TEXT"),  # hex Ed25519 signature over this delegation's own canonical bytes
]


def _ensure_column(conn: sqlite3.Connection, table: str, column: str, declaration: str) -> None:
    """Idempotent `ALTER TABLE ... ADD COLUMN` — SQLite raises
    `OperationalError: duplicate column name` if it already exists, which
    is the portable way to make this safe to run on every `connect()`
    call, on both a fresh database (where the column already exists from
    `_SCHEMA`'s CREATE TABLE) and an old one (where it doesn't yet)."""
    try:
        conn.execute(f"ALTER TABLE {table} ADD COLUMN {column} {declaration}")
    except sqlite3.OperationalError as e:
        if "duplicate column name" not in str(e):
            raise


def connect(db_path: str | Path = DEFAULT_DB_PATH) -> sqlite3.Connection:
    """Open (creating if needed) the lab's SQLite file and ensure schema
    exists. Pass a tmp_path in tests to get a fully isolated database per
    test."""
    db_path = Path(db_path)
    db_path.parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(db_path, check_same_thread=False)
    conn.row_factory = sqlite3.Row
    with _lock:
        conn.executescript(_SCHEMA)
        for column, declaration in _DELEGATION_V06_COLUMNS:
            _ensure_column(conn, "delegations", column, declaration)
        # Index the chain-traversal path (child -> parent) now that the
        # column exists on every database, old or new.
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_delegations_parent ON delegations(parent_delegation_id)"
        )
        conn.commit()
    return conn


def seed(conn: sqlite3.Connection) -> None:
    """Idempotent demo data for two different human users — deliberately
    two different owners, so isolation between them is exercisable end to
    end (tools.py, tests/test_tools.py)."""
    with _lock:
        if conn.execute("SELECT COUNT(*) FROM customers").fetchone()[0] > 0:
            return
        conn.executemany(
            "INSERT INTO customers (customer_id, owner_user_id, name, email, tier) VALUES (?, ?, ?, ?, ?)",
            [
                ("cust_1", "user-alice", "Alice Anders", "alice@example.com", "gold"),
                ("cust_2", "user-bob", "Bob Baker", "bob@example.com", "silver"),
            ],
        )
        conn.executemany(
            "INSERT INTO orders (order_id, customer_id, owner_user_id, status, total_cents, items) VALUES (?, ?, ?, ?, ?, ?)",
            [
                ("ord_1", "cust_1", "user-alice", "shipped", 4599, "widget x2"),
                ("ord_2", "cust_2", "user-bob", "pending", 1299, "gadget x1"),
            ],
        )
        conn.commit()
