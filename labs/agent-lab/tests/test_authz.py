import pytest

from agentlab.authz import AuthorizationError, check_authorized


def test_allowed_tool_passes():
    check_authorized("support-agent", "get_customer")  # must not raise


def test_unregistered_agent_rejected():
    with pytest.raises(AuthorizationError, match="unregistered_agent"):
        check_authorized("rogue-agent", "get_customer")


def test_tool_not_in_allowlist_rejected():
    with pytest.raises(AuthorizationError, match="tool_not_allowed"):
        check_authorized("read-only-agent", "add_case_note")
