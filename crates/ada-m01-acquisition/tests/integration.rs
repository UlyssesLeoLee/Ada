//! Integration tests for the v0.1.0 acquisition layer.
//!
//! The v0.1.0 surface is in-process, so the "integration"
//! tests exercise the public surface the way a real
//! ingestion loop would: build a connector, call `fetch`
//! from a `tokio` task, and assert the shape of the returned
//! `Vec<RawRecord>`.

use std::io::Write;

use ada_m01_acquisition::{
    AcquisitionError, Connector, FileConnector, HttpConnector, SourceDescriptor, SourceKind,
    StdInConnector,
};

fn desc(id: &str, kind: SourceKind, endpoint: &str) -> SourceDescriptor {
    SourceDescriptor::new(id, kind, endpoint)
}

#[tokio::test]
async fn http_connector_round_trip() {
    let c = HttpConnector::new(desc(
        "ingest-http-1",
        SourceKind::Http,
        "https://api.example.com/v1/items",
    ));
    let batch = c.fetch().await.expect("ok");
    assert_eq!(batch.len(), 1);
    let r = &batch[0];
    assert_eq!(r.source_id, "ingest-http-1");
    assert_eq!(
        r.payload["endpoint"],
        serde_json::json!("https://api.example.com/v1/items")
    );
    assert!(r.payload["items"].is_array());
}

#[tokio::test]
async fn http_connector_maps_status_codes() {
    // 401 -> AuthenticationFailed
    let c = HttpConnector::new(desc("a", SourceKind::Http, "https://x"));
    c.set_status_override(Some(401));
    let err = c.fetch().await.expect_err("401");
    assert!(matches!(err, AcquisitionError::AuthenticationFailed(_)));

    // 429 -> RateLimited
    let c = HttpConnector::new(desc("a", SourceKind::Http, "https://x"));
    c.set_status_override(Some(429));
    let err = c.fetch().await.expect_err("429");
    assert!(matches!(err, AcquisitionError::RateLimited(_)));

    // 503 -> SourceUnavailable
    let c = HttpConnector::new(desc("a", SourceKind::Http, "https://x"));
    c.set_status_override(Some(503));
    let err = c.fetch().await.expect_err("503");
    assert!(matches!(err, AcquisitionError::SourceUnavailable(_)));
}

#[tokio::test]
async fn file_connector_reads_ndjson_from_disk() {
    // Build a temp file in std::env::temp_dir() so we don't
    // pull in `tempfile` as a new dev-dep.
    let mut path = std::env::temp_dir();
    path.push(format!("ada-m01-it-{}.ndjson", uuid::Uuid::new_v4()));
    {
        let mut f = std::fs::File::create(&path).expect("create");
        writeln!(f, "{{\"id\": 1, \"name\": \"alpha\"}}").unwrap();
        writeln!(f).unwrap();
        writeln!(f, "{{\"id\": 2, \"name\": \"beta\"}}").unwrap();
        writeln!(f, "{{\"id\": 3, \"name\": \"gamma\"}}").unwrap();
    }
    let endpoint = path.to_string_lossy().to_string();

    let c = FileConnector::new(desc("ingest-file-1", SourceKind::File, &endpoint));
    let batch = c.fetch().await.expect("ok");
    assert_eq!(batch.len(), 3);
    assert_eq!(batch[0].source_id, "ingest-file-1");
    assert_eq!(batch[0].seq, 0);
    assert_eq!(batch[0].payload["id"], serde_json::json!(1));
    assert_eq!(batch[1].seq, 1);
    assert_eq!(batch[2].payload["name"], serde_json::json!("gamma"));

    // Second fetch continues the sequence counter.
    let batch2 = c.fetch().await.expect("ok");
    assert_eq!(batch2.len(), 3);
    assert_eq!(batch2[0].seq, 3);

    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn stdin_connector_reads_ndjson_from_reader() {
    // Build a static NDJSON blob and feed it through the
    // stdin-style connector.
    let payload = b"{\"k\":\"v1\"}\n{\"k\":\"v2\"}\n\n{\"k\":\"v3\"}\n";
    let c = StdInConnector::from_slice(desc("ingest-stdin-1", SourceKind::Stdin, "-"), payload);
    let batch = c.fetch().await.expect("ok");
    // 4 non-empty lines (empty lines are skipped).
    assert_eq!(batch.len(), 3);
    assert_eq!(batch[0].payload["k"], serde_json::json!("v1"));
    assert_eq!(batch[1].payload["k"], serde_json::json!("v2"));
    assert_eq!(batch[2].payload["k"], serde_json::json!("v3"));
    assert_eq!(batch[2].seq, 2);
}
