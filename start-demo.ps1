#Requires -Version 5.1
<#
.SYNOPSIS
    Crypton demo launcher — full stack + live Cloudflare tunnel, one command.

.DESCRIPTION
    Startup order:
      1. Docker infra (Postgres + Redis)
      2. React CRA frontend :3000  (services\crypton-admin — new UI with TrustAnimation,
         AttackSim, BeforeAfter, PricingCard2 etc.)
      3. cloudflared quick tunnel -> :3000
      4. Write identity .env with tunnel WebAuthn vars
      5. crypton-identity  :8080
      6. crypton-gateway   :8090

    Uses prebuilt target\release\*.exe if present; falls back to cargo run --release.
    All logs land in .demo\logs\.    Stop with: .\stop-demo.ps1

.PARAMETER NoBrowser
    Skip auto-opening the browser.

.PARAMETER Rebuild
    Delete prebuilt binaries and recompile with cargo before starting.

.PARAMETER LocalOnly
    Skip cloudflared. Use http://localhost:3000 only.
    WebAuthn uses rpId=localhost — works in Chrome/Edge on localhost without HTTPS.
#>
param(
    [switch]$NoBrowser,
    [switch]$Rebuild,
    [switch]$LocalOnly
)

Set-StrictMode -Off
$ErrorActionPreference = "Stop"

$Root     = $PSScriptRoot
$StateDir = Join-Path $Root ".demo"
$LogDir   = Join-Path $StateDir "logs"
$PidFile  = Join-Path $StateDir "pids.json"
$UrlFile  = Join-Path $StateDir "tunnel_url.txt"

function Write-Step { param([int]$N,[int]$T,[string]$M) Write-Host "`n[Step $N/$T] $M" -ForegroundColor Cyan }
function Write-OK   { param($M) Write-Host "  [OK]  $M" -ForegroundColor Green  }
function Write-Warn { param($M) Write-Host "  [!!]  $M" -ForegroundColor Yellow }
function Write-Fail {
    param($M)
    Write-Host "`n  [FAIL] $M" -ForegroundColor Red
    Write-Host "  Logs: $LogDir" -ForegroundColor DarkGray
    exit 1
}
function Wait-ForHttp {
    param([string]$Url,[string]$Label,[int]$TimeoutSec=120)
    Write-Host "  Waiting for $Label ... " -NoNewline
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        try {
            $r = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 3 -ErrorAction Stop
            if ($r.StatusCode -lt 500) { Write-Host " ready!" -ForegroundColor Green; return $true }
        } catch {}
        Write-Host "." -NoNewline; Start-Sleep -Seconds 1
    }
    Write-Host " TIMED OUT" -ForegroundColor Red; return $false
}

New-Item -ItemType Directory -Path $StateDir -Force | Out-Null
New-Item -ItemType Directory -Path $LogDir   -Force | Out-Null

if (Test-Path $PidFile) {
    Write-Warn "Found previous run — stopping first..."
    $ss = Join-Path $Root "stop-demo.ps1"
    if (Test-Path $ss) { & $ss; Start-Sleep -Seconds 2 }
}

$pids = [ordered]@{}

Write-Host "`n============================================================" -ForegroundColor Magenta
Write-Host "  CRYPTON DEMO LAUNCHER" -ForegroundColor White
Write-Host "============================================================" -ForegroundColor Magenta

# ── Prereqs ────────────────────────────────────────────────────────────────────

try { & docker info 2>&1 | Out-Null } catch { Write-Fail "Docker Desktop is not running." }

if (-not $LocalOnly -and -not (Get-Command cloudflared -ErrorAction SilentlyContinue)) {
    Write-Warn "cloudflared not found on PATH — switching to -LocalOnly mode."
    $LocalOnly = $true
}

$totalSteps = if ($LocalOnly) { 5 } else { 6 }
Write-OK "Prereqs OK"

# ── Step 1: Docker ─────────────────────────────────────────────────────────────

Write-Step 1 $totalSteps "Docker infra — Postgres + Redis"

