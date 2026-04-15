@echo off
REM Crypton Identity Launcher
REM Sets all required env vars explicitly — not affected by .env file changes.
REM Redis 3.0 on localhost has no password — use plain URL.

set APP_PORT=8080
set DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/crypton
set REDIS_URL=redis://127.0.0.1:6379
set RUST_LOG=info
set JWT_SECRET=aefa5ac024afaf0556299762a25041422c6a3dfe3017398b7af1a020159e8ad8
set WEBAUTHN_RP_NAME=Crypton Identity
set WEBAUTHN_RP_ID=localhost
set WEBAUTHN_ORIGIN=http://localhost:3000
set OAUTH_CLIENT_ID=demo-site
set OAUTH_CLIENT_SECRET=demo-secret-change-in-prod
set OAUTH_REDIRECT_URIS=http://localhost:4000/callback
set FRONTEND_ORIGIN=http://localhost:3000

cd /d "%~dp0"
target\release\crypton-identity.exe
