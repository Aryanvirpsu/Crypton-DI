#!/usr/bin/env bash
# Full regression + benchmark runner for the BIG IDEA evidence freeze.
# Windows / Git Bash paths (venv layout .venv/Scripts/python.exe). Run from anywhere.
#
# Prerequisites (not installed by this script):
#   - each lab's own .venv (see that lab's README) for labs/agent-lab, exp-003-*, exp-003c-langgraph
#   - DATABASE_URL pointing at an ISOLATED throwaway Postgres 16, e.g.
#       docker run -d --rm --name crypton-test-pg -e POSTGRES_USER=crypton_test \
#         -e POSTGRES_PASSWORD=<throwaway> -e POSTGRES_DB=crypton_test -p 127.0.0.1:55432:5432 postgres:16-alpine
#       export DATABASE_URL=postgres://crypton_test:<throwaway>@127.0.0.1:55432/crypton_test
#   - BENCH_ENV_DIR containing the four pinned benchmark envs (see labs/validation-benchmark/v0.1/REPRODUCE.md)
#
# Output: crypton-bigidea-2026/evidence/regression/*.txt  (raw) and summary.tsv (one line per suite)
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/crypton-bigidea-2026/evidence/regression"
BENCH_ENV_DIR="${BENCH_ENV_DIR:-$ROOT/../Crypton-DI-benchenvs}"
mkdir -p "$OUT"; : > "$OUT/summary.tsv"
printf 'suite\texit\tresult_line\tcommand\n' >> "$OUT/summary.tsv"

run() { # name, dir, command...
  local name="$1" dir="$2"; shift 2
  ( cd "$ROOT/$dir" && "$@" ) > "$OUT/$name.txt" 2>&1; local rc=$?
  local line; line="$(grep -E '[0-9]+ passed|test result:|Tests: |VERDICT|RESULT:|VERDICT \(secure' "$OUT/$name.txt" | tail -3 | tr '\n' ' ' | cut -c1-230)"
  printf '%s\t%s\t%s\t(cd %s && %s)\n' "$name" "$rc" "$line" "$dir" "$*" >> "$OUT/summary.tsv"
  echo "[$name] exit=$rc  $line"
}

PY=".venv/Scripts/python.exe"
run agent-lab                 labs/agent-lab                          $PY -m pytest -v -rs -p no:cacheprovider
run exp-007a-tests            labs/exp-007-principal-continuity       "$ROOT/labs/agent-lab/$PY" -m pytest -v -p no:cacheprovider
run exp-007a-runner           labs/exp-007-principal-continuity       "$ROOT/labs/agent-lab/$PY" -m exp007.scenario_007a
run exp-003c-langgraph        labs/exp-003c-langgraph                 $PY -m pytest -v -p no:cacheprovider
run exp-003-openai-agents-sdk labs/exp-003-openai-agents-sdk          $PY -m pytest -v -p no:cacheprovider
run exp-003-microsoft-af      labs/exp-003-microsoft-agent-framework  $PY -m pytest -v -p no:cacheprovider
run rust-crypton-id           services/crypton-id                     cargo test --no-fail-fast
run rust-crypton-gateway      services/crypton-gateway                cargo test --no-fail-fast
run sdk-jest                  libs/crypton-sdk                        npx jest
run sdk-typecheck             libs/crypton-sdk                        npx tsc --noEmit
run demo-001-delegation       .                                       "$ROOT/labs/agent-lab/$PY" crypton-bigidea-2026/demo/run_demo.py

B="labs/validation-benchmark/v0.1"
run bench-mcp-1.27.1-vulnerable $B/case-01-mcp-python-sdk-session-binding "$BENCH_ENV_DIR/vuln-mcp-1.27.1/Scripts/python.exe" test_session_binding.py
run bench-mcp-1.27.2-fixed      $B/case-01-mcp-python-sdk-session-binding "$BENCH_ENV_DIR/fixed-mcp-1.27.2/Scripts/python.exe" test_session_binding.py
# dbhub: the script resolves @bytebase/dbhub from node_modules next to it, so run a copy from the pinned env dir.
cp "$ROOT/$B/case-02-dbhub-sqlite-readonly-boundary/test_readonly_boundary.mjs" "$BENCH_ENV_DIR/vuln-dbhub-0.22.5/"
cp "$ROOT/$B/case-02-dbhub-sqlite-readonly-boundary/test_readonly_boundary.mjs" "$BENCH_ENV_DIR/fixed-dbhub-0.22.6/"
( cd "$BENCH_ENV_DIR/vuln-dbhub-0.22.5"  && node test_readonly_boundary.mjs ) > "$OUT/bench-dbhub-0.22.5-vulnerable.txt" 2>&1; rc=$?
printf 'bench-dbhub-0.22.5-vulnerable\t%s\t%s\t(cd $BENCH_ENV_DIR/vuln-dbhub-0.22.5 && node test_readonly_boundary.mjs)\n' "$rc" "$(grep VERDICT "$OUT/bench-dbhub-0.22.5-vulnerable.txt" | cut -c1-120)" >> "$OUT/summary.tsv"
( cd "$BENCH_ENV_DIR/fixed-dbhub-0.22.6" && node test_readonly_boundary.mjs ) > "$OUT/bench-dbhub-0.22.6-fixed.txt" 2>&1; rc=$?
printf 'bench-dbhub-0.22.6-fixed\t%s\t%s\t(cd $BENCH_ENV_DIR/fixed-dbhub-0.22.6 && node test_readonly_boundary.mjs)\n' "$rc" "$(grep VERDICT "$OUT/bench-dbhub-0.22.6-fixed.txt" | cut -c1-120)" >> "$OUT/summary.tsv"
echo "done; see $OUT/summary.tsv"
