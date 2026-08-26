#!/usr/bin/env bash
# Ada DB テストランナー (bash / zsh / Git Bash)
# PostgreSQL 18.6 用の DDL + PL/pgSQL 存过 + 単体テストを実行
#
# Usage:
#   DB=ada_test ./db/run-tests.sh
#   DB=ada_test ./db/run-tests.sh --skip-migrate
#   DB=ada_test ./db/run-tests.sh --verbose
#
# Env:
#   DB    - 接続データベース (default: ada_test)
#   USER  - 接続ユーザー (default: $PGUSER or ada)
#   HOST  - 接続ホスト (default: $PGHOST or localhost)
#   PORT  - 接続ポート (default: $PGPORT or 5432)
#   PGPASSWORD - 接続パスワード (default: prompt)

set -euo pipefail

DB="${DB:-ada_test}"
USER="${USER:-${PGUSER:-ada}}"
HOST="${HOST:-${PGHOST:-localhost}}"
PORT="${PORT:-${PGPORT:-5432}}"

SKIP_MIGRATE=0
VERBOSE=0
for arg in "$@"; do
    case "$arg" in
        --skip-migrate) SKIP_MIGRATE=1 ;;
        --verbose|-v)   VERBOSE=1 ;;
        --help|-h)
            sed -n '2,20p' "$0"
            exit 0
            ;;
        *) echo "unknown arg: $arg" >&2; exit 64 ;;
    esac
done

PSQL="psql -h $HOST -p $PORT -U $USER -d $DB -v ON_ERROR_STOP=1"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MIGRATIONS_DIR="$SCRIPT_DIR/migrations"
TESTS_DIR="$SCRIPT_DIR/tests"

red()    { printf '\033[31m%s\033[0m\n' "$*"; }
green()  { printf '\033[32m%s\033[0m\n' "$*"; }
yellow() { printf '\033[33m%s\033[0m\n' "$*"; }
cyan()   { printf '\033[36m%s\033[0m\n' "$*"; }

# 1. psql 存在チェック
if ! command -v psql >/dev/null 2>&1; then
    red "ERROR: psql not found in PATH"
    echo "  Install: winget install PostgreSQL.PostgreSQL  (Windows)"
    echo "           apt-get install postgresql-client     (Debian/Ubuntu)"
    echo "           brew install postgresql@18             (macOS)"
    exit 127
fi

PSQL_VERSION="$(psql --version)"
cyan "==> psql: $PSQL_VERSION"
cyan "==> target: $DB@$HOST:$PORT as $USER"

# 2. 接続確認
if ! $PSQL -c "SELECT 1" >/dev/null 2>&1; then
    red "ERROR: cannot connect to $DB@$HOST:$PORT as $USER"
    echo "  Set PGPASSWORD or use ~/.pgpass"
    exit 1
fi

# 3. マイグレーション
if [ "$SKIP_MIGRATE" = "0" ]; then
    cyan "==> applying migrations"
    for f in "$MIGRATIONS_DIR"/V*.sql; do
        [ -f "$f" ] || continue
        echo "    $f"
        if [ "$VERBOSE" = "1" ]; then
            $PSQL -f "$f"
        else
            $PSQL -f "$f" -q
        fi
    done
    green "    migrations applied"
else
    yellow "==> skipping migrations (--skip-migrate)"
fi

# 4. テスト
cyan "==> running tests"
PASS=0
FAIL=0
FAILED_FILES=()
for f in "$TESTS_DIR"/V*.sql; do
    [ -f "$f" ] || continue
    echo "    $f"
    out=$($PSQL -f "$f" 2>&1) || {
        red "    FAILED: $f"
        echo "$out" | tail -30
        FAIL=$((FAIL+1))
        FAILED_FILES+=("$f")
        continue
    }
    # PASS / FAIL カウント
    pcount=$(echo "$out" | grep -c '^NOTICE:.*PASS:' || true)
    fcount=$(echo "$out" | grep -cE '^\[t_|FAIL:.*TEST FAIL|EXCEPTION:' || true)
    if [ "$VERBOSE" = "1" ]; then
        echo "$out" | grep -E '^NOTICE:|^ERROR:|^FAIL:' || true
    else
        echo "      $(echo "$out" | grep -E '^NOTICE:.*PASS:' | wc -l) PASS notices"
    fi
    if [ "$fcount" -gt 0 ]; then
        red "    FAILED: $f ($fcount error(s))"
        echo "$out" | grep -E '^\[t_|FAIL:.*TEST FAIL|EXCEPTION:' | head -5
        FAIL=$((FAIL+1))
        FAILED_FILES+=("$f")
    else
        green "    OK: $f ($pcount PASS notices)"
        PASS=$((PASS+1))
    fi
done

# 5. 結果サマリ
echo
cyan "==> summary"
echo "    passed: $PASS file(s)"
echo "    failed: $FAIL file(s)"
if [ "$FAIL" -gt 0 ]; then
    red "FAILED files:"
    for f in "${FAILED_FILES[@]}"; do
        echo "    - $f"
    done
    exit 1
fi
green "==> all tests passed"
exit 0
