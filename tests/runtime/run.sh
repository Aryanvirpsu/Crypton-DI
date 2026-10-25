#!/usr/bin/env bash
# Build and start the real stack, run the runtime verification, tear down.
#   tests/runtime/run.sh            full run
#   KEEP_STACK=1 tests/runtime/run.sh   leave the stack running afterwards
# Needs Docker (compose v2) and Node >= 18. Uses throwaway secrets only.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
# shellcheck disable=SC1091
. tests/runtime/stack-env.sh
export COMPOSE_PROJECT=crypton-stack
docker compose -p "$COMPOSE_PROJECT" -f infra/docker-compose.stack.yml up -d --build
rc=0
node tests/runtime/e2e.mjs || rc=$?
if [ "${KEEP_STACK:-0}" != "1" ]; then
  docker compose -p "$COMPOSE_PROJECT" -f infra/docker-compose.stack.yml down -v >/dev/null 2>&1 || true
fi
exit $rc
