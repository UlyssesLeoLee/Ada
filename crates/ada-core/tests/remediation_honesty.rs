//! A remediation step states what it will do; nothing checked that the
//! deployment could actually do it.
//!
//! ## What was wrong
//!
//! Three facts were true at once, and each one is the kind that only
//! shows up during an incident.
//!
//! **The engine's mode was never consulted for `run_command`.**
//! `main.rs` builds `RemediationEngine::new()`, which is a
//! `DryRunExecutor` -- the service is nominally a rehearsal. But
//! `run_step` dispatched `run_command` by calling `run_shell_command`
//! directly, never going through the executor, so the one step kind
//! with real side effects was the one step kind dry-run did not
//! cover. The shipped `disk-space-low` runbook's second step is
//! `find /var/log -type f -name '*.gz' -mtime +7 -delete`, and it ran
//! for real on every matching alert while the remaining steps
//! reported rehearsed no-ops.
//!
//! **The `executor` field was parsed and then ignored.** All four
//! "outside world" variants carry `executor: ExecutorMode`, defaulting
//! to `DryRun`, and nothing read it -- not the engine, not either
//! executor. Setting `"executor": "real"` in a runbook changed
//! nothing; leaving it absent did not protect anything either.
//!
//! **A notification step reported success it had not achieved.**
//! `notify_slack` returned `ok("... skipped")` when
//! `SLACK_WEBHOOK_URL` was unset, and no shipped manifest ever set
//! it -- the only mention of the variable in the repository was the
//! `env::var` call itself. Three of the five runbooks in
//! `config/remediation/` carry a `notify_slack` step, so every run
//! recorded "notified #ada-ops" and nobody had been notified.
//!
//! ## What these gates hold
//!
//! The executor now short-circuits on mode before dispatch, a missing
//! credential fails the step, and the manifest declares the two keys.
//! These gates are what stop the next version from quietly undoing
//! that.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

/// The workspace root, found by walking up from this test binary.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("ada-core lives two levels under the workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    let path = repo_root().join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

/// Every `env::var("NAME")` key read by the step executor.
///
/// Scanned from the source rather than listed here on purpose: the
/// point is that the manifest and the code agree, and a hand-copied
/// list would agree with itself no matter which one drifted.
fn step_credentials() -> BTreeSet<String> {
    let text = read("crates/ada-remediation/src/executor.rs");
    let mut keys = BTreeSet::new();
    let mut rest = text.as_str();
    while let Some(at) = rest.find("env::var(") {
        rest = &rest[at + "env::var(".len()..];
        let Some(open) = rest.find('"') else { break };
        rest = &rest[open + 1..];
        let Some(close) = rest.find('"') else { break };
        let name = &rest[..close];
        // An env var this step needs, as opposed to a serde field or
        // a test-only string: the executor reads all-caps names.
        if !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        {
            keys.insert(name.to_string());
        }
        rest = &rest[close + 1..];
    }
    keys
}

/// The `stringData:` keys of the Secret the remediation Deployment
/// loads via `envFrom.secretRef`.
fn declared_secret_keys() -> BTreeSet<String> {
    let text = read("deploy/k8s/ada-remediation.yaml");
    let mut keys = BTreeSet::new();
    let mut in_target = false;
    let mut in_string_data = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("kind:") {
            in_target = trimmed == "kind: Secret";
            in_string_data = false;
            continue;
        }
        if !in_target {
            continue;
        }
        if trimmed.starts_with("stringData:") {
            in_string_data = true;
            continue;
        }
        // Any other key at the Secret's own indentation ends the
        // stringData block.
        if in_string_data && !line.starts_with(' ') {
            in_string_data = false;
        }
        if !in_string_data {
            continue;
        }
        let Some((key, _)) = trimmed.split_once(':') else {
            continue;
        };
        let key = key.trim();
        if !key.is_empty() && key.chars().all(|c| c.is_ascii_uppercase() || c == '_') {
            keys.insert(key.to_string());
        }
    }
    keys
}

#[test]
fn every_step_credential_is_declared_in_the_manifest_secret() {
    let required = step_credentials();
    // Anti-vacuity: if the scan finds nothing, the gate passes
    // because it is looking at nothing. Print what it actually read
    // rather than only its length.
    assert!(
        !required.is_empty(),
        "scanned executor.rs and found no credential; the scan is broken, \
         not the manifest"
    );

    let declared = declared_secret_keys();
    assert!(
        !declared.is_empty(),
        "found no stringData keys in the ada-remediation-secrets Secret; \
         the parse is broken, not the Secret"
    );

    let missing: Vec<&String> = required.difference(&declared).collect();
    assert!(
        missing.is_empty(),
        "the executor reads {required:?} but the Secret declares {declared:?}. \
         A step needing {missing:?} boots fine and only fails when an alert \
         fires it. Declare the keys in deploy/k8s/ada-remediation.yaml."
    );
}

#[test]
fn the_shipped_wiring_still_runs_a_dry_run_engine() {
    // A tripwire, not a style rule. `main.rs` is what production runs,
    // and it is what decides whether `run_command` steps reach
    // `run_shell_command`. Wiring a real executor here is a
    // one-word change that makes every shipped runbook's commands
    // real -- including the log prune in `disk-space-low`.
    //
    // To turn it on deliberately, land the RealExecutor wiring and
    // this gate's expectation in the same commit, so the diff shows
    // both halves of the decision.
    let text = read("crates/ada-remediation/src/main.rs");
    assert!(
        !text.contains("with_executor("),
        "main.rs now injects a real step executor. That makes the shipped \
         runbooks live: disk-space-low runs `find ... -delete` on a matching \
         alert. If that is intended, update this gate in the same commit."
    );
    assert!(
        text.contains("RemediationEngine::new()"),
        "expected main.rs to build the default (dry-run) engine"
    );
}
