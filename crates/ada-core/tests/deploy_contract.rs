//! A manifest states facts about its environment as strings, and nothing
//! checks them.
//!
//! ## The two correspondences
//!
//! **A path the container reads.** `ADA_RBAC_POLICY_DIR` and
//! `GM_CONSOLE_STATIC_DIR` are absolute paths. Each one names a directory
//! that only exists because some `COPY` in the corresponding Dockerfile put
//! it there, and each one is *also* baked into that image as an `ENV`. The
//! manifest states the same fact a second time, independently, and CI never
//! exercises its copy of it.
//!
//! The `images` job comes close. It builds each image and needs the health
//! route reachable, and the gateway refuses to bind without its policy
//! files, so a missing policy does fail the job. But the job does not set
//! `ADA_RBAC_POLICY_DIR`: it runs the image's own `ENV`. Change the manifest
//! to a directory nothing copies into and CI stays green, while every real
//! deployment exits at startup.
//!
//! **A host the container connects to.** `ADA_SESSION_REDIS_URL` and
//! `GM_CONSOLE_UPSTREAM` name a `host:port`. The host must be a Service that
//! exists, exposes that port, and -- the part that is easy to miss -- lives
//! in the *same* namespace as the pod resolving it. A bare `svc:port` is
//! namespace-local; it does not cross a namespace boundary. Moving
//! `ada-session-redis` next to `ada-remediation` in `observability` breaks
//! both references with no diff to any manifest that still looks correct.
//!
//! ## Severity
//!
//! Both of these fail closed. The gateway refuses to serve without a
//! credential check, the console serves errors, and neither degrades into
//! silently wrong behaviour the way the runbook mount did. They are
//! deployment-time-only failures -- nothing local reproduces them -- which
//! is what makes them worth a gate rather than a code review convention.
//!
//! ## Method
//!
//! Line-and-indent scanning, matching `kustomization.rs`, `probe_paths.rs`
//! and `mounted_configmaps.rs`: the workspace has no YAML parser in
//! `Cargo.lock`. Both `deploy/k8s` and `deploy/infra` are read, because a
//! working reference topology is both applied together and the gateway's
//! session store lives in the second.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

const K8S_DIR: &str = "deploy/k8s";
const INFRA_DIR: &str = "deploy/infra";
const DOCKER_DIR: &str = "deploy/docker";
const DEFAULT_NAMESPACE: &str = "default";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("ada-core lives two levels under the workspace root")
        .to_path_buf()
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn scalar(value: &str) -> String {
    value
        .split('#')
        .next()
        .unwrap_or_default()
        .trim()
        .trim_matches(['"', '\''])
        .to_string()
}

fn has_content(lines: &[&str]) -> bool {
    lines
        .iter()
        .any(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
}

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

fn find_key(lines: &[&str], from: usize, key: &str, indent: usize) -> Option<usize> {
    (from..lines.len()).find(|&i| {
        let t = lines[i].trim();
        !t.is_empty() && !t.starts_with('#') && indent_of(lines[i]) == indent && t == key
    })
}

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

fn namespace_of(doc: &[&str], top: usize) -> String {
    // Inside the `metadata:` block, at its own indent. Looking for
    // `namespace:` at the top-level indent misses every manifest, because
    // it is written two spaces in under `metadata:`.
    let Some(meta) = find_key(doc, 0, "metadata:", top) else {
        return DEFAULT_NAMESPACE.to_string();
    };
    for i in block_after(doc, meta) {
        if let Some(rest) = doc[i].trim().strip_prefix("namespace:") {
            let ns = scalar(rest);
            if !ns.is_empty() {
                return ns;
            }
        }
    }
    DEFAULT_NAMESPACE.to_string()
}

fn name_of(doc: &[&str], top: usize) -> Option<String> {
    let meta = find_key(doc, 0, "metadata:", top)?;
    for i in block_after(doc, meta) {
        if let Some(rest) = doc[i].trim().strip_prefix("name:") {
            let n = scalar(rest);
            if !n.is_empty() {
                return Some(n);
            }
        }
    }
    None
}

/// Every YAML document under both deploy directories, as
/// (relative path, document).
/// Every YAML file under both deploy directories, as (relative path, text).
///
/// Returning the raw text rather than pre-split documents is what keeps the
/// borrows sound: `documents()` hands back `&str` slices into its argument,
/// so a function returning them would be returning references into a local
/// `String` that is dropped on return. Callers split, so the text outlives
/// the slices.
fn deploy_files() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for dir in [K8S_DIR, INFRA_DIR] {
        let path = repo_root().join(dir);
        if !path.is_dir() {
            continue;
        }
        let mut files: Vec<PathBuf> = fs::read_dir(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                p.extension().is_some_and(|x| {
                    x.eq_ignore_ascii_case("yaml") || x.eq_ignore_ascii_case("yml")
                })
            })
            .collect();
        files.sort();
        for f in files {
            let rel = f
                .strip_prefix(repo_root())
                .unwrap_or(&f)
                .to_string_lossy()
                .replace('\\', "/");
            let text = fs::read_to_string(&f).expect("read manifest");
            out.push((rel, text));
        }
    }
    out
}

