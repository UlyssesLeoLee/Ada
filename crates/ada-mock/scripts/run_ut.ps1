#!/usr/bin/env pwsh
# run_ut.ps1 - Run ada-mock UNIT tests (cargo test --lib) and emit logs.
#
# Purpose:
#   Run the "single module" unit tests of the four capability layers.
#   Integration (tests/) is handled by IT/ST layers; sample_mock_usage is
#   excluded here (only 2 cases, used as IT/ST smoke).
#
# Usage:
#   pwsh scripts/run_ut.ps1                                # default
#   pwsh scripts/run_ut.ps1 -NoFailFast                    # keep going on failure
#   pwsh scripts/run_ut.ps1 -TargetDir .ada-mock-target    # isolate from shared E: cache
#
# Output:
#   test-results/ut/ut-{yyyyMMdd-HHmmss}.log
#   test-results/ut/ut-latest.log                          # copy of most recent run
#
# NOTE: $ErrorActionPreference is set AFTER the param block on purpose.
# PowerShell requires `param` to be the first executable statement of a
# script; setting preferences above `param` triggers a ParserError.
param(
    [switch]$NoFailFast = $false,
    [string]$TargetDir = '.ada-mock-target',
    [int]$TimeoutSec = 600
)
$ErrorActionPreference = 'Stop'

$root = Resolve-Path (Join-Path $PSScriptRoot '..\..\..')
Set-Location $root

$logDir = Join-Path $root 'test-results/ut'
if (-not (Test-Path $logDir)) { New-Item -ItemType Directory -Path $logDir | Out-Null }
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$logFile = Join-Path $logDir "ut-$stamp.log"

Write-Host "=== run_ut.ps1 ===" -ForegroundColor Cyan
Write-Host "  Layer:    UT (cargo test --lib)"
Write-Host "  Target:   $TargetDir"
Write-Host "  Log:      $logFile"
Write-Host "  Timeout:  ${TimeoutSec}s"
Write-Host ""

# Default uses a workspace-local target dir to avoid contention with other agents
# sharing the E: cargo cache. Pass -UseSharedCache (future) to override.
$env:CARGO_TARGET_DIR = Join-Path $root $TargetDir
if (-not (Test-Path $env:CARGO_TARGET_DIR)) {
    New-Item -ItemType Directory -Path $env:CARGO_TARGET_DIR | Out-Null
}

$args = @('test', '-p', 'ada-mock', '--lib')
if ($NoFailFast) { $args += '--no-fail-fast' }

Write-Host "+ cargo $($args -join ' ')" -ForegroundColor Yellow
$proc = Start-Process -FilePath 'cargo' -ArgumentList $args `
    -NoNewWindow -PassThru `
    -RedirectStandardOutput (Join-Path $logDir 'ut-stdout.tmp') `
    -RedirectStandardError  (Join-Path $logDir 'ut-stderr.tmp')
if (-not $proc.WaitForExit($TimeoutSec * 1000)) {
    Write-Host "TIMEOUT (${TimeoutSec}s) - cargo kill" -ForegroundColor Red
    try { $proc.Kill($true) } catch {}
    exit 124
}
$exit = $proc.ExitCode

# Merge stdout/stderr into a timestamped log
$ts = Get-Date -Format 'yyyy-MM-dd HH:mm:ss'
"=== run_ut $ts exit=$exit ===" | Out-File -FilePath $logFile -Encoding UTF8
"--- stdout ---" | Out-File -FilePath $logFile -Append -Encoding UTF8
if (Test-Path (Join-Path $logDir 'ut-stdout.tmp')) {
    Get-Content (Join-Path $logDir 'ut-stdout.tmp') -Raw | Out-File -FilePath $logFile -Append -Encoding UTF8
}
"--- stderr ---" | Out-File -FilePath $logFile -Append -Encoding UTF8
if (Test-Path (Join-Path $logDir 'ut-stderr.tmp')) {
    Get-Content (Join-Path $logDir 'ut-stderr.tmp') -Raw | Out-File -FilePath $logFile -Append -Encoding UTF8
}
Remove-Item (Join-Path $logDir 'ut-stdout.tmp') -ErrorAction SilentlyContinue
Remove-Item (Join-Path $logDir 'ut-stderr.tmp')  -ErrorAction SilentlyContinue

# latest symlink-style copy
$latest = Join-Path $logDir 'ut-latest.log'
if (Test-Path $latest) { Remove-Item $latest -Force }
Copy-Item $logFile $latest

# Extract pass/fail summary from log (test result: ok. NN passed; ...)
$summary = ''
foreach ($line in (Get-Content $logFile)) {
    if ($line -match 'test result:\s*(ok|FAILED).*?(\d+)\s+passed;\s*(\d+)\s+failed') {
        $summary = "  $($Matches[1]) | passed=$($Matches[2]) failed=$($Matches[3])"
        break
    }
}

if ($exit -ne 0) {
    Write-Host "UT FAIL - see $logFile" -ForegroundColor Red
    if ($summary) { Write-Host $summary -ForegroundColor Red }
    exit $exit
}
Write-Host "UT OK" -ForegroundColor Green
if ($summary) { Write-Host $summary }
exit 0
