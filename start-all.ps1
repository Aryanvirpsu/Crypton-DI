#Requires -Version 5.1
<#
.SYNOPSIS
    Crypton - start all services (local dev).

.DESCRIPTION
    Startup order:
      1. Docker infra      - Postgres :5432 + Redis :6379
      2. crypton-id        - Rust identity backend :8080
      3. crypton-gateway   - Rust API gateway :8090
      4. crypton-main      - Marketing site :3000
      5. crypton-demo      - SaaS demo app  :3001

    PIDs saved to .run\pids.json for stop-all.ps1.
    Logs written to .run\logs\.

.PARAMETER Rebuild
    Recompile Rust binaries from source before starting.

.PARAMETER KillExisting
    Kill any process on ports 3000-3002, 8080, 8090 before starting.

.PARAMETER NoBrowser
    Skip auto-opening browser tabs after startup.

.EXAMPLE
    .\start-all.ps1
    .\start-all.ps1 -KillExisting
    .\start-all.ps1 -Rebuild -KillExisting
#>
[CmdletBinding()]
param(
    [switch]$Rebuild,
    [switch]$KillExisting,
    [switch]$NoBrowser,
    [int]$BackendTimeout  = 240,
    [int]$FrontendTimeout = 90
)

Set-StrictMode -Off
$ErrorActionPreference = "Continue"   # Stop causes Docker stderr output to throw; we use explicit FAIL calls
$ProgressPreference    = "SilentlyContinue"

$Root    = $PSScriptRoot
$RunDir  = Join-Path $Root ".run"
$LogDir  = Join-Path $RunDir "logs"
$PidFile = Join-Path $RunDir "pids.json"

New-Item -ItemType Directory -Path $LogDir -Force | Out-Null
$LaunchLog = Join-Path $LogDir "launcher.log"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
function Log  { param($m) Add-Content -Path $LaunchLog -Value "[$((Get-Date -f 'HH:mm:ss'))] $m" -EA SilentlyContinue }
function OK   { param($m) Write-Host "  [OK]  $m" -ForegroundColor Green;    Log "OK   $m" }
function WARN { param($m) Write-Host "  [!!]  $m" -ForegroundColor Yellow;   Log "WARN $m" }
function INFO { param($m) Write-Host "        $m" -ForegroundColor DarkGray; Log "     $m" }
function FAIL { param($m, $h = "")
    Write-Host "`n  [!!] FATAL: $m" -ForegroundColor Red
    if ($h) { Write-Host "       > $h" -ForegroundColor DarkGray }
    Log "FAIL $m"
    exit 1
}
function Banner { param($t) Write-Host "`n  -- $t" -ForegroundColor Cyan; Log "=== $t ===" }

function Wait-Http {
    param([string]$Url, [string]$Label, [int]$TimeoutSec = 60)
    Write-Host "  [..] $Label" -NoNewline
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        try {
            $r = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 3 -EA Stop
            if ($r.StatusCode -lt 500) {
                Write-Host " ready ($($r.StatusCode))" -ForegroundColor Green
                return $true
            }
        } catch {}
        Write-Host "." -NoNewline
        Start-Sleep -Milliseconds 1500
    }
    Write-Host " TIMED OUT" -ForegroundColor Red
    return $false
}

function Test-Port { param([int]$p)
    try { return $null -ne (Get-NetTCPConnection -LocalPort $p -State Listen -EA SilentlyContinue) } catch { return $false }
}

function Kill-Port { param([int]$p)
    try {
        Get-NetTCPConnection -LocalPort $p -State Listen -EA SilentlyContinue |
            Select-Object -ExpandProperty OwningProcess -Unique |
            ForEach-Object {
                if ($_ -gt 4) { & taskkill /f /t /pid $_ 2>&1 | Out-Null; WARN "Killed PID $_ on :$p" }
            }
    } catch {}
    # Wait until OS releases the socket
    $deadline = (Get-Date).AddSeconds(8)
    while ((Get-Date) -lt $deadline) {
        if (-not (Test-Port $p)) { return }
        Start-Sleep -Milliseconds 300
    }
    WARN ":$p may still be in use - proceeding anyway"
}