/// Every `Deployment` under both deploy directories, with the literal env
/// values it hands its containers.
///
/// Each manifest is split inside the loop rather than by a helper that
/// returned the documents: `documents()` lends `&str` slices into its
/// argument, so a function returning those slices would be returning
/// references into a `String` that has already been dropped. CI's first run
/// of this file reported exactly that as a missing lifetime specifier.
fn workloads() -> Vec<Workload> {
    let mut out = Vec::new();
    for (file, text) in deploy_files() {
        for doc in documents(&text) {
            let top = top_indent(&doc);
            if find_key(&doc, 0, "kind: Deployment", top).is_none() {
                continue;
            }
            let Some(name) = name_of(&doc, top) else {
                continue;
            };
            out.push(Workload {
                file: file.clone(),
                namespace: namespace_of(&doc, top),
                name,
                env: env_values(&doc),
            });
        }
    }
    out
}

/// A workload: its file, namespace, name, and literal env values.
struct Workload {
    file: String,
    namespace: String,
    name: String,
    env: Vec<(String, String)>,
}

/// Literal `value:` entries of every `env:` list in a document.
///
/// Only literal values are collected. `valueFrom` entries reference a
/// `ConfigMap`, a `Secret` or the downward API, none of which name a path
/// the image is responsible for.
fn env_values(doc: &[&str]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for i in 0..doc.len() {
        if doc[i].trim() != "env:" {
            continue;
        }
        let body = block_after(doc, i);
        let item_indent = body
            .clone()
            .find(|&j| doc[j].trim_start().starts_with("- ") && !doc[j].trim().is_empty())
            .map(|j| indent_of(doc[j]));
        let Some(item_indent) = item_indent else {
            continue;
        };
        // Only the current entry's name. An entry with `valueFrom` instead
        // of `value` simply never produces a pair, rather than producing one
        // with an empty value that the callers would have to filter out.
        let mut pending_name: Option<String> = None;
        for j in body {
            let t = doc[j].trim();
            if t.is_empty() || t.starts_with('#') {
                continue;
            }
            let ind = indent_of(doc[j]);
            if ind == item_indent {
                let rest = t.trim_start_matches("- ").trim();
                pending_name = rest
                    .strip_prefix("name:")
                    .map(scalar)
                    .filter(|n| !n.is_empty());
                continue;
            }
            // A `value:` directly under the entry, not nested inside the
            // `valueFrom:` block that follows it.
            if ind == item_indent + 2 {
                if let Some(rest) = t.strip_prefix("value:") {
                    let v = scalar(rest);
                    if !v.is_empty() {
                        if let Some(n) = pending_name.take() {
                            out.push((n, v));
                        }
                    }
                }
            }
        }
    }
    out
}

/// `ENV KEY=VALUE` from a Dockerfile, joining `\` continuation lines.
///
/// A single-line regex reads only the first pair of a multi-line `ENV`
/// block, and then reports the rest as absent from the image -- which looks
/// exactly like the defect this gate looks for.
fn docker_envs(text: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    // `take` rather than `as_mut`: appending to the buffer through a
    // borrow while also assigning a new buffer in the same match is a
    // borrowing conflict, and this file cannot be compiled locally.
    let mut pending: Option<String> = None;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(existing) = pending.take() {
            pending = Some(format!("{existing} {line}"));
        } else if let Some(rest) = line.strip_prefix("ENV ") {
            pending = Some(rest.to_string());
        } else {
            continue;
        }
        if line.ends_with('\\') {
            continue;
        }
        let Some(joined) = pending.take() else {
            continue;
        };
        for token in joined.replace('\\', " ").split_whitespace() {
            if let Some((k, v)) = token.split_once('=') {
                out.insert(k.trim().to_string(), v.trim().trim_matches('"').to_string());
            }
        }
    }
    out
}

