# crypton-bigidea-2026 — evidence package

Code freeze: `1b93abe887c9e28f3217cb67e0e707a5a9f618f6` (see `evidence/repo-version.txt`).

| Read this | For |
|---|---|
| `application/technical-evidence-summary.md` | The application-ready summary (verified vs. not verified vs. planned) |
| `evidence/verification-summary.md` | Full test and benchmark report, baseline comparison, acceptance criteria |
| `evidence/CHANGELOG.md` | Code-cleanup and defect-fix changelog |
| `evidence/KNOWN-LIMITATIONS-AND-DEFERRED.md` | Limitations and deferred-work register |
| `evidence/feature-inventory.md` | Capability-by-capability status |
| `demo/` | Reproducible delegation demo (`run_demo.py`), scenario, raw output, sample customer report |
| `evidence/regression/`, `evidence/benchmark/` | Raw output for every suite; pinned benchmark environments and advisories |
| `evidence/baseline-pre-stabilization/` | The first run (542 pass / 16 fail), kept for comparison |
| `evidence/secret-scan.txt` | Secret-scan record |

Reproduce everything: `bash crypton-bigidea-2026/run_all.sh` (prerequisites are listed at the top of the script; benchmark env setup is in `labs/validation-benchmark/v0.1/REPRODUCE.md`).

Not included: `demo-recording.mp4` (no screen recorder in the session; record `demo/run_demo.py` yourself).
