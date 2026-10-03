//! Every k8s probe path must be a route the service actually serves.
//!
//! ## The defect this gate exists for
//!
//! `deploy/k8s/ada-remediation.yaml` probed `/healthz`. The
//! `ada-remediation` binary serves `/health`. Verified against a running
//! process before the fix:
//!
//! ```text
//! GET /healthz  -> 404
//! GET /health   -> 200 ok
//! ```
//!
//! So the Deployment could never run: readiness never went green, the
//! pod was never added to the Service, and liveness killed the
//! container about 35 seconds in — a permanent `CrashLoopBackOff` for a
//! two-replica production service. Nothing caught it, because a probe
//! path that is valid YAML and a valid HTTP path is not an error in
//! either tool. `kubectl apply` is happy. `cargo build` is happy.
//!
//! ## Why the three services disagree
//!
//! They do, and that is the whole reason this needs to be a gate rather
//! than a convention:
//!
//! | service           | serves                          |
//! |-------------------|---------------------------------|
//! | `gm-console`      | `/healthz`                      |
//! | `ada-api-gateway` | `/health/live`, `/health/ready`  |
//! | `ada-remediation` | `/health`                       |
//!
//! There is no shared spelling to copy, so each manifest has to name its
//! own and the only thing that can check it is a test.
//!
//! ## Deliberately crude
//!
//! This scans text rather than parsing YAML, because the workspace has
//! no YAML parser available and the network cannot be relied on to add
//! one. It understands the two probe spellings actually in use (block
//! `path:` and flow `httpGet: { path: ... }`) and the `.route("…")`
//! literal form, which is all it needs to enforce the invariant. A
//! manifest that uses neither is reported rather than silently skipped.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// A deployed service: the manifest that configures it and the source
/// that defines its routes.
///
/// A new service in `deploy/k8s/` **must** be added here —
/// `every_deployment_manifest_is_registered` fails otherwise, so this
/// table cannot quietly fall behind the directory.
const SERVICES: &[(&str, &str)] = &[
    (
        "deploy/k8s/ada-remediation.yaml",
        "crates/ada-remediation/src/http.rs",
    ),
    (
        "deploy/k8s/ada-api-gateway.yaml",
        "crates/ada-m13-api-gateway/src/router.rs",
    ),
    (
        "deploy/k8s/gm-console.yaml",
        "crates/gm-console/src/routes.rs",
    ),
];

/// The workspace root, two levels up from `crates/ada-core`.
///
/// Two, not one: `CARGO_MANIFEST_DIR` is `<root>/crates/ada-core`, so a
/// single `parent()` lands on `<root>/crates` and every path in
/// `SERVICES` resolves to `<root>/crates/deploy/k8s/...`.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("ada-core lives two levels under the workspace root")
        .to_path_buf()
}

/// Indentation width, ignoring tabs and any leading spaces.
fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Extract every probe path declared in a manifest.
///
/// Handles both spellings in use across these manifests:
/// ```yaml
/// readinessProbe:
///   httpGet:
///     path: /health
///     port: http
/// ```
/// and the flow style, where `httpGet` and its `path` share a line:
/// ```yaml
///   readinessProbe:
///     httpGet: { path: /health/ready, port: 8080 }
/// ```
///
/// Block membership is decided by indentation, so a `path:` on a line
/// only counts while it is deeper than the probe key that opened the
/// block. Without that, a `path:` belonging to anything else in the
/// manifest would be counted as a probe.
fn probe_paths(yaml: &str) -> BTreeSet<String> {
    const PROBE_KEYS: [&str; 3] = ["livenessProbe", "readinessProbe", "startupProbe"];
    let mut found = BTreeSet::new();
    // Indentation of the innermost open probe key.
    let mut open_at: Option<usize> = None;

    for line in yaml.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let ind = indent(line);
        let bare = trimmed.strip_prefix("- ").unwrap_or(trimmed);

        // A probe key always (re)opens a block, at any nesting depth.
        if PROBE_KEYS.iter().any(|k| bare.starts_with(k)) {
            open_at = Some(ind);
            if let Some(p) = flow_path(bare) {
                found.insert(p);
            }
            continue;
        }

        match open_at {
            // Still inside the block: deeper than the key that opened it.
            Some(open) if ind > open => {
                if let Some(p) = block_path(bare).or_else(|| flow_path(bare)) {
                    found.insert(p);
                }
            }
            // Returned to or above the key's indentation: block ended.
            Some(_) => open_at = None,
            None => {}
        }
    }
    found
}

/// `path: /health` -> `/health`, for block style.
fn block_path(trimmed: &str) -> Option<String> {
    let rest = trimmed.strip_prefix("path:")?;
    Some(unquote(rest))
}

