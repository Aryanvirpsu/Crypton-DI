# Reproducing the Validation Benchmark (Windows / Git Bash)

Verified 2026-10-12 on Python 3.14.7 and Node v24.19.0. Envs live OUTSIDE the repo; pick any directory for `ENVS`.
Pinned versions are confirmed against the GitHub advisory database (`evidence/benchmark/advisory-*.json` in `crypton-bigidea-2026/`).

```bash
ENVS=../Crypton-DI-benchenvs; mkdir -p $ENVS && cd $ENVS
# Case 01: MCP Python SDK, CVE-2026-52869 / GHSA-jpw9-pfvf-9f58 (vulnerable <= 1.27.1, fixed 1.27.2)
python -m venv vuln-mcp-1.27.1 && ./vuln-mcp-1.27.1/Scripts/python.exe -m pip install mcp==1.27.1
python -m venv fixed-mcp-1.27.2 && ./fixed-mcp-1.27.2/Scripts/python.exe -m pip install mcp==1.27.2
# Case 02: @bytebase/dbhub, CVE-2026-61788 / GHSA-mwwr-p57h-56pf (vulnerable < 0.22.6, fixed 0.22.6); needs Node >= 22.5
for v in 0.22.5 0.22.6; do d=$([ $v = 0.22.5 ] && echo vuln-dbhub-0.22.5 || echo fixed-dbhub-0.22.6)
  mkdir -p $d && (cd $d && npm init -y && npm install @bytebase/dbhub@$v); done
```
Then run `bash crypton-bigidea-2026/run_all.sh` (set `BENCH_ENV_DIR` if the envs are elsewhere), or by hand:
```bash
cd labs/validation-benchmark/v0.1/case-01-mcp-python-sdk-session-binding
$ENVS/vuln-mcp-1.27.1/Scripts/python.exe test_session_binding.py    # expect VERDICT: FAIL
$ENVS/fixed-mcp-1.27.2/Scripts/python.exe test_session_binding.py   # expect VERDICT: PASS
# case-02: copy test_readonly_boundary.mjs into each dbhub env dir, then: node test_readonly_boundary.mjs
```
Exit code is 0 on both versions by design (it means "the negative control held, so the verdict is trustworthy"); the verdict is in the output.

## What this does and does not show
- **Independent regression reproduction:** the vulnerable/fixed pairs and their versions come from the vendors' public advisories, not from Crypton. Anyone can re-run the same scripts against the same pinned packages.
- **Crypton's detection capability:** limited. Both cases are retrospective (the vulnerability and fix were public before the script was written; the script was authored by reading the advisory and the release diff). It shows Crypton's test *distinguishes* a known-vulnerable from a known-fixed release; it does not show Crypton would have found either bug first.
- Each is a single in-process probe of one property, not a full assessment of the product.
