//! Stack inspector.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::error::Result;

/// One frame in a call stack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InspectFrame {
    /// Function name (e.g. "ada_m03_data_flow_engine::Engine::execute").
    pub name: String,
    /// Argument names and JSON values. Order in the `HashMap` is
    /// undefined; the skeleton does not preserve argument order.
    pub args: HashMap<String, Value>,
    /// Local variable names and JSON values.
    pub locals: HashMap<String, Value>,
    /// 1-based line number, if known.
    pub line: Option<u32>,
}

impl InspectFrame {
    /// Create a frame with empty args/locals.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            args: HashMap::new(),
            locals: HashMap::new(),
            line: None,
        }
    }

    /// Builder-style: add an argument.
    #[must_use]
    pub fn with_arg(mut self, name: impl Into<String>, value: Value) -> Self {
        self.args.insert(name.into(), value);
        self
    }

    /// Builder-style: add a local.
    #[must_use]
    pub fn with_local(mut self, name: impl Into<String>, value: Value) -> Self {
        self.locals.insert(name.into(), value);
        self
    }

    /// Builder-style: set the line number.
    #[must_use]
    pub fn with_line(mut self, line: u32) -> Self {
        self.line = Some(line);
        self
    }
}

/// Walks a stack of frames. The skeleton is a simple `Vec`
/// wrapper that the user pushes frames into and pops frames
/// off; production code would attach to a real process.
#[derive(Debug, Default, Clone)]
pub struct Inspector {
    frames: Vec<InspectFrame>,
}

impl Inspector {
    /// Create an empty inspector.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a frame. The newest frame is the "current" one.
    pub fn push(&mut self, frame: InspectFrame) {
        self.frames.push(frame);
    }

    /// Pop the most recent frame and return it.
    pub fn pop(&mut self) -> Option<InspectFrame> {
        self.frames.pop()
    }

    /// Peek the most recent frame without removing it.
    #[must_use]
    pub fn current(&self) -> Option<&InspectFrame> {
        self.frames.last()
    }

    /// Number of frames currently on the stack.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.frames.len()
    }

    /// Snapshot the full stack, oldest first.
    #[must_use]
    pub fn stack(&self) -> Vec<InspectFrame> {
        self.frames.clone()
    }

    /// Walk a fictional `run(input)` and produce a 1-frame
    /// snapshot. Convenience for the skeleton's unit tests.
    pub fn inspect_one(
        &mut self,
        function: impl Into<String>,
        input: Value,
    ) -> Result<&InspectFrame> {
        let frame = InspectFrame::new(function).with_arg("input", input);
        self.push(frame);
        self.current()
            .ok_or(crate::DebugError::InspectorUnavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn empty_inspector_has_no_current() {
        let i = Inspector::new();
        assert!(i.current().is_none());
        assert_eq!(i.depth(), 0);
    }

    #[test]
    fn push_and_pop_round_trip() {
        let mut i = Inspector::new();
        i.push(InspectFrame::new("a"));
        i.push(InspectFrame::new("b"));
        assert_eq!(i.depth(), 2);
        let popped = i.pop().expect("frame");
        assert_eq!(popped.name, "b");
        assert_eq!(i.depth(), 1);
    }

    #[test]
    fn current_returns_top() {
        let mut i = Inspector::new();
        i.push(InspectFrame::new("a"));
        i.push(InspectFrame::new("b"));
        let cur = i.current().expect("current");
        assert_eq!(cur.name, "b");
    }

    #[test]
    fn stack_returns_oldest_first() {
        let mut i = Inspector::new();
        i.push(InspectFrame::new("a"));
        i.push(InspectFrame::new("b"));
        i.push(InspectFrame::new("c"));
        let s = i.stack();
        assert_eq!(
            s.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
            vec!["a", "b", "c"]
        );
    }

    #[test]
    fn frame_builder_fills_args_locals_line() {
        let f = InspectFrame::new("foo")
            .with_arg("x", json!(1))
            .with_local("y", json!("hello"))
            .with_line(42);
        assert_eq!(f.args.get("x"), Some(&json!(1)));
        assert_eq!(f.locals.get("y"), Some(&json!("hello")));
        assert_eq!(f.line, Some(42));
    }

    #[test]
    fn inspect_one_creates_frame() {
        let mut i = Inspector::new();
        let f = i
            .inspect_one("run", json!({"k": "v"}))
            .expect("frame")
            .clone();
        assert_eq!(f.name, "run");
        assert_eq!(f.args.get("input"), Some(&json!({"k": "v"})));
        assert_eq!(i.depth(), 1);
    }
}
