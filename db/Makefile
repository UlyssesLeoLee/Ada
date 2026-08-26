# Ada DB Migration Makefile
# PostgreSQL 18.6 用の DDL + PL/pgSQL 存过 + 単体テストの実行ランナー
#
# 使い方:
#   make -C db migrate    # マイグレーションのみ適用
#   make -C db test       # マイグレーション + テスト実行
#   make -C db clean      # テストデータ全消去 (スキーマは残す)
#   make -C db drop       # db/ が作成したオブジェクト全削除 (要確認)
#
# 環境変数 (上書き可):
#   DB    - 接続先データベース名 (default: ada_dev)
#   USER  - 接続ユーザー (default: $PGUSER or ada)
#   HOST  - 接続ホスト (default: $PGHOST or localhost)
#   PORT  - 接続ポート (default: $PGPORT or 5432)

DB    ?= ada_dev
USER  ?= $(or $(PGUSER),ada)
HOST  ?= $(or $(PGHOST),localhost)
PORT  ?= $(or $(PGPORT),5432)

MIGRATIONS_DIR := migrations
TESTS_DIR      := tests

PSQL       := psql -h $(HOST) -p $(PORT) -U $(USER) -d $(DB) -v ON_ERROR_STOP=1
PSQL_QUIET := psql -h $(HOST) -p $(PORT) -U $(USER) -d $(DB) -v ON_ERROR_STOP=1 -q -X

MIGRATIONS := $(wildcard $(MIGRATIONS_DIR)/V*.sql)
TESTS      := $(wildcard $(TESTS_DIR)/V*.sql)

.PHONY: help migrate test clean drop status check

help:
	@echo "Ada DB targets:"
	@echo "  make -C db migrate   apply migrations (idempotent)"
	@echo "  make -C db test      apply migrations + run tests"
	@echo "  make -C db clean     TRUNCATE all tables (data only, schema remains)"
	@echo "  make -C db drop      DROP all db/* created objects (DANGEROUS)"
	@echo "  make -C db status    show connection + object count"
	@echo "  make -C db check     syntax-check (psql --check, no execution)"

migrate:
	@echo "==> applying migrations to $(DB)@$(HOST):$(PORT) as $(USER)"
	@for f in $(MIGRATIONS); do \
		echo "    $$f"; \
		$(PSQL) -f $$f || exit 1; \
	done
	@echo "==> migrations applied: $(words $(MIGRATIONS)) files"

test: migrate
	@echo "==> running tests"
	@for f in $(TESTS); do \
		echo "    $$f"; \
		$(PSQL) -f $$f 2>&1 | grep -E '^(PASS:|FAIL:|\[t_|NOTICE:)' || true; \
		$(PSQL_QUIET) -f $$f > /dev/null 2>&1 || { \
			echo "    FAILED: $$f"; exit 1; \
		}; \
	done
	@echo "==> all tests passed: $(words $(TESTS)) files"

clean:
	@echo "==> TRUNCATE all tables (test data + history)"
	@$(PSQL) -c "TRUNCATE \
		event_log, consumer_offset, event_subscription, event_topic, \
		module_upgrade_history, module_instance, module_registry, \
		leader_lease, shard_assignment, cluster_node, \
		tenant RESTART IDENTITY CASCADE;" || exit 1
	@echo "==> truncated (sequence reset)"

drop:
	@echo "==> DROPPING all db/* created objects"
	@$(PSQL) -c "DROP TABLE IF EXISTS \
		module_instance, module_upgrade_history, module_registry, \
		event_log, consumer_offset, event_subscription, event_topic, \
		leader_lease, shard_assignment, cluster_node, \
		tenant CASCADE; \
		DROP SEQUENCE IF EXISTS event_seq_global;" || exit 1
	@echo "==> dropped (functions retained - re-run V002 to recreate)"

status:
	@$(PSQL) -c "\dt+ public.*" 2>&1 | head -40
	@$(PSQL) -c "SELECT proname FROM pg_proc WHERE pronamespace = 'public'::regnamespace ORDER BY proname;" 2>&1

check:
	@echo "==> syntax check (psql --no-execute mode)"
	@for f in $(MIGRATIONS) $(TESTS); do \
		ok=`$(PSQL) --set ON_ERROR_STOP=on -c "\set ECHO_HIDDEN off" -f $$f 2>&1 | grep -E '(syntax error|ERROR)' | head -1`; \
		if [ -n "$$ok" ]; then \
			echo "    SYNTAX ERROR in $$f:"; echo "$$ok"; exit 1; \
		else \
			echo "    ok: $$f"; \
		fi; \
	done
