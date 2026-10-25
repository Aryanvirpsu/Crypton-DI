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
  cargo audit --file "$c/Cargo.lock" "${ignore[@]}" || rc=1
  echo "::endgroup::"
done
exit $rc
