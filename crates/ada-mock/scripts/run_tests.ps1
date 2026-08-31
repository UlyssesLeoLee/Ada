#!/usr/bin/env pwsh
# run_tests.ps1 — 跑 mock crate 测试并出结构化结果.
#
# 用法:
#   pwsh scripts/run_tests.ps1                  # 全测
#   pwsh scripts/run_tests.ps1 -Feature server  # 开 server feature
#   pwsh scripts/run_tests.ps1 -NoFailFast      # 失败也跑到底
$ErrorActionPreference = 'Stop'

param(
    [switch]$Feature = $false,
    [string]$FeatureName = 'server',
    [switch]$NoFailFast = $false
)

$root = Resolve-Path (Join-Path $PSScriptRoot '..\..\..')
Set-Location $root

$args = @('test', '-p', 'ada-mock')
if ($Feature) { $args += @('--features', $FeatureName) }
if ($NoFailFast) { $args += '--no-fail-fast' }

$logDir = Join-Path $root 'test-results'
if (-not (Test-Path $logDir)) { New-Item -ItemType Directory -Path $logDir | Out-Null }
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$logFile = Join-Path $logDir "ada-mock-$stamp.txt"

Write-Host "=== run_tests.ps1 ===" -ForegroundColor Cyan
Write-Host "Log: $logFile"

& cargo @args 2>&1 | Tee-Object -FilePath $logFile
if ($LASTEXITCODE -ne 0) {
    Write-Host "FAIL — see $logFile" -ForegroundColor Red
    exit $LASTEXITCODE
}
Write-Host "OK"
exit 0
