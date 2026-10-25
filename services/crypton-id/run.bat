@echo off
REM Crypton Identity Launcher (dev)
REM
REM Secrets live in .env.local, which is gitignored and generated on first
REM run - nothing secret is ever committed. Everything else has a plain-text
REM dev default here since none of it is sensitive.
cd /d "%~dp0"

if not exist "jwt_signing_key.local.pem" (
    echo [crypton-id] No local dev JWT keypair found - generating a fresh Ed25519 pair.
    where openssl >nul 2>nul
    if errorlevel 1 (
        echo [crypton-id] openssl not found on PATH - install it, or set JWT_SIGNING_KEY_PEM_FILE / JWT_PUBLIC_KEY_PEM_FILE manually.
    ) else (
        openssl genpkey -algorithm ed25519 -out jwt_signing_key.local.pem
        openssl pkey -in jwt_signing_key.local.pem -pubout -out jwt_public_key.local.pem
    )
)
if not defined JWT_SIGNING_KEY_PEM_FILE set JWT_SIGNING_KEY_PEM_FILE=%~dp0jwt_signing_key.local.pem
if not defined JWT_PUBLIC_KEY_PEM_FILE set JWT_PUBLIC_KEY_PEM_FILE=%~dp0jwt_public_key.local.pem

if exist ".env.local" (
    for /f "usebackq tokens=1,2 delims==" %%A in (".env.local") do set "%%A=%%B"
)

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
