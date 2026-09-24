#!/usr/bin/env pwsh
# run_regression.ps1 - Drive UT -> IT -> ST in sequence and aggregate results into
# test-results/regression-{ts}/summary.{md,json,log} ?????.
#
# ??:
#   "mock ??????? UT/IT/ST ????????????" ??????????.
#   ?????? .latest.log ????, PASS/FAIL/?????.
#   1 ??? FAIL ????? 0 ? exit (=????).
#
# ??:
#   pwsh scripts/run_regression.ps1                          # ?? (?? + ??)
#   pwsh scripts/run_regression.ps1 -SkipCoverage            # llvm-cov ??????
#   pwsh scripts/run_regression.ps1 -TargetDir .ada-mock-target
#
# ??:
#   test-results/regression-{yyyyMMdd-HHmmss}/summary.md
#   test-results/regression-{yyyyMMdd-HHmmss}/summary.json
#   test-results/regression-{yyyyMMdd-HHmmss}/run.log        # ???????
#   test-results/regression-{yyyyMMdd-HHmmss}/ut-latest.log  # ??????????
#   test-results/regression-{yyyyMMdd-HHmmss}/it-latest.log
#   test-results/regression-{yyyyMMdd-HHmmss}/st-latest.log

param(
    [switch]$SkipCoverage = $false,
    [string]$TargetDir = '.ada-mock-target',
    [int]$TimeoutSec = 600,
    [switch]$DryRun = $false
)
$ErrorActionPreference = 'Stop'

$root = Resolve-Path (Join-Path $PSScriptRoot '..\..\..')
Set-Location $root

$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$outDir = Join-Path $root "test-results/regression-$stamp"
if (-not (Test-Path $outDir)) { New-Item -ItemType Directory -Path $outDir | Out-Null }

$runLog = Join-Path $outDir 'run.log'
"" | Out-File -FilePath $runLog -Encoding UTF8

function Log([string]$msg) {
    $line = "$(Get-Date -Format 'HH:mm:ss') $msg"
    Write-Host $line
    Add-Content -Path $runLog -Value $line
}

function Run-Layer([string]$Layer, [string]$Script) {
    Log "--- ${Layer}: pwsh ${Script} -TargetDir $TargetDir ---"
    if ($DryRun) { Log "  (DryRun - skip)"; return 0 }
    $scriptPath = Join-Path $PSScriptRoot $Script
    $p = Start-Process -FilePath 'pwsh' -ArgumentList @('-NoProfile', '-File', $scriptPath, '-TargetDir', $TargetDir, '-TimeoutSec', "$TimeoutSec") `
        -NoNewWindow -PassThru -Wait `
        -RedirectStandardOutput (Join-Path $outDir "$Layer-stdout.tmp") `
        -RedirectStandardError  (Join-Path $outDir "$Layer-stderr.tmp")
    if (Test-Path (Join-Path $outDir "$Layer-stdout.tmp")) {
        Get-Content (Join-Path $outDir "$Layer-stdout.tmp") | Add-Content -Path $runLog
    }
    if (Test-Path (Join-Path $outDir "$Layer-stderr.tmp")) {
        Get-Content (Join-Path $outDir "$Layer-stderr.tmp") | Add-Content -Path $runLog
    }
    Remove-Item (Join-Path $outDir "$Layer-stdout.tmp") -ErrorAction SilentlyContinue
    Remove-Item (Join-Path $outDir "$Layer-stderr.tmp") -ErrorAction SilentlyContinue
    return $p.ExitCode
}

Log "=== ada-mock regression run $stamp ==="
Log "Workspace: $root"
Log "Target:    $TargetDir"
Log ""

$utExit = Run-Layer 'UT' 'run_ut.ps1'
Log "UT exit=$utExit"
Log ""
$itExit = Run-Layer 'IT' 'run_it.ps1'
Log "IT exit=$itExit"
Log ""
$stExit = Run-Layer 'ST' 'run_st.ps1'
Log "ST exit=$stExit"
Log ""

# ???????????
$utLatest = Join-Path $root 'test-results/ut/ut-latest.log'
$itLatest = Join-Path $root 'test-results/it/it-latest.log'
$stLatest = Join-Path $root 'test-results/st/st-latest.log'
foreach ($pair in @(
        @{ Src = $utLatest; Dst = 'ut-latest.log' },
        @{ Src = $itLatest; Dst = 'it-latest.log' },
        @{ Src = $stLatest; Dst = 'st-latest.log' }
    )) {
    if (Test-Path $pair.Src) {
        Copy-Item $pair.Src (Join-Path $outDir $pair.Dst) -Force
    }
}

# ?? (Python ???)
$aggScript = Join-Path $PSScriptRoot 'aggregate_results.py'
$jsonOut   = Join-Path $outDir 'summary.json'
$mdOut     = Join-Path $outDir 'summary.md'
Log "Aggregating via $aggScript ..."
$env:REGRESSION_OUT_DIR = $outDir
python $aggScript 2>&1 | Tee-Object -FilePath (Join-Path $outDir 'aggregate-stdout.log')
if ($LASTEXITCODE -ne 0) {
    Log "aggregator FAILED (exit=$LASTEXITCODE) - continuing with raw exit codes"
} else {
    Log "summary.json: $jsonOut"
    Log "summary.md:   $mdOut"
}

Log ""
Log "=== regression summary ==="
if (Test-Path $jsonOut) {
    Get-Content $jsonOut | Add-Content -Path $runLog
}

# 1 ??????? exit 1
$overall = if (($utExit -eq 0) -and ($itExit -eq 0) -and ($stExit -eq 0)) { 0 } else { 1 }
Log "OVERALL: ut=$utExit it=$itExit st=$stExit -> exit $overall"
exit $overall
