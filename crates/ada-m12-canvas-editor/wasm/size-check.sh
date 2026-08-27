#!/usr/bin/env bash
# 单独跑 WASM 尺寸校验 (per D-05 raw 8 MB / gzip 3 MB)
#
# 用法:
#   ./wasm/size-check.sh
#
# 退出码:
#   0  尺寸合规
#   1  超阈值 / 找不到 artifact
set -euo pipefail

CRATE_DIR="$(cd "$(dirname "$0")/.." && pwd)"
PKG_DIR="$CRATE_DIR/pkg"

# 阈值 (per docs/decisions/02-design-adrs.md D-05)
RAW_MAX=$((8 * 1024 * 1024))        # 8 MB
GZIP_MAX=$((3 * 1024 * 1024))       # 3 MB

WASM_FILE="$PKG_DIR/ada_m12_canvas_editor_bg.wasm"

if [[ ! -f "$WASM_FILE" ]]; then
    echo "error: $WASM_FILE not found" >&2
    echo "  run: wasm-pack build --target web --release --features wasm" >&2
    exit 1
fi

# Windows + Git Bash: 'stat -c' 不存在, 用 fallback
if stat -c%s "$WASM_FILE" >/dev/null 2>&1; then
    RAW_BYTES=$(stat -c%s "$WASM_FILE")
else
    RAW_BYTES=$(wc -c < "$WASM_FILE" | tr -d ' ')
fi

GZIP_BYTES=$(gzip -c "$WASM_FILE" | wc -c | tr -d ' ')

RAW_MB=$(awk "BEGIN { printf \"%.3f\", $RAW_BYTES / 1024 / 1024 }")
GZIP_MB=$(awk "BEGIN { printf \"%.3f\", $GZIP_BYTES / 1024 / 1024 }")

printf "  raw   %s bytes (%s MiB)  (limit %d)\n" "$RAW_BYTES" "$RAW_MB" "$RAW_MAX"
printf "  gzip  %s bytes (%s MiB)  (limit %d)\n" "$GZIP_BYTES" "$GZIP_MB" "$GZIP_MAX"

if [[ $RAW_BYTES -gt $RAW_MAX ]]; then
    echo "FAIL: raw .wasm exceeds D-05 8 MB ceiling" >&2
    exit 1
fi
if [[ $GZIP_BYTES -gt $GZIP_MAX ]]; then
    echo "FAIL: gzipped .wasm exceeds D-05 3 MB ceiling" >&2
    exit 1
fi

echo "OK: D-05 size budget respected"
