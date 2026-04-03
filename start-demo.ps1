#Requires -Version 5.1
<#
.SYNOPSIS
    Crypton demo launcher — full stack with optional Cloudflare tunnel.

.DESCRIPTION
    Startup order:
      1.  Docker infra   — Postgres :5432 + Redis :6379
      2.  Rust builds    — cargo build --release  (only if -Rebuild or binary absent)
      3.  React frontend — :3000
      4.  Cloudflare tunnel → :3000  (only if -UseTunnel)
      5.  Write crypton-identity .env  (WebAuthn vars: tunnel hostname or localhost)
      6.  crypton-identity — :8080
      7.  crypton-gateway  — :8090

    ┌─────────────────────────────────────────────────────────────────────────┐
    │  TUNNEL DESIGN NOTE                                                     │
    │  The frontend (port 3000) is tunneled, not the backend directly.        │
    │  The React CRA dev-proxy forwards /auth/*, /devices/*, etc. to the      │
    │  gateway at :8090 → identity at :8080 transparently.                    │
    │  The browser only ever sees the one public tunnel URL, so WebAuthn      │
    │  rpId and origin resolve to that single hostname. This is the           │
    │  simplest and most reliable approach for cross-device passkey testing.  │
    └─────────────────────────────────────────────────────────────────────────┘

    ┌─────────────────────────────────────────────────────────────────────────┐
    │  CROSS-DEVICE PASSKEY NOTES                                             │
    │  • RP_ID and ORIGIN must match the URL the browser actually opens.      │
    │  • Passkeys registered on localhost are bound to rpId=localhost and     │
    │    will NOT work when connecting via a tunnel (different rpId).         │
    │  • When switching modes, register a fresh passkey for that origin.      │
    │  • To reset: docker compose -f infra\docker-compose.yml down -v         │
    └─────────────────────────────────────────────────────────────────────────┘

    Logs  : .demo\logs\
    PIDs  : .demo\pids.json
    Tunnel: .demo\tunnel_url.txt

.PARAMETER Rebuild
    Delete stale Rust binaries and recompile identity + gateway from source
    (cargo build --release). Prevents "I changed code but the old binary ran" bugs.
    Frontend node_modules are reinstalled only when missing.

.PARAMETER LocalOnly
    No tunnel. WebAuthn uses rpId=localhost, origin=http://localhost:3000.
    Works in Chrome/Edge on Windows localhost without HTTPS.
    Mutually exclusive with -UseTunnel (LocalOnly wins if both are given).

.PARAMETER UseTunnel
    Start a Cloudflare quick tunnel exposing :3000 publicly.
    Captures the generated https://*.trycloudflare.com URL and writes correct
    WebAuthn env vars before identity starts. Requires cloudflared on PATH.

.PARAMETER TunnelProvider
    Tunnel provider. Only "cloudflare" is supported currently.

.PARAMETER CleanLogs
    Delete every .log file in .demo\logs\ before starting. Useful for a clean
    debug session. Without this flag, old logs are overwritten per-service.

.PARAMETER KillExisting
    Kill any process currently bound to port 3000, 8080, or 8090 before
    starting. Prevents "port already in use" failures from a prior run.

.PARAMETER NoBrowser
    Do not auto-open a browser window once all services are healthy.

.PARAMETER IdentityTimeout
    Seconds to wait for identity health check. Default 240.
    Increase to 360+ on a first-time build when cargo must compile.

.PARAMETER GatewayTimeout
    Seconds to wait for gateway health check. Default 60.

.PARAMETER FrontendTimeout
    Seconds to wait for frontend health check. Default 120.

.EXAMPLE
    # Simplest local run
    .\start-demo.ps1 -LocalOnly

    # Cross-device demo with tunnel, reuse existing binaries
    .\start-demo.ps1 -UseTunnel

    # Full clean rebuild + tunnel
    .\start-demo.ps1 -UseTunnel -Rebuild -CleanLogs -KillExisting

    # Debug: verbose, kill old processes, rebuild
    .\start-demo.ps1 -LocalOnly -Rebuild -KillExisting -CleanLogs
#>
[CmdletBinding()]
param(
    [switch]$Rebuild,
    [switch]$LocalOnly,
    [switch]$UseTunnel,
    [ValidateSet("cloudflare")]
    [string]$TunnelProvider  = "cloudflare",
    [switch]$CleanLogs,
    [switch]$KillExisting,
    [switch]$NoBrowser,
    [int]$IdentityTimeout    = 240,
    [int]$GatewayTimeout     = 60,
    [int]$FrontendTimeout    = 120
)

Set-StrictMode -Off
$ErrorActionPreference  = "Stop"
$ProgressPreference     = "SilentlyContinue"   # suppress Invoke-WebRequest progress bars

# ── Path constants ────────────────────────────────────────────────────────────
$Root       = $PSScriptRoot
$StateDir   = Join-Path $Root ".demo"
$LogDir     = Join-Path $StateDir "logs"
$PidFile    = Join-Path $StateDir "pids.json"
$UrlFile    = Join-Path $StateDir "tunnel_url.txt"
$LaunchLog  = Join-Path $LogDir "launcher.log"

# ══════════════════════════════════════════════════════════════════════════════
#  HELPER FUNCTIONS
# ══════════════════════════════════════════════════════════════════════════════

function Ensure-Dirs {
    New-Item -ItemType Directory -Path $StateDir -Force | Out-Null
    New-Item -ItemType Directory -Path $LogDir   -Force | Out-Null
}

# Write a timestamped line to launcher.log (fire-and-forget — never throws).
function Write-Log {
    param([string]$Message, [string]$Level = "INFO")
    $ts   = (Get-Date).ToString("yyyy-MM-dd HH:mm:ss")
    $line = "[$ts] [$($Level.PadRight(5))] $Message"
    Add-Content -Path $LaunchLog -Value $line -ErrorAction SilentlyContinue
}

function Write-Banner {
    param([string]$Text)
    Write-Host ""
    Write-Host "  $Text" -ForegroundColor Cyan
    Write-Log "═══ $Text ═══"
}

function Write-OK {
    param([string]$Message)
    Write-Host "  [OK]  $Message" -ForegroundColor Green
    Write-Log "OK    $Message"
}

function Write-Warn {
    param([string]$Message)
    Write-Host "  [!!]  $Message" -ForegroundColor Yellow
    Write-Log "WARN  $Message" "WARN"
}

function Write-Info {
    param([string]$Message)
    Write-Host "        $Message" -ForegroundColor DarkGray
    Write-Log "      $Message"
}

# Print a fatal error with optional log hint, then exit.
function Write-Fail {
    param([string]$Message, [string]$LogHint = "")
    Write-Host ""
    Write-Host "  [!!] FATAL: $Message" -ForegroundColor Red
    if ($LogHint) { Write-Host "       ► $LogHint" -ForegroundColor DarkGray }
    Write-Host "       ► All logs: $LogDir" -ForegroundColor DarkGray
    Write-Log "FATAL $Message" "ERROR"
    exit 1
}

# Print the last N lines of a log file to the console (for post-failure diagnosis).
function Show-LogTail {
    param([string]$Path, [int]$Lines = 25, [string]$Label = "")
    if (-not (Test-Path $Path)) { return }
    Write-Host ""
    $header = if ($Label) { "  ── Last $Lines lines of $Label ──" } else { "  ── Last $Lines lines of $Path ──" }
    Write-Host $header -ForegroundColor DarkRed
    Get-Content $Path -Tail $Lines -ErrorAction SilentlyContinue |
        ForEach-Object { Write-Host "    $_" -ForegroundColor DarkGray }
    Write-Host ""
}

# Poll an HTTP endpoint until it returns status < 500 or the timeout expires.
# Returns $true on success, $false on timeout.
function Wait-ForHttp {
    param(
        [string]$Url,
        [string]$Label,
        [int]$TimeoutSec = 60,
        [int]$PollMs     = 1500
    )
    Write-Host "  [..] $Label — $Url" -NoNewline
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        try {
            $r = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 4 -ErrorAction Stop
            if ($r.StatusCode -lt 500) {
                Write-Host "  ready ($($r.StatusCode))" -ForegroundColor Green
                Write-Log "Health OK: $Label at $Url [$($r.StatusCode)]"
                return $true
            }
        } catch { }
        Write-Host "." -NoNewline
        Start-Sleep -Milliseconds $PollMs
    }
    Write-Host "  TIMED OUT" -ForegroundColor Red
    Write-Log "Health TIMEOUT: $Label at $Url after ${TimeoutSec}s" "ERROR"
    return $false
}

# True if any process is already listening on the given TCP port.
function Test-PortBound {
    param([int]$Port)
    try {
        $c = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue
        return ($null -ne $c -and $c.Count -gt 0)
    } catch { return $false }
}

# Kill all processes listening on $Port (uses taskkill /t for full process trees).
function Stop-PortProcesses {
    param([int]$Port)
    try {
        $ownerPids = Get-NetTCPConnection -LocalPort $Port -State Listen `
                        -ErrorAction SilentlyContinue |
                     Select-Object -ExpandProperty OwningProcess -Unique
        foreach ($pid in $ownerPids) {
            if ($pid -gt 4) {
                $null = & taskkill /f /t /pid $pid 2>&1
                Write-Warn "Killed process tree PID $pid (was on :$Port)"
                Start-Sleep -Milliseconds 500
            }
        }
    } catch { }
}

# Find the React frontend directory: must contain package.json and src/.
function Find-FrontendDir {
    $candidates = @(
        (Join-Path $Root "services\crypton-admin"),
        (Join-Path $Root "services\frontend"),
        (Join-Path $Root "services\app"),
        (Join-Path $Root "frontend"),
        (Join-Path $Root "app"),
        $Root
    )
    foreach ($c in $candidates) {
        if ((Test-Path (Join-Path $c "package.json")) -and
            (Test-Path (Join-Path $c "src"))) {
            return $c
        }
    }
    # Broader search under services/
    $svcDir = Join-Path $Root "services"
    if (Test-Path $svcDir) {
        $found = Get-ChildItem -Path $svcDir -Directory -ErrorAction SilentlyContinue |
                 Where-Object {
                     (Test-Path (Join-Path $_.FullName "package.json")) -and
                     (Test-Path (Join-Path $_.FullName "src"))
                 } | Select-Object -First 1
        if ($found) { return $found.FullName }
    }
    return $null
}

# Find a Rust service directory by conventional name (must contain Cargo.toml).
function Find-RustDir {
    param([string]$Name)
    $candidates = @(
        (Join-Path $Root "services\$Name"),
        (Join-Path $Root $Name)
    )
    foreach ($c in $candidates) {
        if (Test-Path (Join-Path $c "Cargo.toml")) { return $c }
    }
    return $null
}

# Start a service process with separate stdout/stderr log files.
# Returns the Process object. Handles empty argument lists correctly.
function Start-Svc {
    param(
        [string]  $Label,
        [string]  $Exe,
        [string[]]$Arguments,
        [string]  $WorkDir,
        [string]  $OutLog,
        [string]  $ErrLog
    )
    Write-Info "cmd   : $Exe $($Arguments -join ' ')"
    Write-Info "cwd   : $WorkDir"
    Write-Info "stdout: $OutLog"
    Write-Info "stderr: $ErrLog"

    $splat = @{
        FilePath               = $Exe
        WorkingDirectory       = $WorkDir
        RedirectStandardOutput = $OutLog
        RedirectStandardError  = $ErrLog
        WindowStyle            = "Hidden"
        PassThru               = $true
    }
    if ($Arguments -and $Arguments.Count -gt 0) {
        $splat["ArgumentList"] = $Arguments
    }

    $proc = Start-Process @splat
    Write-Log "Started $Label PID=$($proc.Id)"
    return $proc
}

# Compile a Rust workspace member with cargo build --release.
# PowerShell's Start-Process requires distinct paths for stdout and stderr —
# they cannot be the same file. We use separate .out / .err files and merge
# them for display on failure.
# Exits the script on failure.
function Invoke-CargoBuild {
    param([string]$Dir, [string]$Label)
    $buildOut = Join-Path $LogDir "$Label-build-out.log"
    $buildErr = Join-Path $LogDir "$Label-build-err.log"
    Write-Info "cargo build --release"
    Write-Info "  stdout → $buildOut"
    Write-Info "  stderr → $buildErr"
    $proc = Start-Process "cargo" `
        -ArgumentList @("build", "--release") `
        -WorkingDirectory $Dir `
        -RedirectStandardOutput $buildOut `
        -RedirectStandardError  $buildErr `
        -WindowStyle Hidden -PassThru -Wait
    if ($proc.ExitCode -ne 0) {
        # cargo writes errors to stderr; show that first
        Show-LogTail -Path $buildErr -Lines 40 -Label "$Label build stderr"
        Show-LogTail -Path $buildOut -Lines 10 -Label "$Label build stdout"
        Write-Fail "cargo build --release failed for $Label (exit $($proc.ExitCode))" $buildErr
    }
    Write-OK "$Label compiled"
}

# Warn if any .rs source file is newer than the compiled binary.
function Test-BinaryFreshness {
    param([string]$BinPath, [string]$SrcDir, [string]$Label)
    if (-not (Test-Path $BinPath)) { return }
    $binTime = (Get-Item $BinPath).LastWriteTime
    $newer   = Get-ChildItem -Path $SrcDir -Filter "*.rs" -Recurse -ErrorAction SilentlyContinue |
               Where-Object { $_.LastWriteTime -gt $binTime } |
               Select-Object -First 1
    if ($newer) {
        $ageMin = [int]((Get-Date) - $binTime).TotalMinutes
        Write-Warn "$Label binary is ${ageMin}min old — source file '$($newer.Name)' is newer."
        Write-Warn "  Run .\start-demo.ps1 -Rebuild to recompile and avoid stale-binary bugs."
    } else {
        Write-OK "$Label binary is current"
    }
}

# Read an existing JWT_SECRET from a .env file, or generate a secure random one.
function Get-OrCreateJwtSecret {
    param([string]$EnvPath)
    if (Test-Path $EnvPath) {
        foreach ($line in (Get-Content $EnvPath -ErrorAction SilentlyContinue)) {
            if ($line -match '^JWT_SECRET=(.+)$') {
                $secret = $Matches[1].Trim()
                if ($secret.Length -ge 32) { return $secret }
            }
        }
    }
    # Generate 64 random hex chars via .NET CSPRNG
    $bytes = New-Object byte[] 32
    [System.Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($bytes)
    return ($bytes | ForEach-Object { $_.ToString("x2") }) -join ""
}

# Write the crypton-identity .env file with all required variables.
# IMPORTANT: uses [System.IO.File]::WriteAllText with UTF8Encoding($false)
# to write WITHOUT a BOM. PowerShell 5.1's Set-Content -Encoding UTF8 writes
# a UTF-8 BOM at the start of the file, which can confuse parsers that do not
# explicitly strip it (including some versions of dotenvy/dotenv).
function Write-IdentityEnv {
    param(
        [string]$EnvPath,
        [string]$RpId,
        [string]$Origin,
        [string]$JwtSecret,
        [string]$FrontendOrigin = "http://localhost:3000"
    )
    $content  = "APP_PORT=8080`r`n"
    $content += "DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/crypton`r`n"
    $content += "REDIS_URL=redis://127.0.0.1:6379`r`n"
    $content += "RUST_LOG=info`r`n"
    $content += "JWT_SECRET=$JwtSecret`r`n"
    $content += "WEBAUTHN_RP_NAME=Crypton Identity`r`n"
    $content += "WEBAUTHN_RP_ID=$RpId`r`n"
    $content += "WEBAUTHN_ORIGIN=$Origin`r`n"
    $content += "OAUTH_CLIENT_ID=demo-site`r`n"
    $content += "OAUTH_CLIENT_SECRET=demo-secret-change-in-prod`r`n"
    $content += "OAUTH_REDIRECT_URIS=http://localhost:4000/callback`r`n"
    $content += "FRONTEND_ORIGIN=$FrontendOrigin`r`n"
    $utf8NoBom = New-Object System.Text.UTF8Encoding $false
    [System.IO.File]::WriteAllText($EnvPath, $content, $utf8NoBom)
    Write-OK ".env written → WEBAUTHN_RP_ID=$RpId"
    Write-OK "              WEBAUTHN_ORIGIN=$Origin"
    Write-OK "              FRONTEND_ORIGIN=$FrontendOrigin"
}

# Scan cloudflared log files for the public tunnel URL.
# Returns the https://*.trycloudflare.com URL, or $null on timeout.
function Get-CloudflareTunnelUrl {
    param(
        [string]$OutLog,
        [string]$ErrLog,
        [int]$TimeoutSec = 70
    )
    $pattern  = 'https://[a-z0-9][a-z0-9\-]+\.trycloudflare\.com'
    $deadline = (Get-Date).AddSeconds($TimeoutSec)

    Write-Host "  [..] Waiting for Cloudflare tunnel URL (up to ${TimeoutSec}s)" -NoNewline
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 900
        Write-Host "." -NoNewline
        foreach ($f in @($OutLog, $ErrLog)) {
            if (-not (Test-Path $f)) { continue }
            $raw = Get-Content $f -Raw -ErrorAction SilentlyContinue
            if ($raw -and ($raw -match $pattern)) {
                Write-Host "  found!" -ForegroundColor Green
                return $Matches[0]
            }
        }
    }
    Write-Host "  TIMED OUT" -ForegroundColor Red
    return $null
}

# Validate that WebAuthn env vars are consistent with tunnel/local mode.
function Assert-WebAuthnConsistency {
    param([string]$RpId, [string]$Origin, [bool]$TunnelActive)
    if ($TunnelActive) {
        if ($RpId -eq "localhost") {
            Write-Warn "WEBAUTHN_RP_ID='localhost' but tunnel mode is active!"
            Write-Warn "  Passkeys registered via the public URL will NOT work."
        }
        if ($Origin -match "^http://localhost") {
            Write-Warn "WEBAUTHN_ORIGIN is localhost but tunnel mode is active!"
            Write-Warn "  Browsers will reject WebAuthn requests with mismatched origin."
        }
    } else {
        if ($RpId -ne "localhost") {
            Write-Warn "WEBAUTHN_RP_ID='$RpId' in local mode (expected 'localhost')."
            Write-Warn "  Passkeys registered for this RP_ID may not work on localhost."
        }
    }
}

# ══════════════════════════════════════════════════════════════════════════════
#  MAIN
# ══════════════════════════════════════════════════════════════════════════════

# Resolve flag conflicts
if ($LocalOnly -and $UseTunnel) {
    Write-Host "  [!!] -LocalOnly and -UseTunnel are mutually exclusive — -LocalOnly wins." -ForegroundColor Yellow
    $UseTunnel = $false
}

# Ensure .demo/ and .demo/logs/ exist before any Write-Log call
Ensure-Dirs

# Optionally wipe logs
if ($CleanLogs) {
    Get-ChildItem -Path $LogDir -Filter "*.log" -ErrorAction SilentlyContinue | Remove-Item -Force
    Write-Host "  [OK] Logs cleared ($LogDir)" -ForegroundColor DarkGray
}

Write-Log "═══════════════════════════════════════════════════════════════"
Write-Log "CRYPTON DEMO LAUNCHER  $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
Write-Log "Flags: Rebuild=$Rebuild LocalOnly=$LocalOnly UseTunnel=$UseTunnel CleanLogs=$CleanLogs KillExisting=$KillExisting"
Write-Log "═══════════════════════════════════════════════════════════════"

Write-Host ""
Write-Host "  ══════════════════════════════════════════════" -ForegroundColor Magenta
Write-Host "    CRYPTON DEMO LAUNCHER" -ForegroundColor White
Write-Host "    $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')" -ForegroundColor DarkGray
Write-Host "  ══════════════════════════════════════════════" -ForegroundColor Magenta
Write-Host ""

# ── Locate service directories ────────────────────────────────────────────────
Write-Banner "Locating service directories"

$FeDir       = Find-FrontendDir
$IdentityDir = Find-RustDir "crypton-identity"
$GatewayDir  = Find-RustDir "crypton-gateway"

if (-not $FeDir)       { Write-Fail "Cannot find React frontend directory (needs src/ + package.json)." }
if (-not $IdentityDir) { Write-Fail "Cannot find crypton-identity (needs Cargo.toml under services\crypton-identity or root)." }
if (-not $GatewayDir)  { Write-Fail "Cannot find crypton-gateway (needs Cargo.toml under services\crypton-gateway or root)." }

$IdentityBin = Join-Path $IdentityDir "target\release\crypton-identity.exe"
$GatewayBin  = Join-Path $GatewayDir  "target\release\crypton-gateway.exe"
$IdentityEnv = Join-Path $IdentityDir ".env"
$DemoDir     = Join-Path $Root "services\demo-site"

Write-OK "Frontend  : $FeDir"
Write-OK "Identity  : $IdentityDir"
Write-OK "Gateway   : $GatewayDir"
if (Test-Path $DemoDir) { Write-OK "Demo site : $DemoDir" } else { Write-Warn "Demo site not found at $DemoDir — Step 8 will be skipped." }

# ── Optionally kill existing port holders ─────────────────────────────────────
if ($KillExisting) {
    Write-Banner "Clearing ports 3000, 4000, 8080, 8090"
    foreach ($p in @(3000, 4000, 8080, 8090)) {
        if (Test-PortBound $p) {
            Write-Warn "Port :$p is in use — killing..."
            Stop-PortProcesses -Port $p
        } else {
            Write-Info ":$p is free"
        }
    }
    Start-Sleep -Seconds 1
}

# ── Prerequisites ─────────────────────────────────────────────────────────────
Write-Banner "Checking prerequisites"

# Docker
try {
    $null = & docker info 2>&1
    if ($LASTEXITCODE -ne 0) { throw "docker info failed" }
} catch {
    Write-Fail "Docker Desktop is not running or 'docker' is not on PATH." `
               "Start Docker Desktop and retry."
}
Write-OK "Docker is running"

# cloudflared (required only in tunnel mode)
if ($UseTunnel) {
    if (-not (Get-Command "cloudflared" -ErrorAction SilentlyContinue)) {
        Write-Fail @"
'cloudflared' is not on PATH.
Install via:  winget install Cloudflare.cloudflared
Download:     https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/downloads/
"@
    }
    $cfVer = (& cloudflared version 2>&1 | Select-Object -First 1)
    Write-OK "cloudflared: $cfVer"
}

# npm
if (-not (Get-Command "npm" -ErrorAction SilentlyContinue)) {
    Write-Fail "'npm' not found. Install Node.js from https://nodejs.org/"
}
Write-OK "npm: $(& npm --version 2>&1)"

# cargo
if (-not (Get-Command "cargo" -ErrorAction SilentlyContinue)) {
    Write-Fail "'cargo' not found. Install Rust from https://rustup.rs/"
}
Write-OK "cargo: $(& cargo --version 2>&1)"

# ── Step 1 — Docker infra ─────────────────────────────────────────────────────
Write-Banner "Step 1/7 — Docker infra (Postgres + Redis)"

$composeFile = Join-Path $Root "infra\docker-compose.yml"
if (-not (Test-Path $composeFile)) {
    Write-Fail "infra\docker-compose.yml not found at: $composeFile"
}

$infraLog = Join-Path $LogDir "infra.log"
Write-Info "docker compose up -d --wait"
& docker compose -f $composeFile up -d --wait 2>&1 | Tee-Object -FilePath $infraLog | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Warn "--wait flag unsupported or timed out; falling back to plain 'up -d' + 8s sleep"
    & docker compose -f $composeFile up -d 2>&1 | Tee-Object -FilePath $infraLog -Append | Out-Null
    Start-Sleep -Seconds 8
}
Write-OK "Postgres :5432 and Redis :6379 are up"

# ── Step 2 — Build Rust services ──────────────────────────────────────────────
Write-Banner "Step 2/7 — Rust services"

if ($Rebuild) {
    # Delete stale binaries so we never accidentally run old code
    foreach ($bin in @($IdentityBin, $GatewayBin)) {
        if (Test-Path $bin) {
            Remove-Item $bin -Force
            Write-Info "Deleted stale binary: $(Split-Path $bin -Leaf)"
        }
    }
    Write-Info "Building crypton-identity..."
    Invoke-CargoBuild -Dir $IdentityDir -Label "identity"

    Write-Info "Building crypton-gateway..."
    Invoke-CargoBuild -Dir $GatewayDir -Label "gateway"
} else {
    # Warn if source files are newer than the prebuilt binary
    Test-BinaryFreshness -BinPath $IdentityBin `
                         -SrcDir  (Join-Path $IdentityDir "src") `
                         -Label   "identity"
    Test-BinaryFreshness -BinPath $GatewayBin `
                         -SrcDir  (Join-Path $GatewayDir "src") `
                         -Label   "gateway"

    if (-not (Test-Path $IdentityBin)) {
        Write-Warn "No prebuilt identity binary — 'cargo run --release' will compile on demand."
        Write-Warn "  First-time build takes 2-4 min. IdentityTimeout=$IdentityTimeout s."
        Write-Warn "  Run with -Rebuild to pre-compile."
    }
    if (-not (Test-Path $GatewayBin)) {
        Write-Warn "No prebuilt gateway binary — 'cargo run --release' will compile on demand."
    }
}

# ── Step 3 — React frontend ───────────────────────────────────────────────────
Write-Banner "Step 3/7 — React frontend (:3000)"

$nodeModules = Join-Path $FeDir "node_modules"
if (-not (Test-Path $nodeModules)) {
    Write-Warn "node_modules missing — running npm install (may take ~60s)..."
    $npmLog  = Join-Path $LogDir "npm-install.log"
    $npmProc = Start-Process "cmd.exe" `
        -ArgumentList "/c npm install" `
        -WorkingDirectory $FeDir `
        -RedirectStandardOutput $npmLog `
        -RedirectStandardError  $npmLog `
        -WindowStyle Hidden -PassThru -Wait
    if ($npmProc.ExitCode -ne 0) {
        Show-LogTail -Path $npmLog -Lines 20 -Label "npm install log"
        Write-Fail "npm install failed (exit $($npmProc.ExitCode))" $npmLog
    }
    Write-OK "npm install complete"
}

# CRA settings: prevent auto-browser + allow access from tunnel
$env:BROWSER                        = "none"
$env:DANGEROUSLY_DISABLE_HOST_CHECK = "true"

$feOut  = Join-Path $LogDir "frontend-out.log"
$feErr  = Join-Path $LogDir "frontend-err.log"
$feProc = Start-Svc -Label "frontend" -Exe "cmd.exe" `
    -Arguments @("/c", "npm start") `
    -WorkDir $FeDir -OutLog $feOut -ErrLog $feErr

$pids = [ordered]@{}
$pids["frontend"] = $feProc.Id

Start-Sleep -Milliseconds 2500
if ($feProc.HasExited) {
    Show-LogTail -Path $feErr -Lines 25 -Label "frontend-err.log"
    Write-Fail "Frontend process exited immediately (exit $($feProc.ExitCode))" $feErr
}

if (-not (Wait-ForHttp "http://127.0.0.1:3000" "frontend :3000" $FrontendTimeout)) {
    Show-LogTail -Path $feErr -Lines 25 -Label "frontend-err.log"
    Write-Fail "Frontend did not become healthy within ${FrontendTimeout}s" $feErr
}

# Clean up env vars (child processes already inherited them)
Remove-Item Env:BROWSER                        -ErrorAction SilentlyContinue
Remove-Item Env:DANGEROUSLY_DISABLE_HOST_CHECK -ErrorAction SilentlyContinue

# ── Step 4 — Cloudflare tunnel ────────────────────────────────────────────────
$tunnelUrl      = "http://localhost:3000"
$tunnelHostname = "localhost"
$tunnelActive   = $false

if ($UseTunnel) {
    Write-Banner "Step 4/7 — Cloudflare quick tunnel → :3000"

    $cfOut = Join-Path $LogDir "tunnel-out.log"
    $cfErr = Join-Path $LogDir "tunnel-err.log"

    # Kill any cloudflared process that still holds the old log files open,
    # then delete the logs so URL extraction always scans a fresh file.
    # (-Force on Remove-Item only suppresses the "are you sure?" prompt —
    #  it does NOT override OS-level file locks. Kill the owner first.)
    $existingCf = Get-Process -Name "cloudflared" -ErrorAction SilentlyContinue
    if ($existingCf) {
        Write-Warn "Found existing cloudflared process(es) — killing before restart."
        $existingCf | ForEach-Object { $null = & taskkill /f /t /pid $_.Id 2>&1 }
        Start-Sleep -Milliseconds 800
    }
    foreach ($f in @($cfOut, $cfErr)) {
        Remove-Item $f -Force -ErrorAction SilentlyContinue
    }

    $cfProc = Start-Svc -Label "tunnel" -Exe "cloudflared" `
        -Arguments @("tunnel", "--url", "http://localhost:3000") `
        -WorkDir $Root -OutLog $cfOut -ErrLog $cfErr

    $pids["tunnel"] = $cfProc.Id

    $foundUrl = Get-CloudflareTunnelUrl -OutLog $cfOut -ErrLog $cfErr -TimeoutSec 70

    if (-not $foundUrl) {
        Write-Warn "Could not capture tunnel URL. Falling back to localhost mode."
        Write-Warn "  Check: $cfOut"
        Write-Warn "  Check: $cfErr"
    } else {
        $tunnelUrl      = $foundUrl
        $tunnelHostname = ([System.Uri]$tunnelUrl).Host
        $tunnelActive   = $true
        Set-Content -Path $UrlFile -Value $tunnelUrl -Encoding UTF8

        Write-Host ""
        Write-Host "  ┌───────────────────────────────────────────────────────┐" -ForegroundColor Magenta
        Write-Host "  │  PUBLIC URL — open on any device:                      │" -ForegroundColor Magenta
        Write-Host "  │  $($tunnelUrl.PadRight(54))│" -ForegroundColor Yellow
        Write-Host "  └───────────────────────────────────────────────────────┘" -ForegroundColor Magenta
        Write-Host ""
        Write-OK "Tunnel hostname : $tunnelHostname"
    }
} else {
    Set-Content -Path $UrlFile -Value $tunnelUrl -Encoding UTF8
    Write-Banner "Step 4/7 — Tunnel skipped (local mode)"
    Write-Info "Pass -UseTunnel to expose the app via a public Cloudflare URL."
}

# ── Step 5 — Write identity .env ──────────────────────────────────────────────
Write-Banner "Step 5/7 — Writing crypton-identity .env"

$jwtSecret      = Get-OrCreateJwtSecret -EnvPath $IdentityEnv
$webauthnOrigin = if ($tunnelActive) { $tunnelUrl } else { "http://localhost:3000" }
$webauthnRpId   = $tunnelHostname   # "localhost" in local mode
$frontendOrigin = $webauthnOrigin   # React app URL — same as WebAuthn origin

Write-IdentityEnv -EnvPath        $IdentityEnv `
                  -RpId           $webauthnRpId `
                  -Origin         $webauthnOrigin `
                  -JwtSecret      $jwtSecret `
                  -FrontendOrigin $frontendOrigin

Assert-WebAuthnConsistency -RpId $webauthnRpId -Origin $webauthnOrigin -TunnelActive $tunnelActive

if ($tunnelActive) {
    Write-Host ""
    Write-Host "  ─── WebAuthn env for this session ──────────────────────" -ForegroundColor Cyan
    Write-Host "  WEBAUTHN_RP_ID     = $webauthnRpId"     -ForegroundColor Yellow
    Write-Host "  WEBAUTHN_ORIGIN    = $webauthnOrigin"   -ForegroundColor Yellow
    Write-Host "  ⚠  Passkeys registered on localhost won't work here."    -ForegroundColor Yellow
    Write-Host "     Register a fresh passkey using the tunnel URL."        -ForegroundColor Yellow
    Write-Host ""
}

# ── Step 6 — crypton-identity ─────────────────────────────────────────────────
Write-Banner "Step 6/7 — crypton-identity (:8080)"

$idOut = Join-Path $LogDir "identity-out.log"
$idErr = Join-Path $LogDir "identity-err.log"

# Belt-and-suspenders: set all required vars in the PowerShell session so the
# child process inherits them via the Windows process environment block.
# dotenvy::dotenv() reads from the working-directory .env file first, but
# Start-Process -WorkingDirectory can behave unexpectedly on long OneDrive paths
# in PowerShell 5.1. Env var inheritance is always reliable regardless.
# dotenvy does NOT override vars that are already set in the environment, so
# these values take precedence over any stale .env content.
$env:APP_PORT              = "8080"
$env:DATABASE_URL          = "postgres://postgres:postgres@127.0.0.1:5432/crypton"
$env:REDIS_URL             = "redis://127.0.0.1:6379"
$env:RUST_LOG              = "info"
$env:JWT_SECRET            = $jwtSecret
$env:WEBAUTHN_RP_NAME      = "Crypton Identity"
$env:WEBAUTHN_RP_ID        = $webauthnRpId
$env:WEBAUTHN_ORIGIN       = $webauthnOrigin
$env:OAUTH_CLIENT_ID       = "demo-site"
$env:OAUTH_CLIENT_SECRET   = "demo-secret-change-in-prod"
$env:OAUTH_REDIRECT_URIS   = "http://localhost:4000/callback"
$env:FRONTEND_ORIGIN       = $frontendOrigin

if (Test-Path $IdentityBin) {
    Write-OK "Using prebuilt binary ($(Split-Path $IdentityBin -Leaf))"
    $idProc = Start-Svc -Label "identity" -Exe $IdentityBin `
        -Arguments @() -WorkDir $IdentityDir -OutLog $idOut -ErrLog $idErr
} else {
    Write-Warn "No prebuilt binary — using 'cargo run --release' (first run: 2-4 min)"
    $idProc = Start-Svc -Label "identity" -Exe "cargo" `
        -Arguments @("run", "--release") -WorkDir $IdentityDir -OutLog $idOut -ErrLog $idErr
}

# Remove identity env vars from the PowerShell session immediately after the
# process is created. The child process already has its own copy of the
# environment block and is not affected by this removal.
# This prevents these vars from leaking into the gateway process (which uses
# its own .env with APP_PORT=8090) started in the next step.
foreach ($v in @("APP_PORT","DATABASE_URL","REDIS_URL","RUST_LOG","JWT_SECRET",
                  "WEBAUTHN_RP_NAME","WEBAUTHN_RP_ID","WEBAUTHN_ORIGIN",
                  "OAUTH_CLIENT_ID","OAUTH_CLIENT_SECRET","OAUTH_REDIRECT_URIS","FRONTEND_ORIGIN")) {
    Remove-Item "Env:$v" -ErrorAction SilentlyContinue
}

$pids["identity"] = $idProc.Id
Start-Sleep -Milliseconds 2000

if ($idProc.HasExited) {
    Show-LogTail -Path $idErr -Lines 30 -Label "identity-err.log"
    Write-Fail "Identity exited immediately. Common causes:
       • Missing env vars  — check: $IdentityEnv
       • DB unavailable    — check: docker compose ps
       • Port :8080 taken  — run with -KillExisting
       • Compile error     — run with -Rebuild" $idErr
}

if (-not (Wait-ForHttp "http://127.0.0.1:8080/health" "identity :8080" $IdentityTimeout)) {
    Show-LogTail -Path $idErr -Lines 30 -Label "identity-err.log"
    Write-Fail "Identity did not become healthy within ${IdentityTimeout}s.
       • If compiling for first time, retry with -IdentityTimeout 420
       • Check migration errors in: $idErr
       • Check Redis/PG connectivity" $idErr
}

# ── Step 7 — crypton-gateway ──────────────────────────────────────────────────
Write-Banner "Step 7/7 — crypton-gateway (:8090)"

# The gateway reads APP_PORT and IDENTITY_URL from its own .env
# (services\crypton-gateway\.env) which already has correct defaults.
# We set RUST_LOG here for visibility but the gateway defaults to 'info' anyway.
$env:RUST_LOG = "info"

$gwOut = Join-Path $LogDir "gateway-out.log"
$gwErr = Join-Path $LogDir "gateway-err.log"

if (Test-Path $GatewayBin) {
    Write-OK "Using prebuilt binary ($(Split-Path $GatewayBin -Leaf))"
    $gwProc = Start-Svc -Label "gateway" -Exe $GatewayBin `
        -Arguments @() -WorkDir $GatewayDir -OutLog $gwOut -ErrLog $gwErr
} else {
    Write-Warn "No prebuilt binary — using 'cargo run --release'"
    $gwProc = Start-Svc -Label "gateway" -Exe "cargo" `
        -Arguments @("run", "--release") -WorkDir $GatewayDir -OutLog $gwOut -ErrLog $gwErr
}

$pids["gateway"] = $gwProc.Id
Start-Sleep -Milliseconds 1500

if ($gwProc.HasExited) {
    Show-LogTail -Path $gwErr -Lines 20 -Label "gateway-err.log"
    Write-Fail "Gateway exited immediately" $gwErr
}

if (-not (Wait-ForHttp "http://127.0.0.1:8090/health" "gateway :8090" $GatewayTimeout)) {
    Show-LogTail -Path $gwErr -Lines 20 -Label "gateway-err.log"
    Write-Fail "Gateway did not become healthy within ${GatewayTimeout}s" $gwErr
}

Remove-Item Env:RUST_LOG -ErrorAction SilentlyContinue

# ── Step 8 — demo-site (:4000) ────────────────────────────────────────────────
Write-Banner "Step 8/8 — demo-site (:4000)  [Login with Crypton]"

if (Test-Path $DemoDir) {
    # Install node_modules if absent
    $demoModules = Join-Path $DemoDir "node_modules"
    if (-not (Test-Path $demoModules)) {
        Write-Warn "demo-site node_modules missing — running npm install..."
        $demoNpmLog  = Join-Path $LogDir "demo-npm-install.log"
        $demoNpmProc = Start-Process "cmd.exe" `
            -ArgumentList "/c npm install" `
            -WorkingDirectory $DemoDir `
            -RedirectStandardOutput $demoNpmLog `
            -RedirectStandardError  $demoNpmLog `
            -WindowStyle Hidden -PassThru -Wait
        if ($demoNpmProc.ExitCode -ne 0) {
            Show-LogTail -Path $demoNpmLog -Lines 20 -Label "demo npm install"
            Write-Fail "npm install failed for demo-site (exit $($demoNpmProc.ExitCode))" $demoNpmLog
        }
        Write-OK "demo-site npm install complete"
    }

    # Set demo-site env vars
    $env:PORT           = "4000"
    $env:CRYPTON_URL    = "http://localhost:8080"
    $env:CLIENT_ID      = "demo-site"
    $env:CLIENT_SECRET  = "demo-secret-change-in-prod"
    $env:REDIRECT_URI   = "http://localhost:4000/callback"

    $demoOut  = Join-Path $LogDir "demo-out.log"
    $demoErr  = Join-Path $LogDir "demo-err.log"
    $demoProc = Start-Svc -Label "demo" -Exe "cmd.exe" `
        -Arguments @("/c", "node server.js") `
        -WorkDir $DemoDir -OutLog $demoOut -ErrLog $demoErr

    $pids["demo"] = $demoProc.Id

    foreach ($v in @("PORT","CRYPTON_URL","CLIENT_ID","CLIENT_SECRET","REDIRECT_URI")) {
        Remove-Item "Env:$v" -ErrorAction SilentlyContinue
    }

    Start-Sleep -Milliseconds 1500
    if ($demoProc.HasExited) {
        Show-LogTail -Path $demoErr -Lines 20 -Label "demo-err.log"
        Write-Fail "Demo site exited immediately" $demoErr
    }

    if (-not (Wait-ForHttp "http://127.0.0.1:4000" "demo-site :4000" 30)) {
        Show-LogTail -Path $demoErr -Lines 20 -Label "demo-err.log"
        Write-Fail "Demo site did not become healthy within 30s" $demoErr
    }
} else {
    Write-Warn "Demo site directory not found — skipping Step 8."
    Write-Warn "  Expected: $DemoDir"
    Write-Warn "  The OAuth demo flow will not be available."
}

# ── Save PIDs for stop-demo.ps1 ───────────────────────────────────────────────
$pids | ConvertTo-Json | Set-Content -Path $PidFile -Encoding UTF8
Write-Log "PIDs saved: $($pids | ConvertTo-Json -Compress)"

# ══════════════════════════════════════════════════════════════════════════════
#  FINAL SUMMARY
# ══════════════════════════════════════════════════════════════════════════════

$publicUrl = $tunnelUrl
$isLocal   = ($publicUrl -eq "http://localhost:3000")
$demoUrl   = "http://localhost:4000"
$demoLive  = Test-Path $DemoDir

Write-Host ""
Write-Host "  ══════════════════════════════════════════════" -ForegroundColor Magenta
Write-Host "    ✓  CRYPTON IS LIVE" -ForegroundColor Green
Write-Host "  ══════════════════════════════════════════════" -ForegroundColor Magenta
Write-Host ""

if (-not $isLocal) {
    Write-Host "  PUBLIC URL  : $publicUrl" -ForegroundColor Yellow
    Write-Host "                ↑ open on phone / other laptop / any browser" -ForegroundColor DarkGray
    Write-Host ""
}

if ($demoLive) {
    Write-Host "  Demo site   : $demoUrl" -ForegroundColor Green
    Write-Host "                ↑ primary entrypoint — Login with Crypton" -ForegroundColor DarkGray
    Write-Host ""
}
Write-Host "  React app   : http://localhost:3000"          -ForegroundColor Cyan
Write-Host "  Identity    : http://localhost:8080/health"   -ForegroundColor DarkGray
Write-Host "  Gateway     : http://localhost:8090/health"   -ForegroundColor DarkGray
Write-Host "  Logs        : $LogDir"                        -ForegroundColor DarkGray
Write-Host "  PIDs        : $PidFile"                       -ForegroundColor DarkGray

if (-not $isLocal) {
    Write-Host ""
    Write-Host "  ── WebAuthn / Passkey ────────────────────────" -ForegroundColor DarkGray
    Write-Host "  RP_ID   : $webauthnRpId"    -ForegroundColor DarkGray
    Write-Host "  Origin  : $webauthnOrigin"  -ForegroundColor DarkGray
    Write-Host "  ⚠  Old passkeys from localhost sessions won't work." -ForegroundColor Yellow
    Write-Host "     Register a new passkey after opening the tunnel URL." -ForegroundColor Yellow
}

Write-Host ""
Write-Host "  Log files per service:"                                       -ForegroundColor DarkGray
Write-Host "    identity  stdout → $LogDir\identity-out.log"               -ForegroundColor DarkGray
Write-Host "    identity  stderr → $LogDir\identity-err.log"               -ForegroundColor DarkGray
Write-Host "    gateway   stdout → $LogDir\gateway-out.log"                -ForegroundColor DarkGray
Write-Host "    gateway   stderr → $LogDir\gateway-err.log"                -ForegroundColor DarkGray
Write-Host "    frontend  stdout → $LogDir\frontend-out.log"               -ForegroundColor DarkGray
Write-Host "    frontend  stderr → $LogDir\frontend-err.log"               -ForegroundColor DarkGray
if ($demoLive) {
    Write-Host "    demo-site stdout → $LogDir\demo-out.log"               -ForegroundColor DarkGray
    Write-Host "    demo-site stderr → $LogDir\demo-err.log"               -ForegroundColor DarkGray
}
if ($tunnelActive) {
    Write-Host "    tunnel    stdout → $LogDir\tunnel-out.log"              -ForegroundColor DarkGray
    Write-Host "    tunnel    stderr → $LogDir\tunnel-err.log"              -ForegroundColor DarkGray
}
Write-Host "    launcher  log    → $LaunchLog"                              -ForegroundColor DarkGray

Write-Host ""
Write-Host "  To stop all services:  .\stop-demo.ps1"            -ForegroundColor DarkGray
Write-Host "  ══════════════════════════════════════════════"     -ForegroundColor Magenta
Write-Host ""

Write-Log "LIVE — Demo=$demoUrl  React=$publicUrl  RpId=$webauthnRpId  PIDs=$($pids | ConvertTo-Json -Compress)"

if (-not $NoBrowser) {
    $browserTarget = if ($demoLive) { $demoUrl } else { $publicUrl }
    Write-Host "  Opening browser: $browserTarget" -ForegroundColor Cyan
    Start-Process $browserTarget
}
