//! Breakpoint model.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{DebugError, Result};

/// Stable, opaque identifier for a breakpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BreakpointId(pub Uuid);

impl BreakpointId {
    /// Create a fresh random id.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for BreakpointId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for BreakpointId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A breakpoint location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Location {
    /// Source file + line. `line` is 1-based.
    Line {
        /// Path or module identifier (e.g. "ada-m03-data-flow-engine").
        file: String,
        /// 1-based line number.
        line: u32,
    },
    /// Function entry (e.g. "ada_m03_data_flow_engine::Engine::execute").
    Function(String),
}

impl Location {
    /// Validate. Returns `Err(DebugError::InvalidLocation)` if
    /// `line` is 0 or the function name is empty.
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Line { line, .. } if *line == 0 => Err(DebugError::InvalidLocation {
                reason: "line must be >= 1".into(),
            }),
            Self::Function(name) if name.is_empty() => Err(DebugError::InvalidLocation {
                reason: "function name is empty".into(),
            }),
            _ => Ok(()),
        }
    }
}

/// The three canonical breakpoint kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BreakpointKind {
    /// Unconditional line / function breakpoint.
    Line,
    /// Conditional breakpoint with an expression string. The
    /// skeleton does not evaluate the expression; the field is
    /// stored verbatim.
    Conditional,
    /// Function entry breakpoint.
    Entry,
}

impl std::fmt::Display for BreakpointKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Line => "line",
            Self::Conditional => "conditional",
            Self::Entry => "entry",
        };
        f.write_str(s)
    }
}

/// The three breakpoint states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BreakpointState {
    /// Active; will be checked on every hit.
    Active,
    /// Disabled; not checked.
    Disabled,
    /// Hit at least once; the skeleton stays in this state.
    Hit,
}

impl std::fmt::Display for BreakpointState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Active => "active",
            Self::Disabled => "disabled",
            Self::Hit => "hit",
        };
        f.write_str(s)
    }
}

/// A breakpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Breakpoint {
    /// Stable id.
    pub id: BreakpointId,
    /// Where to break.
    pub location: Location,
    /// What kind.
    pub kind: BreakpointKind,
    /// Current state.
    pub state: BreakpointState,
}

impl Breakpoint {
    /// Create a new active breakpoint at `location` of the given
    /// `kind`. Validates `location`.
    pub fn new(location: Location, kind: BreakpointKind) -> Result<Self> {
        location.validate()?;
        Ok(Self {
            id: BreakpointId::new(),
            location,
            kind,
            state: BreakpointState::Active,
        })
    }

    /// Mark the breakpoint as `Hit` (the debug adapter reported
    /// a hit). No-op if the breakpoint is already `Hit`.
    pub fn mark_hit(&mut self) {
        if self.state != BreakpointState::Hit {
            self.state = BreakpointState::Hit;
        }
    }

    /// Enable / disable the breakpoint.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.state = if enabled {
            BreakpointState::Active
        } else {
            BreakpointState::Disabled
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_location_must_be_1_based() {
        let loc = Location::Line {
            file: "x.rs".into(),
            line: 0,
        };
        let err = loc.validate().unwrap_err();
        assert!(matches!(err, DebugError::InvalidLocation { .. }));
    }

    #[test]
    fn function_location_must_be_nonempty() {
        let loc = Location::Function(String::new());
        let err = loc.validate().unwrap_err();
        assert!(matches!(err, DebugError::InvalidLocation { .. }));
    }

    #[test]
    fn line_location_validates() {
        Location::Line {
            file: "x.rs".into(),
            line: 1,
        }
        .validate()
        .expect("valid");
    }

    #[test]
    fn function_location_validates() {
        Location::Function("foo::bar".into())
            .validate()
            .expect("valid");
    }

    #[test]
    fn breakpoint_kind_display() {
        assert_eq!(BreakpointKind::Line.to_string(), "line");
        assert_eq!(BreakpointKind::Conditional.to_string(), "conditional");
        assert_eq!(BreakpointKind::Entry.to_string(), "entry");
    }

    #[test]
    fn breakpoint_state_display() {
        assert_eq!(BreakpointState::Active.to_string(), "active");
        assert_eq!(BreakpointState::Disabled.to_string(), "disabled");
        assert_eq!(BreakpointState::Hit.to_string(), "hit");
    }

    #[test]
    fn breakpoint_new_starts_active() {
        let bp =
            Breakpoint::new(Location::Function("foo".into()), BreakpointKind::Entry).expect("ok");
        assert_eq!(bp.state, BreakpointState::Active);
    }

    #[test]
    fn breakpoint_mark_hit_is_idempotent() {
        let mut bp =
            Breakpoint::new(Location::Function("foo".into()), BreakpointKind::Entry).expect("ok");
        bp.mark_hit();
        bp.mark_hit();
        assert_eq!(bp.state, BreakpointState::Hit);
    }

    #[test]
    fn breakpoint_set_enabled_toggles() {
        let mut bp =
            Breakpoint::new(Location::Function("foo".into()), BreakpointKind::Line).expect("ok");
        bp.set_enabled(false);
        assert_eq!(bp.state, BreakpointState::Disabled);
        bp.set_enabled(true);
        assert_eq!(bp.state, BreakpointState::Active);
    }

    #[test]
    fn breakpoint_new_with_invalid_location_errors() {
        let err =
            Breakpoint::new(Location::Function(String::new()), BreakpointKind::Entry).unwrap_err();
        assert!(matches!(err, DebugError::InvalidLocation { .. }));
    }
}
