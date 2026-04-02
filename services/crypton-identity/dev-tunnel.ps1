<#
.SYNOPSIS
    Start a Cloudflare quick-tunnel so you can test WebAuthn across devices on
    networks that isolate clients (eduroam, dorm WiFi, hotspots, etc.).

.DESCRIPTION
    Cloudflare Tunnel gives your Rust server a public HTTPS URL
    (e.g., https://abc123.trycloudflare.com) reachable from any device on any
    network, including networks that block device-to-device traffic.

    WebAuthn REQUIRES HTTPS (or localhost). Plain LAN IP won't work unless you
    have a valid cert for that IP. The tunnel gives you real HTTPS for free.

.PREREQUISITES
    1. cloudflared installed:
         winget install Cloudflare.cloudflared
       or download from: https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/downloads/

    2. The Rust server is running (./run-dev.ps1 in another terminal).

.WORKFLOW
    Step 1 — Run this script to get the tunnel URL (e.g., https://abc123.trycloudflare.com).
    Step 2 — Update .env with the tunnel URL:
                WEBAUTHN_RP_ID=abc123.trycloudflare.com
                WEBAUTHN_ORIGIN=https://abc123.trycloudflare.com
    Step 3 — Restart the Rust server (./run-dev.ps1).
    Step 4 — Open https://abc123.trycloudflare.com/test on any device.

    NOTE: The tunnel URL changes each run with free quick-tunnels. Steps 2-3
    need to be repeated if you restart the tunnel.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Check that cloudflared is on PATH
if (-not (Get-Command cloudflared -ErrorAction SilentlyContinue)) {
    Write-Host ""
    Write-Host "cloudflared not found. Install it with:" -ForegroundColor Red
    Write-Host "  winget install Cloudflare.cloudflared" -ForegroundColor Yellow
    Write-Host "or download from https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/downloads/" -ForegroundColor Yellow
    exit 1
}

# Read APP_PORT from .env if present
$envFile = Join-Path $PSScriptRoot '.env'
$port = 8080
if (Test-Path $envFile) {
    Get-Content $envFile | ForEach-Object {
        $line = $_.Trim()
        if ($line -match '^APP_PORT\s*=\s*(\d+)') { $port = $Matches[1] }
    }
}

Write-Host ""
Write-Host "Starting Cloudflare quick-tunnel to http://localhost:$port" -ForegroundColor Cyan
Write-Host ""
Write-Host "Once the tunnel URL appears (something like https://abc123.trycloudflare.com):" -ForegroundColor Yellow
Write-Host "  1. Copy the URL" -ForegroundColor Yellow
Write-Host "  2. Edit .env and set:" -ForegroundColor Yellow
Write-Host "       WEBAUTHN_RP_ID=<subdomain>.trycloudflare.com" -ForegroundColor Yellow
Write-Host "       WEBAUTHN_ORIGIN=https://<subdomain>.trycloudflare.com" -ForegroundColor Yellow
Write-Host "  3. Restart the Rust server (./run-dev.ps1)" -ForegroundColor Yellow
Write-Host "  4. Open https://<subdomain>.trycloudflare.com/test on any device" -ForegroundColor Yellow
Write-Host ""

cloudflared tunnel --url "http://localhost:$port"
