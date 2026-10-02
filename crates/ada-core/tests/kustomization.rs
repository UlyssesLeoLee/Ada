//! `deploy/k8s/kustomization.yaml` must be one document, free of
//! duplicate keys, and must not use a selector-injecting transformer.
//!
//! ## The defects this gate exists for
//!
//! **The apply command had never worked.** The file opened with `---`
//! and then had a *second* `---` after the comment block. That makes
//! the first YAML document empty, and kustomize reads the first
//! document, so `kubectl kustomize deploy/k8s/` -- the command the
//! README and the file's own header document -- reported
//! `error: kustomization.yaml is empty`. A multi-document YAML file is
//! perfectly legal, so no linter complains.
//!
//! **A duplicate key silently destroyed per-service labelling.** One
//! `labels` entry had `app.kubernetes.io/component: remediator` and
//! `app.kubernetes.io/component: gm-console` in the same `pairs` map.
//! That is invalid YAML; a strict loader rejects the file, and a
//! lenient one keeps a single value and applies it to everything.
//!
//! **`includeSelectors: true` rewrote a `NetworkPolicy`'s scope.** The
//! transformer overwrote `app.kubernetes.io/name: ada-remediation`
//! with `ada-platform` in the `NetworkPolicy`'s `podSelector`, and
//! every pod in the directory carries `ada-platform`. Rendering before
//! the fix:
//!
//! ```text
//! NetworkPolicy ada-remediation
//!   podSelector: {app.kubernetes.io/name: ada-platform, ...}
//! ```
//!
//! A policy written to restrict ingress to the remediation webhook
//! would have selected gm-console and ada-api-gateway as well. This is
//! invisible in the source manifests, in any YAML lint, and in
//! `kubectl apply -f`; it exists only in the rendered output, which is
//! what actually gets applied.
//!
//! ## What is NOT checked here, and why
//!
//! "Does it render at all" is not checked, because that needs
//! `kubectl`. The `rust` CI job has it (ubuntu-latest ships it), so
//! `.github/workflows/ci.yml` runs `kubectl kustomize deploy/k8s/` as
//! its own step. Everything checked here needs no external tool, so it
//! runs on every `cargo test` -- including on a contributor's machine
//! with no cluster and no kubectl.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("ada-core lives two levels under the workspace root")
        .to_path_buf()
}

fn kustomization() -> String {
    let p = repo_root().join("deploy/k8s/kustomization.yaml");
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("could not read {}: {e}", p.display()))
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// A leading `---` starts an empty first document, and kustomize reads
/// the first one. A single separator at the very top is fine and is the
/// convention here; a second one is what breaks it.
#[test]
fn the_kustomization_is_a_single_document() {
    let yaml = kustomization();
    let seps = yaml
        .lines()
        .filter(|l| matches!(l.trim(), "---" | "..."))
        .count();
    assert_eq!(
        seps, 1,
        "deploy/k8s/kustomization.yaml has {seps} `---` separators, expected 1.\n\
         A leading `---` plus a comment block plus a second `---` leaves the \
         first YAML document empty, and kustomize reads the first document, so \
         `kubectl kustomize deploy/k8s/` fails with \
         `error: kustomization.yaml is empty` and the documented apply command \
         has never worked. Keep the comment block after the single leading \
         `---` and do not add another."
    );
}

