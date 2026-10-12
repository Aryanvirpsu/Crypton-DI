"""
v0.6.1 — the minimal repeatable engagement structure.

An engagement is a directory of plain files — no database, per this
milestone's explicit instruction. `engagement.json` is the small envelope
naming the other files; everything else is either prose (Markdown) or a
flat JSON list (`findings.json`, `evidence_manifest.json`). This is
deliberately boring: an assessor should be able to open any file in an
engagement directory in a text editor and understand it without this
module.

**2026-09-30 remediation pass:** `engagement.json` previously had only 4
real, structured fields (title/label/date/assessor) plus 6 file pointers —
the 2026-09-30 evidence-readiness audit (§5.1) found this schema too thin
to answer basic questions (which assessment_id is this? which product and
version was tested? what commit?) without opening and parsing prose files
that didn't reliably contain the answer. The fields below `assessor` are
new, structured, and optional (default `None`) — loading an older
engagement directory that predates this schema version still works, and
no value is fabricated for a field an old engagement never recorded.
`scope`/`workflow`/`limitations`/`executive_summary` keep their existing
file-pointer design (long-form prose genuinely belongs in a readable
Markdown file, not crammed into one JSON string) — but the short,
answerable facts that were previously *only* buried in that prose now also
have a dedicated structured field.

See `assessment-engagement/TEMPLATE/` for a blank starter kit, and
`assessment-engagement/reference-agentlab-dryrun/` for the populated
synthetic example this milestone's acceptance criteria requires.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

from .evidence import EvidenceManifest
from .models import Finding

ENVELOPE_FILENAME = "engagement.json"


class EngagementLoadError(ValueError):
    """Raised for a malformed engagement directory — a missing envelope
    file, a referenced file that doesn't exist, or (via
    `EvidenceManifest.validate_findings`) a dangling evidence reference.
    Deliberately fails loudly rather than rendering a report from
    incomplete or inconsistent data."""


@dataclass(frozen=True)
class Engagement:
    """Everything `report.render_report` needs. `label` is the short,
    prominent tag a report's header shows — for the reference dry run
    this is literally the string "Crypton Reference Agent Lab —
    Methodology Dry Run", so the rendered output can never be mistaken
    for a real external assessment (see reference_dryrun.py)."""

    title: str
    label: str
    date: str
    assessor: str
    scope: str
    workflow: str
    limitations: str
    executive_summary: str
    findings: tuple[Finding, ...]
    evidence: EvidenceManifest
    # --- 2026-09-30 expansion: structured engagement metadata ---
    assessment_id: str | None = None
    client_identifier: str | None = None
    system_identifier: str | None = None
    assessment_type: str | None = None
    tested_workflow: str | None = None  # short structured label; `workflow` above holds the long-form prose
    tested_layers: tuple[str, ...] = ()
    environment: str | None = None
    target_product: str | None = None
    target_version: str | None = None
    target_commit: str | None = None
    testing_window: str | None = None
    configuration: str | None = None
    excluded_scope: str | None = None
    # Optional, additive, prose-only canonical-report sections (§ item 11 of
    # the 2026-09-30 remediation spec). Each defaults to None and the
    # renderer prints an explicit "not populated" statement rather than
    # fabricating content — see report.py.
    authority_map: str | None = None
    action_queue: str | None = None
    verified_controls: str | None = None

    def __post_init__(self) -> None:
        # Fail loudly on a dangling reference at construction time, not
        # only when someone happens to render this engagement.
        self.evidence.validate_findings(self.findings)

    @classmethod
    def load(cls, directory: str | Path) -> "Engagement":
        directory = Path(directory)
        envelope_path = directory / ENVELOPE_FILENAME
        if not envelope_path.exists():
            raise EngagementLoadError(f"no {ENVELOPE_FILENAME} found in {directory}")
        envelope = json.loads(envelope_path.read_text(encoding="utf-8"))

        def _read_text(key: str) -> str:
            filename = envelope.get(key)
            if not filename:
                raise EngagementLoadError(f"engagement.json is missing required key {key!r}")
            path = directory / filename
            if not path.exists():
                raise EngagementLoadError(f"{key} references {filename!r}, which does not exist in {directory}")
            return path.read_text(encoding="utf-8").strip()

        evidence_path = directory / envelope.get("evidence_manifest_file", "")
        if not envelope.get("evidence_manifest_file") or not evidence_path.exists():
            raise EngagementLoadError(f"engagement.json's evidence_manifest_file does not exist in {directory}")
        evidence = EvidenceManifest.load(evidence_path)

        findings_path = directory / envelope.get("findings_file", "")
        if not envelope.get("findings_file") or not findings_path.exists():
            raise EngagementLoadError(f"engagement.json's findings_file does not exist in {directory}")
        findings_data = json.loads(findings_path.read_text(encoding="utf-8"))
        findings = tuple(Finding.from_dict(f) for f in findings_data)

        return cls(
            title=envelope.get("title", ""),
            label=envelope.get("label", ""),
            date=envelope.get("date", ""),
            assessor=envelope.get("assessor", ""),
            scope=_read_text("scope_file"),
            workflow=_read_text("workflow_file"),
            limitations=_read_text("limitations_file"),
            executive_summary=_read_text("executive_summary_file"),
            findings=findings,
            evidence=evidence,
            assessment_id=envelope.get("assessment_id"),
            client_identifier=envelope.get("client_identifier"),
            system_identifier=envelope.get("system_identifier"),
            assessment_type=envelope.get("assessment_type"),
            tested_workflow=envelope.get("tested_workflow"),
            tested_layers=tuple(envelope.get("tested_layers", [])),
            environment=envelope.get("environment"),
            target_product=envelope.get("target_product"),
            target_version=envelope.get("target_version"),
            target_commit=envelope.get("target_commit"),
            testing_window=envelope.get("testing_window"),
            configuration=envelope.get("configuration"),
            excluded_scope=envelope.get("excluded_scope"),
            authority_map=envelope.get("authority_map"),
            action_queue=envelope.get("action_queue"),
            verified_controls=envelope.get("verified_controls"),
        )

    def save(self, directory: str | Path) -> None:
        """Write this engagement out as a directory of files, using the
        conventional filenames. Used by `reference_dryrun.py` to persist
        the generated synthetic engagement; a real engagement is more
        often hand-authored file-by-file than round-tripped through this
        method, but it's kept symmetric with `load` for tests and for
        anyone scripting a repeatable engagement setup."""
        directory = Path(directory)
        directory.mkdir(parents=True, exist_ok=True)

        (directory / "scope.md").write_text(self.scope.strip() + "\n", encoding="utf-8")
        (directory / "workflow.md").write_text(self.workflow.strip() + "\n", encoding="utf-8")
        (directory / "limitations.md").write_text(self.limitations.strip() + "\n", encoding="utf-8")
        (directory / "executive_summary.md").write_text(self.executive_summary.strip() + "\n", encoding="utf-8")
        self.evidence.save(directory / "evidence_manifest.json")
        (directory / "findings.json").write_text(
            json.dumps([f.to_dict() for f in self.findings], indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
        )
        envelope = {
            "title": self.title,
            "label": self.label,
            "date": self.date,
            "assessor": self.assessor,
            "scope_file": "scope.md",
            "workflow_file": "workflow.md",
            "evidence_manifest_file": "evidence_manifest.json",
            "findings_file": "findings.json",
            "limitations_file": "limitations.md",
            "executive_summary_file": "executive_summary.md",
            "assessment_id": self.assessment_id,
            "client_identifier": self.client_identifier,
            "system_identifier": self.system_identifier,
            "assessment_type": self.assessment_type,
            "tested_workflow": self.tested_workflow,
            "tested_layers": list(self.tested_layers),
            "environment": self.environment,
            "target_product": self.target_product,
            "target_version": self.target_version,
            "target_commit": self.target_commit,
            "testing_window": self.testing_window,
            "configuration": self.configuration,
            "excluded_scope": self.excluded_scope,
            "authority_map": self.authority_map,
            "action_queue": self.action_queue,
            "verified_controls": self.verified_controls,
        }
        (directory / ENVELOPE_FILENAME).write_text(json.dumps(envelope, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
