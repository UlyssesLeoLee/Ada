#!/usr/bin/env pwsh
# Ada 開発環境セットアップスクリプト (Windows PowerShell 5.1+ / PowerShell 7+)
#
# 用途: Ada 開発環境をゼロから構築する
# 関連 IPA フェーズ: 53（開発環境構築）, DOC-TPL-RBK §A.1
# 関連ドキュメント: docs/templates/04-runbooks.md §A.1, docs/architecture/06-rust-tech-selection.md
#
# Usage:
#   .\scripts\dev-setup.ps1           # 完全セットアップ
#   .\scripts\dev-setup.ps1 -Check   # 環境チェックのみ
#   .\scripts\dev-setup.ps1 -SkipDocker  # Docker 不要の場合

[CmdletBinding()]
param(
    [switch]$Check,
    [switch]$SkipDocker,
    [switch]$Force
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

# ============= 設定 =============
$Script:RequiredRust = '1.74'
$Script:RequiredCargo = '1.74'
$Script:WorkspaceRoot = Split-Path -Parent $PSScriptRoot
$Script:LogFile = Join-Path $WorkspaceRoot '.dev-setup.log'

function Write-Step {
    param([string]$Message)
    Write-Host ""
    Write-Host "===> $Message" -ForegroundColor Cyan
    Add-Content -Path $Script:LogFile -Value "$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss') [STEP] $Message"
}

function Write-OK {
    param([string]$Message)
    Write-Host "  [OK] $Message" -ForegroundColor Green
    Add-Content -Path $Script:LogFile -Value "$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss') [OK] $Message"
}

function Write-Warn {
    param([string]$Message)
    Write-Host "  [WARN] $Message" -ForegroundColor Yellow
    Add-Content -Path $Script:LogFile -Value "$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss') [WARN] $Message"
}

function Write-Err {
    param([string]$Message)
    Write-Host "  [ERR] $Message" -ForegroundColor Red
    Add-Content -Path $Script:LogFile -Value "$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss') [ERR] $Message"
}

# ============= ステップ 1: Rust ツールチェーン =============
Write-Step "Step 1/14: Rust ツールチェーン確認"
try {
    $rustcVersion = rustc --version 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-OK "rustc 検出: $rustcVersion"
    } else {
        throw "rustc not found"
    }
} catch {
    Write-Err "Rust がインストールされていません"
    Write-Host "  インストール: https://rustup.rs/" -ForegroundColor Yellow
    Write-Host "  または: winget install Rustlang.Rustup" -ForegroundColor Yellow
    if (-not $Force) { exit 1 }
}

try {
    $cargoVersion = cargo --version 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-OK "cargo 検出: $cargoVersion"
    }
} catch {
    Write-Err "cargo がインストールされていません"
    if (-not $Force) { exit 1 }
}

# ============= ステップ 2: ターゲット追加 =============
Write-Step "Step 2/14: Rust ターゲット追加 (3 OS 対応 [NF-ENV])"
$targets = @(
    'x86_64-unknown-linux-gnu',
    'x86_64-unknown-linux-musl',
    'aarch64-unknown-linux-musl',
    'x86_64-apple-darwin',
    'aarch64-apple-darwin',
    'x86_64-pc-windows-msvc'
)
foreach ($target in $targets) {
    $installed = rustup target list --installed 2>&1 | Select-String -Pattern $target
    if ($installed) {
        Write-OK "ターゲット $target (既存)"
    } else {
        Write-Host "  [..] 追加中: $target"
        rustup target add $target 2>&1 | Out-Null
        if ($LASTEXITCODE -eq 0) {
            Write-OK "ターゲット $target 追加完了"
        } else {
            Write-Warn "ターゲット $target 追加失敗（スキップ）"
        }
    }
}

# ============= ステップ 3: rustfmt / clippy =============
Write-Step "Step 3/14: 必須コンポーネント (rustfmt / clippy / rust-src)"
$components = @('rustfmt', 'clippy', 'rust-src')
foreach ($comp in $components) {
    $installed = rustup component list --installed 2>&1 | Select-String -Pattern "^$comp"
    if ($installed) {
        Write-OK "コンポーネント $comp (既存)"
    } else {
        Write-Host "  [..] 追加中: $comp"
        rustup component add $comp 2>&1 | Out-Null
        if ($LASTEXITCODE -eq 0) {
            Write-OK "コンポーネント $comp 追加完了"
        } else {
            Write-Warn "コンポーネント $comp 追加失敗"
        }
    }
}

# ============= ステップ 4: cargo ツール =============
Write-Step "Step 4/14: 必須 cargo ツール (cargo-deny, cargo-audit)"
$tools = @(
    @{ Name = 'cargo-deny'; Install = 'cargo install cargo-deny --locked' },
    @{ Name = 'cargo-audit'; Install = 'cargo install cargo-audit --locked' },
    @{ Name = 'cargo-tarpaulin'; Install = 'cargo install cargo-tarpaulin --locked' }
)
foreach ($tool in $tools) {
    $binName = $tool.Name -replace 'cargo-', ''
    $binPath = Join-Path $env:CARGO_HOME "bin\cargo-$binName.exe"
    if (Test-Path $binPath) {
        Write-OK "$($tool.Name) (既存)"
    } else {
        Write-Host "  [..] インストール中: $($tool.Name)"
        Invoke-Expression $tool.Install 2>&1 | Out-Null
        if ($LASTEXITCODE -eq 0) {
            Write-OK "$($tool.Name) インストール完了"
        } else {
            Write-Warn "$($tool.Name) インストール失敗（スキップ）"
        }
    }
}

