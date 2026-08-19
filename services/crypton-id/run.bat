@echo off
REM Crypton Identity Launcher (dev)
REM
REM Secrets live in .env.local, which is gitignored and generated on first
REM run - nothing secret is ever committed. Everything else has a plain-text
REM dev default here since none of it is sensitive.
cd /d "%~dp0"

if not exist ".env.local" (
    echo [crypton-id] No .env.local found - generating a local-only dev JWT secret.
    powershell -NoProfile -Command "$b=(1..32|ForEach-Object{Get-Random -Maximum 256}); 'JWT_SECRET=' + (($b | ForEach-Object { '{0:x2}' -f $_ }) -join '')" > .env.local
)

for /f "usebackq tokens=1,2 delims==" %%A in (".env.local") do set "%%A=%%B"

if not defined APP_PORT set APP_PORT=8080
if not defined DATABASE_URL set DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/crypton
if not defined REDIS_URL set REDIS_URL=redis://127.0.0.1:6379
if not defined RUST_LOG set RUST_LOG=info
if not defined WEBAUTHN_RP_NAME set WEBAUTHN_RP_NAME=Crypton Identity
if not defined WEBAUTHN_RP_ID set WEBAUTHN_RP_ID=localhost
if not defined WEBAUTHN_ORIGIN set WEBAUTHN_ORIGIN=http://localhost:3000

REM OAuth is an experimental, incomplete surface - see routes/oauth.rs.
REM It stays unmounted unless explicitly enabled.
if not defined ENABLE_OAUTH set ENABLE_OAUTH=false

target\release\crypton-identity.exe
