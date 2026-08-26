//! Integration tests for the v0.1.0 normalizer.
//!
//! The v0.1.0 surface is in-process, so the "integration"
//! tests exercise the public surface the way a real
//! ingestion loop would: build a `NormalizationPipeline`,
//! feed it `RawRecord`s acquired from the upstream
//! `ada-m01-acquisition` crate, and assert the shape of
//! the `NormalizedRecord` stream.

use ada_m01_acquisition::{Connector, FileConnector, RawRecord, SourceDescriptor, SourceKind};
use ada_m02_normalizer::{NormalizationPipeline, NormalizationRule, NormalizerError, RuleKind};
use std::io::Write;

fn tmp_file(contents: &str, suffix: &str) -> String {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "ada-m02-it-{}-{suffix}.ndjson",
        uuid::Uuid::new_v4()
    ));
    {
        let mut f = std::fs::File::create(&path).expect("create");
        f.write_all(contents.as_bytes()).unwrap();
    }
    path.to_string_lossy().to_string()
}

async fn build_acquired(endpoint: String) -> Vec<RawRecord> {
    let c = FileConnector::new(SourceDescriptor::new(
        "ingest-m02",
        SourceKind::File,
        endpoint,
    ));
    c.fetch().await.expect("fetch")
}

#[tokio::test]
async fn end_to_end_acquire_then_normalize() {
    // 1. Acquire: write a 2-line NDJSON file and pull it
    //    through the file connector.
    let path = tmp_file(
        "{\"email\":\"  Foo@Example.COM  \",\"age\":42}\n{\"email\":\"  BAR@Example.com  \",\"age\":7}\n",
        "e2e",
    );
    let records = build_acquired(path.clone()).await;
    assert_eq!(records.len(), 2);

    // 2. Normalize: trim + lowercase + regex.
    let pipeline = NormalizationPipeline::builder(vec![
        NormalizationRule::new("r1", "email", RuleKind::Trim),
        NormalizationRule::new("r2", "email", RuleKind::Lowercase),
        NormalizationRule::new(
            "r3",
            "email",
            RuleKind::Regex {
                pattern: r"@example\.com$".into(),
                replacement: "@example.org".into(),
            },
        ),
    ])
    .expect("build");

    let mut normalized = Vec::new();
    for r in records {
        let n = pipeline
            .apply(&r.source_id, r.seq, r.payload)
            .expect("normalize");
        normalized.push(n);
    }

    assert_eq!(normalized.len(), 2);
    assert_eq!(
        normalized[0].payload["email"],
        serde_json::json!("foo@example.org")
    );
    assert_eq!(normalized[0].payload["age"], serde_json::json!(42));
    assert_eq!(
        normalized[1].payload["email"],
        serde_json::json!("bar@example.org")
    );
    assert_eq!(normalized[1].seq, 1);

    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn pipeline_rejects_invalid_regex_at_build_time() {
    let err = NormalizationPipeline::builder(vec![NormalizationRule::new(
        "r",
        "x",
        RuleKind::Regex {
            pattern: "[unclosed".into(),
            replacement: String::new(),
        },
    )])
    .expect_err("bad regex");
    assert!(matches!(err, NormalizerError::InvalidRegex(_)));
}

#[tokio::test]
async fn date_and_coalesce_compose_in_one_pipeline() {
    let pipeline = NormalizationPipeline::builder(vec![
        NormalizationRule::new(
            "d",
            "ts",
            RuleKind::Date {
                input_format: "%Y-%m-%d".into(),
                output_format: "%Y/%m/%d".into(),
            },
        ),
        NormalizationRule::new(
            "c",
            "display",
            RuleKind::Coalesce {
                candidates: vec!["primary".into(), "secondary".into()],
            },
        ),
    ])
    .expect("build");

    let out = pipeline
        .apply(
            "s",
            0,
            serde_json::json!({
                "ts": "2026-01-15",
                "primary": null,
                "secondary": "fallback"
            }),
        )
        .expect("apply");
    assert_eq!(out.payload["ts"], serde_json::json!("2026/01/15"));
    assert_eq!(out.payload["display"], serde_json::json!("fallback"));
}

#[tokio::test]
async fn missing_field_surfaces_unknown_field_error() {
    let pipeline = NormalizationPipeline::builder(vec![NormalizationRule::new(
        "r",
        "missing",
        RuleKind::Trim,
    )])
    .expect("build");
    let err = pipeline
        .apply("s", 0, serde_json::json!({"present": "x"}))
        .expect_err("missing");
    assert!(matches!(err, NormalizerError::UnknownField(_)));
}
