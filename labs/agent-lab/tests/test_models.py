import pytest
from pydantic import ValidationError

from agentlab.models import GetCustomerArgs


def test_missing_required_field_rejected():
    with pytest.raises(ValidationError):
        GetCustomerArgs.model_validate({})


def test_unexpected_field_rejected():
    with pytest.raises(ValidationError):
        GetCustomerArgs.model_validate({"customer_id": "cust_1", "unexpected": "shell_command"})


def test_wrong_type_rejected():
    with pytest.raises(ValidationError):
        GetCustomerArgs.model_validate({"customer_id": 12345})
