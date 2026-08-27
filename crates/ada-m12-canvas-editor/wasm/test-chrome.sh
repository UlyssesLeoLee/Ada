#!/usr/bin/env bash
# test-chrome.sh — E2E wasm-pack test in headless Chrome.
#
# Per `docs/observability/11-phased-rollout.md` §6 Phase 4 +
# `docs/observability/05-tracing-design.md` §3.4 (W3C Trace
# Context propagation), the m12 canvas WASM build must be
# tested in a real browser environment, not just node. This
# script runs the test suite under Chrome (or Node.js as a
# fallback). It is run by:
#   - manual developer testing (`./wasm/test-chrome.sh`)
#   - CI pipeline (added in a follow-up WT)
#
# 退出码:
#   0  - all tests pass
#   1  - test failure
#   2  - chrome / wasm-pack not found
#
# 与 `wasm/test.sh` 的区别:
#   - `test.sh` 接受任意浏览器 flag (chrome/firefox/safari/node),
#     适合 dev 调试。
#   - `test-chrome.sh` 是 CI 入口,只关心 chrome → node fallback
#     二选一,失败时用统一 exit code 2 表示工具链缺失,方便
#     CI runner 区分 "测试 fail" vs "环境 fail"。
#
# 设计依据:
#   - `docs/observability/11-phased-rollout.md` §6 Phase 4
#   - `docs/observability/05-tracing-design.md` §3.4
#   - `docs/modules/M-12-canvas-editor-frontend.md` §3.4
#     (WASM ↔ JS 桥接契约, wasm-bindgen-test 跑浏览器内集成)
set -euo pipefail

CRATE_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$CRATE_DIR"

if ! command -v wasm-pack >/dev/null 2>&1; then
    echo "ERROR: wasm-pack not found. Install with:" >&2
    echo "  cargo install wasm-pack" >&2
    exit 2
fi

# Default to Chrome; fall back to Node.js if Chrome is not
# available (Node.js path tests everything except the DOM).
# CI runner 必须有 chrome; 本地 dev 允许 node fallback。
if command -v google-chrome >/dev/null 2>&1 \
    || command -v chrome >/dev/null 2>&1 \
    || command -v chromium >/dev/null 2>&1 \
    || command -v "Google Chrome" >/dev/null 2>&1; then
    echo "==> Running wasm-pack test in headless Chrome"
    exec wasm-pack test \
        --headless \
        --chrome \
        --features wasm-test \
        -p ada-m12-canvas-editor
elif command -v node >/dev/null 2>&1; then
    echo "==> Chrome not found, falling back to wasm-pack test in Node.js"
    exec wasm-pack test \
        --node \
        --features wasm-test \
        -p ada-m12-canvas-editor
else
    echo "ERROR: neither chrome nor node found" >&2
    exit 2
fi
