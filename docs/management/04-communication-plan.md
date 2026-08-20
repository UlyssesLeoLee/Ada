# コミュニケーション計画（Communication Plan）

> **本文件の目的**：[ステークホルダ登録簿 §5](../upstream/02-stakeholder-register.md) を踏まえ、**会議体・報告・通知**の頻度・参加者・媒体を定める。  
> 関連 IPA 工程: 140（会議・報告）+ 132-138（管理プロセス横断）。

> **ドキュメントID**：DOC-MGT-COM-001
> **文書分類**：管理文書
> **バージョン**：v1.0.0
> **制定日**：2026-08-20
> **最終更新日**：2026-08-20
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：[`docs/upstream/02-stakeholder-register.md`](../upstream/02-stakeholder-register.md)
> **下位文書**：[`docs/templates/03-process-management.md` §A.5 会議議事録](../templates/03-process-management.md#a5-会議アジェンダ--議事録テンプレートipa-工程-140)
> **関連文書**：[`docs/architecture/08-workflow-overview.md`](../architecture/08-workflow-overview.md)
> **適用 IPA 標準**：
> - IPA「共通フレーム2018 (SLCP-JCF2018)」
> - PMBOK Guide 第 7 版

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-20 | 初版制定（IPA 工程 140 に対応） | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 会議体一覧
2. 報告
3. 通知
4. エスカレーション
5. ツール
6. 言語
7. 完了基準
8. 用語集
9. 参考文献

---

## 1. 会議体一覧

| 会議 | 頻度 | 時刻 | 参加者 | 目的 | 議事録テンプレ |
|---|---|---|---|---|---|
| 日次 standup | 毎日 | 09:30-09:45 | Dev チーム全員 | 昨日 / 今日 / 障壁 | [§A.5](../templates/03-process-management.md) |
| 週次進捗 | 週 1（金） | 16:00-17:00 | PM + 全ロール | 進捗 / 課題 / 来週 | [§A.5](../templates/03-process-management.md) |
| 週次 PO レビュー | 週 1（月） | 10:00-11:00 | PO + PM + アーキ | 優先度 / スコープ | [§A.5](../templates/03-process-management.md) |
| 月次ステアリング | 月 1 | 第 1 月曜 14:00-16:00 | PO + PM + 経営層 | 月次報告 / 経営判断 | [§A.5](../templates/03-process-management.md) |
| 四半期 QBR | 四半期 | 四半期最終週 | 全ステークホルダ | 振り返り / 次期計画 | [§A.5](../templates/03-process-management.md) |
| Gate レビュー | 各 Gate | TBD | [DOC-MGT-REV-001 §3](../management/02-review-schedule.md) | Gate 判定 | [DOC-TPL-REV §A.X](../templates/01-reviews.md) |
| 臨時インシデント | 必要時 | 即時 | SRE + 関連者 | 障害対応 | [DOC-TPL-OPS §A.7 Postmortem](../templates/05-operations.md#a7-postmortem-テンプレートipa-工程-115) |
| アーキ会議 | 隔週 | 火曜 15:00 | アーキ + テック + リード | 技術決定 | [§A.5](../templates/03-process-management.md) |
| セキュリティレビュー | 月 1 | 第 3 水曜 | SecO + アーキ | 脆弱性 / コンプラ | [DOC-TPL-CHG §A.6](../templates/06-change-management.md#a6-脆弱性対応ログipa-工程-123) |
| 変更諮問会 (CAB) | 必要時 | TBD | PO + PM + アーキ + SRE + SecO | 変更承認 | [DOC-TPL-CHG §A.3](../templates/06-change-management.md#a3-変更承認記録ipa-工程-120) |

## 2. 報告

| 報告 | 頻度 | 作成者 | 提出先 | テンプレ |
|---|---|---|---|---|
| 週次進捗レポート | 週 1 | PM | PO + チーム | [§A.2 進捗レポート](../templates/03-process-management.md#a2-進捗レポートテンプレートipa-工程-133) |
| 月次ステアリング | 月 1 | PM | 経営層 | [§A.2 + ステアリング特化](../templates/03-process-management.md#a2-進捗レポートテンプレートipa-工程-133) |
| 四半期 QBR | 四半期 | PM | 全ステークホルダ | QBR テンプレ（[§A.5](../templates/03-process-management.md#a5-会議アジェンダ--議事録テンプレートipa-工程-140) 流用） |
| SLA レポート | 月 1 | SRE | PO + 経営層 | [DOC-TPL-OPS §A.2 監視](../templates/05-operations.md#a2-監視設定書ipa-工程-110) |
| セキュリティレポート | 月 1 | SecO | PO + 経営層 | [DOC-TPL-QUA §A.2 品質評価](../templates/07-quality.md#a2-品質評価レポートipa-工程-129) |
| 完了報告 | PJ 完了時 | PM | 経営層 + 全ステークホルダ | [DOC-TPL-CLO §A.2 完了報告](../templates/08-closure.md#a2-完了報告書ipa-工程-147) |

## 3. 通知

| 通知 | タイミング | 送信元 | 送信先 | 媒体 |
|---|---|---|---|---|
| インシデント発生 | 即時 | SRE | 全関係者 | PagerDuty + Slack |
| Sev1/Sev2 | 即時 | SRE | PO + PM + 経営層 | 電話 |
| リリース判定 | 24h 前 | PM | 関係者 | Slack + Email |
| 計画停止 | 7 日前 | SRE | 顧客 + 全関係者 | Email + ステータスページ |
| セキュリティ脆弱性 | 即時 | SecO | PO + PM + Dev | Slack + 緊急会議 |
| コンプライアンス変更 | 検知時 | SecO | PO + 経営層 | 緊急会議 |
| 採用情報 | 必要時 | PM | 全員 | Slack |

## 4. エスカレーション

| レベル | 条件 | 連絡先 | 応答時間 |
|---|---|---|---|
| L1 一次対応 | インシデント | on-call SRE | 5 分 |
| L2 二次対応 | 30 分未解決 | SRE Lead + Dev | 30 分 |
| L3 三次対応 | 1 時間未解決 | PM + アーキ | 1 時間 |
| L4 経営判断 | 4 時間未解決 / Sev1 | PO + 経営層 | 即時 |

## 5. ツール

| 用途 | ツール | 備考 |
|---|---|---|
| IM | Slack | #pj-ada, #incident, #hypercare |
| ビデオ会議 | Zoom | 録画可 |
| 文書 | GitHub | docs/ |
| タスク | GitHub Issues / Projects | sprint board |
| コード | GitHub | main + feature branches |
| CI | GitHub Actions | — |
| 監視 | PagerDuty | on-call |
| ナレッジ | Notion (将来) | 検討中 |
| 図面 | draw.io / Mermaid | docs/architecture/ |

## 6. 言語

| 用途 | 言語 |
|---|---|
| 日本語 | 国内会議、ドキュメント |
| 英語 | 国際会議、技術ドキュメント、ログ |
| 中国語 | 中国顧客向け、必要時 |

## 7. 完了基準

- 全会議体の頻度・参加者・目的が定義
- 報告と通知のテンプレ整備
- エスカレーションフロー確立
- ツール選定完了

## 8. 用語集

| 用語 | 説明 |
|---|---|
| Standup | 日次短時間会議 |
| QBR | Quarterly Business Review（四半期レビュー） |
| CAB | Change Advisory Board（変更諮問委員会） |
| エスカレーション | 上位者へ問題を移管 |

## 9. 参考文献

1. PMBOK Guide 第 7 版、Project Management Institute、2021 年
2. IPA「共通フレーム2018 (SLCP-JCF2018)」、2018 年 3 月
3. Ada プロジェクトチーム「[DOC-UP-002 ステークホルダ登録簿](../upstream/02-stakeholder-register.md)」、2026-08-20

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
