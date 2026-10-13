"""
v0.6.1 pre-commit audit — regression test for
`cli._reconfigure_stdout_utf8_best_effort`.

`--render-assessment` prints report Markdown containing legitimate
non-ASCII punctuation (em-dashes). A console whose stdout encoding can't
represent that character (e.g. a legacy Windows OEM codepage, or `ascii`
— used below as the strictest, most portable stand-in for "restrictive
encoding," since it fails identically on every OS and needs no
Windows-specific codepage) crashes `print()` with `UnicodeEncodeError`
unless stdout is reconfigured to UTF-8 first. This function is the
narrow, localized fix — scoped to that one print, not applied globally to
every CLI command (see its own docstring). Three things must hold:

1. It actually fixes the crash: a stream that would previously raise
   `UnicodeEncodeError` on an em-dash succeeds after reconfiguration and
   writes the character as real UTF-8 bytes, not a mangled substitute.
2. It's best-effort against a stream with no `.reconfigure` at all
   (`AttributeError`) — must not raise.
3. It's best-effort against a stream whose `.reconfigure` rejects the call
   (`ValueError`) — must not raise, and must not raise any other type
   either (only these two are ever expected from a real stream).
"""

import io

import pytest

from agentlab.cli import _reconfigure_stdout_utf8_best_effort


def test_reconfigure_fixes_unicode_encode_error_on_narrow_console():
    # A TextIOWrapper pinned to a restrictive encoding reproduces the real
    # console failure mode: an em-dash cannot be encoded in it at all.
    buf = io.BytesIO()
    stream = io.TextIOWrapper(buf, encoding="ascii")
    with pytest.raises(UnicodeEncodeError):
        stream.write("—")
        stream.flush()  # TextIOWrapper buffers; the encode error surfaces on flush, not write

    # Fresh wrapper, same underlying restrictive encoding, now reconfigured.
    buf2 = io.BytesIO()
    stream2 = io.TextIOWrapper(buf2, encoding="ascii")
    _reconfigure_stdout_utf8_best_effort(stream2)
    stream2.write("—")  # must not raise
    stream2.flush()
    assert buf2.getvalue() == "—".encode("utf-8")


def test_reconfigure_is_best_effort_when_stream_lacks_reconfigure():
    class NoReconfigure:
        pass

    stream = NoReconfigure()
    _reconfigure_stdout_utf8_best_effort(stream)  # must not raise AttributeError


def test_reconfigure_is_best_effort_when_reconfigure_raises_value_error():
    class RejectsReconfigure:
        def reconfigure(self, **kwargs):
            raise ValueError("stream already consumed")

    stream = RejectsReconfigure()
    _reconfigure_stdout_utf8_best_effort(stream)  # must not raise ValueError


def test_reconfigure_defaults_to_sys_stdout(monkeypatch):
    calls = []

    class FakeStdout:
        def reconfigure(self, **kwargs):
            calls.append(kwargs)

    monkeypatch.setattr("agentlab.cli.sys.stdout", FakeStdout())
    _reconfigure_stdout_utf8_best_effort()  # no argument -> defaults to sys.stdout
    assert calls == [{"encoding": "utf-8", "errors": "replace"}]