/// The keys of every `pairs:` mapping in the file, grouped per mapping.
///
/// A repeated key inside one mapping is invalid YAML. The specific
/// damage is that `app.kubernetes.io/component` was listed twice with
/// two different values in a single `pairs`, so the per-service
/// distinction could not survive whichever value the parser kept.
#[test]
fn no_pairs_mapping_contains_a_duplicate_key() {
    let yaml = kustomization();

    // (pairs_indent, keys_seen_in_this_mapping)
    let mut blocks: Vec<(usize, Vec<String>)> = Vec::new();
    let mut current: Option<(usize, Vec<String>)> = None;
    let mut duplicate: Option<String> = None;

    for line in yaml.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let ind = indent_of(line);

        if trimmed.starts_with("pairs:") {
            if let Some(done) = current.take() {
                blocks.push(done);
            }
            current = Some((ind, Vec::new()));
            continue;
        }

        let Some((pairs_indent, keys)) = current.as_mut() else {
            continue;
        };
        // Dedented out of the mapping.
        if ind <= *pairs_indent {
            blocks.push(current.take().expect("just checked"));
            continue;
        }
        if let Some((key, _)) = trimmed.split_once(':') {
            let key = key.trim().to_owned();
            if keys.contains(&key) {
                duplicate.get_or_insert_with(|| key.clone());
            } else {
                keys.push(key);
            }
        }
    }
    if let Some(done) = current {
        blocks.push(done);
    }

    assert!(
        !blocks.is_empty(),
        "found no `pairs:` mapping in deploy/k8s/kustomization.yaml -- this \
         gate is measuring nothing and would pass on any file"
    );
    assert!(
        duplicate.is_none(),
        "deploy/k8s/kustomization.yaml: `{}` appears twice in one `pairs` map.\n\
         A repeated key in one mapping is invalid YAML: a strict loader rejects \
         the file, and a lenient one keeps one value and stamps it on every \
         resource in the directory. That is why the per-service \
         `app.kubernetes.io/component` distinction was gone rather than merely \
         misspelled. Per-service labels belong in each service's own manifest.",
        duplicate.unwrap_or_default()
    );
}

/// A label transformer that injects into selectors will rewrite
/// `spec.selector` and a `NetworkPolicy`'s `podSelector` as well as
/// labels, and it *overwrites* values rather than adding to them. With
/// more than one service in the kustomization that silently changes
/// which pods a policy covers.
#[test]
fn the_label_transformer_does_not_inject_into_selectors() {
    let yaml = kustomization();
    let services = yaml
        .lines()
        .filter(|l| l.trim_start().starts_with("- ") && l.contains(".yaml"))
        .count();
    assert!(
        services > 1,
        "expected several resources, found {services}. This gate's reasoning \
         assumes a shared transformer spanning more than one service, and would \
         need re-examining if that changed."
    );

    for (i, line) in yaml.lines().enumerate() {
        // The setting appears as a YAML sequence item, so the line is
        // `- includeSelectors: true`. Comparing the raw trimmed line
        // would never match, and the gate would pass on exactly the
        // input it exists to reject — found by mutating the file back to
        // the broken state and watching this test stay green.
        let value = line.trim().strip_prefix("- ").unwrap_or(line.trim()).trim();
        assert_ne!(
            value,
            "includeSelectors: true",
            "deploy/k8s/kustomization.yaml:{} sets `includeSelectors: true` with \
             {services} services in one kustomization.\n\
             That makes kustomize rewrite `spec.selector` and NetworkPolicy \
             `podSelector` as well as labels, overwriting what it finds. It is \
             what turned the ada-remediation NetworkPolicy's \
             `app.kubernetes.io/name: ada-remediation` into `ada-platform` in \
             the rendered output -- a label every pod carries, so a policy \
             scoped to one service would cover all of them. Only a render \
             reveals that, which is why it is checked here and the render itself \
             is checked in CI.",
            i + 1
        );
    }
}

/// Per-service values must not live in a shared transformer, because
/// kustomize overwrites each resource's own value with the shared one.
#[test]
fn the_shared_transformer_holds_no_per_service_keys() {
    let yaml = kustomization();
    for banned in [
        "app.kubernetes.io/name",
        "app.kubernetes.io/version",
        "app.kubernetes.io/component",
    ] {
        let present = yaml
            .lines()
            .any(|l| l.trim().starts_with(&format!("{banned}:")));
        assert!(
            !present,
            "deploy/k8s/kustomization.yaml puts `{banned}` in the shared `pairs` \
             map. It is a per-service value: kustomize overwrites the resource's \
             own value with the shared one, which is how ada-remediation's \
             `app.kubernetes.io/version: v0.7.1` became `v0.1.0` in 25 places of \
             the rendered output. Set it in each service's own manifest."
        );
    }
}
