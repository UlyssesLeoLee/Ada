//! Every `ConfigMap` a workload mounts must exist, must not be `optional`,
//! and must carry the data the repository says it carries.
//!
//! ## The defect this gate exists for
//!
//! `deploy/k8s/ada-remediation.yaml` mounted a `ConfigMap` named
//! `ada-remediation-runbooks` and never defined it, with `optional: true`
//! on the volume. Kubernetes creates the mount point regardless, so
//! `/etc/ada-remediation/runbooks` existed and was empty.
//!
//! Nothing reported it. `load_runbooks_from_dir` returns
//! `Ok(Vec::new())` for a directory that does not exist and
//! `Ok(Vec::new())` for one that is empty, so the service started, logged
//! `loaded runbooks from disk count=0` at **info**, passed both probes,
//! and answered a correctly signed webhook with a success -- having
//! matched zero actions. The whole point of the deployment is to execute
//! runbooks, and it was a silent no-op that looked healthy to every probe
//! and to the metric scrape.
//!
//! ## Why defining it was not the whole fix
//!
//! Defining the `ConfigMap` closes the hole that was open on the day it was
//! found. It does not close the hole the *next* day, when someone adds a
//! sixth runbook to `config/remediation/` and deploys. That runbook would
//! exist in the repository, be absent from the manifest, and be silently
//! unmatched -- the identical failure, arrived at from the other direction,
//! and this time looking like a working deployment the whole way.
//!
//! So there are two gates here, and the second is the load-bearing one:
//!
//! 1. `every_mounted_configmap_is_defined_in_the_kustomization` -- the
//!    mount resolves to something.
//! 2. `every_runbook_on_disk_is_mounted_by_the_deployment` -- the thing it
//!    resolves to is the runbooks in this repository, byte for byte.
//!
//! ## Why `optional: true` is reported as a failure of its own
//!
//! `optional: true` is the correct spelling of "this `ConfigMap` may not
//! exist" -- which is exactly why it is also the perfect way to make a
//! missing `ConfigMap` invisible. The pod starts, nothing in the event
//! stream says a volume is empty, and `kubectl get` shows a healthy pod.
//!
//! Now that the `ConfigMap` is defined with real content, the only thing
//! the flag still does is turn a `kubectl delete`, a partial apply, or a
//! namespace mistake into a service that starts and remediates nothing.
//! An operator who genuinely wants different runbooks overrides the
//! manifest -- that is what editing a checked-in file is for -- rather than
//! relying on the mount quietly resolving to nothing.
//!
//! The `Secret` right below the runbooks needs no such treatment: it *is*
//! defined, with `PLACEHOLDER_*` values, so deleting it is an event rather
//! than a silent degradation. That is the pattern this gate points at.
//!
//! ## Scope and method
//!
//! This scans `deploy/k8s` only. `deploy/infra/ada-session-redis.yaml` is
//! outside it and declares no `ConfigMap` volume.
//!
//! The manifests are read with the same line-and-indent scanner the
//! existing `kustomization.rs`, `probe_paths.rs` and `deploy_images.rs`
//! gates use, because the workspace has no YAML parser in `Cargo.lock` and
//! adding one to a gate would be a worse trade than matching the local
//! style. The scanner walks each `volumes:` block by indentation and takes
//! **every** sequence entry in it. An earlier version of this file instead
//! required a `- name:` line to be *immediately* preceded by `volumes:`,
//! which only ever matched the first entry in the block -- so the first
//! volume was checked and the rest were invisible, which is the same class
//! of hole the gate exists to close.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

const DEPLOY_DIR: &str = "deploy/k8s";
const RUNBOOKS_DIR: &str = "config/remediation";
const RUNBOOKS_CONFIGMAP: &str = "ada-remediation-runbooks";

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

/// Strip a trailing comment and surrounding quotes from a scalar value.
fn scalar(value: &str) -> String {
    value
        .split('#')
        .next()
        .unwrap_or_default()
        .trim()
        .trim_matches(['"', '\''])
        .to_string()
}

/// Every manifest under `deploy/k8s`, as (filename, text).
fn manifests() -> Vec<(String, String)> {
    let dir = repo_root().join(DEPLOY_DIR);
    let mut out: Vec<(String, String)> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "yaml" || x == "yml"))
        .filter_map(|p| {
            let name = p.file_name()?.to_str()?.to_string();
            Some((name, fs::read_to_string(&p).expect("read manifest")))
        })
        .collect();
    out.sort();
    out
}

