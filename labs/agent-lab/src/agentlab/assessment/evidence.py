"""
v0.6.1 — the evidence manifest.

This is an index, not a document-management platform: a manifest says
"evidence E-011 is an authorization log, sourced from <this file/query>,
described as <this>" — it does not store the log itself. A real
engagement's actual evidence (source excerpts, log exports, screenshots,
staging-test output) lives wherever the assessor collected it; the
manifest is what a `Finding.evidence_ids` entry resolves to, and what a
rendered report cites.

`EvidenceManifest` enforces the two integrity properties an assessment
report actually needs to be trustworthy:

  1. No two evidence entries share an ID (a silent overwrite would make
     "Finding AD-04 cites E-011" ambiguous about which E-011).
  2. No finding cites an evidence ID the manifest doesn't have (a dangling
     reference in a report is worse than no evidence at all — it *looks*
     cited).
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Iterable, Iterator

from .models import EvidenceReference, Finding


class DuplicateEvidenceIdError(ValueError):
    """Raised when adding an EvidenceReference whose evidence_id already
    exists in the manifest."""


class DanglingEvidenceReferenceError(ValueError):
    """Raised by `validate_findings` when a Finding cites an evidence_id
    the manifest does not contain."""


class EvidenceManifest:
    """An ordered collection of `EvidenceReference`s, keyed by
    `evidence_id`. Insertion order is preserved (evidence is usually
    listed in collection order in a report) — this is a thin wrapper
    around a `dict`, not a database."""

    def __init__(self, entries: Iterable[EvidenceReference] = ()):
        self._entries: dict[str, EvidenceReference] = {}
        for entry in entries:
            self.add(entry)

    def add(self, entry: EvidenceReference) -> None:
        if entry.evidence_id in self._entries:
            raise DuplicateEvidenceIdError(
                f"evidence_id {entry.evidence_id!r} is already in this manifest — "
                "evidence IDs must be unique within an engagement"
            )
        self._entries[entry.evidence_id] = entry

    def get(self, evidence_id: str) -> EvidenceReference | None:
        return self._entries.get(evidence_id)

    def __contains__(self, evidence_id: str) -> bool:
        return evidence_id in self._entries

    def __len__(self) -> int:
        return len(self._entries)

    def __iter__(self) -> Iterator[EvidenceReference]:
        return iter(self._entries.values())

    def ids(self) -> list[str]:
        return list(self._entries.keys())

    def validate_findings(self, findings: Iterable[Finding]) -> None:
        """Raise `DanglingEvidenceReferenceError` on the first finding
        that cites an evidence_id this manifest doesn't have. Called by
        `engagement.Engagement.load`/`report.render_report` before doing
        anything with the data — a report is never rendered from data
        with a dangling reference, even a citation-only one."""
        for finding in findings:
            for evidence_id in finding.evidence_ids:
                if evidence_id not in self._entries:
                    raise DanglingEvidenceReferenceError(
                        f"finding {finding.finding_id!r} cites evidence_id {evidence_id!r}, "
                        "which is not in the evidence manifest"
                    )

    def to_list(self) -> list[dict[str, Any]]:
        return [e.to_dict() for e in self._entries.values()]

    @classmethod
    def from_list(cls, data: list[dict[str, Any]]) -> "EvidenceManifest":
        return cls(EvidenceReference.from_dict(d) for d in data)

    def save(self, path: str | Path) -> None:
        Path(path).write_text(json.dumps(self.to_list(), indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    @classmethod
    def load(cls, path: str | Path) -> "EvidenceManifest":
        data = json.loads(Path(path).read_text(encoding="utf-8"))
        return cls.from_list(data)
