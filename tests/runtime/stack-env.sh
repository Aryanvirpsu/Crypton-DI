#!/usr/bin/env bash
# Source this: generates THROWAWAY secrets for a local runtime verification stack
# into .stack-secrets/ (gitignored) and exports the variables docker-compose.stack.yml
# requires. Never reuse these values anywhere real.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SECRETS="$ROOT/.stack-secrets"
mkdir -p "$SECRETS"
if [ ! -f "$SECRETS/jwt_signing_key.pem" ]; then
  openssl genpkey -algorithm ed25519 -out "$SECRETS/jwt_signing_key.pem"
  openssl pkey -in "$SECRETS/jwt_signing_key.pem" -pubout -out "$SECRETS/jwt_public_key.pem"
fi
rand() { python -c "import secrets;print(secrets.token_hex(16))" 2>/dev/null || python3 -c "import secrets;print(secrets.token_hex(16))"; }
[ -f "$SECRETS/env.sh" ] || {
  printf 'export POSTGRES_PASSWORD=%s\nexport GATEWAY_SHARED_SECRET=%s\nexport INTERNAL_TOKEN=%s\n' \
    "$(rand)" "$(rand)" "$(rand)" > "$SECRETS/env.sh"
}
# shellcheck disable=SC1091
. "$SECRETS/env.sh"
export STACK_SECRETS_DIR="$SECRETS"
export GATEWAY_HOST_PORT="${GATEWAY_HOST_PORT:-18090}"
