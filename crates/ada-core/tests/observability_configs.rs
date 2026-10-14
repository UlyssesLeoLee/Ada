//! The observability stack is 34 tracked config files that nothing parsed,
//! and the README that tells an operator what to expect had drifted from it.
//!
//! ## What was wrong
//!
//! `observability/scripts/validate-configs.py` existed to YAML/JSON-lint
//! that tree. Two independent faults made it useless:
//!
//! **It had been failing.** Five of its hardcoded paths named
//! `prometheus/alerts/<name>.yml` for rules that live in
//! `prometheus/alerts-disabled/`. The `except` clause turned the missing
//! file into a `FAIL` line, so the script printed `28/33` and exited 1.
//! `observability/README.md` listed it as an available check, so a reader
//! would reasonably assume it had been run.
//!
//! **Nothing ran it.** No workflow in `.github/workflows/` mentioned
//! `observability/` at all. Not the script, not the directory: 34 files
//! covering Prometheus, Alertmanager, Loki, Promtail, Jaeger, the `OTel`
//! collector, Tempo and 11 Grafana dashboards had never been parsed by CI.
//!
//! The list was also the right length by accident. 23 YAML entries for 23
//! YAML files, with five of the paths wrong and one tracked dashboard
//! (`phase8-remediation-overview.json`) missing entirely -- the totals
//! matched, so a human skimming the output saw a plausible number.
//!
//! ## Why the script now walks the tree
//!
//! A hardcoded list is the shape that rots. `validate-configs.py` now
//! discovers what is on disk, so a new config is validated without editing
//! it, and the count it prints is the count on disk. Its exit codes are
//! load-bearing: 1 for a malformed config, 2 for a broken walk, so a run
//! that validates nothing cannot look green.
//!
//! ## What these gates hold
//!
//! The wiring, the numbers, and the one boundary that makes "disabled"
//! mean something. `alerts-disabled/` is inert only because
//! `prometheus.yml`'s `rule_files` globs do not reach it. Add
//! `alerts-disabled/*.yml` there and five rules that depend on metrics no
//! service emits become live -- an incident channel that fires on nothing,
//! which is the failure `alerts-disabled/README.md` was written to avoid.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const WORKFLOWS: &str = ".github/workflows";
const ALERTS_DIR: &str = "observability/prometheus/alerts";
const DISABLED_DIR: &str = "observability/prometheus/alerts-disabled";
const DASHBOARDS_DIR: &str = "observability/grafana/dashboards";
const SCRIPT: &str = "observability/scripts/validate-configs.py";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("ada-core lives two levels under the workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    let path = repo_root().join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

fn files_in(rel_dir: &str, ext: &str) -> Vec<PathBuf> {
    let dir = repo_root().join(rel_dir);
    let Ok(entries) = fs::read_dir(&dir) else {
        panic!(
            "{} does not exist -- this gate is pointed at a tree it cannot read",
            dir.display()
        );
    };
    let mut out: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext)))
        .collect();
    out.sort();
    out
}

/// Alert rules in one directory. The shape in these files is
/// `      - alert: ServiceDown`, so the line starts with a YAML sequence
/// marker rather than the key, and commented-out examples are skipped.
fn alert_rules_in(rel_dir: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for path in files_in(rel_dir, "yml") {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for line in text.lines() {
            let body = line.trim_start().trim_start_matches('-').trim_start();
            if body.starts_with('#') {
                continue;
            }
            let Some(rest) = body.strip_prefix("alert:") else {
                continue;
            };
            let name = rest.trim();
            if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                out.insert(name.to_string());
            }
        }
    }
    out
}

