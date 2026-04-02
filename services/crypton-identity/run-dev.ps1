<#
Run the identity service for local LAN testing.

Usage:
  Open a PowerShell prompt in this folder and run:
    .\run-dev.ps1

This script loads environment variables from `.env` in this folder and
then runs `cargo run` in the same directory so `dotenvy` and the app
read the correct values.
#>

Set-StrictMode -Version Latest

$envFile = Join-Path $PSScriptRoot '.env'
if (Test-Path $envFile) {
    Get-Content $envFile | ForEach-Object {
        $line = $_.Trim()
        if ($line -eq '' -or $line.StartsWith('#')) { return }
        $parts = $line -split '=', 2
        if ($parts.Count -ne 2) { return }
        $k = $parts[0].Trim()
        $v = $parts[1].Trim()
        # strip surrounding quotes if present
        if ($v.StartsWith('"') -and $v.EndsWith('"')) { $v = $v.Substring(1,$v.Length-2) }
        Set-Item -Path "Env:$k" -Value $v
    }
} else {
    Write-Host "No .env file found in $PSScriptRoot" -ForegroundColor Yellow
}

Write-Host "Starting crypton-identity (this will run in foreground)." -ForegroundColor Cyan
cargo run
