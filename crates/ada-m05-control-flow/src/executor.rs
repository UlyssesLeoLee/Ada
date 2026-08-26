//! [`ControlFlowExecutor`] — walks a step table and returns
//! an [`ExecutionResult`].
//!
//! The v0.1.0 surface is intentionally simple:
//!
//! - The executor owns a `HashMap<String, ControlStep>` and
//!   a `current` step id. The caller passes the entry step
//!   id to [`execute`](Self::execute).
//! - Each iteration the executor looks up `current`, runs
//!   it, and updates `current` according to the step's
//!   `kind`.
//! - The executor bounds the loop with a `max_iterations`
//!   cap and an optional time budget. Either cap surfaces a
//!   distinct error variant so callers can pick the right
//!   recovery.
//! - The result is an [`ExecutionResult`] with the final
//!   context (`HashMap<String, Value>`) and a trace
//!   (`Vec<String>` of step ids in execution order).
//!
//! See [`DOC-MOD-005`](../docs/modules/M-05-control-flow.md)
//! §3.5 for the full execution pipeline.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::error::{ExecutorError, Result};
use crate::step::{ControlStep, StepKind};

/// The result of an [`ControlFlowExecutor::execute`] call.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ExecutionResult {
    /// Step id where execution terminated. `None` when the
    /// executor exited because `current` was an empty
    /// string (terminal).
    pub final_step: Option<String>,
    /// Snapshot of the context at termination.
    pub context: HashMap<String, Value>,
    /// Step ids in execution order (most recent last).
    pub trace: Vec<String>,
}

impl ExecutionResult {
    /// True if the executor reached a terminal step (one
    /// whose `next_step` is empty).
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        self.final_step.is_none()
    }

    /// Number of steps executed.
    #[must_use]
    pub fn trace_len(&self) -> usize {
        self.trace.len()
    }
}

/// Default iteration cap for the executor. The skeleton
/// keeps the cap low so an accidental infinite loop in a
/// step table surfaces as a `MaxRecursionExceeded` error
/// rather than a process hang.
pub const DEFAULT_MAX_ITERATIONS: usize = 1024;

/// Default time budget for the executor.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// The step walker.
#[derive(Debug, Default)]
pub struct ControlFlowExecutor {
    /// The step table. `HashMap` so `step(id)` is O(1).
    steps: HashMap<String, ControlStep>,
    /// Iteration cap.
    max_iterations: usize,
    /// Time budget.
    timeout: Duration,
}

