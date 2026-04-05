<#
.SYNOPSIS
    Deploy Crypton from the `dev` branch to staging (local + Cloudflare tunnel).

.DESCRIPTION
    1. Checks out `dev` and pulls latest.
    2. Validates .env contains required vars (no placeholder values).
    3. Verifies migrations 007 and 008 are present on disk.
    4. Builds Rust binaries (release) and React frontend.
    5. Delegates full startup to start-demo.ps1 -UseTunnel -Rebuild.

.PARAMETER SkipBuild
    Skip cargo + npm build (use existing binaries). Faster for re-deploys.

.PARAMETER LocalOnly
    Start without a Cloudflare tunnel (localhost WebAuthn only).

.PARAMETER KillExisting
    Kill any processes already using ports 3000, 4000, 8080, 8090.

.EXAMPLE
    .\deploy-staging.ps1
    .\deploy-staging.ps1 -SkipBuild
    .\deploy-staging.ps1 -LocalOnly -KillExisting
#>

param(
    [switch]$SkipBuild,
    [switch]$LocalOnly,
    [switch]$KillExisting
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$Root = $PSScriptRoot

function Fail([string]$msg) { Write-Host "DEPLOY FAILED: $msg" -ForegroundColor Red; exit 1 }
function Info([string]$msg) { Write-Host $msg -ForegroundColor Cyan }
function Ok([string]$msg)   { Write-Host "OK  $msg" -ForegroundColor Green }

# ── 1. Branch ─────────────────────────────────────────────────────────────────
Info "=== STAGING DEPLOY (branch: dev) ==="
Push-Location $Root
$branch = (git rev-parse --abbrev-ref HEAD).Trim()
if ($branch -ne "dev") {
    Info "Switching to dev..."
    git checkout dev 2>&1 | Out-Null
}
git pull origin dev 2>&1 | Out-Null
$sha = (git rev-parse --short HEAD).Trim()
Ok "dev @ $sha"

# ── 2. Env validation ─────────────────────────────────────────────────────────
Info "Validating .env..."
$envFile = "$Root\services\crypton-identity\.env"
if (-not (Test-Path $envFile)) {
    Fail ".env not found at $envFile`nCopy from .env.staging.example and fill in values."
}

$envContent = Get-Content $envFile | Where-Object { $_ -match '=' -and $_ -notmatch '^\s*#' }
$envMap = @{}
foreach ($line in $envContent) {
    $k, $v = $line -split '=', 2
    $envMap[$k.Trim()] = $v.Trim()
}

$required = @("WEBAUTHN_RP_ID","WEBAUTHN_ORIGIN","FRONTEND_ORIGIN","DATABASE_URL","REDIS_URL","JWT_SECRET")
foreach ($key in $required) {
    if (-not $envMap.ContainsKey($key) -or $envMap[$key] -eq "" -or $envMap[$key] -match "^<") {
        Fail "$key is missing or still a placeholder in .env"
    }
}
# Warn if JWT_SECRET looks like the default
if ($envMap["JWT_SECRET"] -eq "change_me_to_a_long_random_secret") {
    Fail "JWT_SECRET is still the default value — set a real secret before deploying."
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
$startArgs = @()
if ($LocalOnly)     { $startArgs += "-LocalOnly" }   else { $startArgs += "-UseTunnel" }
if ($KillExisting)  { $startArgs += "-KillExisting" }

& "$Root\start-demo.ps1" @startArgs -NoBrowser:$false

Pop-Location
