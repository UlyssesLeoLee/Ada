//! End-to-end integration tests for the M-07 debug crate.

use ada_m07_debug::{
    Breakpoint, BreakpointKind, BreakpointState, InspectFrame, Inspector, Location, TraceEvent,
    TraceKind, TraceRecorder,
};
use serde_json::json;

#[test]
fn breakpoint_lifecycle() {
    let mut bp = Breakpoint::new(
        Location::Line {
            file: "ada-m03.rs".into(),
            line: 42,
        },
        BreakpointKind::Line,
    )
    .expect("bp");
    assert_eq!(bp.state, BreakpointState::Active);
    bp.mark_hit();
    assert_eq!(bp.state, BreakpointState::Hit);
    bp.set_enabled(false);
    assert_eq!(bp.state, BreakpointState::Disabled);
}

#[test]
fn inspector_walks_call_stack() {
    let mut i = Inspector::new();
    i.push(InspectFrame::new("main").with_local("argc", json!(1)));
    i.push(InspectFrame::new("process").with_arg("input", json!({"k": "v"})));
    i.push(InspectFrame::new("worker").with_line(99));
    assert_eq!(i.depth(), 3);
    let cur = i.current().expect("top");
    assert_eq!(cur.name, "worker");
    assert_eq!(cur.line, Some(99));
    let stack = i.stack();
    assert_eq!(stack.len(), 3);
    assert_eq!(stack[0].name, "main");
    assert_eq!(stack[1].args.get("input"), Some(&json!({"k": "v"})));
}

#[test]
fn trace_recorder_overflow_signals_correctly() {
    let r = TraceRecorder::new(2).expect("recorder");
    r.record(TraceEvent::now(TraceKind::Log, "a", "1"));
    r.record(TraceEvent::now(TraceKind::Log, "b", "2"));
    assert!(!r.overflowed());
    r.record(TraceEvent::now(TraceKind::Log, "c", "3"));
    assert!(r.overflowed());
    let drained = r.drain();
    assert_eq!(drained.len(), 2);
    assert_eq!(drained.first().expect("first").payload, "2");
}

#[test]
fn trace_event_kinds_serialize_correctly() {
    let e = TraceEvent::now(TraceKind::Span, "ada-m03", "execute");
    assert_eq!(e.kind, TraceKind::Span);
    let v: serde_json::Value = serde_json::to_value(&e).expect("json");
    assert_eq!(v["kind"], serde_json::json!("Span"));
    assert_eq!(v["target"], serde_json::json!("ada-m03"));
}
