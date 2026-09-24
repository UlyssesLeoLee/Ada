#!/usr/bin/env pwsh
# run_st.ps1 - Run ada-mock SYSTEM tests (cargo test --test sample_mock_usage --features server) and emit logs.
#
# ??:
#   4 ?????? HTTP/OTLP ?????? (FakeOtlpServer, TcpListener ???) ???
#   E2E smoke. ?? TCP ??????? POST /v1/metrics ????????? + ?? body ???.
#
# ??????????????:
#   - Windows ? `os error 10053` (WSAECONNABORTED) ???. 8/31 16:11 JST ??.
#     ada-mock ? `four_layer_with_otlp_capture` ? Connection: close ?????????
#     ???????????, CI ????????? server ? retry ????.
#   - nightly / cargo-llvm-cov ????? (ST ????????).
#
# ??:
#   pwsh scripts/run_st.ps1
#   pwsh scripts/run_st.ps1 -NoFailFast
#   pwsh scripts/run_st.ps1 -TargetDir .ada-mock-target
#
# ??:
#   test-results/st/st-{yyyyMMdd-HHmmss}.log
#   test-results/st/st-latest.log

param(
    [switch]$NoFailFast = $false,
    [string]$TargetDir = '.ada-mock-target',
    [int]$TimeoutSec = 600
)
$ErrorActionPreference = 'Stop'

$root = Resolve-Path (Join-Path $PSScriptRoot '..\..\..')
Set-Location $root

$logDir = Join-Path $root 'test-results/st'
if (-not (Test-Path $logDir)) { New-Item -ItemType Directory -Path $logDir | Out-Null }
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$logFile = Join-Path $logDir "st-$stamp.log"

Write-Host "=== run_st.ps1 ===" -ForegroundColor Cyan
Write-Host "  Layer:    ST (cargo test --test sample_mock_usage --features server)"
Write-Host "  Target:   $TargetDir"
Write-Host "  Log:      $logFile"
Write-Host ""

$env:CARGO_TARGET_DIR = Join-Path $root $TargetDir
if (-not (Test-Path $env:CARGO_TARGET_DIR)) {
    New-Item -ItemType Directory -Path $env:CARGO_TARGET_DIR | Out-Null
}

# server feature ? ON ?? FakeOtlpServer ????
$args = @('test', '-p', 'ada-mock', '--test', 'sample_mock_usage', '--features', 'server')
if ($NoFailFast) { $args += '--no-fail-fast' }

Write-Host "+ cargo $($args -join ' ')" -ForegroundColor Yellow
$proc = Start-Process -FilePath 'cargo' -ArgumentList $args `
    -NoNewWindow -PassThru `
    -RedirectStandardOutput (Join-Path $logDir 'st-stdout.tmp') `
    -RedirectStandardError  (Join-Path $logDir 'st-stderr.tmp')
if (-not $proc.WaitForExit($TimeoutSec * 1000)) {
    Write-Host "TIMEOUT (${TimeoutSec}s) - cargo kill" -ForegroundColor Red
    try { $proc.Kill($true) } catch {}
    exit 124
}
$exit = $proc.ExitCode

$ts = Get-Date -Format 'yyyy-MM-dd HH:mm:ss'
"=== run_st $ts exit=$exit ===" | Out-File -FilePath $logFile -Encoding UTF8
"--- stdout ---" | Out-File -FilePath $logFile -Append -Encoding UTF8
if (Test-Path (Join-Path $logDir 'st-stdout.tmp')) {
    Get-Content (Join-Path $logDir 'st-stdout.tmp') -Raw | Out-File -FilePath $logFile -Append -Encoding UTF8
}
"--- stderr ---" | Out-File -FilePath $logFile -Append -Encoding UTF8
if (Test-Path (Join-Path $logDir 'st-stderr.tmp')) {
    Get-Content (Join-Path $logDir 'st-stderr.tmp') -Raw | Out-File -FilePath $logFile -Append -Encoding UTF8
}
Remove-Item (Join-Path $logDir 'st-stdout.tmp') -ErrorAction SilentlyContinue
Remove-Item (Join-Path $logDir 'st-stderr.tmp')  -ErrorAction SilentlyContinue

$latest = Join-Path $logDir 'st-latest.log'
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
    Write-Host "ST FAIL - see $logFile" -ForegroundColor Red
    if ($summary) { Write-Host $summary -ForegroundColor Red }
    exit $exit
}
Write-Host "ST OK" -ForegroundColor Green
if ($summary) { Write-Host $summary }
exit 0