/// Destination of every `COPY` in a Dockerfile.
///
/// `COPY --from=stage a b` has three tokens; taking the second as the
/// destination reports the source as the destination.
fn docker_copy_dests(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|raw| {
            let line = raw.trim();
            line.strip_prefix("COPY ").map(|rest| {
                rest.split_whitespace()
                    .rfind(|t| !t.starts_with("--"))
                    .unwrap_or_default()
                    .to_string()
            })
        })
        .collect()
}

/// Lines of a Dockerfile with `\` continuations joined.
///
/// The uid is created by a `RUN` that wraps across lines, so reading it one
/// physical line at a time finds a flag on one line and its value nowhere.
/// The backslash itself is dropped while joining: keeping it leaves `--uid`
/// followed by `\`, and the digit scan that reads the value stops at a
/// character that is not a digit and reports the flag as absent.
fn logical_lines(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut buf = String::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if buf.is_empty() {
            buf = line.trim_end_matches('\\').to_string();
        } else {
            buf.push(' ');
            buf.push_str(line.trim_end_matches('\\'));
        }
        if line.ends_with('\\') {
            continue;
        }
        out.push(std::mem::take(&mut buf));
    }
    if !buf.is_empty() {
        out.push(buf);
    }
    out
}

/// The first value of `flag` in a Dockerfile, as written.
fn docker_flag(text: &str, flag: &str) -> Option<String> {
    logical_lines(text).into_iter().find_map(|line| {
        let idx = line.find(flag)?;
        let rest = line[idx + flag.len()..].trim_start();
        let token: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if token.is_empty() {
            None
        } else {
            Some(token)
        }
    })
}

/// `runAsUser:` / `runAsGroup:` from a Deployment's pod security context.
fn run_as(doc: &[&str], key: &str) -> Option<String> {
    let top = top_indent(doc);
    let spec_at = find_key(doc, 0, "spec:", top)?;
    let pod = {
        let spec = block_after(doc, spec_at);
        doc[spec.clone()]
            .iter()
            .position(|l| l.trim() == "template:")
            .map_or(0, |p| p + spec.start)
    };
    let template = block_after(doc, pod);
    let inner = doc[template.clone()]
        .iter()
        .position(|l| l.trim() == "spec:")
        .map_or(0, |p| p + template.start);
    let pod_spec = block_after(doc, inner);
    let sec = doc[pod_spec.clone()]
        .iter()
        .position(|l| l.trim() == "securityContext:")
        .map_or(0, |p| p + pod_spec.start);
    let sec_indent = indent_of(doc[sec]);
    // An explicit loop rather than `doc[block_after(..)].into_iter()`: that
    // indexes a slice to a place of unsized type, and asking such a place for
    // an iterator is not something the compiler resolves the way the same
    // expression over an array would be. A file that cannot be compiled
    // locally is not the place to be clever.
    for j in block_after(doc, sec) {
        if indent_of(doc[j]) == sec_indent + 2 && doc[j].trim_start().starts_with(key) {
            return Some(scalar(
                doc[j].trim_start().strip_prefix(key).unwrap_or_default(),
            ));
        }
    }
    None
}

fn dockerfile_for(workload: &str) -> Option<(String, String)> {
    let path = repo_root()
        .join(DOCKER_DIR)
        .join(format!("{workload}.Dockerfile"));
    if !path.is_file() {
        return None;
    }
    let text = fs::read_to_string(&path).expect("read Dockerfile");
    let rel = path
        .strip_prefix(repo_root())
        .unwrap_or(&path)
        .to_string_lossy()
        .replace('\\', "/");
    Some((rel, text))
}