/// Counts the README states, as `(line_number, claimed)`, for every line
/// that mentions `subject`.
///
/// Two details of the prose decide whether this finds anything, and both
/// were wrong in the first version:
///
/// * The repository writes the counter with a space -- `3 つ` and
///   `11 個`, not `3つ`. A walk-back that stops at the first non-digit
///   finds no run at all, and the gate then reports "this gate cannot
///   read a claim", which reads like an absent claim and is not one.
/// * The counter is not one character. Small counts take `つ`; a
///   dashboard count of eleven takes `個`. Matching only `つ` silently
///   skipped every larger claim.
///
/// `3 つ目のコマンド` is an ordinal -- the third command, not a count of
/// three -- so a `つ` followed by `目` is skipped rather than read as a
/// claim.
fn readme_count_claims(subject: &str) -> Vec<(usize, usize)> {
    let text = read("observability/README.md");
    let mut out = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        if !line.contains(subject) {
            continue;
        }
        let chars: Vec<char> = line.chars().collect();
        for (i, &c) in chars.iter().enumerate() {
            if !matches!(c, 'つ' | '個') {
                continue;
            }
            if c == 'つ' && chars.get(i + 1) == Some(&'目') {
                continue; // ordinal, not a count
            }
            // Back over the separating whitespace...
            let mut run_start = i;
            while run_start > 0 && matches!(chars[run_start - 1], ' ' | '\u{3000}') {
                run_start -= 1;
            }
            // ...then over the digits themselves.
            let mut start = run_start;
            while start > 0 && chars[start - 1].is_ascii_digit() {
                start -= 1;
            }
            if start == run_start {
                continue;
            }
            let digits: String = chars[start..run_start].iter().collect();
            if let Ok(n) = digits.parse::<usize>() {
                out.push((idx + 1, n));
                break; // one claim per line: the first counter wins
            }
        }
    }
    out
}

#[test]
fn ci_actually_runs_the_observability_config_validator() {
    let dir = repo_root().join(WORKFLOWS);
    let entries = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().is_some_and(|e| {
                    e.eq_ignore_ascii_case("yml") || e.eq_ignore_ascii_case("yaml")
                })
        })
        .collect::<Vec<_>>();

    assert!(
        !entries.is_empty(),
        "no workflow files found under {WORKFLOWS} -- the scan is broken, \
         not the workflows"
    );

    let mut invocations: Vec<String> = Vec::new();
    let mut mentions = 0usize;
    for path in &entries {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        if !text.contains("validate-configs.py") {
            continue;
        }
        mentions += 1;
        // A mention is not an invocation. The first version of this gate
        // accepted any occurrence of the string, and the job header comment
        // -- which explains what the script does -- was enough to satisfy
        // it. Deleting the `run:` step left the gate green, because the
        // explanation of how to run it survived the removal of the run.
        // Only a non-comment line outside a `name:` counts.
        let invokes = text.lines().any(|line| {
            let t = line.trim();
            !t.starts_with('#')
                && !t.starts_with("- name:")
                && !t.starts_with("name:")
                && t.contains("validate-configs.py")
        });
        if invokes {
            invocations.push(
                path.strip_prefix(repo_root())
                    .unwrap_or(path)
                    .display()
                    .to_string(),
            );
        }
    }

    assert!(
        !invocations.is_empty(),
        "no workflow under {WORKFLOWS} *runs* {SCRIPT} -- {mentions} workflow(s) \
         mention the path but none execute it. The script parses the 34 \
         observability config files; without a caller it is documentation, not \
         a gate, and it is currently trustworthy only because it has never \
         been run against a tree that broke."
    );

    // The script is the thing being invoked, so it has to exist. Naming a
    // path is the same claim as the old manifest comment that cited a gate
    // file nobody ever wrote.
    assert!(
        repo_root().join(SCRIPT).is_file(),
        "{SCRIPT} is invoked by {invocations:?} but does not exist"
    );
}

