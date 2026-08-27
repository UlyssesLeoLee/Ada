//! Integration tests for `ada-telemetry` v0.2.0.
//!
//! Most of the crate is exercised through the in-`#[cfg(test)]`
//! unit tests in `src/lib.rs` because the telemetry pipeline
//! installs a process-global subscriber. End-to-end behaviour
//! (e.g. real OTLP gRPC push to a collector) is covered in
//! `observability/` (Phase 1) and in dedicated integration
//! suites that wire a fake `SpanExporter` into the same code
//! path used by [`ada_telemetry::init`].
//!
//! For now this file is a stub so `cargo test --workspace`
//! picks up the explicit `[[test]]` target that the
//! crate's `Cargo.toml` declares (Cargo 1.85+ drops
//! `tests/*.rs` from `cargo test --workspace` if the
//! `[lib]` block has an explicit `path = ...` and no
//! `[[test]]` is also declared).

#[test]
fn stub_integration_target() {
    // The real assertions live in `src/lib.rs::tests` and in
    // Phase 1 + 2 integration suites. Keep this as a single
    // green light so the target is always present in CI.
    assert!(ada_telemetry::VERSION.starts_with("0."));
    assert_eq!(ada_telemetry::LAYER, "shared");
}
