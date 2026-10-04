//! Every non-default feature a workspace crate declares must be compiled by a
//! CI lane that also builds test targets.
//!
//! ## What this protects
//!
//! The same defect was found three times in this repository, in three
//! different shapes, and none of them was visible in a code review:
//!
//! 1. `ada-m12-canvas-editor`'s `crdt.rs` — 1860 lines behind
//!    `#[cfg(feature = "crdt")]`, which no job supplied.
//! 2. `ada-telemetry`'s `prometheus` — the lane existed, but
//!    `src/testing.rs` is `#[cfg(any(feature = "testing", test))]` and
//!    every item in it is further gated on `prometheus`. `cargo check
//!    --features prometheus` has no `test` cfg, and `cargo test
//!    --workspace` has no `prometheus`, so the intersection was built by
//!    nothing. `TestHandle` and `test_recorder` sat in the gap.
//! 3. `ada-mock`'s `server` — the whole `FakeOtlpServer` module, plus
//!    the integration test that exercises it, which is itself
//!    `#[cfg(feature = "server")]`. `cargo clippy --workspace
//!    --all-targets` compiled the test file and linted nothing in it.
//!
//! Finding all three was an audit. This gate is what stops it being an
//! audit next time.
//!
//! ## Why `--all-targets` and not merely "the feature is mentioned"
//!
//! The ada-telemetry lane already passed the weaker test "this feature
//! appears in some cargo command", and still compiled none of the code
//! at issue. A lane that names a feature but omits `--all-targets`
//! builds the library only: `cfg(test)` code, and any test target gated
//! on that feature, is skipped. So the property checked here is the
//! conjunction — *named* **and** *built with test targets* — which is
//! exactly the conjunction both real failures violated.
//!
//! ## Why the matrix is resolved rather than ignored
//!
//! The `feature-matrix` job does not list its features on the `run:`
//! line; it passes `--features ${{ matrix.feature }}` and declares them
//! ten lines above. A gate that only scanned `run:` lines would report
//! all ten `ada-m12-canvas-editor` features as uncovered, and the fix
//! would be to delete a working lane. So `${{ matrix.<key> }}` is
//! resolved against the `matrix:` block of the same file, and
//! `matrix_expansion_is_actually_resolved` asserts that resolution
//! yields real values, so a parsing change cannot silently turn the
//! matrix features into "no features" (which would make this gate
//! vacuously pass).
//!
//! ## Scope and limits
//!
//! Source-scanning gates are brittle, and this one reads three formats
//! (Cargo.toml `[features]`, workspace `members`, workflow YAML) with
//! hand-rolled std-only parsers — the workspace has no YAML parser
//! available to Rust and the network is not something a test may
//! depend on. It is worth the brittleness because the alternative is a
//! coverage gap nobody notices until the code it covers is needed.
//!
//! It checks *declared* `[features]` only. Implicit features Cargo
//! derives from optional dependencies are not required to appear; that
//! is deliberate, since requiring them would flag every `dep:`-less
//! optional dependency in the workspace rather than any real gap.
//!
//! Each property below was proven to fail when the property is broken.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("ada-core lives two levels under the workspace root")
        .to_path_buf()
}

/// Indentation of a line, counting leading spaces.
fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// The first `"..."` on a line, if any.
fn first_quoted(line: &str) -> Option<String> {
    let start = line.find('"')?;
    let rest = &line[start + 1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Every `"..."` on a line, in order.
fn all_quoted(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find('"') {
        rest = &rest[start + 1..];
        match rest.find('"') {
            Some(end) => {
                out.push(rest[..end].to_string());
                rest = &rest[end + 1..];
            }
            None => break,
        }
    }
    out
}

struct Package {
    name: String,
    /// Every key in `[features]`.
    features: Vec<String>,
    /// The contents of `default = [...]`.
    defaults: BTreeSet<String>,
    /// `required-features` of each `[[bin]]` / `[[test]]` / `[[bench]]` /
    /// `[[example]]` target, as (target kind, features). Cargo refuses to
    /// build such a target unless the features are enabled, which makes
    /// it the sharpest version of the same trap: the target is not
    /// "under-tested", it is never compiled.
    target_required_features: Vec<(String, Vec<String>)>,
}

fn read_package(dir: &Path) -> Package {
    let text = fs::read_to_string(dir.join("Cargo.toml"))
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.join("Cargo.toml").display()));

    let mut section = String::new();
    let mut name = String::new();
    let mut features = Vec::new();
    let mut defaults = BTreeSet::new();
    let mut target_required_features = Vec::new();

    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            section = t.trim_matches(|c| c == '[' || c == ']').to_string();
            continue;
        }
        let Some((key, value)) = t.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.split('#').next().unwrap_or("").trim();

        if section == "package" && key == "name" && name.is_empty() {
            name = first_quoted(value).unwrap_or_default();
        } else if section == "features" {
            let Some(feature_key) = key
                .trim_matches('"')
                .split(',')
                .next()
                .map(str::trim)
                .filter(|k| !k.is_empty() && !k.contains(' '))
            else {
                continue;
            };
            if !features.contains(&feature_key.to_string()) {
                features.push(feature_key.to_string());
            }
            if feature_key == "default" {
                defaults = all_quoted(value).into_iter().collect();
            }
        } else if matches!(section.as_str(), "bin" | "test" | "bench" | "example")
            && key == "required-features"
        {
            target_required_features.push((section.clone(), all_quoted(value)));
        }
    }

    assert!(!name.is_empty(), "no [package] name in {}", dir.display());
    Package {
        name,
        features,
        defaults,
        target_required_features,
    }
}