$composeFile = Join-Path $Root "infra\docker-compose.yml"
if (-not (Test-Path $composeFile)) { Write-Fail "infra\docker-compose.yml not found" }

& docker compose -f $composeFile up -d --wait 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) {
    & docker compose -f $composeFile up -d 2>&1 | Out-Null
    Start-Sleep -Seconds 8
}
Write-OK "Postgres :5432 and Redis :6379 are up"

# ── Step 2: React frontend — REPO ROOT (new UI) ────────────────────────────────

Write-Step 2 $totalSteps "React CRA frontend (new UI) — port 3000"

$feDir    = Join-Path $Root "services\crypton-admin"
$feLog    = Join-Path $LogDir "frontend.log"
$feErrLog = Join-Path $LogDir "frontend-err.log"

if (-not (Test-Path (Join-Path $feDir "src\App.js"))) {
    Write-Fail "src\App.js not found in services\crypton-admin"
}
if (-not (Test-Path (Join-Path $feDir "node_modules"))) {
    Write-Warn "node_modules missing — running npm install (~60s)..."
    $npm = Start-Process "cmd.exe" -ArgumentList "/c npm install" `
        -WorkingDirectory $feDir -Wait -PassThru -WindowStyle Hidden
    if ($npm.ExitCode -ne 0) { Write-Fail "npm install failed at repo root" }
    Write-OK "npm install done"
}

$env:BROWSER                        = "none"
$env:DANGEROUSLY_DISABLE_HOST_CHECK = "true"

$feProc = Start-Process "cmd.exe" -ArgumentList "/c npm start" `
    -WorkingDirectory $feDir `
    -RedirectStandardOutput $feLog `
    -RedirectStandardError  $feErrLog `
    -WindowStyle Hidden -PassThru

$pids["frontend"] = $feProc.Id
Write-OK "Frontend started (PID $($feProc.Id))"
Start-Sleep -Milliseconds 2500
if ($feProc.HasExited) { Write-Fail "Frontend exited immediately. Check: $feErrLog" }

if (-not (Wait-ForHttp "http://127.0.0.1:3000" "Frontend :3000" 120)) {
    Write-Fail "Frontend did not start. Check: $feErrLog"
}

# ── Step 3: Cloudflare tunnel ──────────────────────────────────────────────────

$tunnelUrl      = "http://localhost:3000"
$tunnelHostname = "localhost"

if (-not $LocalOnly) {
    Write-Step 3 $totalSteps "Cloudflare quick tunnel → localhost:3000"

    $cfLog    = Join-Path $LogDir "cloudflared.log"
    $cfErrLog = Join-Path $LogDir "cloudflared-err.log"
    foreach ($f in @($cfLog,$cfErrLog)) { if (Test-Path $f) { Remove-Item $f -Force } }

    $cfProc = Start-Process "cloudflared" `
        -ArgumentList @("tunnel","--url","http://localhost:3000") `
        -RedirectStandardOutput $cfLog `
        -RedirectStandardError  $cfErrLog `
        -WindowStyle Hidden -PassThru

    $pids["cloudflared"] = $cfProc.Id
    Write-Host "  Scanning cloudflared output for tunnel URL (up to 60s)..." -NoNewline

    $deadline = (Get-Date).AddSeconds(60)
    $urlRegex = 'https://[a-z0-9][a-z0-9\-]+\.trycloudflare\.com'

    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 800; Write-Host "." -NoNewline
        if ($cfProc.HasExited) {
            Write-Host ""
            Write-Fail "cloudflared exited (code $($cfProc.ExitCode)). Check: $cfErrLog"
        }
        foreach ($lf in @($cfLog,$cfErrLog)) {
            if (-not (Test-Path $lf)) { continue }
            $raw = Get-Content $lf -Raw -ErrorAction SilentlyContinue
            if ($raw -and ($raw -match $urlRegex)) { $tunnelUrl = $Matches[0]; break }
        }
        if ($tunnelUrl -ne "http://localhost:3000") { break }
    }

    if ($tunnelUrl -eq "http://localhost:3000") {
        Write-Host " TIMED OUT" -ForegroundColor Yellow
        Write-Warn "Tunnel URL not found — falling back to localhost-only mode."
        $LocalOnly = $true
    } else {
        Write-Host " found!" -ForegroundColor Green
        $tunnelHostname = ([System.Uri]$tunnelUrl).Host
        Write-OK "Tunnel URL  : $tunnelUrl"
        Write-OK "WebAuthn RP : $tunnelHostname"
    }
}

Set-Content -Path $UrlFile -Value $tunnelUrl -Encoding UTF8

# ── Step 4: Identity env ───────────────────────────────────────────────────────

$s4 = if ($LocalOnly -and (-not $pids.Contains("cloudflared"))) { 3 } else { 4 }
Write-Step $s4 $totalSteps "Writing identity env (WebAuthn rpId=$tunnelHostname)"

$identityDir     = Join-Path $Root "services\crypton-identity"
$identityEnvPath = Join-Path $identityDir ".env"

$jwtSecret = "aefa5ac024afaf0556299762a25041422c6a3dfe3017398b7af1a020159e8ad8"
if (Test-Path $identityEnvPath) {
    foreach ($line in (Get-Content $identityEnvPath -ErrorAction SilentlyContinue)) {
        if ($line -match '^JWT_SECRET=(.+)$') { $jwtSecret = $Matches[1].Trim(); break }
    }
}

$webauthnOrigin = if ($LocalOnly) { "http://localhost:3000" } else { $tunnelUrl }
$webauthnRpId   = $tunnelHostname

Set-Content -Path $identityEnvPath -Value @"
APP_PORT=8080
DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/crypton
REDIS_URL=redis://127.0.0.1:6379
RUST_LOG=info
JWT_SECRET=$jwtSecret
WEBAUTHN_RP_NAME=Crypton Identity
WEBAUTHN_RP_ID=$webauthnRpId
WEBAUTHN_ORIGIN=$webauthnOrigin
"@ -Encoding UTF8

$env:APP_PORT         = "8080"
$env:DATABASE_URL     = "postgres://postgres:postgres@127.0.0.1:5432/crypton"
$env:REDIS_URL        = "redis://127.0.0.1:6379"
$env:RUST_LOG         = "info"
$env:JWT_SECRET       = $jwtSecret
$env:WEBAUTHN_RP_NAME = "Crypton Identity"
$env:WEBAUTHN_RP_ID   = $webauthnRpId
$env:WEBAUTHN_ORIGIN  = $webauthnOrigin
Write-OK ".env written and env vars set"

# ── Step 5: crypton-identity ───────────────────────────────────────────────────

$s5 = if ($LocalOnly -and (-not $pids.Contains("cloudflared"))) { 4 } else { 5 }
Write-Step $s5 $totalSteps "crypton-identity — port 8080"

$identityBin = Join-Path $identityDir "target\release\crypton-identity.exe"
$idLog       = Join-Path $LogDir "identity.log"
$idErrLog    = Join-Path $LogDir "identity-err.log"

if ($Rebuild -and (Test-Path $identityBin)) { Remove-Item $identityBin -Force }

$idTimeout = 30
if (Test-Path $identityBin) {
    Write-OK "Using prebuilt: $identityBin"
    $idProc = Start-Process $identityBin `
        -WorkingDirectory $identityDir `
        -RedirectStandardOutput $idLog -RedirectStandardError $idErrLog `
        -WindowStyle Hidden -PassThru
} else {
    Write-Warn "No prebuilt binary — cargo run --release (first time: 2-4 min)..."
    $idTimeout = 360
    $idProc = Start-Process "cargo" -ArgumentList @("run","--release") `
        -WorkingDirectory $identityDir `
        -RedirectStandardOutput $idLog -RedirectStandardError $idErrLog `
        -WindowStyle Hidden -PassThru
}

$pids["identity"] = $idProc.Id
Write-OK "Identity started (PID $($idProc.Id))"
Start-Sleep -Milliseconds 1500
if ($idProc.HasExited) { Write-Fail "Identity exited immediately. Check: $idErrLog" }

if (-not (Wait-ForHttp "http://127.0.0.1:8080/health" "Identity :8080" $idTimeout)) {
    Write-Fail "Identity unhealthy in ${idTimeout}s. Check: $idErrLog"
}

foreach ($v in @("WEBAUTHN_RP_ID","WEBAUTHN_ORIGIN","WEBAUTHN_RP_NAME",
                 "JWT_SECRET","DATABASE_URL","REDIS_URL","APP_PORT")) {
    Remove-Item "Env:$v" -ErrorAction SilentlyContinue
}

# ── Step 6: crypton-gateway ────────────────────────────────────────────────────

$s6 = if ($LocalOnly -and (-not $pids.Contains("cloudflared"))) { 5 } else { 6 }
Write-Step $s6 $totalSteps "crypton-gateway — port 8090"

$gatewayDir = Join-Path $Root "services\crypton-gateway"
$gatewayBin = Join-Path $gatewayDir "target\release\crypton-gateway.exe"
$gwLog      = Join-Path $LogDir "gateway.log"
$gwErrLog   = Join-Path $LogDir "gateway-err.log"

if ($Rebuild -and (Test-Path $gatewayBin)) { Remove-Item $gatewayBin -Force }

$env:APP_PORT     = "8090"
$env:IDENTITY_URL = "http://127.0.0.1:8080"
$env:RUST_LOG     = "info"

$gwTimeout = 20
if (Test-Path $gatewayBin) {
    Write-OK "Using prebuilt: $gatewayBin"
    $gwProc = Start-Process $gatewayBin `
        -WorkingDirectory $gatewayDir `
        -RedirectStandardOutput $gwLog -RedirectStandardError $gwErrLog `
        -WindowStyle Hidden -PassThru
} else {
    Write-Warn "No prebuilt gateway binary — cargo run --release..."
    $gwTimeout = 360
    $gwProc = Start-Process "cargo" -ArgumentList @("run","--release") `
        -WorkingDirectory $gatewayDir `
        -RedirectStandardOutput $gwLog -RedirectStandardError $gwErrLog `
        -WindowStyle Hidden -PassThru
}

$pids["gateway"] = $gwProc.Id
Write-OK "Gateway started (PID $($gwProc.Id))"
Start-Sleep -Milliseconds 1000
if ($gwProc.HasExited) { Write-Fail "Gateway exited immediately. Check: $gwErrLog" }

if (-not (Wait-ForHttp "http://127.0.0.1:8090/health" "Gateway :8090" $gwTimeout)) {
    Write-Fail "Gateway unhealthy in ${gwTimeout}s. Check: $gwErrLog"
}

# ── Save and output ────────────────────────────────────────────────────────────

$pids | ConvertTo-Json | Set-Content -Path $PidFile -Encoding UTF8

$publicUrl = if ($LocalOnly -and $tunnelUrl -eq "http://localhost:3000") { "http://localhost:3000" } else { $tunnelUrl }

Write-Host ""
Write-Host "============================================================" -ForegroundColor Magenta
Write-Host "  CRYPTON IS LIVE" -ForegroundColor White
Write-Host "============================================================" -ForegroundColor Magenta
Write-Host ""
Write-Host "  Public URL  : $publicUrl" -ForegroundColor Yellow
if ($publicUrl -ne "http://localhost:3000") {
    Write-Host "  (share with phone / other devices)" -ForegroundColor DarkGray
}
Write-Host ""
Write-Host "  Local URL   : http://localhost:3000" -ForegroundColor Gray
Write-Host "  Identity    : http://localhost:8080/health" -ForegroundColor DarkGray
Write-Host "  Gateway     : http://localhost:8090/health" -ForegroundColor DarkGray
Write-Host "  Logs        : $LogDir" -ForegroundColor DarkGray
Write-Host "  Tunnel URL  : $UrlFile" -ForegroundColor DarkGray
Write-Host ""
Write-Host "  To stop     : .\stop-demo.ps1" -ForegroundColor DarkGray
Write-Host "============================================================" -ForegroundColor Magenta

if (-not $NoBrowser) {
    Write-Host "`n  Opening browser..." -ForegroundColor Cyan
    Start-Process $publicUrl
}
