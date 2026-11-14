#!/usr/bin/env bash
# Audit every Rust lockfile with cargo-audit (RustSec).
# Fails on any vulnerability that is not listed, with a justification, in
# scripts/rust-audit-ignore.txt. Warnings (unmaintained, unsound, yanked) are printed
# but do not fail the run. Usage: bash scripts/rust-audit.sh
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

ignore=()
while read -r id _; do
  case "$id" in RUSTSEC-*) ignore+=(--ignore "$id") ;; esac
done < scripts/rust-audit-ignore.txt

rc=0
for c in services/crypton-id services/crypton-gateway risk-engine policy-engine; do
  echo "::group::cargo audit $c"
  crate_ignore=("${ignore[@]}")
  case "$c" in
    services/crypton-id|services/crypton-gateway|risk-engine)
      # SQLx's optional SQLite driver is locked but unused. Recheck the graph
      # on every audit so enabling SQLite makes the advisory fail the gate.
      features=()
      if [[ "$c" == risk-engine ]]; then features=(--all-features); fi
      if tree=$(cargo tree --manifest-path "$c/Cargo.toml" --locked \
        --target all --edges normal,build,dev --prefix none "${features[@]}") \
        && ! grep -q '^libsqlite3-sys ' <<< "$tree"; then
        echo 'RUSTSEC-2026-0459: libsqlite3-sys is locked but absent from the enabled graph.'
        crate_ignore+=(--ignore RUSTSEC-2026-0459)
      fi
      ;;
  esac
  cargo audit --file "$c/Cargo.lock" "${crate_ignore[@]}" || rc=1
  echo "::endgroup::"
done
exit $rc