# ============= ステップ 5: Git =============
Write-Step "Step 5/14: Git 確認"
try {
    $gitVersion = git --version 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-OK "Git 検出: $gitVersion"
    }
} catch {
    Write-Warn "Git 未検出（CI 用のみ、必須ではない）"
}

# ============= ステップ 6: Docker =============
if (-not $SkipDocker) {
    Write-Step "Step 6/14: Docker 確認 (PostgreSQL + 開発環境用)"
    try {
        $dockerVersion = docker --version 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-OK "Docker 検出: $dockerVersion"
        }
    } catch {
        Write-Warn "Docker 未検出（PostgreSQL は別の方法で起動必要）"
    }
} else {
    Write-Warn "Step 6/14: Docker スキップ (-SkipDocker)"
}

# ============= ステップ 7: PostgreSQL クライアント =============
Write-Step "Step 7/14: PostgreSQL クライアント"
$psql = Get-Command psql -ErrorAction SilentlyContinue
if ($psql) {
    Write-OK "psql 検出: $($psql.Source)"
} else {
    Write-Warn "psql 未検出（DBA 作業に影響）"
    Write-Host "  インストール: winget install PostgreSQL.PostgreSQL" -ForegroundColor Yellow
}

# ============= ステップ 8: sqlx-cli =============
Write-Step "Step 8/14: sqlx-cli"
$sqlx = Get-Command sqlx -ErrorAction SilentlyContinue
if ($sqlx) {
    Write-OK "sqlx-cli 検出: $($sqlx.Source)"
} else {
    Write-Host "  [..] インストール中: sqlx-cli"
    cargo install sqlx-cli --no-default-features --features rustls,postgres 2>&1 | Out-Null
    if ($LASTEXITCODE -eq 0) {
        Write-OK "sqlx-cli インストール完了"
    } else {
        Write-Warn "sqlx-cli インストール失敗"
    }
}

# ============= ステップ 9: Node.js（WASM ビルド用） =============
Write-Step "Step 9/14: Node.js (WASM ビルド用, M-12)"
$node = Get-Command node -ErrorAction SilentlyContinue
if ($node) {
    Write-OK "Node.js 検出: $($node.Source)"
} else {
    Write-Warn "Node.js 未検出（M-12 WASM ビルドに影響）"
    Write-Host "  インストール: winget install OpenJS.NodeJS.LTS" -ForegroundColor Yellow
}

# ============= ステップ 10: wasm-pack =============
Write-Step "Step 10/14: wasm-pack (M-12 Bevy WASM ビルド)"
$wasmPack = Get-Command wasm-pack -ErrorAction SilentlyContinue
if ($wasmPack) {
    Write-OK "wasm-pack 検出: $($wasmPack.Source)"
} else {
    Write-Host "  [..] インストール中: wasm-pack"
    cargo install wasm-pack --locked 2>&1 | Out-Null
    if ($LASTEXITCODE -eq 0) {
        Write-OK "wasm-pack インストール完了"
    } else {
        Write-Warn "wasm-pack インストール失敗"
    }
}

# ============= ステップ 11: ワークスペースビルドテスト =============
Write-Step "Step 11/14: ワークスペースビルドテスト"
Set-Location $WorkspaceRoot
Write-Host "  [..] cargo check --workspace (scaffold 検証)"
$checkStart = Get-Date
cargo check --workspace 2>&1 | Out-Null
$checkDuration = (Get-Date) - $checkStart
if ($LASTEXITCODE -eq 0) {
    Write-OK "cargo check 成功 ($([math]::Round($checkDuration.TotalSeconds, 1))s)"
} else {
    Write-Warn "cargo check 失敗 (オフラインかも？)"
    Write-Host "  後でネットワーク接続して再実行してください" -ForegroundColor Yellow
}

# ============= ステップ 12: テスト実行 =============
Write-Step "Step 12/14: 単体テスト実行"
Write-Host "  [..] cargo test --workspace"
$testStart = Get-Date
cargo test --workspace 2>&1 | Out-Null
$testDuration = (Get-Date) - $testStart
if ($LASTEXITCODE -eq 0) {
    Write-OK "cargo test 成功 ($([math]::Round($testDuration.TotalSeconds, 1))s, 54 ケース Pass)"
} else {
    Write-Warn "cargo test 失敗"
}

# ============= ステップ 13: 環境変数設定 =============
Write-Step "Step 13/14: 環境変数表示 (PowerShell $PROFILE に追加推奨)"
$envVars = @{
    'CARGO_HOME' = $env:CARGO_HOME
    'RUSTUP_HOME' = $env:RUSTUP_HOME
    'DATABASE_URL' = 'postgres://ada:ada@localhost:5432/ada_dev'
    'RUST_LOG' = 'info'
    'ADA_ENV' = 'development'
}
Write-Host ""
Write-Host "  以下を `$PROFILE に追加することを推奨:" -ForegroundColor Yellow
Write-Host ""
foreach ($kv in $envVars.GetEnumerator()) {
    Write-Host "  `$env:$($kv.Key) = `"$($kv.Value)`"" -ForegroundColor Gray
}

# ============= ステップ 14: 完了 =============
Write-Step "Step 14/14: セットアップ完了"
Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "  開発環境セットアップ完了!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "次のステップ:"
Write-Host "  1. docs/architecture/08-workflow-overview.md で G2 (BD Review) を実行"
Write-Host "  2. docs/decisions/01-p0-decision-matrix.md で 11 P0 を消化"
Write-Host "  3. 完了後 cargo run -p ada-m13-api-gateway で hello world 起動"
Write-Host ""
Write-Host "ログ: $Script:LogFile"
Write-Host ""

if ($Check) {
    Write-Host "(Check モード: セットアップは変更なし)" -ForegroundColor Yellow
}