/// (namespace, name) -> ports, for every `Service` under `deploy/`.
fn services() -> BTreeMap<(String, String), Vec<String>> {
    let mut out = BTreeMap::new();
    for (_file, text) in deploy_files() {
        for doc in documents(&text) {
            let top = top_indent(&doc);
            if find_key(&doc, 0, "kind: Service", top).is_none() {
                continue;
            }
            let Some(name) = name_of(&doc, top) else {
                continue;
            };
            let spec_at = find_key(&doc, 0, "spec:", top);
            let ports = spec_at
                .map(|s| {
                    block_after(&doc, s)
                        .filter(|&j| {
                            // `port: 6379`, as a mapping entry or as a
                            // sequence item. Matching the bare key `port:`
                            // found none of them and every Service came out
                            // exposing no ports at all.
                            doc[j]
                                .trim_start()
                                .trim_start_matches("- ")
                                .starts_with("port:")
                        })
                        .map(|j| {
                            let rest = doc[j].trim().trim_start_matches("- ");
                            scalar(rest.strip_prefix("port:").unwrap_or_default())
                        })
                        .filter(|p| !p.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            out.insert((namespace_of(&doc, top), name), ports);
        }
    }
    out
}

/// `host:port` from a value that looks like a URL, if it has both.
fn host_and_port(value: &str) -> Option<(String, String)> {
    let (_scheme, rest) = value.split_once("://")?;
    let authority = rest.split('/').next().unwrap_or(rest);
    if authority.is_empty() {
        return None;
    }
    let (host, port) = authority.rsplit_once(':')?;
    if port.is_empty() || !port.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some((host.to_string(), port.to_string()))
}

/// A literal address like `0.0.0.0:8080` is a bind address, not a
/// reference to another workload.
fn is_bind_address(host: &str) -> bool {
    host == "0.0.0.0"
        || host == "127.0.0.1"
        || host == "::"
        || host == "[::]"
        || host.parse::<std::net::Ipv4Addr>().is_ok()
}

/// Every absolute path a manifest tells a container to read must be a
/// directory its image actually populates, and must match the value the
/// image bakes for the same variable.
#[test]
fn manifest_env_paths_match_the_image_and_its_copy_destinations() {
    let workloads = workloads();
    let mut checked = 0usize;
    let mut problems: Vec<String> = Vec::new();

    for w in &workloads {
        let Some((docker_rel, docker_text)) = dockerfile_for(&w.name) else {
            continue;
        };
        let baked = docker_envs(&docker_text);
        let dests = docker_copy_dests(&docker_text);

        for (var, value) in &w.env {
            if !value.starts_with('/') {
                continue;
            }
            checked += 1;
            match baked.get(var.as_str()) {
                Some(image_value) if image_value == value => {}
                Some(image_value) => problems.push(format!(
                    "{}: {var}={value}, but {docker_rel} bakes {var}={image_value}. \
                     The manifest's value wins at runtime, and nothing runs the \
                     image's default, so the two must agree.",
                    w.file
                )),
                None => problems.push(format!(
                    "{}: {var}={value} is not baked into {docker_rel}. The value \
                     has a single source, the manifest, and no image-side copy \
                     of it can catch the two drifting apart.",
                    w.file
                )),
            }
            if !dests
                .iter()
                .any(|d| d.trim_end_matches('/') == value.trim_end_matches('/'))
            {
                problems.push(format!(
                    "{}: {var}={value} names a directory, but no COPY in \
                     {docker_rel} writes there. The container starts and reads \
                     an absent path.",
                    w.file
                ));
            }
        }
    }

    assert!(
        checked > 0,
        "no absolute-path env value was checked. Either the manifests stopped \
         naming paths or this gate stopped reading them, and either way it is \
         currently measuring nothing."
    );
    assert!(
        problems.is_empty(),
        "{} absolute path(s) a manifest asks for an image to provide, and the \
         image does not agree:\n         {}\n\
         These are deployment-time failures: the image builds, CI builds it \
         too, and the job passes because the container runs the image's own \
         ENV rather than the manifest's. The break only appears when someone \
         deploys.",
        problems.len(),
        problems.join("\n         ")
    );
}

/// Every `host:port` a manifest hands a container must name a `Service` in
/// the same namespace that exposes that port.
#[test]
fn manifest_network_references_resolve_within_their_own_namespace() {
    let workloads = workloads();
    let svc = services();
    let mut checked = 0usize;
    let mut problems: Vec<String> = Vec::new();

    for w in &workloads {
        for (var, value) in &w.env {
            let Some((host, port)) = host_and_port(value) else {
                continue;
            };
            if is_bind_address(&host) {
                continue;
            }
            checked += 1;
            let key = (w.namespace.clone(), host.clone());
            let Some(ports) = svc.get(&key) else {
                problems.push(format!(
                    "{}: {var}={value} names Service `{host}` in namespace \
                     `{}`, which no manifest under deploy/ defines",
                    w.file, w.namespace
                ));
                continue;
            };
            if !ports.iter().any(|p| p == &port) {
                problems.push(format!(
                    "{}: {var}={value} uses port {port}, but Service `{host}` \
                     exposes {ports:?}",
                    w.file
                ));
            }
        }
    }

    assert!(
        checked > 0,
        "no host:port env value was checked. Either the manifests stopped \
         naming services or this gate stopped reading them, and either way it \
         is currently measuring nothing."
    );
    assert!(
        problems.is_empty(),
        "{} network reference(s) in a manifest do not resolve:\n         {}\n\
         A bare `svc:port` is namespace-local. Moving a Service into another \
         namespace breaks every reference to it without changing any manifest \
         that still reads as correct, and the only symptom is a connection \
         failure in a cluster.",
        problems.len(),
        problems.join("\n         ")
    );
}

/// Anti-vacuity for the Service index the second gate depends on.
#[test]
fn the_service_index_spans_both_deploy_directories() {
    let svc = services();
    let namespaces: BTreeSet<&str> = svc.keys().map(|(ns, _)| ns.as_str()).collect();
    assert!(
        svc.len() >= 4,
        "found only {} Service(s) under deploy/, expected at least four \
         (gateway, remediation, console, session store). A shrinking index \
         would make the network-reference gate pass by finding nothing.",
        svc.len()
    );
    assert!(
        namespaces.contains(DEFAULT_NAMESPACE),
        "no Service found in the default namespace; got {namespaces:?}. \
         `ada-session-redis` lives in deploy/infra and is loaded on the same \
         basis as deploy/k8s, so a scan that missed it would report a real \
         Service as undefined."
    );
}

/// The uid a manifest pins must be the uid its image creates.
///
/// `runAsUser: 65532` in a manifest and `--uid 65532` in a Dockerfile are
/// the same fact stated twice, and the two are written at different times
/// by different edits. When they agree, the pod starts as an unprivileged
/// uid with a writable home. When they do not, every replica fails to start
/// with a `runAsNonRoot` error, and the CI image job cannot see it: it runs
/// the container without the manifest's `securityContext`.
///
/// `ada-session-redis` is skipped because it has no Dockerfile. It is an
/// upstream image, the same distinction `deploy_images.rs` already draws
/// when it refuses `redis:7-alpine` in `deploy/k8s`.
#[test]
fn manifest_run_as_uid_matches_the_user_its_image_creates() {
    let mut checked = 0usize;
    let mut problems: Vec<String> = Vec::new();

    for (file, text) in deploy_files() {
        for doc in documents(&text) {
            let top = top_indent(&doc);
            if find_key(&doc, 0, "kind: Deployment", top).is_none() {
                continue;
            }
            let Some(name) = name_of(&doc, top) else {
                continue;
            };
            let Some((docker_rel, docker_text)) = dockerfile_for(&name) else {
                continue;
            };
            let pairs = [("runAsUser:", "--uid"), ("runAsGroup:", "--gid")];
            for (manifest_key, docker_flag_name) in pairs {
                let (Some(manifest_uid), Some(image_uid)) = (
                    run_as(&doc, manifest_key),
                    docker_flag(&docker_text, docker_flag_name),
                ) else {
                    continue;
                };
                checked += 1;
                if manifest_uid != image_uid {
                    problems.push(format!(
                        "{file}: {manifest_key} {manifest_uid}, but {docker_rel} \
                         creates the user with `{docker_flag_name} {image_uid}`. \
                         The kubelet refuses to start the container when the \
                         pinned uid is not the one the image has, and no CI job \
                         applies this manifest's security context."
                    ));
                }
            }
        }
    }

    assert!(
        checked >= 6,
        "only {checked} uid comparison(s) were made; there are three images in \
         this repository, each pinning a user and a group. A count this low \
         means the manifest or the Dockerfile parser stopped finding them, \
         and the comparison would be silently vacuous."
    );
    assert!(
        problems.is_empty(),
        "{} uid(s) a manifest pins disagree with the image it deploys:\n         {}\n\
         A pod cannot start as a uid its image does not have, so this is a \
         total outage for that workload rather than a degraded one.",
        problems.len(),
        problems.join("\n         ")
    );
}
