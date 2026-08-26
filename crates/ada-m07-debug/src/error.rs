//! Error surface for the M-07 debug crate.

use thiserror::Error;

use crate::breakpoint::BreakpointId;

/// Failure modes surfaced by the debug facilities.
#[derive(Debug, Error)]
pub enum DebugError {
    /// The breakpoint id is not registered.
    #[error("breakpoint not found: {0}")]
    BreakpointNotFound(BreakpointId),

    /// An inspector call was issued when no inspector was active.
    #[error("inspector unavailable")]
    InspectorUnavailable,

    /// The trace recorder's bounded buffer overflowed; the
    /// oldest entries were evicted.
    #[error("trace buffer overflow (capacity {capacity})")]
    TraceOverflow {
        /// Configured maximum.
        capacity: usize,
    },

    /// The location was malformed (e.g. line 0, empty function).
    #[error("invalid location: {reason}")]
    InvalidLocation {
        /// Human-readable reason.
        reason: String,
    },

    /// The backing store (file, debugger, ...) failed.
    #[error("backend error: {0}")]
    BackendError(String),
}

/// `Result` alias for fallible debug operations.
pub type Result<T> = core::result::Result<T, DebugError>;

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn breakpoint_not_found_display() {
        let e = DebugError::BreakpointNotFound(BreakpointId(Uuid::new_v4()));
        let s = e.to_string();
        assert!(s.starts_with("breakpoint not found: "), "got: {s}");
    }

    #[test]
    fn inspector_unavailable_display() {
        assert_eq!(
            DebugError::InspectorUnavailable.to_string(),
            "inspector unavailable"
        );
    }

    #[test]
    fn trace_overflow_display() {
        let e = DebugError::TraceOverflow { capacity: 1024 };
        assert_eq!(e.to_string(), "trace buffer overflow (capacity 1024)");
    }

    #[test]
    fn invalid_location_display() {
        let e = DebugError::InvalidLocation {
            reason: "line 0".into(),
        };
        assert_eq!(e.to_string(), "invalid location: line 0");
    }

    #[test]
    fn backend_error_display() {
        let e = DebugError::BackendError("ptrace: not permitted".into());
        assert_eq!(e.to_string(), "backend error: ptrace: not permitted");
    }
}
