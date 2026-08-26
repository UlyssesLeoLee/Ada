//! Integration tests for the v0.1.0 control flow executor.
//!
//! The v0.1.0 surface is in-process, so the "integration"
//! tests exercise the public surface the way a real
//! canvas-driven loop would: build a step table, hand it
//! to `ControlFlowExecutor`, and assert the shape of the
//! returned `ExecutionResult`.

use std::collections::HashMap;

use ada_m05_control_flow::{
    Condition, ControlFlowExecutor, ControlStep, ExecutorError, StepKind, SwitchCase,
};
use serde_json::json;

fn ctx() -> HashMap<String, serde_json::Value> {
    HashMap::new()
}

#[tokio::test]
async fn end_to_end_branch_dispatch() {
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
        ControlStep::action("yes", "done"),
        ControlStep::action("no", "done"),
        ControlStep::action("done", ""),
    ];
    let exec = ControlFlowExecutor::new(steps);
    let mut c = ctx();
    c.insert("k".into(), json!(1));
    let out = exec.execute("if", c).expect("ok");
    assert_eq!(out.trace, vec!["if", "yes", "done"]);
    assert!(out.is_terminal());
}

#[tokio::test]
async fn end_to_end_loop_with_counter_terminates() {
    // Body points back to the loop head; the condition
    // reads the auto-incremented `__loop_iter__loop` field
    // so the loop terminates after 4 iterations.
    let steps = vec![
        ControlStep::loop_step(
            "loop",
            Condition::Lt {
                left: "__loop_iter__loop".into(),
                right: 4.0,
            },
            vec!["tick".into()],
            "after",
        ),
        ControlStep::action("tick", "loop"),
        ControlStep::action("after", ""),
    ];
    let exec = ControlFlowExecutor::new(steps);
    let out = exec.execute("loop", ctx()).expect("ok");
    // loop -> tick -> loop -> tick -> loop -> tick -> loop -> tick -> loop -> after
    assert_eq!(
        out.trace,
        vec!["loop", "tick", "loop", "tick", "loop", "tick", "loop", "tick", "loop", "after"]
    );
    assert!(out.is_terminal());
}

#[tokio::test]
async fn end_to_end_switch_with_default() {
    let steps = vec![
        ControlStep::switch(
            "sw",
            "tag",
            vec![
                SwitchCase {
                    value: json!("a"),
                    step: "a".into(),
                },
                SwitchCase {
                    value: json!("b"),
                    step: "b".into(),
                },
            ],
            "default",
        ),
        ControlStep::action("a", ""),
        ControlStep::action("b", ""),
        ControlStep::action("default", ""),
    ];
    let exec = ControlFlowExecutor::new(steps);
    let mut c = ctx();
    c.insert("tag".into(), json!("z"));
    let out = exec.execute("sw", c).expect("ok");
    assert_eq!(out.trace, vec!["sw", "default"]);
}

#[tokio::test]
async fn step_level_condition_skips_step() {
    let gated = ControlStep {
        id: "gated".into(),
        kind: StepKind::Action,
        condition: Some(Condition::Eq {
            left: "k".into(),
            right: json!(1),
        }),
        next_step: "after".into(),
    };
    let steps = vec![gated, ControlStep::action("after", "")];
    let exec = ControlFlowExecutor::new(steps);
    let mut c = ctx();
    c.insert("k".into(), json!(2));
    let out = exec.execute("gated", c).expect("ok");
    // `gated` is recorded in the trace but its body is
    // skipped (the step-level condition evaluated
    // false), and the executor follows `next_step`.
    assert_eq!(out.trace, vec!["gated", "after"]);
}

#[tokio::test]
async fn max_iterations_cap_surfaces_max_recursion() {
    // Self-loop that would never terminate. The cap
    // trips.
    let steps = vec![ControlStep::action("loop", "loop")];
    let exec = ControlFlowExecutor::new(steps).with_max_iterations(4);
    let err = exec.execute("loop", ctx()).expect_err("cap");
    assert!(matches!(err, ExecutorError::MaxRecursionExceeded(_)));
}