/// Split a YAML stream into documents on `---`, dropping empty ones.
///
/// The manifests are multi-document files, and the leading `---` that opens
/// `ada-remediation.yaml` produces an empty first document. Keeping it would
/// contribute a nameless entry to every name set built from these files.
fn documents(text: &str) -> Vec<Vec<&str>> {
    let mut docs: Vec<Vec<&str>> = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for line in text.lines() {
        if line.trim() == "---" {
            if has_content(&current) {
                docs.push(std::mem::take(&mut current));
            } else {
                current.clear();
            }
            continue;
        }
        current.push(line);
    }
    if has_content(&current) {
        docs.push(current);
    }
    docs
}

fn has_content(lines: &[&str]) -> bool {
    lines
        .iter()
        .any(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
}

/// The indentation every top-level key of a document shares.
fn top_indent(doc: &[&str]) -> usize {
    doc.iter()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with('#')
        })
        .map(|l| indent_of(l))
        .min()
        .unwrap_or(0)
}

/// Index of the first line at or after `from` whose trimmed text is exactly
/// `key` at indentation `indent`.
fn find_key(lines: &[&str], from: usize, key: &str, indent: usize) -> Option<usize> {
    (from..lines.len()).find(|&i| {
        let t = lines[i].trim();
        !t.is_empty() && !t.starts_with('#') && indent_of(lines[i]) == indent && t == key
    })
}

/// The lines belonging to the mapping opened at `i`: everything more deeply
/// indented, up to the next line that is not. Blank lines and comments are
/// carried along, because they sit inside blocks too.
fn block_after(lines: &[&str], i: usize) -> Range<usize> {
    let open = indent_of(lines[i]);
    let mut end = i + 1;
    while end < lines.len() {
        let t = lines[end].trim();
        if !t.is_empty() && !t.starts_with('#') && indent_of(lines[end]) <= open {
            break;
        }
        end += 1;
    }
    i + 1..end
}

/// The `metadata.name` of the document, given the index of its `kind:` line.
fn document_name(doc: &[&str], kind_at: usize) -> Option<String> {
    let top = indent_of(doc[kind_at]);
    let meta = find_key(doc, kind_at, "metadata:", top)?;
    for i in block_after(doc, meta) {
        if let Some(rest) = doc[i].trim().strip_prefix("name:") {
            let name = scalar(rest);
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    None
}

/// Names of every `ConfigMap` defined under `deploy/k8s`.
fn defined_configmaps() -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for (_, text) in manifests() {
        for doc in documents(&text) {
            let top = top_indent(&doc);
            let Some(kind_at) = find_key(&doc, 0, "kind: ConfigMap", top) else {
                continue;
            };
            if let Some(name) = document_name(&doc, kind_at) {
                names.insert(name);
            }
        }
    }
    names
}

/// A `configMap:` volume found under `deploy/k8s`.
#[derive(Debug)]
struct Mounted {
    file: String,
    volume: String,
    config_map: String,
    optional: bool,
}

/// Every volume entry under `deploy/k8s`: its name, and the line range of
/// the entry.
fn volume_entries(doc: &[&str], volumes_at: usize) -> Vec<(String, Range<usize>)> {
    let body = block_after(doc, volumes_at);
    let body_end = body.end;

    // Sequence items in one block share one indent; learn it from the first
    // rather than assuming two, so a reformatted manifest still parses.
    let item_indent = body
        .clone()
        .find(|&j| doc[j].trim_start().starts_with("- ") && !doc[j].trim().is_empty())
        .map(|j| indent_of(doc[j]));
    let Some(item_indent) = item_indent else {
        return Vec::new();
    };

    let mut starts: Vec<usize> = Vec::new();
    let mut names: Vec<String> = Vec::new();
    for j in body {
        let t = doc[j].trim();
        if t.is_empty() || t.starts_with('#') || !t.starts_with("- ") {
            continue;
        }
        if indent_of(doc[j]) != item_indent {
            // A `- ` nested deeper belongs to the current entry's own value
            // (a list under `projected:`, say), not to a new volume.
            continue;
        }
        let rest = t.trim_start_matches("- ").trim();
        starts.push(j);
        names.push(scalar(rest.strip_prefix("name:").unwrap_or(rest)));
    }

    names
        .into_iter()
        .zip(starts.iter().copied().enumerate())
        .map(|(name, (idx, start))| {
            let end = starts.get(idx + 1).copied().unwrap_or(body_end);
            (name, start..end)
        })
        .collect()
}

/// The `configMap:` a volume entry mounts, and whether it is `optional`.
///
/// `configMap:` counts only at the entry's own child indent. One nested
/// deeper -- under `projected.sources`, say -- is not the volume's source
/// and must not be mistaken for one.
fn configmap_source(doc: &[&str], range: &Range<usize>) -> Option<(String, bool)> {
    let child_indent = range
        .clone()
        .map(|j| doc[j])
        .find(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with('#') && !t.starts_with("- ")
        })
        .map(indent_of)?;

    for j in range.clone() {
        if indent_of(doc[j]) != child_indent {
            continue;
        }
        let Some(rest) = doc[j].trim().strip_prefix("configMap:") else {
            continue;
        };
        assert!(
            rest.trim().is_empty(),
            "{}: `configMap: {{...}}` on one line is not parsed by this gate. \
             Spread it over multiple lines, which is what every other volume \
             in this directory does, so the scan cannot quietly skip it.",
            j + 1
        );
        let mut name = String::new();
        let mut optional = false;
        for k in block_after(doc, j) {
            let kt = doc[k].trim();
            if let Some(r) = kt.strip_prefix("name:") {
                name = scalar(r);
            } else if kt == "optional: true" {
                optional = true;
            }
        }
        return Some((name, optional));
    }
    None
}

