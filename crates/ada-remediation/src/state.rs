//! State machine for the remediation engine.
//!
//! The state machine is intentionally a tiny pure data type —
//! the executor is the one that *drives* the transitions, but
//! the *legal* transitions are defined here. This split is
//! what makes the engine unit-testable in isolation: a test
//! can hold the machine in any state and assert that
//! `transition(Idle, Executing)` succeeds while
//! `transition(Cooldown, Executing)` does not.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EngineState {
    /// Engine is idle, waiting for an alert to evaluate.
    Idle,
    /// An alert is being matched against the runbook table.
    Evaluating,
    /// Steps are currently being executed.
    Executing,
    /// Last action succeeded and is now in cooldown.
    Cooldown,
    /// Last action failed (no more retries).
    Failed,
    /// Mid-retry backoff between attempts.
    Retrying,
}

impl EngineState {
    /// Returns `true` if a transition from `self` to `next` is
    /// legal per the state diagram in `crate` root docs.
    #[must_use]
    pub fn can_transition_to(self, next: Self) -> bool {
        use EngineState::{Cooldown, Evaluating, Executing, Failed, Idle, Retrying};
        #[allow(clippy::unnested_or_patterns)]
        let legal = matches!(
            (self, next),
            (Idle, Evaluating)
                | (Evaluating, Executing)
                | (Evaluating, Idle)
                | (Executing, Cooldown)
                | (Executing, Failed)
                | (Executing, Retrying)
                | (Retrying, Executing)
                | (Retrying, Failed)
                | (Cooldown, Idle)
                | (Failed, Idle)
        );
        legal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use EngineState::{Cooldown, Evaluating, Executing, Failed, Idle, Retrying};

    #[test]
    fn happy_path_transitions_are_legal() {
        assert!(Idle.can_transition_to(Evaluating));
        assert!(Evaluating.can_transition_to(Executing));
        assert!(Executing.can_transition_to(Cooldown));
        assert!(Cooldown.can_transition_to(Idle));
    }

    #[test]
    fn retry_loop_is_legal() {
        assert!(Executing.can_transition_to(Retrying));
        assert!(Retrying.can_transition_to(Executing));
        assert!(Retrying.can_transition_to(Failed));
    }

    #[test]
    fn no_match_short_circuits_back_to_idle() {
        assert!(Evaluating.can_transition_to(Idle));
    }

    #[test]
    fn cannot_jump_from_cooldown_to_executing() {
        assert!(!Cooldown.can_transition_to(Executing));
        assert!(!Failed.can_transition_to(Executing));
    }

    #[test]
    fn cannot_skip_evaluating() {
        assert!(!Idle.can_transition_to(Executing));
        assert!(!Idle.can_transition_to(Cooldown));
    }
}
