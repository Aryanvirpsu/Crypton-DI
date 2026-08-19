import pytest

from agentlab.lifecycle import LifecycleError, TaskStatus, parse_status, validate_transition


@pytest.mark.parametrize(
    "current,new",
    [
        (TaskStatus.CREATED, TaskStatus.RUNNING),
        (TaskStatus.CREATED, TaskStatus.CANCELLED),
        (TaskStatus.RUNNING, TaskStatus.COMPLETED),
        (TaskStatus.RUNNING, TaskStatus.FAILED),
        (TaskStatus.RUNNING, TaskStatus.CANCELLED),
    ],
)
def test_valid_transitions(current, new):
    validate_transition(current, new)  # must not raise


@pytest.mark.parametrize(
    "current,new",
    [
        (TaskStatus.CREATED, TaskStatus.COMPLETED),  # skips running
        (TaskStatus.CREATED, TaskStatus.FAILED),
        (TaskStatus.COMPLETED, TaskStatus.RUNNING),  # terminal -> active
        (TaskStatus.FAILED, TaskStatus.RUNNING),
        (TaskStatus.CANCELLED, TaskStatus.RUNNING),
        (TaskStatus.COMPLETED, TaskStatus.FAILED),  # terminal -> terminal
        (TaskStatus.RUNNING, TaskStatus.CREATED),  # backwards
    ],
)
def test_invalid_transitions_rejected(current, new):
    with pytest.raises(LifecycleError, match="invalid_transition"):
        validate_transition(current, new)


def test_parse_known_status():
    assert parse_status("running") == TaskStatus.RUNNING


def test_parse_unknown_status_rejected():
    with pytest.raises(LifecycleError, match="unknown_status"):
        parse_status("in_orbit")
