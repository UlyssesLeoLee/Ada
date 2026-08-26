//! Integration tests for the v0.1.0 exporter.

use std::collections::HashMap;

use ada_m09_exporter::{
    Exporter, ExporterError, InMemoryExporter, Metric, MetricKind, MetricRegistry, NoopExporter,
    OtlpExporter,
};

fn counter(name: &str, value: f64) -> Metric {
    Metric::now(name, MetricKind::Counter, value, HashMap::new())
}

fn gauge(name: &str, value: f64) -> Metric {
    Metric::now(name, MetricKind::Gauge, value, HashMap::new())
}

#[test]
fn record_snapshot_clear_lifecycle() {
    let r = MetricRegistry::new();
    r.record(counter("ada.events", 1.0));
    r.record(gauge("ada.queue_depth", 7.0));
    assert_eq!(r.len(), 2);
    let snap = r.snapshot();
    assert_eq!(snap.len(), 2);
    r.clear();
    assert!(r.is_empty());
}

#[test]
fn in_memory_exporter_accumulates_across_calls() {
    let exp = InMemoryExporter::new();
    exp.export(&[counter("a", 1.0)]).expect("export 1");
    exp.export(&[counter("b", 2.0), counter("c", 3.0)])
        .expect("export 2");
    let acc = exp.accumulated();
    let names: Vec<&str> = acc.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, vec!["a", "b", "c"]);
}

#[test]
fn noop_exporter_swallows_anything() {
    let exp = NoopExporter;
    exp.export(&[counter("a", 1.0)]).expect("ok");
    exp.export(&[]).expect("ok on empty");
    // Repeated calls are idempotent; nothing to assert on
    // state because Noop has no state.
    assert_eq!(exp.name(), "noop");
    assert_eq!(OtlpExporter::endpoint_kind(&exp), "noop");
}

#[test]
fn exporter_rejects_invalid_metric_in_snapshot() {
    let exp = InMemoryExporter::new();
    let bad = Metric::now("", MetricKind::Counter, 1.0, HashMap::new());
    let err = exp.export(&[bad]).expect_err("invalid");
    assert!(matches!(err, ExporterError::InvalidMetric(_)));
    assert!(exp.is_empty(), "failed export must not write");
}

#[test]
fn registry_drops_invalid_metric_and_keeps_valid() {
    let r = MetricRegistry::new();
    r.record(counter("good", 1.0));
    r.record(Metric::now("", MetricKind::Counter, 1.0, HashMap::new()));
    r.record(Metric::now(
        "nan",
        MetricKind::Counter,
        f64::NAN,
        HashMap::new(),
    ));
    assert_eq!(r.len(), 1);
    assert_eq!(r.snapshot()[0].name, "good");
}

#[test]
fn exporter_works_via_dyn_dispatch() {
    // Prove the trait surface is enough to plug exporters via
    // `Box<dyn Exporter>` — the real production wiring will
    // dispatch through a similar shape.
    let exporters: Vec<Box<dyn Exporter>> =
        vec![Box::new(NoopExporter), Box::new(InMemoryExporter::new())];
    for exp in exporters {
        exp.export(&[counter("a", 1.0)]).expect("ok");
    }
}
