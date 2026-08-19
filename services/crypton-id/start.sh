#!/bin/bash
# Crypton Identity Launcher (dev)
#
# Secrets live in .env.local, which is gitignored and generated on first
# run — nothing secret is ever committed. Everything else has a plain-text
# dev default here since none of it is sensitive.
set -e
cd "$(dirname "$0")"

ENV_LOCAL=".env.local"
if [ ! -f "$ENV_LOCAL" ]; then
    echo "[crypton-id] No $ENV_LOCAL found — generating a local-only dev JWT secret."
    SECRET=$(openssl rand -hex 32 2>/dev/null || head -c32 /dev/urandom | xxd -p -c256)
    echo "JWT_SECRET=$SECRET" > "$ENV_LOCAL"
fi
set -a
source "$ENV_LOCAL"
set +a

export APP_PORT=${APP_PORT:-8080}
export DATABASE_URL=${DATABASE_URL:-"postgres://postgres:postgres@127.0.0.1:5432/crypton"}
export REDIS_URL=${REDIS_URL:-"redis://127.0.0.1:6379"}
export RUST_LOG=${RUST_LOG:-info}
export WEBAUTHN_RP_NAME=${WEBAUTHN_RP_NAME:-"Crypton Identity"}
export WEBAUTHN_RP_ID=${WEBAUTHN_RP_ID:-localhost}
export WEBAUTHN_ORIGIN=${WEBAUTHN_ORIGIN:-"http://localhost:3000"}

# OAuth is an experimental, incomplete surface — see routes/oauth.rs.
# It stays unmounted unless explicitly enabled.
export ENABLE_OAUTH=${ENABLE_OAUTH:-false}

exec ./target/release/crypton-identity
