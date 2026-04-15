#!/bin/bash
# Crypton Identity Launcher
# Env vars set explicitly — not affected by .env file changes.
# Redis 3.0 on localhost has no password — plain URL required.

export APP_PORT=8080
export DATABASE_URL="postgres://postgres:postgres@127.0.0.1:5432/crypton"
export REDIS_URL="redis://127.0.0.1:6379"
export RUST_LOG=info
export JWT_SECRET="aefa5ac024afaf0556299762a25041422c6a3dfe3017398b7af1a020159e8ad8"
export WEBAUTHN_RP_NAME="Crypton Identity"
export WEBAUTHN_RP_ID="localhost"
export WEBAUTHN_ORIGIN="http://localhost:3000"
export OAUTH_CLIENT_ID="demo-site"
export OAUTH_CLIENT_SECRET="demo-secret-change-in-prod"
export OAUTH_REDIRECT_URIS="http://localhost:4000/callback"
export FRONTEND_ORIGIN="http://localhost:3000"

cd "$(dirname "$0")"
exec ./target/release/crypton-identity
