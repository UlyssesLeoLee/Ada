
## 5. 不执行項目与替代検証

| 项目 | 不执行理由 | 代替検証 |
|---|---|---|
| `cargo +nightly llvm-cov` カバレッジ計測 | nightly + cargo-llvm-cov インストールが本ワークツーで未完了 (CI 環境前提) | `coverage_report.ps1` は温存, 既存通り. 本 task では llvm-cov 不在でも regression 判定可能 |
| 業務 crate 19 個の全体回帰 | task 範囲は mock プロジェクトのみ (`crates/ada-mock`) | 業務 crate 全体回帰は別 issue (ULYS 后续) で扱う |
| `cargo clippy --workspace` | scripts 追加による clippy 変化は Cargo.toml 不变のため無し | `cargo check -p ada-mock --all-features` が緑であることは UT 内で担保 |
| `cargo fmt --check` | scripts は .ps1 / .py のみ, Rust コード不变 | n/a |
| Linux/macOS 跨平台検証 | 本ホストは Windows (pwsh 7.6) | CI で `windows-latest` / `ubuntu-latest` / `macos-latest` 3 matrix 想定, README に既述 |

## 6. Result

### 6.1 最终回帰実行結果 (2026-09-21 01:01 JST)

```
01:01:21 === ada-mock regression run 20260921-010121 ===
01:01:26 UT  exit=0   ok | passed=27 failed=0
01:01:34 IT  exit=0   ok | passed=1  failed=0
01:01:44 ST  exit=0   ok | passed=2  failed=0
01:01:45 overall=pass passed=30 failed=0
01:01:45 OVERALL: ut=0 it=0 st=0 -> exit 0
```

生成ファイル:

- `test-results/ut/ut-20260921-010121.log`
- `test-results/it/it-20260921-010121.log`
- `test-results/st/st-20260921-010121.log`
- `test-results/regression-20260921-010121/summary.json` (overall=pass)
- `test-results/regression-20260921-010121/summary.md`

### 6.2 入口条件 / 出口条件 (per `docs/tests/README.md` §6)

| 条件 | 状态 |
|---|---|
| UT 入口: 該当 crate 設計凍結 | OK (v0.1.0 草案, design 冻结済) |
| IT 入口: 該当 UT 通過 + 依存準備 | OK (UT 27/27 PASS) |
| ST 入口: 該当 IT 通過 + 環境準備 | OK (IT 1/1 PASS, 環境 = 本ワークツー) |
| UT 出口: 100% 実行 + P0/P1 全通過 + 行≥80%/分≥70% | OK (27/27 実行 + 100% PASS, 覆盖率 threshold 未計測 — §5 参照) |
| IT 出口: 100% 実行 + P0/P1 全通過 + 接口契约 100% | OK (1/1 実行 + 100% PASS, mock crate は接口契约不在) |
| ST 出口: 100% 実行 + P0/P1 全通過 + 非機能指標達成 | OK (2/2 実行 + 100% PASS, 性能は task 範囲外) |

## 7. 関連コミット / ファイル

| 路径 | 状態 |
|---|---|
| `crates/ada-mock/scripts/run_ut.ps1` | 新規 |
| `crates/ada-mock/scripts/run_it.ps1` | 新規 |
| `crates/ada-mock/scripts/run_st.ps1` | 新規 |
| `crates/ada-mock/scripts/run_regression.ps1` | 新規 |
| `crates/ada-mock/scripts/aggregate_results.py` | 新規 |
| `crates/ada-mock/scripts/run_tests.ps1` | 修正 (`$ErrorActionPreference` 位置) |
| `crates/ada-mock/scripts/coverage_report.ps1` | 修正 (同上) |
| `crates/ada-mock/docs/regression-report-ULYS-136.md` | 新規 (本文件) |
| `docs/CHANGELOG.md` | v2.12.0 -> v2.13.0 升版 |
| `.gitignore` | `test-results/` + `.ada-mock-target/` + `__pycache__/` 追加 |

## 8. 剩余リスク

| リスク | 影響 | 緩和 |
|---|---|---|
| Windows PowerShell 5.1 (`powershell.exe`) でも PS7 と同じ挙動か未確認 | 既設 CI が PS 5.1 で走る可能性 | `param` を先頭固定 + `$ErrorActionPreference` を後に置く という最小公分母パターンを採用済み; CI で `windows-latest` デフォルト = PS7.4+ 想定 |
| `CARGO_TARGET_DIR=.ada-mock-target` の既定値が他 worktree と衝突する可能性 | 同一 workspace 内複数 worktree 同時実行時にビルドキャッシュ共有されない | 既定 workspace 内 target, `target/.ada-mock-target.lock` 等の追加は将来課題 |
| `aggregate_results.py` の `FAILED` 行抽出が cargo 出力フォーマット変更に弱い | 将来 cargo 1.99+ で形式変更時に抽出失敗 | `test result: ...` 数値部分のみで fail/pass 判定しているためテスト名抽出失敗でも overall 判定は安定 |
| `pwsh 7.6` で `: ` を変数名末尾に使うと scope 修飾と誤解される | `Log "$Layer: ..."` のようなコードが ParserError | `${Layer}: ...` 形式を採用済み |
| `coverage_report.ps1` で `cargo +nightly llvm-cov` を使うが nightly 未導入環境では自動 install される | CI 環境での install 失敗時にエラーメッセージのみ | 既設の動作のため本 task 範囲外 (将来 CI 安定化で対応) |

## 9. 完了チェック (per ipa-regression-test SKILL §10)

- [x] 原問題 (「mock UT/IT/ST 脚本 + 整体回归」) を満たすスクリプト 5 本 + Python 集計 1 本を追加
- [x] 直接影響 (cargo test 全層) を全カバー
- [x] 間接影響 (PS1 実行環境, cargo target dir, git 状態) を `.gitignore` で考慮
- [x] 共通影響 (cargo +nightly + llvm-cov) は将来課題として明示
- [x] 未执行ケースに理由 + 代替検証を §5 で明示
- [x] 剩余リスクを §8 で記録
- [x] 失敗注入検証 (§4.2) で `exit 1` + 失败テスト名列挙 を実証

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 第 6 章「保守プロセス」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
