#Requires -Version 5.1
<#
.SYNOPSIS
    Crypton — print real-time system status for all services.

.DESCRIPTION
    Checks which services are running, prints ports in use,
    hits health endpoints, and reports frontend reachability.
    No side effects — read-only.

.EXAMPLE
    .\status.ps1
#>

Set-StrictMode -Off
$ErrorActionPreference = "SilentlyContinue"
$ProgressPreference    = "SilentlyContinue"

$Root = $PSScriptRoot

function Check-Http {
    param([string]$Url, [int]$TimeoutSec = 5)
    try {
        $r = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec $TimeoutSec -EA Stop
        return @{ ok = $true; code = $r.StatusCode; body = ($r.Content | Select-Object -First 200) }
    } catch {
        return @{ ok = $false; code = 0; body = $_.Exception.Message }
    }
}

function Test-Port {
    param([int]$p)
    try { return $null -ne (Get-NetTCPConnection -LocalPort $p -State Listen -EA SilentlyContinue) } catch { return $false }
}

function Status-Line {
    param([string]$Label, [bool]$Up, [string]$Detail = "")
    $icon = if($Up){"[OK] "}else{"[--] "}
    $color = if($Up){"Green"}else{"DarkGray"}
    $line = "  $icon $Label"
    if($Detail){ $line += "  $Detail" }
    Write-Host $line -ForegroundColor $color
}

Write-Host ""
Write-Host "  ══════════════════════════════════════════════════" -ForegroundColor Magenta
Write-Host "    CRYPTON — SYSTEM STATUS" -ForegroundColor White
Write-Host "    $((Get-Date -Format 'yyyy-MM-dd HH:mm:ss'))" -ForegroundColor DarkGray
Write-Host "  ══════════════════════════════════════════════════" -ForegroundColor Magenta
Write-Host ""

# ── Backend ───────────────────────────────────────────────────────────────────
Write-Host "  BACKEND" -ForegroundColor DarkGray
$id = Check-Http "http://127.0.0.1:8080/health"
Status-Line "crypton-id     :8080   (identity backend)" $id.ok $(if($id.ok){"healthy / $($id.code)"}else{"unreachable"})

$gw = Check-Http "http://127.0.0.1:8090/health"
Status-Line "crypton-gateway:8090   (API gateway)     " $gw.ok $(if($gw.ok){"healthy / $($gw.code)"}else{"unreachable"})

Write-Host ""

# ── Frontends ─────────────────────────────────────────────────────────────────
Write-Host "  FRONTENDS" -ForegroundColor DarkGray
$main  = Check-Http "http://127.0.0.1:3000"
$demo  = Check-Http "http://127.0.0.1:3001"
$admin = Check-Http "http://127.0.0.1:3002"
Status-Line "crypton-main   :3000   (marketing site)  " $main.ok  $(if($main.ok) {"reachable / $($main.code)" }else{"unreachable"})
Status-Line "crypton-demo   :3001   (SaaS demo app)   " $demo.ok  $(if($demo.ok) {"reachable / $($demo.code)" }else{"unreachable"})
Status-Line "crypton-admin  :3002   (operator panel)  " $admin.ok $(if($admin.ok){"reachable / $($admin.code)"}else{"unreachable"})

Write-Host ""

# ── Infra ─────────────────────────────────────────────────────────────────────
Write-Host "  INFRASTRUCTURE" -ForegroundColor DarkGray
$pg = Test-Port 5432
Status-Line "postgres       :5432" $pg $(if($pg){"listening"}else{"not listening (run docker compose up)"})

$rd = Test-Port 6379
Status-Line "redis          :6379" $rd $(if($rd){"listening"}else{"not listening (run docker compose up)"})

# Docker check
try {
    $di = & docker info 2>&1
    $dockerOk = $LASTEXITCODE -eq 0
} catch { $dockerOk = $false }
Status-Line "docker desktop        " $dockerOk $(if($dockerOk){"running"}else{"not running"})

Write-Host ""

# ── PID file ──────────────────────────────────────────────────────────────────
$pidFile = Join-Path $Root ".run\pids.json"
if(Test-Path $pidFile) {
    Write-Host "  SAVED PIDs (.demo\pids.json)" -ForegroundColor DarkGray
    $saved = Get-Content $pidFile -Raw | ConvertFrom-Json
    foreach($prop in $saved.PSObject.Properties) {
        $pid = [int]$prop.Value
        $alive = $null -ne (Get-Process -Id $pid -EA SilentlyContinue)
        Status-Line "$($prop.Name.PadRight(20)) PID $pid" $alive $(if($alive){"running"}else{"dead/exited"})
    }
    Write-Host ""
}

# ── Quick-action hints ────────────────────────────────────────────────────────
Write-Host "  ACTIONS" -ForegroundColor DarkGray
Write-Host "    Start: .\start-all.ps1" -ForegroundColor DarkGray
Write-Host "    Stop:  .\stop-all.ps1" -ForegroundColor DarkGray
Write-Host "    Logs:  .run\logs\" -ForegroundColor DarkGray
Write-Host ""
