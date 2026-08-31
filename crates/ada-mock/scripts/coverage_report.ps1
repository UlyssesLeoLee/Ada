#!/usr/bin/env pwsh
# coverage_report.ps1 — 跑全 workspace 覆盖率并出 HTML 报告.
#
# 依赖:
#   - rustup 已装 `nightly` toolchain (llvm-cov 在 stable 上不可用 per rust 1.98)
#   - cargo-llvm-cov 已装 (`cargo install cargo-llvm-cov`)
#
# 用法:
#   pwsh scripts/coverage_report.ps1                  # 全 workspace, HTML 输出
#   pwsh scripts/coverage_report.ps1 -Crate ada-mock   # 只跑指定 crate
#   pwsh scripts/coverage_report.ps1 -Threshold 80     # 关键模块 80% 门槛
#
# 输出:
#   target/coverage/html/index.html
#   target/coverage/summary.txt   (本脚本附加生成)
$ErrorActionPreference = 'Stop'

param(
    [string]$Crate = '',
    [int]$Threshold = 0
)

$root = Resolve-Path (Join-Path $PSScriptRoot '..\..\..')
Set-Location $root

$manifest = $Crate
if ($Crate) {
    $manifest = "-p $Crate"
}

Write-Host "=== coverage_report.ps1 ===" -ForegroundColor Cyan
Write-Host "Root: $root"
Write-Host "Manifest: $($manifest.Trim() -replace '^-p ', '-p ')"
Write-Host "Threshold: $Threshold%"

# nightly toolchain 必须存在
$nightly = (& rustup toolchain list 2>$null) | Where-Object { $_.Trim() -eq 'nightly-x86_64-pc-windows-msvc' }
if (-not $nightly) {
    Write-Host "Installing nightly toolchain (for llvm-cov)..." -ForegroundColor Yellow
    rustup toolchain install nightly --component llvm-tools-preview
} else {
    Write-Host "Nightly already installed." -ForegroundColor Green
}

# 生成 HTML
& cargo +nightly llvm-cov $manifest.Split(' ') --workspace --all-features --html --output-dir target/coverage
if ($LASTEXITCODE -ne 0) {
    Write-Host "llvm-cov failed" -ForegroundColor Red
    exit 1
}

# 文本摘要
$summary = & cargo +nightly llvm-cov $manifest.Split(' ') --workspace --all-features --summary-only 2>&1
$summary | Out-File -FilePath target/coverage/summary.txt -Encoding UTF8
Write-Host $summary

# 阈值校验 (粗糙: 扫描 summary 文本, 行内含百分比)
if ($Threshold -gt 0) {
    $violations = @()
    foreach ($line in $summary) {
        if ($line -match '^(?<file>[\w\-/\\.]+)\s+(?<cov>[\d.]+)%') {
            $pct = [double]$Matches.cov
            if ($pct -lt $Threshold) {
                $violations += "$($Matches.file): $pct% < $Threshold%"
            }
        }
    }
    if ($violations.Count -gt 0) {
        Write-Host "Coverage threshold NOT met:" -ForegroundColor Red
        $violations | ForEach-Object { Write-Host "  $_" -ForegroundColor Red }
        exit 2
    } else {
        Write-Host "All files >= $Threshold%." -ForegroundColor Green
    }
}

Write-Host "Report: target/coverage/html/index.html"
exit 0