/// Every `ConfigMap` volume under `deploy/k8s`, and the total number of
/// volume entries found alongside them.
fn mounted_configmaps() -> (Vec<Mounted>, usize) {
    let mut mounted = Vec::new();
    let mut total = 0usize;
    for (file, text) in manifests() {
        for doc in documents(&text) {
            for i in 0..doc.len() {
                if doc[i].trim() != "volumes:" {
                    continue;
                }
                for (volume, range) in volume_entries(&doc, i) {
                    total += 1;
                    if let Some((config_map, optional)) = configmap_source(&doc, &range) {
                        mounted.push(Mounted {
                            file: file.clone(),
                            volume,
                            config_map,
                            optional,
                        });
                    }
                }
            }
        }
    }
    (mounted, total)
}

/// The lines belonging to the literal block scalar whose key is at `key_line`.
///
/// Unlike [`block_after`], a comment at or above the key's own indent ends
/// the scalar. YAML block scalars end at the first non-empty line that is
/// not more indented than the key, and a comment at that level is a comment
/// rather than content. Carrying it in -- which is right for the structural
/// walker, where comments sit inside mappings -- made the reconstruction
/// panic on a perfectly legal manifest the first time anyone annotated a key
/// under `data:`.
fn scalar_block_after(lines: &[&str], i: usize) -> Range<usize> {
    let open = indent_of(lines[i]);
    let mut end = i + 1;
    while end < lines.len() {
        if !lines[end].trim().is_empty() && indent_of(lines[end]) <= open {
            break;
        }
        end += 1;
    }
    i + 1..end
}

/// The value of the literal block scalar whose key is at `key_line`.
///
/// `indicator` is the text following `key:` on that line. Block indentation
/// is taken from the first non-blank line of the body, which is the rule
/// YAML itself uses, so this does not assume a two-space step.
///
/// Trailing blank lines are dropped before the chomping indicator is
/// applied, because they are separators between keys rather than content.
/// The manifests separate runbooks with a blank line, so keeping them would
/// append a spurious newline to every value and make every comparison fail.
fn block_scalar(doc: &[&str], key_line: usize, indicator: &str) -> String {
    let mut body: Vec<usize> = scalar_block_after(doc, key_line).collect();
    while body.last().is_some_and(|&j| doc[j].trim().is_empty()) {
        body.pop();
    }

    let indent = body
        .iter()
        .find(|&&j| !doc[j].trim().is_empty())
        .map_or(0, |&j| indent_of(doc[j]));

    let mut value = String::new();
    for &j in &body {
        let line = doc[j];
        if line.trim().is_empty() {
            value.push('\n');
            continue;
        }
        let stripped = line.strip_prefix(&" ".repeat(indent)).unwrap_or_else(|| {
            panic!(
                "{}: a literal block scalar indented less than {indent} spaces; \
                 this gate will not guess the value",
                j + 1
            )
        });
        value.push_str(stripped);
        value.push('\n');
    }

    // Clip keeps the final line break, strip removes it.
    match indicator {
        "|" => {}
        "|-" => {
            value.pop();
        }
        other => panic!(
            "line {}: block scalar indicator `{other}` is not supported by this \
             gate; use `|` or `|-` so the deployed bytes are predictable",
            key_line + 1
        ),
    }
    value
}

