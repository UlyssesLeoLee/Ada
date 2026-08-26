//! [`ControlStep`], [`StepKind`], [`BranchStep`],
//! [`LoopStep`], [`SwitchStep`], [`ParallelStep`].
//!
//! A [`ControlStep`] is the unit of execution the
//! [`ControlFlowExecutor`](crate::executor::ControlFlowExecutor)
//! consumes. The v0.1.0 surface supports five step kinds:
//!
//! - [`StepKind::Action`] — a single named step; the
//!   executor increments the trace and moves to `next_step`.
//! - [`StepKind::Branch`] — if/else. The executor
//!   evaluates `condition`; on `true` it follows
//!   `then_step`, on `false` `else_step`.
//! - [`StepKind::Loop`] — for / while. The executor
//!   evaluates `condition` and either re-enters `body` or
//!   follows `next_step`.
//! - [`StepKind::Switch`] — case dispatch. The executor
//!   finds the first `case` whose literal equals the
//!   value of `field` and follows its step; if none
//!   match, `default_step` is followed.
//! - [`StepKind::Parallel`] — fan-out. The executor runs
//!   `branches` in any order (sequential in v0.1.0;
//!   parallel in B5+) and joins at `next_step`.
//!
//! The skeleton stores `body` as `Vec<String>` (a list of
//! step ids) so the executor can walk it without
//! re-allocating.
//!
//! See [`DOC-MOD-005`](../docs/modules/M-05-control-flow.md)
//! §3.2 for the canonical step schema.

use serde::{Deserialize, Serialize};

use crate::condition::Condition;

/// The five step kinds the v0.1.0 executor understands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StepKind {
    /// A leaf action; the executor just records the trace
    /// entry and moves on. The `body` is the (optional)
    /// sub-step id (rarely used; v0.1.0 mostly treats
    /// actions as a no-op).
    Action,
    /// `if (condition) { then } else { else }`.
    Branch(BranchStep),
    /// `while (condition) { body }`; the executor re-enters
    /// `body` until the condition is `false`.
    Loop(LoopStep),
    /// `switch (field) { case v1 -> s1; case v2 -> s2; default -> s3 }`.
    Switch(SwitchStep),
    /// Fan-out: run `branches` (a list of step ids) in
    /// any order; production will run them in parallel,
    /// v0.1.0 runs them sequentially.
    Parallel(ParallelStep),
}

/// `if/else` branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BranchStep {
    /// Branch condition. `then_step` is followed when the
    /// condition is `true`.
    pub condition: Condition,
    /// Step id to follow when the condition is `true`.
    pub then_step: String,
    /// Step id to follow when the condition is `false`.
    /// May be empty (terminal branch).
    #[serde(default)]
    pub else_step: String,
}

/// `while` loop. The skeleton keeps the body as a list of
/// step ids that are executed in order on every iteration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoopStep {
    /// Loop condition. The body is re-entered while the
    /// condition evaluates to `true`.
    pub condition: Condition,
    /// Step ids that form the loop body, executed in
    /// order. The executor returns to the loop head after
    /// the last entry.
    pub body: Vec<String>,
    /// Step id to follow when the loop exits. May be
    /// empty.
    #[serde(default)]
    pub next_step: String,
}

/// `switch` / case dispatch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwitchStep {
    /// Context field whose value is matched against the
    /// `cases` literals.
    pub field: String,
    /// Ordered list of `(literal, step_id)` pairs. The
    /// first match wins.
    pub cases: Vec<SwitchCase>,
    /// Step id to follow when no case matches. May be
    /// empty.
    #[serde(default)]
    pub default_step: String,
}

/// A single `case` arm in a [`SwitchStep`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwitchCase {
    /// Literal value to match against `field`.
    pub value: serde_json::Value,
    /// Step id to follow on a match.
    pub step: String,
}

/// Fan-out branches for a [`StepKind::Parallel`] step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParallelStep {
    /// Branch step ids; each is executed in order (v0.1.0)
    /// or in parallel (B5+).
    pub branches: Vec<String>,
    /// Step id to follow after every branch completes.
    #[serde(default)]
    pub next_step: String,
}