fn workspace_members(root: &Path) -> Vec<PathBuf> {
    let text = fs::read_to_string(root.join("Cargo.toml")).expect("read workspace Cargo.toml");
    let mut members = Vec::new();
    let mut in_members = false;
    for line in text.lines() {
        let t = line.trim();
        if !in_members {
            if t.starts_with("members") && t.contains('[') {
                in_members = true;
            }
            continue;
        }
        // A line may hold both the last member and the closing bracket.
        let body = match t.find(']') {
            Some(i) => {
                in_members = false;
                &t[..i]
            }
            None => t,
        };
        if let Some(m) = first_quoted(body) {
            members.push(root.join(m));
        }
    }
    assert!(
        members.len() >= 20,
        "workspace member list parsed as {} entries -- the parser is broken, not the workspace",
        members.len()
    );
    members
}

/// `matrix:` blocks in one workflow file, as key -> values.
fn parse_matrices(lines: &[&str]) -> BTreeMap<String, Vec<String>> {
    let mut matrices: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim() == "matrix:" {
            let matrix_indent = indent_of(lines[i]);
            i += 1;
            let mut current: Option<String> = None;
            while i < lines.len() {
                let line = lines[i];
                if !line.trim().is_empty() && indent_of(line) <= matrix_indent {
                    break;
                }
                let t = line.trim();
                if let Some(rest) = t.strip_prefix("- ") {
                    if let Some(key) = current.clone() {
                        matrices
                            .entry(key)
                            .or_default()
                            .push(first_quoted(rest).unwrap_or_else(|| rest.trim().to_string()));
                    }
                } else if let Some(key) = t.strip_suffix(':') {
                    current = Some(key.trim().to_string());
                }
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    matrices
}

struct Lane {
    crate_name: Option<String>,
    features: Vec<String>,
    all_targets: bool,
    source: String,
    /// Did the raw command reference a `${{ matrix.* }}` expression?
    ///
    /// This is the only reliable way to tell a matrix lane from a
    /// hand-written one, and the gate got it wrong for a long time by
    /// proxying on `features.len() > 2` instead. That proxy happened to
    /// work only while the hand-written m12 clippy lane named exactly two
    /// features (`full,crdt`); adding a third to it silently reclassified
    /// the lane as a matrix lane and the gate began failing on correct
    /// configuration. Feature count is a number that changes when someone
    /// does something unrelated; whether the command interpolates a
    /// matrix entry is a property of the command.
    uses_matrix: bool,
}

/// Feature list of one command, with `${{ matrix.<key> }}` expanded.
///
/// The whole command is expanded first rather than the `--features`
/// argument, because `${{ matrix.feature }}` contains spaces: splitting
/// the raw line on whitespace yields `${{` as the argument and loses
/// the reference entirely. That bug shipped in the first draft of this
/// file and was caught by the gate below reporting ten working matrix
/// features as uncovered.
fn resolve_features(command: &str, matrices: &BTreeMap<String, Vec<String>>) -> Vec<String> {
    let mut expanded = String::with_capacity(command.len());
    let mut rest = command;
    while let Some(start) = rest.find("${{") {
        expanded.push_str(&rest[..start]);
        let after = &rest[start..];
        let Some(end) = after.find("}}") else {
            expanded.push_str(after);
            rest = "";
            break;
        };
        let inner = &after[3..end];
        match inner.split_once("matrix.") {
            Some((_, key)) => {
                let key: String = key
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                    .collect();
                match matrices.get(&key) {
                    Some(values) => expanded.push_str(&values.join(",")),
                    // Unresolvable: a token that cannot collide with a
                    // real feature name, so the gate reports the feature
                    // as uncovered instead of quietly treating the
                    // reference as satisfied.
                    None => {
                        let _ = write!(expanded, "unresolved-matrix-{key}");
                    }
                }
            }
            None => {
                let _ = write!(expanded, "literal-{inner}");
            }
        }
        rest = &after[end + 2..];
    }
    expanded.push_str(rest);

    expanded
        .split_whitespace()
        .collect::<Vec<_>>()
        .windows(2)
        .find(|w| w[0] == "--features")
        .map(|w| {
            w[1].split(',')
                .map(str::trim)
                .filter(|f| !f.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn cargo_lanes(root: &Path) -> Vec<Lane> {
    let dir = root.join(".github").join("workflows");
    let mut lanes = Vec::new();

    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e == "yml" || e == "yaml")
        })
        .collect();
    files.sort();

    for file in files {
        let text = fs::read_to_string(&file).expect("read workflow");
        let workflow_lines: Vec<&str> = text.lines().collect();
        let matrices = parse_matrices(&workflow_lines);

        for line in workflow_lines {
            let t = line.trim();
            let Some(command) = t.strip_prefix("run:") else {
                continue;
            };
            let command = command.trim();
            if !command.contains("cargo ") {
                continue;
            }

            let crate_name = command
                .split_whitespace()
                .collect::<Vec<_>>()
                .windows(2)
                .find(|w| w[0] == "-p")
                .map(|w| w[1].to_string());

            let features = resolve_features(command, &matrices);

            lanes.push(Lane {
                crate_name,
                features,
                all_targets: command.contains("--all-targets"),
                source: format!("{}: {}", file.display(), t),
                uses_matrix: t.contains("${{"),
            });
        }
    }
    lanes
}

#[test]
fn every_non_default_feature_is_built_by_a_lane_that_includes_test_targets() {
    let root = repo_root();
    let lanes = cargo_lanes(&root);

    let mut uncovered: Vec<String> = Vec::new();
    for dir in workspace_members(&root) {
        let pkg = read_package(&dir);
        for feature in &pkg.features {
            // `default` is never required to appear in a `--features`
            // list: it is on unless `--no-default-features` is passed,
            // and the `--workspace` lanes are what build it.
            if feature == "default" || pkg.defaults.contains(feature) {
                continue;
            }
            let covered = lanes.iter().any(|lane| {
                lane.crate_name.as_deref() == Some(pkg.name.as_str())
                    && lane.all_targets
                    && lane.features.iter().any(|f| f == feature)
            });
            if !covered {
                uncovered.push(format!("{}/{feature}", pkg.name));
            }
        }
    }

    assert!(
        uncovered.is_empty(),
        "no CI lane builds these features with --all-targets, so code \
         behind them is compiled by nothing: {}\n\
         Fix by adding a lane that names the crate, the feature and --all-targets. \
         A `cargo check`/`cargo build` lane is not enough: it builds the library \
         only and skips cfg(test) code and feature-gated test targets.",
        uncovered.join(", ")
    );
}

#[test]
fn every_required_features_target_is_built_by_some_lane() {
    // `required-features` is the sharpest form of the same trap. Cargo
    // refuses to build the target at all unless the features are on, so a
    // target whose features no lane supplies is not "under-tested" — it
    // is never compiled. Both instances in this workspace were already
    // covered (`ada-remediation`'s `[[bin]]` on `bin`, the m12
    // `crdt_sync` `[[test]]` on `crdt`), so this gate is about the third
    // one someone adds next.
    let root = repo_root();
    let lanes = cargo_lanes(&root);

    let mut found_any = false;
    let mut uncovered: Vec<String> = Vec::new();
    for dir in workspace_members(&root) {
        let pkg = read_package(&dir);
        for (kind, required) in &pkg.target_required_features {
            if required.is_empty() {
                continue;
            }
            found_any = true;
            for feature in required {
                let covered = lanes.iter().any(|lane| {
                    lane.crate_name.as_deref() == Some(pkg.name.as_str())
                        && lane.features.iter().any(|f| f == feature)
                });
                if !covered {
                    uncovered.push(format!("{}/{kind} target needs {feature}", pkg.name));
                }
            }
        }
    }

    assert!(
        found_any,
        "no [[bin]]/[[test]]/[[bench]]/[[example]] in the workspace declares \
         required-features; if that is genuinely true the parser is broken \
         (ada-remediation's [[bin]] requires `bin`)"
    );
    assert!(
        uncovered.is_empty(),
        "these targets are never built because no CI lane supplies their \
         required features: {}\nA target behind required-features is not \
         under-tested, it is not compiled at all.",
        uncovered.join(", ")
    );
}

#[test]
fn matrix_expansion_is_actually_resolved() {
    // Guards this file against its own most dangerous failure mode: if
    // `matrix.feature` stopped resolving, every matrix lane would report
    // no features and `every_non_default_feature_is_built...` would pass
    // while checking nothing.
    //
    // The first draft of this gate summed features across all lanes and
    // required >= 15, which the broken parser still satisfied: the
    // unresolved reference was recorded as a literal token, and those
    // phantom tokens were enough to clear the bar. So the assertion is
    // now about the matrix lane specifically, and it also rejects
    // `unresolved-matrix-` tokens outright.
    let root = repo_root();
    let lanes = cargo_lanes(&root);

    // Checked before the count assertions, because an unresolvable key and
    // a shrunken matrix are different failures and the message should say
    // which one happened.
    for lane in &lanes {
        assert!(
            !lane
                .features
                .iter()
                .any(|f| f.starts_with("unresolved-matrix-") || f.starts_with("literal-")),
            "lane references a matrix key this file could not resolve: {}",
            lane.source
        );
    }

    // The matrix lane is identified by the command interpolating a matrix
    // entry, not by how many features it resolved to. The earlier
    // `features.len() > 2` proxy broke the moment a hand-written m12 lane
    // named a third feature.
    let matrix_lanes: Vec<&Lane> = lanes
        .iter()
        .filter(|l| l.crate_name.as_deref() == Some("ada-m12-canvas-editor"))
        .filter(|l| l.uses_matrix)
        .collect();
    assert!(
        !matrix_lanes.is_empty(),
        "no ada-m12-canvas-editor lane interpolates a matrix entry; \
         the m12 matrix (10 entries) is not being expanded"
    );
    for lane in &matrix_lanes {
        assert!(
            lane.features.len() >= 10,
            "matrix lane resolved to {} features, expected the 10 declared in \
             the matrix: {}",
            lane.features.len(),
            lane.source
        );
    }

    // A hand-written lane is not required to name all ten, and saying so
    // would be wrong: `full,crdt` is a deliberate combination, not a
    // shrunken matrix. This is the assertion the old proxy could not make.
    for lane in lanes
        .iter()
        .filter(|l| l.crate_name.as_deref() == Some("ada-m12-canvas-editor"))
        .filter(|l| !l.uses_matrix)
    {
        assert!(
            !lane.features.is_empty(),
            "hand-written m12 lane names no features at all: {}",
            lane.source
        );
    }
}

#[test]
fn the_default_workspace_lane_still_builds_test_targets() {
    // Default features are covered by no `--features` flag at all, so the
    // only thing keeping them linted is a `--workspace` lane that also
    // builds test targets. Without it, dropping `--workspace` from CI
    // would silently unlint every crate in the repository.
    let root = repo_root();
    let lanes = cargo_lanes(&root);
    assert!(
        lanes.iter().any(|l| {
            l.source.contains("--workspace") && l.all_targets && l.source.contains("clippy")
        }),
        "no `cargo clippy --workspace ... --all-targets` lane found in \
         .github/workflows; default features would be unlinted"
    );
}

#[test]
fn the_lane_parser_finds_the_lanes_this_gate_relies_on() {
    // Anti-vacuity: a parser that returned nothing would satisfy the gate
    // above for the wrong reason. These are the lanes that actually carry
    // the coverage, named so a future refactor that stops parsing them
    // fails loudly instead of silently passing.
    let root = repo_root();
    let lanes = cargo_lanes(&root);
    assert!(
        lanes.len() >= 10,
        "only {} cargo lanes parsed out of .github/workflows",
        lanes.len()
    );

    let pairs: BTreeSet<(Option<String>, Vec<String>)> = lanes
        .iter()
        .map(|l| (l.crate_name.clone(), l.features.clone()))
        .collect();
    for expected in [
        ("ada-mock", vec!["server".to_string()]),
        (
            "ada-telemetry",
            vec!["prometheus".to_string(), "testing".to_string()],
        ),
        ("ada-remediation", vec!["bin".to_string()]),
        ("ada-rbac-casbin", vec!["hand-rolled".to_string()]),
        ("ada-m13-api-gateway", vec!["rbac-hand-rolled".to_string()]),
    ] {
        assert!(
            pairs.contains(&(Some(expected.0.to_string()), expected.1.clone())),
            "lane `{expected:?}` (crate + exact --features list) is not among \
             the {} parsed lanes; the parser no longer understands the \
             workflow's cargo commands",
            lanes.len()
        );
    }
}
