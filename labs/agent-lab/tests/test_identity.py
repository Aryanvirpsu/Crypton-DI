import pytest

from agentlab.identity import IdentityError, issue_dev_token, verify_human_identity

SECRET = "test-secret-value-that-is-long-enough-for-hs256"


def test_verify_roundtrip():
    token = issue_dev_token("user-alice", "alice", "cred-1", secret=SECRET)
    identity = verify_human_identity(token, secret=SECRET)
    assert identity.user_id == "user-alice"
    assert identity.username == "alice"
    assert identity.cred_id == "cred-1"


def test_wrong_secret_rejected():
    token = issue_dev_token("user-alice", "alice", "cred-1", secret=SECRET)
    with pytest.raises(IdentityError):
        verify_human_identity(token, secret="a-different-secret")


def test_garbage_token_rejected():
    with pytest.raises(IdentityError):
        verify_human_identity("not-a-jwt-at-all", secret=SECRET)


def test_missing_secret_rejected(monkeypatch):
    monkeypatch.delenv("JWT_SECRET", raising=False)
    token = issue_dev_token("user-alice", "alice", "cred-1", secret=SECRET)
    with pytest.raises(IdentityError):
        verify_human_identity(token)  # no secret configured anywhere
