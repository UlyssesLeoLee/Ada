#!/usr/bin/env bash
# 一键构建 + 尺寸校验 (per D-05 WASM 8 MB / gzip 3 MB)
#
# 用法:
#   ./wasm/build.sh                     # 默认: target=web, release
#   ./wasm/build.sh --target nodejs      # 显式 target
#   ./wasm/build.sh --features full     # 同时启用 wasm + bevy
#   ./wasm/build.sh --skip-size-check   # 跳过尺寸校验
#
# 退出码:
#   0  build + size check 全过
#   1  build 失败 / 尺寸超阈值 / 工具链缺失
set -euo pipefail

CRATE_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$CRATE_DIR"

TARGET="web"
FEATURES="wasm"
SKIP_SIZE=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --target) TARGET="$2"; shift 2;;
        --features) FEATURES="$2"; shift 2;;
        --skip-size-check) SKIP_SIZE=1; shift;;
        -h|--help)
            sed -n '2,16p' "$0"
            exit 0;;
        *) echo "unknown arg: $1" >&2; exit 1;;
    esac
done

# 1) 工具链前置
# 兼容 Git Bash / MSYS:Windows 风格 PATH 下 `command -v wasm-pack`
# 可能找不到,但实际 PATH 里有(因 mintty / MSYS PATH 处理)。
# 因此先 `command -v`,失败再尝试绝对路径常见位置。
WASM_PACK="${WASM_PACK:-$(command -v wasm-pack 2>/dev/null || true)}"
if [[ -z "$WASM_PACK" ]]; then
    # Fallback 列表覆盖 Git Bash / MSYS / WSL2 下的常见位置。
    for cand in \
        "$HOME/.cargo/bin/wasm-pack" \
        "$HOME/.cargo/bin/wasm-pack.exe" \
        "/c/Users/$USER/.cargo/bin/wasm-pack.exe" \
        "/e/DevCache/cargo/bin/wasm-pack.exe" \
        "/c/ProgramData/chocolatey/bin/wasm-pack.exe" \
        "/mnt/e/DevCache/cargo/bin/wasm-pack.exe" \
        "/mnt/c/Users/$USER/.cargo/bin/wasm-pack.exe"; do
        if [[ -x "$cand" ]]; then
            WASM_PACK="$cand"
            break
        fi
    done
fi
if [[ -z "$WASM_PACK" || ! -x "$WASM_PACK" ]]; then
    echo "error: wasm-pack not found" >&2
    echo "  install: cargo install wasm-pack" >&2
    echo "  or:      curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh" >&2
    echo "  or set:  WASM_PACK=/path/to/wasm-pack" >&2
    exit 1
fi

if ! rustup target list --installed 2>/dev/null | grep -q "wasm32-unknown-unknown"; then
    echo "error: wasm32-unknown-unknown target not installed" >&2
    echo "  install: rustup target add wasm32-unknown-unknown" >&2
    echo "  (or:    'rustup target add wasm32-unknown-unknown' from PowerShell)" >&2
    exit 1
fi

# 2) 构建
echo "=== $WASM_PACK build --target $TARGET --features $FEATURES ==="
"$WASM_PACK" build \
    --target "$TARGET" \
    --release \
    --features "$FEATURES" \
    --out-dir pkg

# 3) 尺寸校验 (D-05)
if [[ $SKIP_SIZE -eq 0 ]]; then
    echo "=== size check (D-05: raw 8 MB / gzip 3 MB) ==="
    exec ./wasm/size-check.sh
fi

echo "OK"