/// The `data` of a `ConfigMap` defined under `deploy/k8s`, as key -> value.
fn configmap_data(wanted: &str) -> BTreeMap<String, String> {
    let mut data = BTreeMap::new();
    for (_, text) in manifests() {
        for doc in documents(&text) {
            let top = top_indent(&doc);
            let Some(kind_at) = find_key(&doc, 0, "kind: ConfigMap", top) else {
                continue;
            };
            if document_name(&doc, kind_at).as_deref() != Some(wanted) {
                continue;
            }
            let Some(data_at) = find_key(&doc, kind_at, "data:", top) else {
                continue;
            };
            let Some(child_indent) = block_after(&doc, data_at)
                .map(|j| doc[j])
                .find(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
                .map(indent_of)
            else {
                continue;
            };
            for j in block_after(&doc, data_at) {
                if indent_of(doc[j]) != child_indent {
                    continue;
                }
                let t = doc[j].trim();
                let Some((key, indicator)) = t.split_once(':') else {
                    continue;
                };
                data.insert(scalar(key), block_scalar(&doc, j, indicator.trim()));
            }
        }
    }
    data
}

/// Every `*.json` runbook in the repository, as filename -> contents.
fn runbooks_on_disk() -> BTreeMap<String, String> {
    let dir = repo_root().join(RUNBOOKS_DIR);
    let mut out = BTreeMap::new();
    for entry in fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .filter_map(Result::ok)
    {
        let path = entry.path();
        // A subdirectory named `something.json` would satisfy the extension
        // test and then fail `read_to_string` with a panic about a runbook.
        // The directory is documented as flat, but the check is one line.
        if !entry.file_type().is_ok_and(|t| t.is_file()) {
            continue;
        }
        // `extension()` rather than `name.ends_with(".json")`: the workspace
        // denies warnings and clippy flags the case-sensitive form. Testing
        // the real extension is also the more accurate question -- a file
        // called `x.JSON` is a runbook on a case-insensitive filesystem and
        // would be skipped by the string form on a case-sensitive one.
        if !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
        {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        out.insert(
            name.to_string(),
            fs::read_to_string(&path).expect("read runbook"),
        );
    }
    out
}

/// Every mounted `ConfigMap` resolves to one this directory defines.
///
/// The `optional: true` case is asserted separately, because defining the
/// `ConfigMap` silences the first check while leaving the hazard in place:
/// `optional` still means a later delete or a partial apply degrades the pod
/// instead of failing it.
#[test]
fn every_mounted_configmap_is_defined_in_the_kustomization() {
    let (mounted, total) = mounted_configmaps();
    let defined = defined_configmaps();

    // Anti-vacuity, as facts about the repository rather than a guess. The
    // named-volume check is the strong one: it fails outright if the
    // `volumes:` walk stopped finding entries, which is the failure mode of
    // the predecessor scanner that only ever saw the first entry.
    assert!(
        defined.contains("ada-remediation-config"),
        "no ConfigMap named ada-remediation-config was found under {DEPLOY_DIR} \
         (parsed: {defined:?}); this gate is no longer reading the manifests"
    );
    assert!(
        total >= 4,
        "found {total} volume entries under {DEPLOY_DIR}; there are four \
         (three `tmp` emptyDirs and one runbook mount). A count this low means \
         the `volumes:` scan is missing entries, and every assertion below \
         would pass while a ConfigMap volume went unchecked"
    );
    assert!(
        mounted.iter().any(|m| m.file == "ada-remediation.yaml"
            && m.volume == "runbooks"
            && m.config_map == RUNBOOKS_CONFIGMAP),
        "the runbooks volume in ada-remediation.yaml was not found; it mounts \
         `{RUNBOOKS_CONFIGMAP}` and is the volume this gate exists for. \
         Scanned: {mounted:#?}"
    );

    let missing: Vec<String> = mounted
        .iter()
        .filter(|m| !defined.contains(&m.config_map))
        .map(|m| {
            let optional = if m.optional {
                " (marked `optional: true`, so the pod starts with an empty mount)"
            } else {
                ""
            };
            format!(
                "{}: volume `{}` mounts ConfigMap `{}`{}, which no manifest in \
                 {DEPLOY_DIR} defines",
                m.file, m.volume, m.config_map, optional
            )
        })
        .collect();

    assert!(
        missing.is_empty(),
        "these ConfigMap volumes resolve to nothing: {}\n\
         The pod will start, the mount point will exist and be empty, and \
         nothing in the API surface or the event stream will say so. A \
         service that reads an empty directory this way serves its probes \
         and does nothing: `load_runbooks_from_dir` returns an empty Vec \
         rather than an error, so the only trace is a `count=0` log line at \
         info level. Define the ConfigMap, or mount a volume that carries \
         the data.",
        missing.join("\n         ")
    );

    let optional: Vec<String> = mounted
        .iter()
        .filter(|m| m.optional)
        .map(|m| {
            format!(
                "{}: volume `{}` -> ConfigMap `{}` is `optional: true`",
                m.file, m.volume, m.config_map
            )
        })
        .collect();

    assert!(
        optional.is_empty(),
        "a ConfigMap volume marked `optional: true` cannot fail loudly: {}\n\
         Remove the flag. `optional` is the correct spelling of \"this \
         ConfigMap may not exist\", and that is exactly why it is the perfect \
         way to make a missing ConfigMap invisible. The Secret next to it \
         shows the pattern to use instead: define it with `PLACEHOLDER_*` \
         values, so deleting it is an event rather than a silent degradation.",
        optional.join("\n         ")
    );
}

/// Every runbook in the repository is in the deployment, and matches it.
///
/// This is the gate that stops the defect from coming back through the
/// other door. Adding a runbook to `config/remediation/` without adding it
/// to the `ConfigMap` leaves a deployment that starts, passes its probes and
/// silently matches nothing for that trigger -- indistinguishable, from
/// every probe and every metric, from the state this file was written
/// against.
#[test]
fn every_runbook_on_disk_is_mounted_by_the_deployment() {
    let disk = runbooks_on_disk();
    let deployed = configmap_data(RUNBOOKS_CONFIGMAP);

    assert!(
        !disk.is_empty(),
        "no *.json runbooks found in {RUNBOOKS_DIR}; the gate is measuring \
         nothing. `ada-remediation` reads its runbooks from that directory, \
         so an empty one means the loader has nothing to load"
    );
    assert!(
        !deployed.is_empty(),
        "ConfigMap `{RUNBOOKS_CONFIGMAP}` has no readable `data`; the gate is \
         measuring nothing. Either the ConfigMap is gone or its values are \
         written in a form this gate refuses to parse"
    );

    let absent: Vec<&String> = disk.keys().filter(|k| !deployed.contains_key(*k)).collect();
    assert!(
        absent.is_empty(),
        "{} runbook(s) exist in {RUNBOOKS_DIR} but are not keys of ConfigMap \
         `{RUNBOOKS_CONFIGMAP}`: {}\n\
         The Deployment mounts that ConfigMap at the directory \
         `REMEDIATION_RUNBOOK_DIR` points to, so a runbook that is not a key \
         is a runbook the deployed service never loads -- it starts, passes \
         its probes, and matches zero actions for that trigger. Add the \
         runbook as a literal block scalar under `data` in that ConfigMap.",
        absent.len(),
        absent
            .iter()
            .map(|k| k.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    let drifted: Vec<String> = disk
        .iter()
        .filter(|(k, _)| deployed.contains_key(*k))
        .filter(|(k, v)| deployed[*k] != **v)
        .map(|(k, v)| {
            format!(
                "{k}: on disk {}B, in ConfigMap {}B",
                v.len(),
                deployed[k].len()
            )
        })
        .collect();
    assert!(
        drifted.is_empty(),
        "these runbooks differ between {RUNBOOKS_DIR} and ConfigMap \
         `{RUNBOOKS_CONFIGMAP}`: {}\n\
         The ConfigMap is generated from the directory, so a difference means \
         it was edited by hand after generation, or regenerated from an older \
         tree. What runs in the cluster is the ConfigMap, and what operators \
         review and edit is the file, so a difference is an alert rule that \
         matches on the wrong trigger.",
        drifted.join("; ")
    );

    let orphans: Vec<&String> = deployed.keys().filter(|k| !disk.contains_key(*k)).collect();
    assert!(
        orphans.is_empty(),
        "ConfigMap `{RUNBOOKS_CONFIGMAP}` carries {} key(s) that no longer \
         exist in {RUNBOOKS_DIR}: {}\n\
         A runbook in the manifest but not in the repository is not \
         version-controlled, not reviewable, and will survive the next \
         regeneration.",
        orphans.len(),
        orphans
            .iter()
            .map(|k| k.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
}
