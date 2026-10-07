//! Every image this repository publishes is tagged with the workspace version.
//!
//! ## What this catches
//!
//! Three manifests in `deploy/k8s/` each name an image and a tag, twice over
//! -- once in the manifest itself so `kubectl apply -f <file>` works without
//! the kustomization, and once in the `images:` override block. Six places,
//! three services.
//!
//! One of those services was stamped `v0.7.1` in all of its places while the
//! workspace it is built from has been `0.1.0` since the beginning, and the
//! other two services were stamped `v0.1.0`. Nothing compared them. A stale
//! tag is not a cosmetic difference: the tag is what a node pulls, so it is
//! the difference between a running pod and an `ImagePullBackOff`.
//!
//! The repository publishes nothing today -- the `images` CI job builds each
//! service and deliberately does not push -- so nothing in this repository
//! can catch a tag that names no buildable thing. That gap is documented in
//! `deploy/k8s/README.md` and is not what this test is about. This test is
//! about the manifests agreeing with the crate they deploy, which is checkable
//! with no network and no registry.
//!
//! ## Scope
//!
//! Only images in this project's own registry namespace are checked. A
//! third-party image — a proxy, a metrics exporter, a base image pulled by a
//! sidecar — is pinned to its upstream's version by definition and is out of
//! scope by construction rather than by an exemption list that the next real
//! defect can hide in.
//!
//! ## Reading the manifest, not re-implementing it
//!
//! The tags are read out of the YAML as text: `image:` lines and `newTag:`
//! lines, each anchored on its key. This deliberately does not parse YAML and
//! re-derive the image list, because a second parser that disagrees with
//! `kustomize build` is a second source of truth for the same fact.

use std::fs;
use std::path::{Path, PathBuf};

/// The namespace this repository publishes under.
const OWN_REGISTRY_PREFIX: &str = "ghcr.io/ulyssesleolee/";

const MANIFEST_DIR: &str = "deploy/k8s";

/// Six references exist today: three manifests and three `newTag` overrides.
/// A floor well under that, so a walk that stops finding them is red rather
/// than silently clean.
const MIN_REFERENCES: usize = 4;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("ada-core lives two levels under the workspace root")
        .to_path_buf()
}

/// The version the workspace builds, read out of `[workspace.package]`.
///
/// Crates that say `version.workspace = true` inherit this, so it is the
/// number a manifest must agree with. Parsed as text because the whole file
/// is a fixed shape and a TOML parser is not available to a std-only test.
fn workspace_version() -> String {
    let text = fs::read_to_string(repo_root().join("Cargo.toml")).expect("workspace Cargo.toml");
    let mut in_section = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_section = trimmed == "[workspace.package]";
            continue;
        }
        if in_section {
            if let Some(rest) = trimmed.strip_prefix("version") {
                let rest = rest.trim_start();
                if let Some(value) = rest.strip_prefix('=') {
                    return value.trim().trim_matches('"').to_string();
                }
            }
        }
    }
    panic!("no version under [workspace.package] in Cargo.toml; the parse is broken, not the file");
}

/// Image references declared as `image: <ref>` or `newTag: <ref>`.
///
/// Returns `(reference, value)` so the failure message can name the key that
/// carried it.
fn image_references(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("image:") {
            let value = rest.trim();
            if value.starts_with(OWN_REGISTRY_PREFIX) {
                out.push(("image".to_string(), value.to_string()));
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("newTag:") {
            let value = rest.trim();
            if !value.is_empty() {
                out.push(("newTag".to_string(), value.to_string()));
            }
        }
    }
    out
}

#[test]
fn every_published_image_is_tagged_with_the_workspace_version() {
    let root = repo_root();
    let version = workspace_version();
    let expected = format!("v{version}");

    let dir = root.join(MANIFEST_DIR);
    let entries = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{MANIFEST_DIR} must be readable, and it is not: {e}"));
    let mut manifests: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("yaml"))
        })
        .collect();
    manifests.sort();
    assert!(
        manifests.len() >= 3,
        "only {} manifests found under {MANIFEST_DIR}",
        manifests.len()
    );

    let mut checked = 0usize;
    let mut wrong: Vec<String> = Vec::new();

    for path in &manifests {
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .display()
            .to_string();
        let text = fs::read_to_string(path).expect("manifest is readable");
        for (key, value) in image_references(&text) {
            let tag = value.rsplit(':').next().unwrap_or_default();
            checked += 1;
            if tag != expected {
                wrong.push(format!(
                    "{rel}: {key} {value} is tagged {tag}, not {expected}"
                ));
            }
        }
    }

    assert!(
        checked >= MIN_REFERENCES,
        "only {checked} image references found under {MANIFEST_DIR}; the walk has \
         stopped finding references rather than finding them all correct"
    );
    assert!(
        wrong.is_empty(),
        "{} of {checked} image references are not tagged {expected}, which is the \
         version this workspace builds. A manifest that names a tag no build \
         produces applies a pod the node cannot pull:\n  {}",
        wrong.len(),
        wrong.join("\n  ")
    );
}

/// The `images:` override block has to cover every service the manifests
/// name, or retargeting one environment silently leaves another behind.
///
/// `deploy_images.rs` already holds the manifests against the kustomization.
/// This test holds it against the *list*, which is the part that was stale:
/// a manifest drifted to a tag nothing else knew about while the override
/// block still agreed with its neighbours.
#[test]
fn the_override_block_retargets_every_service() {
    let root = repo_root();
    let dir = root.join(MANIFEST_DIR);

    let mut declared: BTreeServices = BTreeServices::default();
    for entry in fs::read_dir(&dir).expect("manifest dir").flatten() {
        let path = entry.path();
        let is_yaml = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("yaml"));
        if !is_yaml {
            continue;
        }
        let text = fs::read_to_string(&path).expect("manifest is readable");
        for line in text.lines() {
            let trimmed = line.trim();
            let Some(rest) = trimmed.strip_prefix("image:") else {
                continue;
            };
            let value = rest.trim();
            if !value.starts_with(OWN_REGISTRY_PREFIX) {
                continue;
            }
            // The name is the segment BEFORE the tag. `rsplit` yields from
            // the right, so asking it for the first element returns `v0.1.0`
            // and reports a service called after a version.
            let name = value
                .trim_start_matches(OWN_REGISTRY_PREFIX)
                .split(':')
                .next()
                .unwrap_or_default()
                .to_string();
            declared.insert(name);
        }
    }

    let kustomization = fs::read_to_string(dir.join("kustomization.yaml")).expect("kustomization");
    for service in &declared {
        assert!(
            kustomization.contains(&format!("- name: {service}")),
            "{service} is deployed from a manifest but the kustomization `images:` \
             block does not retarget it, so `kustomize edit set image` cannot move it"
        );
    }
    assert!(
        declared.len() >= 3,
        "only {} services declared; the manifest walk has narrowed",
        declared.len()
    );
}

type BTreeServices = std::collections::BTreeSet<String>;