/// `httpGet: { path: /healthz, port: 8080 }` -> `/healthz`, for flow
/// style.
fn flow_path(trimmed: &str) -> Option<String> {
    let start = trimmed.find('{')?;
    let inner = &trimmed[start..];
    // Both `{ path:` and `{path:` appear; and the space after the colon
    // has to be skipped before looking for the terminator, or the first
    // character found is always that space.
    let after = inner
        .trim_start_matches('{')
        .trim_start()
        .strip_prefix("path:")
        .map(str::trim_start)?;
    // `[char; N]` is a `Pattern`, so this is a plain array search.
    let end = after.find([',', '}', ' '])?;
    Some(unquote(&after[..end]))
}

fn unquote(s: &str) -> String {
    s.trim().trim_matches('"').trim_matches('\'').to_owned()
}

/// Extract every path literal passed to `.route("…")`.
///
/// Only string literals are recognised. A path built at runtime would be
/// invisible here, and that is acceptable: a probe path has to be a
/// literal in the manifest, so a runtime-built route cannot be the thing
/// a probe names.
fn route_paths(rust: &str) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    let mut cursor = rust;
    while let Some(idx) = cursor.find(".route(\"") {
        cursor = &cursor[idx + ".route(\"".len()..];
        let Some(end) = cursor.find('"') else { break };
        paths.insert(cursor[..end].to_owned());
        cursor = &cursor[end..];
    }
    paths
}

fn read(root: &Path, rel: &str) -> String {
    let p = root.join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("could not read {}: {e}", p.display()))
}

#[test]
fn probe_paths_match_the_routes_they_call() {
    let root = repo_root();
    let mut checked = 0;

    for (manifest, source) in SERVICES {
        let probes = probe_paths(&read(&root, manifest));
        let routes = route_paths(&read(&root, source));

        assert!(
            !routes.is_empty(),
            "{source} yielded no `.route(\"...\")` literals — the extractor \
             is broken, and every service would then look like it had no \
             routes and every assertion below would be vacuous"
        );
        assert!(
            !probes.is_empty(),
            "{manifest} declares no probe paths. If the probes were \
             removed this gate is now measuring nothing; assert that \
             deliberately rather than letting it pass silently."
        );

        for probe in &probes {
            assert!(
                routes.contains(probe),
                "{manifest} probes {probe:?}, but {source} does not serve \
                 it.\n  served by {source}: {routes:?}\n  A probe path that \
                 404s makes readiness fail forever (the pod is never added \
                 to the Service) and liveness kill the container after \
                 `initialDelaySeconds + failureThreshold * periodSeconds`.\n  \
                 Neither `cargo build` nor `kubectl apply` can catch this: \
                 the path is valid YAML and a valid HTTP path, it is just \
                 not routed."
            );
        }
        checked += probes.len();
    }

    assert!(checked > 0, "no probe paths were compared at all");
}

/// A service added to `deploy/k8s/` without a row in `SERVICES` would
/// have no probe check, and would not be noticed. This fails instead.
///
/// The Deployment `name` is used rather than the filename, because the
/// filename is a convention and the name is what a reader recognises.
#[test]
fn every_deployment_manifest_is_registered() {
    let root = repo_root();
    let dir = root.join("deploy/k8s");

    let mut deployed = BTreeSet::new();
    for entry in std::fs::read_dir(&dir).expect("deploy/k8s is readable") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(std::ffi::OsStr::to_str) != Some("yaml") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("manifest is readable");
        if !text.contains("kind: Deployment") {
            continue;
        }
        let name = deployment_name(&text).unwrap_or_else(|| {
            panic!(
                "{} declares a Deployment with no `name:` — cannot match it \
                 against SERVICES",
                path.display()
            )
        });
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        deployed.insert((name, rel));
    }

    assert!(
        !deployed.is_empty(),
        "no Deployment found in deploy/k8s — the manifest directory moved \
         and this gate is now looking at nothing"
    );

    for (name, rel) in &deployed {
        assert!(
            SERVICES.iter().any(|(m, _)| m == rel),
            "{rel} declares Deployment {name:?} but is not listed in \
             SERVICES (crates/ada-core/tests/probe_paths.rs). Its probe \
             paths are unchecked, and the ada-remediation probe bug is \
             exactly what that omission would have let through again."
        );
    }
}

/// First `name:` after `kind: Deployment`.
fn deployment_name(yaml: &str) -> Option<String> {
    let mut after_kind = false;
    for line in yaml.lines() {
        let t = line.trim();
        if t == "kind: Deployment" {
            after_kind = true;
            continue;
        }
        if after_kind {
            if let Some(rest) = t.strip_prefix("name:") {
                return Some(unquote(rest));
            }
            if t == "kind: Service" || t.starts_with("---") {
                return None;
            }
        }
    }
    None
}
