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
    touch "$ENV_LOCAL"
fi

if [ ! -f "jwt_signing_key.local.pem" ]; then
    echo "[crypton-id] No local dev JWT keypair found — generating a fresh Ed25519 pair."
    openssl genpkey -algorithm ed25519 -out jwt_signing_key.local.pem
    openssl pkey -in jwt_signing_key.local.pem -pubout -out jwt_public_key.local.pem
fi
export JWT_SIGNING_KEY_PEM_FILE=${JWT_SIGNING_KEY_PEM_FILE:-"$(pwd)/jwt_signing_key.local.pem"}
export JWT_PUBLIC_KEY_PEM_FILE=${JWT_PUBLIC_KEY_PEM_FILE:-"$(pwd)/jwt_public_key.local.pem"}

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