function Start-Svc { param($Label, $Exe, $SvcArgs, $Cwd, $Out, $Err)
    $sp = @{
        FilePath               = $Exe
        WorkingDirectory       = $Cwd
        RedirectStandardOutput = $Out
        RedirectStandardError  = $Err
        WindowStyle            = "Hidden"
        PassThru               = $true
    }
    if ($SvcArgs -and $SvcArgs.Count -gt 0) { $sp["ArgumentList"] = $SvcArgs }
    $proc = Start-Process @sp
    Log "Started $Label PID=$($proc.Id)"
    return $proc
}

function Build-Rust { param($Dir, $Label)
    $bout = Join-Path $LogDir "$Label-build.out"
    $berr = Join-Path $LogDir "$Label-build.err"
    INFO "Building $Label (cargo build --release) ..."
    $p = Start-Process "cargo" -ArgumentList @("build", "--release") `
         -WorkingDirectory $Dir `
         -RedirectStandardOutput $bout `
         -RedirectStandardError  $berr `
         -WindowStyle Hidden -PassThru -Wait
    if ($p.ExitCode -ne 0) {
        Get-Content $berr -Tail 25 -EA SilentlyContinue | ForEach-Object { Write-Host "    $_" -ForegroundColor DarkGray }
        FAIL "$Label build failed - see $berr"
    }
    OK "$Label compiled"
}

# ---------------------------------------------------------------------------
# Header
# ---------------------------------------------------------------------------
Write-Host ""
Write-Host "  =================================================" -ForegroundColor Magenta
Write-Host "    CRYPTON  --  START ALL"                          -ForegroundColor White
Write-Host "    $((Get-Date -Format 'ddd dd MMM yyyy  HH:mm:ss'))" -ForegroundColor DarkGray
Write-Host "  =================================================" -ForegroundColor Magenta
Write-Host ""
Write-Host "    :3000  crypton-main    marketing site"   -ForegroundColor DarkGray
Write-Host "    :3001  crypton-demo    SaaS demo app"    -ForegroundColor DarkGray
Write-Host "    :8080  crypton-id      identity backend" -ForegroundColor DarkGray
Write-Host "    :8090  crypton-gateway API gateway"      -ForegroundColor DarkGray
Write-Host ""
Log "=== START-ALL $((Get-Date -f 'yyyy-MM-dd HH:mm:ss')) ==="

# ---------------------------------------------------------------------------
# Service paths
# ---------------------------------------------------------------------------
$IdDir    = Join-Path $Root "services\crypton-id"
$GwDir    = Join-Path $Root "services\crypton-gateway"
$MainDir  = Join-Path $Root "services\crypton-main"
$DemoDir  = Join-Path $Root "services\crypton-demo"
$AdminDir = Join-Path $Root "services\crypton-admin"

$checks = @(
    @{ path = Join-Path $IdDir    "Cargo.toml";   label = "crypton-id" },
    @{ path = Join-Path $GwDir    "Cargo.toml";   label = "crypton-gateway" },
    @{ path = Join-Path $MainDir  "package.json"; label = "crypton-main" },
    @{ path = Join-Path $DemoDir  "package.json"; label = "crypton-demo" }
)
foreach ($c in $checks) {
    if (-not (Test-Path $c.path)) { FAIL "$($c.label) not found at $($c.path)" }
}
OK "All service directories found"

$IdBin = Join-Path $IdDir "target\release\crypton-identity.exe"
$GwBin = Join-Path $GwDir "target\release\crypton-gateway.exe"

# ---------------------------------------------------------------------------
# Kill ports (optional)
# ---------------------------------------------------------------------------
if ($KillExisting) {
    Banner "Clearing ports"
    foreach ($p in @(3000, 3001, 8080, 8090)) {
        if (Test-Port $p) { Kill-Port $p } else { INFO ":$p free" }
    }
}

# ---------------------------------------------------------------------------
# Prerequisites
# ---------------------------------------------------------------------------
Banner "Checking prerequisites"
$null = & docker info 2>$null
if ($LASTEXITCODE -ne 0) { FAIL "Docker Desktop not running - start it and retry." }
OK "Docker running"

foreach ($cmd in @("npm", "cargo")) {
    if (-not (Get-Command $cmd -EA SilentlyContinue)) { FAIL "$cmd not found" }
}
OK "npm $(& npm --version 2>&1)   cargo $(& cargo --version 2>&1)"

# ---------------------------------------------------------------------------
# Step 1: Docker infra
# ---------------------------------------------------------------------------
Banner "Step 1/4 -- Docker infra  (Postgres :5432 + Redis :6379)"
$compose = Join-Path $Root "infra\docker-compose.yml"
if (-not (Test-Path $compose)) { FAIL "infra\docker-compose.yml not found" }

