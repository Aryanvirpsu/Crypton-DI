#Requires -Version 5.1
<#
.SYNOPSIS
    Crypton - stop all running services.

.DESCRIPTION
    Reads .run\pids.json and terminates all tracked processes.
    Falls back to port-based kill if the PID file is missing.

.PARAMETER StopInfra
    Also stop Docker (Postgres + Redis). Omitted by default to keep DB state.

.PARAMETER ClearLogs
    Delete log files in .run\logs\ after stopping.

.EXAMPLE
    .\stop-all.ps1
    .\stop-all.ps1 -StopInfra
    .\stop-all.ps1 -StopInfra -ClearLogs
#>
[CmdletBinding()]
param(
    [switch]$StopInfra,
    [switch]$ClearLogs
)

Set-StrictMode -Off
$ErrorActionPreference = "SilentlyContinue"
$ProgressPreference    = "SilentlyContinue"

$Root    = $PSScriptRoot
$RunDir  = Join-Path $Root ".run"
$LogDir  = Join-Path $RunDir "logs"
$PidFile = Join-Path $RunDir "pids.json"

function OK   { param($t) Write-Host "  [OK]  $t" -ForegroundColor Green }
function WARN { param($t) Write-Host "  [!!]  $t" -ForegroundColor Yellow }
function INFO { param($t) Write-Host "        $t" -ForegroundColor DarkGray }

function Stop-Tree { param([int]$Pid, [string]$Label)
    if ($Pid -le 4) { return }
    if (-not (Get-Process -Id $Pid -EA SilentlyContinue)) { INFO "$Label (PID $Pid) - already gone"; return }
    $r = & taskkill /F /T /PID $Pid 2>&1
    if ($LASTEXITCODE -eq 0 -or ($r -join "") -match "SUCCESS") { OK "$Label (PID $Pid) stopped" }
    else { Stop-Process -Id $Pid -Force -EA SilentlyContinue; WARN "$Label (PID $Pid) force-killed" }
}

function Kill-Port { param([int]$p)
    netstat -ano 2>$null |
        Select-String "TCP\s+\S+:$p\s+\S+\s+LISTENING\s+(\d+)" |
        ForEach-Object {
            $oPid = [int]$_.Matches[0].Groups[1].Value
            if ($oPid -gt 4) { & taskkill /F /T /PID $oPid 2>&1 | Out-Null; WARN ":$p killed PID $oPid"; Start-Sleep -Milliseconds 300 }
        }
}

Write-Host ""
Write-Host "  =================================================" -ForegroundColor Magenta
Write-Host "    CRYPTON  --  STOP ALL"                           -ForegroundColor White
Write-Host "    $((Get-Date -Format 'ddd dd MMM yyyy  HH:mm:ss'))" -ForegroundColor DarkGray
Write-Host "  =================================================" -ForegroundColor Magenta
Write-Host ""

# ---------------------------------------------------------------------------
# Stop via PID file
# ---------------------------------------------------------------------------
if (Test-Path $PidFile) {
    Write-Host "  Stopping tracked processes..." -ForegroundColor Cyan
    try {
        $stream = [System.IO.File]::Open($PidFile, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, [System.IO.FileShare]::ReadWrite)
        $saved  = ([System.IO.StreamReader]::new($stream)).ReadToEnd() | ConvertFrom-Json
        $stream.Dispose()
        # Frontends first, then gateway, then identity
        foreach ($label in @("crypton-admin", "crypton-demo", "crypton-main", "gateway", "identity")) {
            $savedPid = $saved.$label
            if ($savedPid) { Stop-Tree -Pid ([int]$savedPid) -Label $label }
        }
        # Any extras
        foreach ($prop in $saved.PSObject.Properties) {
            if ($prop.Name -notin @("crypton-admin","crypton-demo","crypton-main","gateway","identity")) {
                Stop-Tree -Pid ([int]$prop.Value) -Label $prop.Name
            }
        }
        Remove-Item $PidFile -Force -EA SilentlyContinue
        INFO "PID file removed"
    } catch {
        WARN "Could not read PID file: $_"
    }
} else {
    WARN "No PID file - falling back to process name + port kill"
    foreach ($name in @("crypton-id", "crypton-identity", "crypton-gateway")) {
        Get-Process -Name $name -EA SilentlyContinue | ForEach-Object { Stop-Tree -Pid $_.Id -Label $name }
    }
}

# ---------------------------------------------------------------------------
# Port sweep (always)
# ---------------------------------------------------------------------------
Write-Host ""
INFO "Port sweep :3000 :3001 :3002 :8080 :8090"
foreach ($p in @(3000, 3001, 3002, 8080, 8090)) { Kill-Port $p }

# ---------------------------------------------------------------------------
# Docker infra (optional)
# ---------------------------------------------------------------------------
Write-Host ""
if ($StopInfra) {
    Write-Host "  Stopping Docker infra..." -ForegroundColor Cyan
    $compose = Join-Path $Root "infra\docker-compose.yml"
    if (Test-Path $compose) {
        & docker compose -f $compose down 2>&1 | Out-Null
        if ($LASTEXITCODE -eq 0) { OK "Docker infra stopped" } else { WARN "docker compose down returned $LASTEXITCODE" }
    } else { WARN "infra\docker-compose.yml not found" }
} else {
    INFO "Docker left running (pass -StopInfra to also stop Postgres + Redis)"
}

# ---------------------------------------------------------------------------
# Clear logs (optional)
# ---------------------------------------------------------------------------
if ($ClearLogs -and (Test-Path $LogDir)) {
    $logFiles = Get-ChildItem -Path $LogDir -EA SilentlyContinue
    $logFiles | Remove-Item -Force -EA SilentlyContinue
    Write-Host ""
    OK "Cleared $($logFiles.Count) log files from .run\logs\"
}

Write-Host ""
Write-Host "  =================================================" -ForegroundColor Magenta
Write-Host "    Stopped."                                        -ForegroundColor Green
Write-Host "  =================================================" -ForegroundColor Magenta
Write-Host ""
