# ada-m02-normalizer

M-02: データ正規化 (Data normalization).
Convert raw messages to standard NJSON schema.

## v0.1.0 scope (B5 batch)

This crate is the **minimum skeleton** for the
rule-driven normalization layer that sits between the
acquisition adapters (`ada-m01-acquisition`) and the
data-flow engine (`ada-m03-data-flow-engine`).

The production deployment (full NJSON schema validation,
type coercion, copy-on-write apply semantics, see
`DOC-MOD-002` §3.3-§3.5) is scheduled for B5+ once G4
(実装着手判定) is approved.

### What v0.1.0 provides

- `RuleKind` — `Trim / Lowercase / Regex / Date /
  Coalesce`
- `NormalizationRule` — id, field_path, kind
- `NormalizationPipeline` — ordered `Vec<Rule>`, eager
  `Regex` validation at build time, fail-fast on the
  first rule error
- `NormalizedRecord` — `source_id + seq + payload`
- 5-variant `NormalizerError` (UnknownField,
  RuleExecutionFailed, TypeMismatch, InvalidRegex,
  BackendError)
- 9 unit tests + 4 integration tests

### What v0.1.0 explicitly does **not** do

- Persist normalized records to a topic / table
- Support nested path wildcards (`user.*.email`); only
  top-level + one-segment nested fields are supported
- Snapshot / copy-on-write semantics; a failed apply
  leaves the record partially mutated (the production
  build will use an arena or document the partial-state
  contract)
- Type coercion (string → number, etc.)

## 関連 IPA フェーズ

22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)

## 設計書

`docs/modules/M-02-normalizer.md` (DOC-MOD-002)