& docker compose -f $compose up -d --wait 2>&1 | Out-File (Join-Path $LogDir "infra.log") -Append
if ($LASTEXITCODE -ne 0) {
    & docker compose -f $compose up -d 2>&1 | Out-File (Join-Path $LogDir "infra.log") -Append
    Start-Sleep -Seconds 6
}
OK "Postgres :5432 + Redis :6379 up"

# ---------------------------------------------------------------------------
# Step 2: Rust binaries
# ---------------------------------------------------------------------------
if ($Rebuild) {
    Banner "Step 2/4 -- Building Rust services"
    foreach ($b in @($IdBin, $GwBin)) {
        if (Test-Path $b) { Remove-Item $b -Force; INFO "Removed stale $(Split-Path $b -Leaf)" }
    }
    Build-Rust $IdDir "crypton-id"
    Build-Rust $GwDir "crypton-gateway"
} else {
    Banner "Step 2/4 -- Rust binaries"
    if (-not (Test-Path $IdBin)) { WARN "No crypton-id binary - will compile with cargo (~2-4 min first time)" }
    else { OK "crypton-id binary found" }
    if (-not (Test-Path $GwBin)) { WARN "No crypton-gateway binary - will compile with cargo" }
    else { OK "crypton-gateway binary found" }
}

# ---------------------------------------------------------------------------
# Step 3: Backend
# ---------------------------------------------------------------------------
Banner "Step 3/4 -- Backend"

# Force-clear backend ports regardless of -KillExisting
foreach ($p in @(8080, 8090)) {
    if (Test-Port $p) { WARN ":$p occupied - killing"; Kill-Port $p }
}

$pids = [ordered]@{}

# Load identity env vars from dev.env, launch it, then IMMEDIATELY clean up before gateway
# (gateway also reads APP_PORT - must not inherit 8080)
$devEnvPath = Join-Path $Root "dev.env"
if (-not (Test-Path $devEnvPath)) {
    FAIL "dev.env not found at $devEnvPath" "Copy dev.env.example to dev.env and fill in secrets"
}
$loadedVars = @()
Get-Content $devEnvPath | ForEach-Object {
    $line = $_.Trim()
    if ($line -and -not $line.StartsWith('#')) {
        $parts = $line -split '=', 2
        if ($parts.Count -eq 2) {
            $varName  = $parts[0].Trim()
            $varValue = $parts[1].Trim()
            [System.Environment]::SetEnvironmentVariable($varName, $varValue, 'Process')
            $loadedVars += $varName
        }
    }
}
INFO "Loaded $($loadedVars.Count) env vars from dev.env"

if (Test-Path $IdBin) {
    $idProc = Start-Svc "crypton-id" $IdBin @() $IdDir (Join-Path $LogDir "id.out") (Join-Path $LogDir "id.err")
} else {
    $idProc = Start-Svc "crypton-id" "cargo" @("run", "--release") $IdDir (Join-Path $LogDir "id.out") (Join-Path $LogDir "id.err")
}
$pids["identity"] = $idProc.Id

# Clean up identity env BEFORE launching gateway so it reads its own .env (APP_PORT=8090)
foreach ($v in $loadedVars) {
    Remove-Item "Env:$v" -EA SilentlyContinue
}

if (Test-Path $GwBin) {
    $gwProc = Start-Svc "crypton-gateway" $GwBin @() $GwDir (Join-Path $LogDir "gw.out") (Join-Path $LogDir "gw.err")
} else {
    $gwProc = Start-Svc "crypton-gateway" "cargo" @("run", "--release") $GwDir (Join-Path $LogDir "gw.out") (Join-Path $LogDir "gw.err")
}
$pids["gateway"] = $gwProc.Id

Start-Sleep -Seconds 2
if ($idProc.HasExited) {
    Get-Content (Join-Path $LogDir "id.err") -Tail 20 -EA SilentlyContinue |
        ForEach-Object { Write-Host "    $_" -ForegroundColor DarkGray }
    FAIL "crypton-id exited immediately - check .run\logs\id.err"
}

if (-not (Wait-Http "http://127.0.0.1:8080/health" "crypton-id      :8080" $BackendTimeout)) {
    FAIL "crypton-id did not become healthy in ${BackendTimeout}s"
}
if (-not (Wait-Http "http://127.0.0.1:8090/health" "crypton-gateway :8090" 30)) {
    WARN "crypton-gateway health check timed out (non-fatal)"
}