#[test]
fn readme_alert_rule_counts_match_the_rules_prometheus_loads() {
    let loaded = alert_rules_in(ALERTS_DIR);
    assert!(
        !loaded.is_empty(),
        "no alert rules found under {ALERTS_DIR}; a count derived from an \
         empty set is not evidence"
    );
    let actual = loaded.len();
    let disabled = alert_rules_in(DISABLED_DIR);

    let claims = readme_count_claims("alert rule");
    assert!(
        !claims.is_empty(),
        "observability/README.md states no alert-rule count that this gate \
         can read; the Japanese counter parse is broken, not the README"
    );

    let wrong: Vec<String> = claims
        .iter()
        .filter(|(_, claimed)| *claimed != actual)
        .map(|(line, claimed)| {
            format!(
                "README.md:{line} claims {claimed} alert rules, but {ALERTS_DIR} defines {actual}"
            )
        })
        .collect();

    assert!(
        wrong.is_empty(),
        "the loaded alert-rule count is {actual} ({loaded:?}), with {} more \
         inert in {DISABLED_DIR} ({disabled:?}):\n  {}",
        disabled.len(),
        wrong.join("\n  ")
    );
}

#[test]
fn readme_dashboard_count_matches_the_dashboards_grafana_provisions() {
    let dashboards = files_in(DASHBOARDS_DIR, "json");
    assert!(
        !dashboards.is_empty(),
        "no dashboards found under {DASHBOARDS_DIR}; the count is derived \
         from the directory, so an empty one is a missing tree, not zero"
    );
    let actual = dashboards.len();

    let claims = readme_count_claims("dashboard");
    assert!(
        !claims.is_empty(),
        "observability/README.md states no dashboard count that this gate can \
         read; the Japanese counter parse is broken, not the README"
    );

    let wrong: Vec<String> = claims
        .iter()
        .filter(|(_, claimed)| *claimed != actual)
        .map(|(line, claimed)| {
            format!(
                "README.md:{line} claims {claimed} dashboards, but {DASHBOARDS_DIR} holds {actual}"
            )
        })
        .collect();

    assert!(
        wrong.is_empty(),
        "{DASHBOARDS_DIR} holds {actual} dashboards and Grafana provisions \
         every JSON in it, so the expected value an operator checks against \
         is wrong:\n  {}",
        wrong.join("\n  ")
    );
}

#[test]
fn disabled_alerts_stay_out_of_the_loaded_rule_files() {
    let text = read("observability/prometheus/prometheus.yml");

    // The `rule_files:` list, which is the only thing that decides whether a
    // rule file is loaded. Read out of the file rather than assumed, so a
    // reordering does not silently move the boundary.
    let mut globs: Vec<String> = Vec::new();
    let mut in_rule_files = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("rule_files:") {
            in_rule_files = true;
            continue;
        }
        if !in_rule_files {
            continue;
        }
        // Any key at the top level ends the block.
        if !line.starts_with(' ') && !trimmed.is_empty() {
            in_rule_files = false;
            continue;
        }
        let Some(rest) = trimmed.strip_prefix("- ") else {
            continue;
        };
        globs.push(rest.trim().trim_matches(['"', '\'']).to_string());
    }

    assert!(
        !globs.is_empty(),
        "found no rule_files globs in observability/prometheus/prometheus.yml; \
         the scan is broken, not the config"
    );
    assert!(
        !alert_rules_in(DISABLED_DIR).is_empty(),
        "{DISABLED_DIR} holds no rules, so this gate protects a directory \
         that no longer has anything in it"
    );

    // A glob reaches the disabled directory if it names it, or if it is
    // broad enough to sweep it up (`*.yml`, `**/*.yml`).
    let reaching: Vec<&String> = globs
        .iter()
        .filter(|g| {
            g.contains("alerts-disabled")
                || *g == "*.yml"
                || *g == "*.yaml"
                || g.contains("**/*.yml")
                || g.contains("**/*.yaml")
        })
        .collect();

    assert!(
        reaching.is_empty(),
        "prometheus.yml rule_files {reaching:?} would load rules from \
         {DISABLED_DIR}. Every rule there is inert by design -- \
         alerts-disabled/README.md records that each one queries a metric no \
         shipped service emits, and lists what has to exist first. Loading \
         them turns five rules that can never fire into five that fire with \
         no data behind them."
    );
}
