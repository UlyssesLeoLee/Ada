//! Every image reference in `deploy/k8s/` must agree on one registry and
//! one tag format, and be declared in the kustomization's `images:` block.
//!
//! ## The defects this gate exists for
//!
//! The three services named their images three different ways:
//!
//! ```text
//! ghcr.io/ada-project/ada-api-gateway:0.1.0
//! ghcr.io/ulysse/ada-remediation:v0.7.1      <- a second registry
//! ghcr.io/ada-project/gm-console:0.1.0       <- and no `v` prefix
//! ```
//!
//! Nothing flags this. Every line is a syntactically valid image
//! reference, `kubectl apply` accepts all of them, and each manifest
//! reads fine in isolation. The split is only visible if you collect all
//! three and compare — which is exactly what nobody does. The cost is
//! concrete: two registries means two publish targets, two credential
//! sets, and a Deployment whose pull succeeds or fails depending on which
//! one a given node can reach.
//!
//! The tag format mattered on its own too. `0.1.0` and `v0.7.1` cannot be
//! compared by any tool that expects semver, and `kustomize edit set
//! image` round-trips the string.
//!
//! ## Why the manifests keep the full reference
//!
//! An obvious alternative is to strip the registry from the manifests and
//! let `images:` supply it, so there is literally one place naming the
//! registry. That was rejected: `kubectl apply -f <file>` bypasses the
//! kustomization entirely, and a bare `ada-remediation:v0.7.1` then
//! resolves against the node's default registry — a *silently* wrong
//! image rather than an obviously missing one. Keeping the full reference
//! in the manifest keeps the direct-apply path correct, and the
//! `images:` block stays an override point on top of it. The
//! `declared_in_the_kustomization` gate is what keeps the two from
//! drifting apart.
//!
//! ## What is NOT checked here, and why
//!
//! Whether the images exist, or can be built, is not checked. There is no
//! Dockerfile in this repository and no CI job that builds or pushes one,
//! so all three references currently resolve to nothing and every pod
//! lands in `ImagePullBackOff`. That is a real and separate defect; a
//! gate that asserted it would be a gate that fails on every commit
//! until someone writes three Dockerfiles, which trains people to ignore
//! it. It is documented in `deploy/k8s/README.md` instead.
//!
//! Everything checked here needs no external tool and no network, so it
//! runs on every `cargo test`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("ada-core lives two levels under the workspace root")
        .to_path_buf()
}

fn deploy_dir() -> PathBuf {
    repo_root().join("deploy/k8s")
}

fn kustomization() -> String {
    let p = deploy_dir().join("kustomization.yaml");
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("could not read {}: {e}", p.display()))
}

/// Every `image:` value in every manifest under `deploy/k8s`, as
/// `(file, reference)`.
fn manifest_images() -> Vec<(String, String)> {
    let mut dir_entries: Vec<PathBuf> = std::fs::read_dir(deploy_dir())
        .unwrap_or_else(|e| panic!("could not read {}: {e}", deploy_dir().display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "yaml" || x == "yml"))
        .collect();
    dir_entries.sort();

    let mut out = Vec::new();
    for path in dir_entries {
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("could not read {}: {e}", path.display()));
        for line in text.lines() {
            let t = line.trim();
            if t.starts_with('#') {
                continue;
            }
            // `imagePullPolicy:` does not match: the prefix includes the
            // colon, so the trailing `Policy` breaks it.
            if let Some(rest) = t.strip_prefix("image:") {
                let value = rest.trim();
                if !value.is_empty() {
                    out.push((
                        path.file_name().unwrap().to_string_lossy().into_owned(),
                        value.to_string(),
                    ));
                }
            }
        }
    }
    out
}

/// The `images:` block as `(name, newName, newTag)` triples.
fn declared_images() -> Vec<(String, String, String)> {
    let yaml = kustomization();
    let mut out: Vec<(String, String, String)> = Vec::new();
    let mut current: Option<(String, String, String)> = None;
    let mut in_block = false;

    for line in yaml.lines() {
        if !in_block {
            if line == "images:" {
                in_block = true;
            }
            continue;
        }
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if !line.starts_with(' ') && !line.starts_with('\t') {
            // Dedented back to column 0: the block is over. Anything else
            // would be a sibling top-level key, and `resources:` / `labels:`
            // come first, so stopping here is what keeps those out.
            break;
        }
        let t = t.strip_prefix("- ").unwrap_or(t);
        let Some((key, value)) = t.split_once(':') else {
            continue;
        };
        let value = value.trim().to_string();
        match key.trim() {
            "name" => {
                if let Some(prev) = current.take() {
                    out.push(prev);
                }
                current = Some((value, String::new(), String::new()));
            }
            "newName" => {
                if let Some(c) = current.as_mut() {
                    c.1 = value;
                }
            }
            "newTag" => {
                if let Some(c) = current.as_mut() {
                    c.2 = value;
                }
            }
            _ => {}
        }
    }
    if let Some(c) = current.take() {
        out.push(c);
    }
    out
}

/// `ghcr.io/ada-project/ada-remediation:v0.7.1`
///   -> ("ghcr.io/ada-project", "ada-remediation", "v0.7.1")
fn split_reference(reference: &str) -> (String, String, String) {
    let (path_part, tag) = match reference.rsplit_once(':') {
        Some((p, t)) if !t.contains('/') => (p, t.to_string()),
        _ => (reference, String::new()),
    };
    let (registry, name) = match path_part.rsplit_once('/') {
        Some((r, n)) => (r.to_string(), n.to_string()),
        None => (String::new(), path_part.to_string()),
    };
    (registry, name, tag)
}

