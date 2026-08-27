#!/usr/bin/env bash
# 跑浏览器内单元测试 (wasm-bindgen-test)
#
# 用法:
#   ./wasm/test.sh                     # 默认: headless chrome
#   ./wasm/test.sh --firefox           # 改用 firefox
#   ./wasm/test.sh --node              # 改用 node
#
# 退出码:
#   0  全过
#   1  失败 / 工具链缺失
set -euo pipefail

CRATE_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$CRATE_DIR"

BROWSER="chrome"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --chrome) BROWSER="chrome"; shift;;
        --firefox) BROWSER="firefox"; shift;;
        --safari) BROWSER="safari"; shift;;
        --node) BROWSER="node"; shift;;
        -h|--help)
            sed -n '2,15p' "$0"
            exit 0;;
        *) echo "unknown arg: $1" >&2; exit 1;;
    esac
done

if ! command -v wasm-pack >/dev/null 2>&1; then
    echo "error: wasm-pack not installed" >&2
    exit 1
fi

WASM_PACK="$(command -v wasm-pack)"

case "$BROWSER" in
    chrome|firefox|safari)
        echo "=== $WASM_PACK test --headless --$BROWSER ==="
        "$WASM_PACK" test \
            --headless \
            "--$BROWSER" \
            --features wasm-test \
            -p ada-m12-canvas-editor
        ;;
    node)
        echo "=== $WASM_PACK test --node ==="
        "$WASM_PACK" test \
            --node \
            --features wasm-test \
            -p ada-m12-canvas-editor
        ;;
esac
