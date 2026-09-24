#!/usr/bin/env pwsh
# run_it.ps1 - Run ada-mock INTEGRATION tests (cargo test --test sample_mock_usage, no server feature) and emit logs.
#
# ??:
#   ???? smoke: builder + fixture + mock(event_bus + scheduler + connector) ?
#   ????????????? 3 ?. server ? (FakeOtlpServer) ? ST ????.
#
# ??:
#   pwsh scripts/run_it.ps1
#   pwsh scripts/run_it.ps1 -NoFailFast
#   pwsh scripts/run_it.ps1 -TargetDir .ada-mock-target
#
# ??:
#   test-results/it/it-{yyyyMMdd-HHmmss}.log
#   test-results/it/it-latest.log

param(
    [switch]$NoFailFast = $false,
    [string]$TargetDir = '.ada-mock-target',
    [int]$TimeoutSec = 600
)
$ErrorActionPreference = 'Stop'

$root = Resolve-Path (Join-Path $PSScriptRoot '..\..\..')
Set-Location $root

$logDir = Join-Path $root 'test-results/it'
if (-not (Test-Path $logDir)) { New-Item -ItemType Directory -Path $logDir | Out-Null }
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$logFile = Join-Path $logDir "it-$stamp.log"

Write-Host "=== run_it.ps1 ===" -ForegroundColor Cyan
Write-Host "  Layer:    IT (cargo test --test sample_mock_usage)"
Write-Host "  Target:   $TargetDir"
Write-Host "  Log:      $logFile"
Write-Host ""

$env:CARGO_TARGET_DIR = Join-Path $root $TargetDir
if (-not (Test-Path $env:CARGO_TARGET_DIR)) {
    New-Item -ItemType Directory -Path $env:CARGO_TARGET_DIR | Out-Null
}

# --test sample_mock_usage + --features ?**????** (server ?? = FakeOtlpServer ??)
$args = @('test', '-p', 'ada-mock', '--test', 'sample_mock_usage')
if ($NoFailFast) { $args += '--no-fail-fast' }

Write-Host "+ cargo $($args -join ' ')" -ForegroundColor Yellow
$proc = Start-Process -FilePath 'cargo' -ArgumentList $args `
    -NoNewWindow -PassThru `
    -RedirectStandardOutput (Join-Path $logDir 'it-stdout.tmp') `
    -RedirectStandardError  (Join-Path $logDir 'it-stderr.tmp')
if (-not $proc.WaitForExit($TimeoutSec * 1000)) {
    Write-Host "TIMEOUT (${TimeoutSec}s) - cargo kill" -ForegroundColor Red
    try { $proc.Kill($true) } catch {}
    exit 124
}
$exit = $proc.ExitCode

$ts = Get-Date -Format 'yyyy-MM-dd HH:mm:ss'
"=== run_it $ts exit=$exit ===" | Out-File -FilePath $logFile -Encoding UTF8
"--- stdout ---" | Out-File -FilePath $logFile -Append -Encoding UTF8
if (Test-Path (Join-Path $logDir 'it-stdout.tmp')) {
    Get-Content (Join-Path $logDir 'it-stdout.tmp') -Raw | Out-File -FilePath $logFile -Append -Encoding UTF8
}
"--- stderr ---" | Out-File -FilePath $logFile -Append -Encoding UTF8
if (Test-Path (Join-Path $logDir 'it-stderr.tmp')) {
    Get-Content (Join-Path $logDir 'it-stderr.tmp') -Raw | Out-File -FilePath $logFile -Append -Encoding UTF8
}
Remove-Item (Join-Path $logDir 'it-stdout.tmp') -ErrorAction SilentlyContinue
Remove-Item (Join-Path $logDir 'it-stderr.tmp')  -ErrorAction SilentlyContinue

$latest = Join-Path $logDir 'it-latest.log'
if (Test-Path $latest) { Remove-Item $latest -Force }
Copy-Item $logFile $latest

$summary = ''
foreach ($line in (Get-Content $logFile)) {
    if ($line -match 'test result:\s*(ok|FAILED).*?(\d+)\s+passed;\s*(\d+)\s+failed') {
        $summary = "  $($Matches[1]) | passed=$($Matches[2]) failed=$($Matches[3])"
        break
    }
}

if ($exit -ne 0) {
    Write-Host "IT FAIL - see $logFile" -ForegroundColor Red
    if ($summary) { Write-Host $summary -ForegroundColor Red }
    exit $exit
}
Write-Host "IT OK" -ForegroundColor Green
if ($summary) { Write-Host $summary }
exit 0
