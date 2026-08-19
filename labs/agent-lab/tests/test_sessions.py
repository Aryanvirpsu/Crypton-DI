import pytest

from agentlab import db
from agentlab.sessions import SessionAccessError, create_session, get_session, touch_session


def test_create_session(db_conn, alice):
    session = create_session(db_conn, alice)
    assert session.human_user_id == "user-alice"
    assert session.created_at == session.updated_at
    assert session.session_id


def test_session_persists_across_reconnect(tmp_path, alice):
    db_path = tmp_path / "persist.db"
    conn1 = db.connect(db_path)
    session = create_session(conn1, alice)
    conn1.close()

    # a fresh connection to the same file must see the same row —
    # proves this is durable SQLite state, not an in-memory fixture.
    conn2 = db.connect(db_path)
    reloaded = get_session(conn2, alice, session.session_id)
    assert reloaded.session_id == session.session_id
    assert reloaded.human_user_id == "user-alice"
    conn2.close()


def test_get_session_owner_succeeds(db_conn, alice):
    session = create_session(db_conn, alice)
    reloaded = get_session(db_conn, alice, session.session_id)
    assert reloaded == session


def test_cross_user_session_access_rejected(db_conn, alice, bob):
    session = create_session(db_conn, alice)
    with pytest.raises(SessionAccessError, match="session_not_found"):
        get_session(db_conn, bob, session.session_id)


def test_unknown_session_rejected(db_conn, alice):
    with pytest.raises(SessionAccessError, match="session_not_found"):
        get_session(db_conn, alice, "does-not-exist")


def test_touch_session_bumps_updated_at(db_conn, alice):
    session = create_session(db_conn, alice)
    touched = touch_session(db_conn, session)
    assert touched.updated_at >= session.updated_at
    assert touched.created_at == session.created_at
