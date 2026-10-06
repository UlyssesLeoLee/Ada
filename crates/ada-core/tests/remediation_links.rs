//! A page that carries a link nobody can open is worse than no page, because
//! it looks like the runbook exists.
//!
//! ## What was wrong
//!
//! Every `page_operator` step in the shipped runbooks pointed at
//! `https://runbooks.ada.local/...`:
//!
//! ```text
//! config/remediation/service-down.json            runbook_url
//! config/remediation/slo-budget-burn-rate-fast.json  runbook_url
//! deploy/k8s/ada-remediation.yaml                runbook_url  (twice)
//! ```
//!
//! `.local` is reserved by RFC 6762 for mDNS. Nothing in this repository
//! creates that host, and a Kubernetes cluster will not answer `.local` names,
//! so every page the remediation engine sends carried a link that could not
//! resolve. The step still reported success.
//!
//! This is the same defect PR #18 removed from the Prometheus rules, where
//! three alerts carried `https://wiki.example/runbooks/...` — `example.com` is
//! reserved by RFC 2606 and never resolves either. That fix covered
//! `observability/prometheus/alerts/`. This gate covers the remediation
//! surface, which was not looked at, and generalises the rule so the next
//! reserved name is caught wherever it is written.
//!
//! `docs/observability/14-auto-remediation.md` also contains the same string,
//! but there it is quoted as an example of "the URL itself is not a secret" —
//! that is prose about a policy, not a link a pager will follow, so
//! documentation is deliberately out of scope here.
//!
//! ## What this gate holds
//!
//! No URL in a shipped runbook or manifest may name a host that cannot route:
//! an RFC-reserved TLD, or a bare hostname with no dot at all. A repository-
//! relative path (`docs/...`) is accepted, which is what the four values were
//! repointed to.

use std::fs;
use std::path::{Path, PathBuf};

/// Files whose URLs a human or a pager actually follows.
const SCANNED_DIRS: [&str; 2] = ["config/remediation", "deploy/k8s"];

/// Top-level domains reserved by RFC 2606 / 6761 / 6762. A name under any of
/// them cannot be a production endpoint by construction.
const RESERVED_TLDS: [&str; 5] = ["test", "example", "invalid", "local", "localhost"];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("ada-core lives two levels under the workspace root")
        .to_path_buf()
}

/// Every `runbook_url` value in `text`, as (host, raw_value).
///
/// Deliberately keyed on `runbook_url` and not on "any http:// string". The
/// first version of this gate scanned every URL in `deploy/k8s/` and reported
/// `http://ada-api-gateway:8080` and `http://localhost:8080` as unroutable.
/// Both are correct: the first is a Kubernetes Service DNS name that resolves
/// inside the cluster, the second is a local probe target. A gate that flags
/// the working configuration is as useless as one that flags nothing, and it
/// gets deleted on the next argument. Only the field a pager puts in front of
/// a human is in scope.
fn runbook_urls(text: &str) -> Vec<(Option<String>, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let after = match line.split_once("runbook_url") {
            Some((_, rest)) => rest,
            None => continue,
        };
        let Some((_, rest)) = after.split_once(':') else { continue };
        let raw = rest.trim().trim_end_matches(',').trim().trim_matches('"').trim_matches('\'');
        if raw.is_empty() {
            continue;
        }
        out.push((host_of(raw), raw.to_string()));
    }
    out
}

/// The host of an absolute URL, or `None` for a repository-relative path.
fn host_of(value: &str) -> Option<String> {
    for scheme in ["https://", "http://"] {
        if let Some(rest) = value.strip_prefix(scheme) {
            let end = rest
                .find(|c: char| !(c.is_alphanumeric() || c == '.' || c == '-' || c == ':'))
                .unwrap_or(rest.len());
            return Some(rest[..end].to_string());
        }
    }
    None
}

/// Why this host cannot be reached, or `None` when it looks routable.
fn unroutable_reason(host: &str) -> Option<String> {
    let bare = host.split(':').next().unwrap_or(host).to_ascii_lowercase();
    if bare.is_empty() {
        return Some("empty host".to_string());
    }
    let last = bare.rsplit('.').next().unwrap_or("");
    if RESERVED_TLDS.contains(&last) {
        return Some(format!("`{last}` is an RFC-reserved TLD and never routes"));
    }
    if !bare.contains('.') {
        return Some(format!("`{bare}` has no dot: a bare hostname only resolves inside one network"));
    }
    None
}

#[test]
fn shipped_runbook_urls_name_routable_hosts() {
    let root = repo_root();
    let mut checked = 0usize;
    let mut from_runbooks = 0usize;
    let mut bad: Vec<String> = Vec::new();

    for dir in SCANNED_DIRS {
        let path = root.join(dir);
        let entries = fs::read_dir(&path).unwrap_or_else(|e| panic!("read_dir {}: {e}", path.display()));
        for entry in entries.flatten() {
            let p = entry.path();
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
            let is_scanned = name.ends_with(".json") || name.ends_with(".yaml") || name.ends_with(".yml");
            if !p.is_file() || !is_scanned {
                continue;
            }
            let rel = p.strip_prefix(&root).unwrap_or(&p).display().to_string();
            let text = fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", rel));
            for (host, raw) in runbook_urls(&text) {
                checked += 1;
                if dir == "config/remediation" {
                    from_runbooks += 1;
                }
                // A repository-relative path is the intended replacement and
                // has no host to judge.
                let Some(host) = host else { continue };
                if let Some(why) = unroutable_reason(&host) {
                    bad.push(format!("{rel}: runbook_url={raw} -- {why}"));
                }
            }
        }
    }

    assert!(
        from_runbooks > 0,
        "no runbook_url found in config/remediation/ -- the scanner matched nothing there"
    );
    assert!(checked > 0, "scanned no runbook_url at all");
    assert!(
        bad.is_empty(),
        "shipped runbook URLs point at unroutable hosts:\n  {}\n\n\
         These are followed by whoever receives the page. Point them at a path \
         that exists in this repository (docs/...) or at a host that resolves.",
        bad.join("\n  ")
    );
}