# ---------------------------------------------------------------------------
# Step 4: Frontends
# ---------------------------------------------------------------------------
Banner "Step 4/4 -- Frontends"
$env:BROWSER = "none"

# Ensure SDK is built once before services start
INFO "Building @crypton/sdk..."
$sdkOut = Join-Path $LogDir "sdk-build.out"
$sdkErr = Join-Path $LogDir "sdk-build.err"
$np     = Start-Process "cmd.exe" -ArgumentList @("/c", "npm run build:sdk") `
          -WorkingDirectory $Root `
          -RedirectStandardOutput $sdkOut `
          -RedirectStandardError  $sdkErr `
          -WindowStyle Hidden -PassThru -Wait
if ($np.ExitCode -ne 0) {
    Get-Content $sdkErr -Tail 30 -EA SilentlyContinue | ForEach-Object { Write-Host "    $_" -ForegroundColor DarkGray }
    FAIL "@crypton/sdk build failed - see $sdkErr"
}
OK "@crypton/sdk built successfully"

function Start-Frontend { param($Label, $Dir, $Port, $NpmScript = "start")
    if (-not (Test-Path (Join-Path $Dir "node_modules"))) {
        WARN "node_modules missing in $Label - running npm install..."
        $iout = Join-Path $LogDir "$Label-install.out"
        $ierr = Join-Path $LogDir "$Label-install.err"
        $np   = Start-Process "cmd.exe" -ArgumentList @("/c", "npm install") `
                -WorkingDirectory $Dir `
                -RedirectStandardOutput $iout `
                -RedirectStandardError  $ierr `
                -WindowStyle Hidden -PassThru -Wait
        if ($np.ExitCode -ne 0) { FAIL "$Label npm install failed - see $ierr" }
        OK "$Label npm install done"
    }
    $env:PORT = "$Port"
    $npmCmd   = if ($NpmScript -eq "start") { "npm start" } else { "npm run $NpmScript" }
    $fout     = Join-Path $LogDir "$Label.out"
    $ferr     = Join-Path $LogDir "$Label.err"
    $proc     = Start-Svc $Label "cmd.exe" @("/c", $npmCmd) $Dir $fout $ferr
    Remove-Item Env:PORT -EA SilentlyContinue
    Start-Sleep -Milliseconds 1200
    if ($proc.HasExited) { FAIL "$Label exited immediately - check $ferr" }
    return $proc
}

$mainProc  = Start-Frontend "crypton-main"  $MainDir  3000 "start"
$demoProc  = Start-Frontend "crypton-demo"  $DemoDir  3001 "client:start"

$pids["crypton-main"]  = $mainProc.Id
$pids["crypton-demo"]  = $demoProc.Id

Remove-Item Env:BROWSER -EA SilentlyContinue

if (-not (Wait-Http "http://127.0.0.1:3000" "crypton-main  :3000" $FrontendTimeout)) { WARN "crypton-main  did not respond in time" }
if (-not (Wait-Http "http://127.0.0.1:3001" "crypton-demo  :3001" $FrontendTimeout)) { WARN "crypton-demo  did not respond in time" }

# ---------------------------------------------------------------------------
# Save PIDs
# ---------------------------------------------------------------------------
$pids | ConvertTo-Json | Set-Content -Path $PidFile -Encoding UTF8
OK "PIDs saved to .run\pids.json"

# ---------------------------------------------------------------------------
# Done
# ---------------------------------------------------------------------------
Write-Host ""
Write-Host "  =================================================" -ForegroundColor Magenta
Write-Host "    ALL SYSTEMS RUNNING"                             -ForegroundColor Green
Write-Host "  =================================================" -ForegroundColor Magenta
Write-Host ""
Write-Host "    http://localhost:3000   crypton-main   (marketing)" -ForegroundColor Cyan
Write-Host "    http://localhost:3001   crypton-demo   (SaaS demo & Panel)" -ForegroundColor Cyan
Write-Host ""
Write-Host "    status.ps1    -- health check all services" -ForegroundColor DarkGray
Write-Host "    stop-all.ps1  -- stop everything"           -ForegroundColor DarkGray
Write-Host "    Logs: .run\logs\"                           -ForegroundColor DarkGray
Write-Host ""

if (-not $NoBrowser) {
    Start-Process "http://localhost:3000"
    Start-Sleep -Milliseconds 500
    Start-Process "http://localhost:3001"
}
