//! The one place `ada-remediation` executes a process must not be able to
//! execute anything an alert influenced.
//!
//! ## What this protects
//!
//! `ActionStep::RunCommand` is the only command execution in the service.
//! Its `cmd` and `args` come from a runbook file — trusted configuration.
//! The alert is not: it arrives over a webhook, and whoever can post to
//! that endpoint chooses the label values.
//!
//! The service gets this right in two ways, neither of which was written
//! down anywhere before this gate:
//!
//! 1. **No shell.** `run_shell_command` calls `Command::new(cmd).args(args)`
//!    directly, so `;`, `|`, `&&` and friends are ordinary argument
//!    characters with no meaning.
//! 2. **No interpolation.** The `RunCommand` arm of `run_step` passes the
//!    runbook's own `cmd` and `args` through untouched, and the engine —
//!    the file that owns process execution — does not reference
//!    `AlertEvent::render_template` at all.
//!
//! Add template rendering to the command path and the receiver of a
//! webhook gets arbitrary code execution, because they control the labels.
//! That change would look entirely reasonable: "parameterise the command
//! with the alert's service name" is a natural feature request, it is one
//! line, and nothing in the existing code raises an objection.
//!
//! ## Why a source check rather than a behavioural test
//!
//! The invariant is textual — *this file does not reach the template
//! renderer, and the executor does not go through a shell* — so the test
//! reads the source and pins it. A behavioural test cannot distinguish the
//! two cases here: a runbook whose command is a template placeholder fails
//! to spawn whether the placeholder was rendered or not, so the safe and
//! the vulnerable version produce the same error. A test built on that
//! distinction would pass against the vulnerable code.
//!
//! Source-scanning gates are brittle in general, and this one is worth the
//! brittleness because the alternative is an undocumented boundary on a
//! code-execution path. It matches the shape of the existing
//! `kustomization.rs`, `probe_paths.rs` and `deploy_images.rs` gates, needs
//! no cluster, no kubectl and no network, and therefore runs on every
//! `cargo test`.
//!
//! Each property below was proven to fail when the property is broken.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("ada-core lives two levels under the workspace root")
        .to_path_buf()
}

fn remediation_src(file: &str) -> PathBuf {
    repo_root().join("crates/ada-remediation/src").join(file)
}

fn read(file: &str) -> String {
    let p = remediation_src(file);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("could not read {}: {e}", p.display()))
}

/// The body of the `ActionStep::RunCommand { .. } =>` arm inside `run_step`:
/// from the arm header up to the next `ActionStep::` arm, or the end of the
/// file.
///
/// Scoped to the arm rather than the whole file on purpose, so a failure
/// points at the code that has to change.
fn run_command_arm(engine: &str) -> String {
    let start = engine
        .find("ActionStep::RunCommand {")
        .expect("run_step must handle ActionStep::RunCommand");
    let end = engine[start + 1..]
        .find("ActionStep::")
        .map_or(engine.len(), |i| start + 1 + i);
    engine[start..end].to_string()
}

/// The body of `run_shell_command`: from its signature to the closing brace
/// at column 0.
fn run_shell_command_body(engine: &str) -> String {
    let start = engine
        .find("async fn run_shell_command")
        .expect("engine.rs must define run_shell_command");
    let end = engine[start + 1..]
        .find("\n}\n")
        .map_or(engine.len(), |i| start + 1 + i);
    engine[start..end].to_string()
}

/// The engine is the only file that spawns a process, so it must not be
/// able to reach the template renderer at all.
///
/// This is the invariant that actually stops alert labels reaching a command
/// line, and it is deliberately stronger than "the `RunCommand` arm does not
/// call `render_template`": a renderer call anywhere in this file is a
/// place where a future edit could put one next to the command path, and
/// the file that owns `Command::new` is the wrong place for that to be one
/// line away.
#[test]
fn the_engine_never_reaches_the_template_renderer() {
    let engine = read("engine.rs");
    assert!(
        !engine.contains("render_template"),
        "engine.rs references render_template. This is the file that runs \
         `Command::new`, so a label substituted here is a label an attacker \
         chose, handed to a program they may also influence. If a step \
         genuinely needs a value from the alert, do the interpolation in \
         executor.rs, which is where the message- and URL-building steps \
         already live."
    );
}

/// The arm must forward the runbook's own values, not a re-derived one.
#[test]
fn the_run_command_arm_forwards_the_runbook_verbatim() {
    let arm = run_command_arm(&read("engine.rs"));
    assert!(
        arm.contains("run_shell_command(cmd, args"),
        "the ActionStep::RunCommand arm no longer forwards the runbook's \
         `cmd` and `args` verbatim:\n{arm}\n\
         Anything computed there is a new place for alert data to enter the \
         command line."
    );
}

/// Direct exec, never a shell.
///
/// Without this the first gate buys nothing. A renderer kept out of the
/// engine is no protection if the executor is `sh -c <cmd>`, because then
/// every argument is an injection point again.
#[test]
fn the_executor_does_not_route_through_a_shell() {
    let body = run_shell_command_body(&read("engine.rs"));

    assert!(
        body.contains("Command::new(cmd)"),
        "run_shell_command no longer execs the runbook command directly:\n{body}\n\
         Use Command::new(<the cmd>) so the argument vector goes to the \
         program itself."
    );

    // Interpreter names rather than `-c` / `/C`, which are too common to
    // match safely.
    for shell in [
        "\"sh\"",
        "\"bash\"",
        "\"zsh\"",
        "\"cmd\"",
        "\"cmd.exe\"",
        "\"powershell\"",
        "\"pwsh\"",
    ] {
        assert!(
            !body.contains(shell),
            "run_shell_command references the shell {shell}:\n{body}\n\
             This service execs runbook commands directly, without a shell, \
             so that shell metacharacters in a command are inert."
        );
    }
}

/// `executor.rs` handles every step except `RunCommand`, which it
/// explicitly declines (`"run_command handled by engine"`). That is the
/// right split, and it is load-bearing: it is what keeps the process
/// boundary in one auditable file. A `Command::new` here would be a second
/// boundary that no gate is watching.
#[test]
fn the_step_executor_never_spawns_a_process() {
    let executor = read("executor.rs");
    assert!(
        !executor.contains("Command::new"),
        "executor.rs spawns a process. RunCommand is supposed to be handled \
         by the engine; a second spawn path here is a code-execution \
         boundary that this gate's shell and renderer checks do not cover."
    );
}