impl ControlFlowExecutor {
    /// Build a new executor with the default cap and
    /// timeout.
    #[must_use]
    pub fn new(steps: Vec<ControlStep>) -> Self {
        Self {
            steps: steps.into_iter().map(|s| (s.id.clone(), s)).collect(),
            max_iterations: DEFAULT_MAX_ITERATIONS,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Override the iteration cap. Set to `usize::MAX` to
    /// disable the cap (not recommended).
    #[must_use]
    pub const fn with_max_iterations(mut self, cap: usize) -> Self {
        self.max_iterations = cap;
        self
    }

    /// Override the time budget. Set to `Duration::MAX` to
    /// disable the budget.
    #[must_use]
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Borrow a step by id.
    #[must_use]
    pub fn step(&self, id: &str) -> Option<&ControlStep> {
        self.steps.get(id)
    }

    /// Number of steps in the table.
    #[must_use]
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// True if the step table is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Walk the step table starting at `entry` with the
    /// supplied `initial_context`. The context is mutable
    /// (Action steps can mutate it; the skeleton does not
    /// add `Action` body yet, so mutations land in B5+).
    pub fn execute(
        &self,
        entry: &str,
        initial_context: HashMap<String, Value>,
    ) -> Result<ExecutionResult> {
        let deadline = Instant::now() + self.timeout;
        let mut current: Option<String> = Some(entry.to_string());
        let mut trace: Vec<String> = Vec::new();
        let mut context = initial_context;

        let mut iterations: usize = 0;
        while let Some(id) = current.take() {
            iterations += 1;
            if iterations > self.max_iterations {
                return Err(ExecutorError::MaxRecursionExceeded(format!(
                    "exceeded {} iterations",
                    self.max_iterations
                )));
            }
            if Instant::now() > deadline {
                return Err(ExecutorError::Timeout(self.timeout));
            }
            let step = self
                .steps
                .get(&id)
                .ok_or_else(|| ExecutorError::StepNotFound(id.clone()))?;
            // Honor a step-level condition: skip the step
            // and follow `next_step` on `false`.
            if let Some(cond) = &step.condition {
                if !cond.evaluate(&context)? {
                    current = next_nonempty(&step.next_step);
                    trace.push(id);
                    continue;
                }
            }
            trace.push(id.clone());
            match &step.kind {
                StepKind::Action => {
                    current = next_nonempty(&step.next_step);
                }
                StepKind::Branch(b) => {
                    let take_then = b.condition.evaluate(&context)?;
                    current = if take_then {
                        next_nonempty(&b.then_step)
                    } else {
                        next_nonempty(&b.else_step)
                    };
                }
                StepKind::Loop(l) => {
                    // The executor maintains an
                    // auto-incrementing iteration counter
                    // in the context, keyed by
                    // `__loop_iter__<loop_id>`. The
                    // condition can reference it to bound
                    // a for-style loop:
                    //
                    //   condition: Lt { left: "<n>", right: 3.0 }
                    //   body:      [tick -> loop]
                    //
                    // The body is expected to mutate
                    // `<n>` (or whatever the condition
                    // reads) so the loop terminates; the
                    // skeleton does not auto-mutate any
                    // user field. The iteration counter
                    // itself is exposed for diagnostics
                    // and for conditions that want to
                    // bound on iteration count.
                    let iter_key = format!("__loop_iter__{id}");
                    let iter = context.get(&iter_key).and_then(Value::as_u64).unwrap_or(0);
                    // Insert the iteration counter BEFORE evaluating the
                    // condition, so conditions can read `__loop_iter__<id>`.
                    context.insert(
                        iter_key.clone(),
                        Value::Number(serde_json::Number::from(iter)),
                    );
                    if l.condition.evaluate(&context)? {
                        let Some(first) = l.body.first() else {
                            return Err(ExecutorError::BackendError("loop body is empty".into()));
                        };
                        current = Some(first.clone());
                        // Bump the iteration counter for the next
                        // condition evaluation.
                        context.insert(iter_key, Value::Number(serde_json::Number::from(iter + 1)));
                    } else {
                        current = next_nonempty(&l.next_step);
                    }
                }
                StepKind::Switch(s) => {
                    let value = context.get(&s.field).ok_or_else(|| {
                        ExecutorError::ConditionError(format!("missing field: {}", s.field))
                    })?;
                    let mut matched: Option<String> = None;
                    for c in &s.cases {
                        if &c.value == value {
                            matched = Some(c.step.clone());
                            break;
                        }
                    }
                    current = match matched {
                        Some(s) => next_nonempty(&s),
                        None => next_nonempty(&s.default_step),
                    };
                }
                StepKind::Parallel(p) => {
                    // Sequential v0.1.0: walk branches in
                    // order and accumulate the trace. The
                    // output `current` is the parallel
                    // step's `next_step`.
                    for b in &p.branches {
                        trace.push(b.clone());
                    }
                    current = next_nonempty(&p.next_step);
                }
            }
        }
        Ok(ExecutionResult {
            final_step: None,
            context,
            trace,
        })
    }
}

fn next_nonempty(id: &str) -> Option<String> {
    if id.is_empty() {
        None
    } else {
        Some(id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::condition::Condition;
    use crate::step::ControlStep;
    use serde_json::json;

    fn empty_ctx() -> HashMap<String, Value> {
        HashMap::new()
    }

    #[test]
    fn execute_action_chain() {
        let steps = vec![ControlStep::action("a", "b"), ControlStep::action("b", "")];
        let exec = ControlFlowExecutor::new(steps);
        let out = exec.execute("a", empty_ctx()).expect("ok");
        assert_eq!(out.trace, vec!["a", "b"]);
        assert!(out.is_terminal());
    }

    #[test]
    fn execute_branch_takes_then_when_true() {
        let steps = vec![
            ControlStep::branch(
                "if",
                Condition::Eq {
                    left: "k".into(),
                    right: json!(1),
                },
                "yes",
                "no",
            ),
            ControlStep::action("yes", ""),
            ControlStep::action("no", ""),
        ];
        let exec = ControlFlowExecutor::new(steps);
        let mut ctx = HashMap::new();
        ctx.insert("k".into(), json!(1));
        let out = exec.execute("if", ctx).expect("ok");
        assert_eq!(out.trace, vec!["if", "yes"]);
    }

    #[test]
    fn execute_branch_takes_else_when_false() {
        let steps = vec![
            ControlStep::branch(
                "if",
                Condition::Eq {
                    left: "k".into(),
                    right: json!(2),
                },
                "yes",
                "no",
            ),
            ControlStep::action("yes", ""),
            ControlStep::action("no", ""),
        ];
        let exec = ControlFlowExecutor::new(steps);
        let mut ctx = HashMap::new();
        ctx.insert("k".into(), json!(99));
        let out = exec.execute("if", ctx).expect("ok");
        assert_eq!(out.trace, vec!["if", "no"]);
    }

    #[test]
    fn execute_loop_runs_body_while_condition_is_true() {
        // The executor auto-increments
        // `__loop_iter__<loop_id>` on every iteration.
        // The body points back to the loop head so the
        // condition is re-evaluated; the condition
        // checks the iteration counter so the loop
        // terminates after 3 iterations.
        let steps = vec![
            ControlStep::loop_step(
                "loop",
                Condition::Lt {
                    left: "__loop_iter__loop".into(),
                    right: 3.0,
                },
                vec!["tick".into()],
                "after",
            ),
            ControlStep::action("tick", "loop"),
            ControlStep::action("after", ""),
        ];
        let exec = ControlFlowExecutor::new(steps);
        let out = exec.execute("loop", empty_ctx()).expect("ok");
        // loop -> tick -> loop -> tick -> loop -> tick -> loop -> after
        assert_eq!(
            out.trace,
            vec!["loop", "tick", "loop", "tick", "loop", "tick", "loop", "after"]
        );
    }

    #[test]
    fn execute_switch_picks_matching_case() {
        let steps = vec![
            ControlStep::switch(
                "sw",
                "tag",
                vec![
                    crate::step::SwitchCase {
                        value: json!("a"),
                        step: "step-a".into(),
                    },
                    crate::step::SwitchCase {
                        value: json!("b"),
                        step: "step-b".into(),
                    },
                ],
                "default",
            ),
            ControlStep::action("step-a", ""),
            ControlStep::action("step-b", ""),
            ControlStep::action("default", ""),
        ];
        let exec = ControlFlowExecutor::new(steps);
        let mut ctx = HashMap::new();
        ctx.insert("tag".into(), json!("b"));
        let out = exec.execute("sw", ctx).expect("ok");
        assert_eq!(out.trace, vec!["sw", "step-b"]);
    }

    #[test]
    fn execute_switch_falls_through_to_default() {
        let steps = vec![
            ControlStep::switch(
                "sw",
                "tag",
                vec![crate::step::SwitchCase {
                    value: json!("a"),
                    step: "step-a".into(),
                }],
                "default",
            ),
            ControlStep::action("step-a", ""),
            ControlStep::action("default", ""),
        ];
        let exec = ControlFlowExecutor::new(steps);
        let mut ctx = HashMap::new();
        ctx.insert("tag".into(), json!("z"));
        let out = exec.execute("sw", ctx).expect("ok");
        assert_eq!(out.trace, vec!["sw", "default"]);
    }

    #[test]
    fn execute_parallel_walks_branches_in_order() {
        let steps = vec![
            ControlStep::parallel("par", vec!["b1".into(), "b2".into()], "join"),
            ControlStep::action("join", ""),
        ];
        let exec = ControlFlowExecutor::new(steps);
        let out = exec.execute("par", empty_ctx()).expect("ok");
        assert_eq!(out.trace, vec!["par", "b1", "b2", "join"]);
    }

    #[test]
    fn execute_unknown_entry_surfaces_step_not_found() {
        let exec = ControlFlowExecutor::new(vec![]);
        let err = exec.execute("missing", empty_ctx()).expect_err("missing");
        assert!(matches!(err, ExecutorError::StepNotFound(_)));
    }

    #[test]
    fn execute_max_iterations_cap_surfaces_max_recursion() {
        // Build a self-loop action. The cap will trip.
        let steps = vec![ControlStep::action("loop", "loop")];
        let exec = ControlFlowExecutor::new(steps).with_max_iterations(4);
        let err = exec.execute("loop", empty_ctx()).expect_err("cap");
        assert!(matches!(err, ExecutorError::MaxRecursionExceeded(_)));
    }

    #[test]
    fn execute_timeout_surfaces_timeout() {
        // Self-loop with a tiny timeout.
        let steps = vec![ControlStep::action("loop", "loop")];
        let exec = ControlFlowExecutor::new(steps)
            .with_max_iterations(usize::MAX)
            .with_timeout(Duration::from_millis(0));
        let err = exec.execute("loop", empty_ctx()).expect_err("timeout");
        assert!(matches!(err, ExecutorError::Timeout(_)));
    }

    #[test]
    fn step_level_condition_skips_step_on_false() {
        let steps = vec![
            ControlStep {
                id: "gated".into(),
                kind: StepKind::Action,
                condition: Some(Condition::Eq {
                    left: "k".into(),
                    right: json!(1),
                }),
                next_step: "after".into(),
            },
            ControlStep::action("after", ""),
        ];
        let exec = ControlFlowExecutor::new(steps);
        let mut ctx = HashMap::new();
        ctx.insert("k".into(), json!(2));
        let out = exec.execute("gated", ctx).expect("ok");
        // The trace still records `gated` (we always
        // append before deciding), but the step's
        // body is skipped and the executor moves to
        // `after`.
        assert_eq!(out.trace, vec!["gated", "after"]);
    }
}