/// A single control step. The executor walks the step
/// table by `id` and uses `kind` to decide what to do.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlStep {
    /// Stable step id (must be unique within a step
    /// table).
    pub id: String,
    /// What this step does.
    pub kind: StepKind,
    /// Optional pre-condition. When `Some`, the executor
    /// evaluates the condition and skips the step (and
    /// follows `next_step`) on `false`.
    #[serde(default)]
    pub condition: Option<Condition>,
    /// The default next-step id for `Action` / non-terminal
    /// steps. May be empty (terminal).
    #[serde(default)]
    pub next_step: String,
}

impl ControlStep {
    /// Build a new `Action` step.
    #[must_use]
    pub fn action(id: impl Into<String>, next_step: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: StepKind::Action,
            condition: None,
            next_step: next_step.into(),
        }
    }

    /// Build a new `Branch` step.
    #[must_use]
    pub fn branch(
        id: impl Into<String>,
        condition: Condition,
        then_step: impl Into<String>,
        else_step: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind: StepKind::Branch(BranchStep {
                condition,
                then_step: then_step.into(),
                else_step: else_step.into(),
            }),
            condition: None,
            next_step: String::new(),
        }
    }

    /// Build a new `Loop` step.
    #[must_use]
    pub fn loop_step(
        id: impl Into<String>,
        condition: Condition,
        body: Vec<String>,
        next_step: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind: StepKind::Loop(LoopStep {
                condition,
                body,
                next_step: next_step.into(),
            }),
            condition: None,
            next_step: String::new(),
        }
    }

    /// Build a new `Switch` step.
    #[must_use]
    pub fn switch(
        id: impl Into<String>,
        field: impl Into<String>,
        cases: Vec<SwitchCase>,
        default_step: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind: StepKind::Switch(SwitchStep {
                field: field.into(),
                cases,
                default_step: default_step.into(),
            }),
            condition: None,
            next_step: String::new(),
        }
    }

    /// Build a new `Parallel` step.
    #[must_use]
    pub fn parallel(
        id: impl Into<String>,
        branches: Vec<String>,
        next_step: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind: StepKind::Parallel(ParallelStep {
                branches,
                next_step: next_step.into(),
            }),
            condition: None,
            next_step: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::condition::Condition;
    use serde_json::json;

    #[test]
    fn action_builder() {
        let s = ControlStep::action("a", "b");
        assert_eq!(s.id, "a");
        assert!(matches!(s.kind, StepKind::Action));
        assert_eq!(s.next_step, "b");
    }

    #[test]
    fn branch_builder() {
        let s = ControlStep::branch(
            "if",
            Condition::Eq {
                left: "k".into(),
                right: json!(1),
            },
            "then",
            "else",
        );
        assert!(matches!(s.kind, StepKind::Branch(_)));
    }

    #[test]
    fn loop_builder() {
        let s = ControlStep::loop_step(
            "loop",
            Condition::Lt {
                left: "n".into(),
                right: 10.0,
            },
            vec!["body".into()],
            "after",
        );
        assert!(matches!(s.kind, StepKind::Loop(_)));
    }

    #[test]
    fn switch_builder() {
        let s = ControlStep::switch(
            "switch",
            "tag",
            vec![SwitchCase {
                value: json!("a"),
                step: "step-a".into(),
            }],
            "default",
        );
        assert!(matches!(s.kind, StepKind::Switch(_)));
    }

    #[test]
    fn parallel_builder() {
        let s = ControlStep::parallel("par", vec!["b1".into(), "b2".into()], "join");
        assert!(matches!(s.kind, StepKind::Parallel(_)));
    }

    #[test]
    fn serde_round_trip() {
        let s = ControlStep::branch(
            "if",
            Condition::Eq {
                left: "k".into(),
                right: json!(1),
            },
            "t",
            "e",
        );
        let json = serde_json::to_string(&s).expect("serialize");
        let back: ControlStep = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, s);
    }
}