/// The single registry every image in the directory resolves to.
#[test]
fn every_image_resolves_to_the_same_registry() {
    let images = manifest_images();
    assert!(
        !images.is_empty(),
        "no `image:` found under {} — if the manifests moved, point this gate at them",
        deploy_dir().display()
    );

    let mut registries: BTreeSet<String> = BTreeSet::new();
    for (file, reference) in &images {
        let (registry, _, _) = split_reference(reference);
        assert!(
            !registry.is_empty(),
            "{file} has an image with no registry: `{reference}`. \
             A bare name resolves against the node's default registry, so the \
             same manifest pulls different content on different clusters."
        );
        registries.insert(registry.clone());
        println!("  {file:<24} {registry}/...");
    }

    assert_eq!(
        registries.len(),
        1,
        "deploy/k8s references {} different registries: {:?}.\n\
         Pick one, make all three manifests agree, and publish all three \
         images there. Two registries in one directory means two publish \
         targets, two credential sets, and a Deployment that pulls from \
         whichever one the node happens to be able to reach.",
        registries.len(),
        registries
    );
}

/// Every tag is `vMAJOR.MINOR.PATCH`, so semver tooling can compare them.
#[test]
fn every_image_tag_uses_the_same_v_prefixed_form() {
    let images = manifest_images();
    for (file, reference) in &images {
        let (_, name, tag) = split_reference(reference);
        assert!(
            !tag.is_empty(),
            "{file}: image `{reference}` has no tag, so it resolves to \
             `:latest` and can change under a running Deployment"
        );
        let rest = tag.strip_prefix('v').unwrap_or(&tag);
        let numeric = rest
            .split('.')
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()));
        assert!(
            numeric && tag.starts_with('v'),
            "{file}: image tag `{tag}` is not `vMAJOR.MINOR.PATCH`.\n\
             A mixed `0.1.0` / `v0.7.1` convention cannot be compared by any \
             semver tool, and `kustomize edit set image` round-trips the \
             string verbatim, so the inconsistency survives every retag."
        );
        let _ = name;
    }
}

/// Each manifest image is declared in the kustomization, so one edit
/// retargets all of them.
#[test]
fn every_manifest_image_is_declared_in_the_kustomization() {
    let declared: BTreeSet<String> = declared_images()
        .into_iter()
        .map(|(name, _, _)| name)
        .collect();
    assert!(
        !declared.is_empty(),
        "the `images:` block in kustomization.yaml is empty or missing, so \
         there is no single place to retarget the images"
    );

    for (file, reference) in manifest_images() {
        let (_, name, _) = split_reference(&reference);
        assert!(
            declared.contains(&name),
            "{file} uses image `{name}` but kustomization.yaml does not \
             declare it. Images that are not declared cannot be retargeted \
             together, which is the whole reason the block exists."
        );
    }
}

/// The reverse direction: a declared image nobody uses is a copy-paste
/// leftover that will drift.
#[test]
fn every_declared_image_is_actually_referenced() {
    let used: BTreeSet<String> = manifest_images()
        .into_iter()
        .map(|(_, reference)| split_reference(&reference).1)
        .collect();

    for (name, new_name, tag) in declared_images() {
        assert!(
            used.contains(&name),
            "kustomization.yaml declares image `{name}` ({new_name}:{tag}) \
             but no manifest references it. A stale entry reads as coverage \
             when it is not."
        );
    }
}

/// The `images:` block must point at the same registry the manifests do,
/// or overriding through kustomize silently retargets to somewhere else.
#[test]
fn the_kustomization_images_block_agrees_with_the_manifests() {
    let manifest_registries: BTreeSet<String> = manifest_images()
        .into_iter()
        .map(|(_, r)| split_reference(&r).0)
        .collect();
    let expected = manifest_registries
        .iter()
        .next()
        .expect("at least one manifest image, asserted in the registry test")
        .clone();

    for (name, new_name, tag) in declared_images() {
        // `new_name` carries no tag of its own, so split it for the
        // registry and image name only -- taking a "tag" from it yields an
        // empty string every time. The tag to check is the one the parser
        // already pulled off the `newTag:` line.
        let (declared_registry, declared_image, _) = split_reference(&new_name);
        assert_eq!(
            declared_registry, expected,
            "kustomization.yaml maps `{name}` to `{new_name}`, whose registry \
             is `{declared_registry}`, but the manifests use `{expected}`. \
             Applying the override would retarget this one image somewhere else."
        );
        assert_eq!(
            declared_image, name,
            "kustomization.yaml entry for `{name}` sets newName \
             `{declared_image}`, so the image name changes on override"
        );
        assert!(
            tag.starts_with('v'),
            "kustomization.yaml entry for `{name}` has newTag `{tag}`, \
             which is not v-prefixed like the manifests"
        );

        // The declared tag is the one kustomize actually substitutes, so
        // letting it drift from the manifest's tag means the tag a reader
        // sees in the manifest and the tag that gets pulled are different
        // -- and the manifests are what people grep.
        for (file, reference) in manifest_images() {
            let (m_registry, m_name, m_tag) = split_reference(&reference);
            if m_name == name {
                assert_eq!(
                    m_registry, expected,
                    "{file} image `{reference}` is not on the declared registry"
                );
                assert_eq!(
                    m_tag, tag,
                    "{file} pins `{m_tag}` but kustomization.yaml declares \
                     newTag `{tag}` for `{name}`, so the kustomize override \
                     silently substitutes a different tag than the manifest shows"
                );
            }
        }
    }
}
