//! Trace events and the in-process [`TraceRecorder`].

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{DebugError, Result};

/// The three trace event kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TraceKind {
    /// A tracing span (start / end). The skeleton records the
    /// event as a single record with the span name.
    Span,
    /// A log line at a given level.
    Log,
    /// A point-in-time metric (counter, gauge, ...). The
    /// metric value is carried as a string in `payload`.
    Metric,
}

impl std::fmt::Display for TraceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Span => "span",
            Self::Log => "log",
            Self::Metric => "metric",
        };
        f.write_str(s)
    }
}

/// One trace event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceEvent {
    /// Wall-clock timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Event kind.
    pub kind: TraceKind,
    /// Span / log target / metric name.
    pub target: String,
    /// Free-form payload (log line, metric value as string,
    /// span attributes, ...).
    pub payload: String,
}

impl TraceEvent {
    /// Create an event with the current wall-clock time.
    pub fn now(kind: TraceKind, target: impl Into<String>, payload: impl Into<String>) -> Self {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
        Self {
            timestamp_ms,
            kind,
            target: target.into(),
            payload: payload.into(),
        }
    }
}

/// Bounded in-process recorder. When the buffer is full, the
/// oldest events are evicted.
#[derive(Debug)]
pub struct TraceRecorder {
    capacity: usize,
    events: Mutex<VecDeque<TraceEvent>>,
    overflowed: Mutex<bool>,
}

impl TraceRecorder {
    /// Create a recorder with the given maximum event count.
    /// Capacity must be > 0.
    pub fn new(capacity: usize) -> Result<Self> {
        if capacity == 0 {
            return Err(DebugError::InvalidLocation {
                reason: "trace capacity must be > 0".into(),
            });
        }
        Ok(Self {
            capacity,
            events: Mutex::new(VecDeque::with_capacity(capacity)),
            overflowed: Mutex::new(false),
        })
    }

    /// Record an event. If the buffer is full, the oldest event
    /// is dropped and the recorder is marked as having
    /// overflowed.
    pub fn record(&self, event: TraceEvent) {
        let mut q = self.events.lock();
        if q.len() == self.capacity {
            q.pop_front();
            *self.overflowed.lock() = true;
        }
        q.push_back(event);
    }

    /// Drain the recorder's buffer. Resets the overflow flag.
    pub fn drain(&self) -> Vec<TraceEvent> {
        let mut q = self.events.lock();
        *self.overflowed.lock() = false;
        std::mem::take(&mut *q).into()
    }

    /// `true` if at least one event was dropped since the last
    /// `drain`.
    #[must_use]
    pub fn overflowed(&self) -> bool {
        *self.overflowed.lock()
    }

    /// Number of events currently buffered.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.lock().len()
    }

    /// `true` if no events are buffered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Configured capacity.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recorder_rejects_zero_capacity() {
        let err = TraceRecorder::new(0).unwrap_err();
        assert!(matches!(err, DebugError::InvalidLocation { .. }));
    }

    #[test]
    fn record_and_drain_round_trip() {
        let r = TraceRecorder::new(8).expect("recorder");
        r.record(TraceEvent::now(TraceKind::Log, "test", "hello"));
        r.record(TraceEvent::now(TraceKind::Metric, "m1", "42"));
        assert_eq!(r.len(), 2);
        let drained = r.drain();
        assert_eq!(drained.len(), 2);
        assert!(r.is_empty());
        assert!(!r.overflowed());
    }

    #[test]
    fn overflow_evicts_oldest_and_sets_flag() {
        let r = TraceRecorder::new(2).expect("recorder");
        r.record(TraceEvent::now(TraceKind::Log, "a", "1"));
        r.record(TraceEvent::now(TraceKind::Log, "b", "2"));
        r.record(TraceEvent::now(TraceKind::Log, "c", "3"));
        assert_eq!(r.len(), 2);
        assert!(r.overflowed());
        let drained = r.drain();
        assert_eq!(drained[0].payload, "2");
        assert_eq!(drained[1].payload, "3");
        assert!(!r.overflowed());
    }

    #[test]
    fn trace_kind_display() {
        assert_eq!(TraceKind::Span.to_string(), "span");
        assert_eq!(TraceKind::Log.to_string(), "log");
        assert_eq!(TraceKind::Metric.to_string(), "metric");
    }

    #[test]
    fn trace_event_now_has_nonzero_timestamp() {
        let e = TraceEvent::now(TraceKind::Span, "test", "x");
        assert!(e.timestamp_ms > 0);
        assert_eq!(e.kind, TraceKind::Span);
        assert_eq!(e.target, "test");
    }
}
