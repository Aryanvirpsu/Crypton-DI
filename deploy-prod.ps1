<#
.SYNOPSIS
    Deploy Crypton from the `main` branch to production.

.DESCRIPTION
    1. Checks out `main` and pulls latest.
    2. Validates .env for production-grade values (no defaults/placeholders).
    3. Verifies migrations 007 and 008 exist.
    4. Builds Rust binaries (release) and React frontend.
    5. Delegates full startup to start-demo.ps1 -UseTunnel -Rebuild.

    SAFETY: Will abort if JWT_SECRET or OAUTH_CLIENT_SECRET appear to be
    dev/demo defaults.

.PARAMETER SkipBuild
    Skip cargo + npm build. Use only if binaries are already current.

.PARAMETER KillExisting
    Kill any processes already using ports 3000, 4000, 8080, 8090.

.EXAMPLE
    .\deploy-prod.ps1
    .\deploy-prod.ps1 -SkipBuild -KillExisting
#>

param(
    [switch]$SkipBuild,
    [switch]$KillExisting
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$Root = $PSScriptRoot

function Fail([string]$msg) { Write-Host "DEPLOY FAILED: $msg" -ForegroundColor Red; exit 1 }
function Info([string]$msg) { Write-Host $msg -ForegroundColor Cyan }
function Ok([string]$msg)   { Write-Host "OK  $msg" -ForegroundColor Green }

# ── 1. Branch ─────────────────────────────────────────────────────────────────
Info "=== PRODUCTION DEPLOY (branch: main) ==="
Push-Location $Root
$branch = (git rev-parse --abbrev-ref HEAD).Trim()
if ($branch -ne "main") {
    Info "Switching to main..."
    git checkout main 2>&1 | Out-Null
}
git pull origin main 2>&1 | Out-Null
$sha = (git rev-parse --short HEAD).Trim()
Ok "main @ $sha"

# ── 2. Env validation ─────────────────────────────────────────────────────────
Info "Validating .env..."
$envFile = "$Root\services\crypton-identity\.env"
if (-not (Test-Path $envFile)) {
    Fail ".env not found at $envFile`nCopy from .env.production.example and fill in values."
}

$envContent = Get-Content $envFile | Where-Object { $_ -match '=' -and $_ -notmatch '^\s*#' }
$envMap = @{}
foreach ($line in $envContent) {
    $k, $v = $line -split '=', 2
    $envMap[$k.Trim()] = $v.Trim()
}

$required = @("WEBAUTHN_RP_ID","WEBAUTHN_ORIGIN","FRONTEND_ORIGIN","DATABASE_URL","REDIS_URL","JWT_SECRET","OAUTH_CLIENT_SECRET")
foreach ($key in $required) {
    if (-not $envMap.ContainsKey($key) -or $envMap[$key] -eq "" -or $envMap[$key] -match "^<") {
        Fail "$key is missing or still a placeholder — fill in .env before deploying to production."
    }
}

# Hard stop on known-bad secret values
$badSecrets = @(
    "change_me_to_a_long_random_secret",
    "demo-secret-change-in-prod",
    "demo-secret-staging"
)
foreach ($bad in $badSecrets) {
    if ($envMap["JWT_SECRET"] -eq $bad -or $envMap["OAUTH_CLIENT_SECRET"] -eq $bad) {
        Fail "Detected a dev/demo placeholder in JWT_SECRET or OAUTH_CLIENT_SECRET. Set real secrets before production deploy."
    }
}

# Warn if WEBAUTHN_RP_ID is still a tunnel (ephemeral — passkeys will break on restart)
if ($envMap["WEBAUTHN_RP_ID"] -match "trycloudflare\.com") {
    Write-Host ""
    Write-Host "  WARNING: WEBAUTHN_RP_ID is a Cloudflare quick-tunnel hostname." -ForegroundColor Yellow
    Write-Host "  Passkeys enrolled under this RP_ID will break when the tunnel URL changes." -ForegroundColor Yellow
    Write-Host "  For production, use a stable domain. Continuing anyway..." -ForegroundColor Yellow
    Write-Host ""
}

Ok "All required env vars present"

# ── 3. Migration files ────────────────────────────────────────────────────────
Info "Verifying migrations..."
$migDir = "$Root\services\crypton-identity\migrations"
foreach ($mig in @("007_add_lost_status.sql","008_recovery_requests.sql")) {
    if (-not (Test-Path "$migDir\$mig")) { Fail "Migration not found: $mig" }
}
Ok "Migrations 007 + 008 present"

# ── 4. Build ──────────────────────────────────────────────────────────────────
if (-not $SkipBuild) {
    Info "Building crypton-identity (cargo release)..."
    Push-Location "$Root\services\crypton-identity"
    cargo build --release 2>&1
    if ($LASTEXITCODE -ne 0) { Fail "cargo build failed" }
    Pop-Location
    Ok "crypton-identity built"

    Info "Building crypton-gateway (cargo release)..."
    Push-Location "$Root\services\crypton-gateway"
    cargo build --release 2>&1
    if ($LASTEXITCODE -ne 0) { Fail "cargo build failed (gateway)" }
    Pop-Location
    Ok "crypton-gateway built"

    Info "Building React frontend (npm run build)..."
    Push-Location "$Root\services\crypton-admin"
    npm run build 2>&1
    if ($LASTEXITCODE -ne 0) { Fail "npm run build failed" }
    Pop-Location
    Ok "React frontend built"
} else {
    Info "Skipping build (--SkipBuild)"
}

# ── 5. Delegate to start-demo.ps1 ─────────────────────────────────────────────
Info "Starting services..."
$startArgs = @("-UseTunnel")
if ($KillExisting) { $startArgs += "-KillExisting" }

& "$Root\start-demo.ps1" @startArgs

Pop-Location
