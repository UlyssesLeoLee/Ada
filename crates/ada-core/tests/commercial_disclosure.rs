//! `SUB_PROCESSORS.md` is a disclosure to customers and to regulators. It
//! named five processors this repository does not integrate and omitted
//! three that its code can reach.
//!
//! ## What was wrong
//!
//! The list read: Cloudflare, AWS, GitHub, Sentry, Postmark. Searching
//! `crates/*/src`, `apps/*/lib` and `deploy/`, none of those five is
//! wired up — no dependency, no configuration, no code path. Its own DRAFT
//! banner said so, and justified itself with a claim that was also false:
//! "Cloudflare appears only as a hostname in a CORS allow-list". The CORS
//! allow-list is `gm-console.kanvas.dev`, `staging.gm-console.kanvas.dev`
//! and `http://localhost:8080` (`crates/gm-console/src/config.rs`,
//! `deploy/k8s/gm-console.yaml`). The one line a reader would have checked
//! cited an artifact that does not exist.
//!
//! Meanwhile three processors the tree *can* reach were absent:
//!
//! * `Stripe` — `DEFAULT_STRIPE_BASE_URL = "https://api.stripe.com/v1"`,
//!   a hard-coded constant in `crates/ada-billing/src/config.rs`.
//! * `PagerDuty` — `crates/ada-remediation/src/executor.rs` POSTs to
//!   `https://events.pagerduty.com/v2/enqueue`.
//! * `Slack` — `notify_slack` steps POST to `SLACK_WEBHOOK_URL`; three of
//!   the five runbooks in `config/remediation/` carry one.
//!
//! `docs/commercial/google-play-data-safety.json` then restated the five
//! placeholder rows to Google Play as the answer to "who do you share with",
//! in the same file whose "Crash logs" scope said no crash-reporting SDK
//! ships. That is the shape that reaches a regulator.
//!
//! ## What these gates hold
//!
//! Two directions, because either one alone is trivially satisfiable:
//!
//! * Every third-party host that shipped code can reach is named in the
//!   **integrated** table. Adding `https://api.example-incident.com/` to
//!   the remediation executor without declaring it fails.
//! * Every row in the **integrated** table appears in the tree, and every
//!   row in the **not-integrated** table does not. A placeholder row that
//!   someone later wires up is stale in the safe direction: the document
//!   keeps telling a customer their data does not go somewhere it now goes.
//!
//! Both tables are delimited by `<!-- gate:integrated -->` /
//! `<!-- end -->` markers. Renaming a processor is therefore a deliberate
//! two-line change rather than an edit that silently empties a table.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const DISCLOSURE: &str = "docs/commercial/SUB_PROCESSORS.md";

/// Roots scanned for both host literals and processor names. Code and
/// manifests only — prose is deliberately excluded, so a deployment *plan*
/// in a README cannot make a processor look integrated, and cannot make an
/// unintegrated one look integrated either.
const CODE_ROOTS: &[&str] = &["crates", "deploy", "config"];

/// `apps/` is Flutter source with no third-party endpoints, but the mobile
/// client's default API base is an own-domain https URL, so it is scanned
/// separately to keep the "own domain" exclusion honest.
const APP_ROOTS: &[&str] = &["apps"];

/// Domains that are either ours or provably not a sub-processor.
///
/// Each entry earns its place. An unexplained exclusion list would let the
/// next real processor walk straight through the gate, so every entry says
/// what it is.
const NOT_A_PROCESSOR: &[(&str, &str)] = &[
    ("kanvas.dev", "our own reserved domain: hosts, CORS allow-list, legal pages"),
    (
        "github.com",
        "source hosting and a LICENSE link rendered into a response; no Customer Data call. GitHub is still listed in the not-integrated table because that is a fact about where the source lives.",
    ),
    ("crates.io", "crate registry, build-time only"),
    ("rust-lang.org", "toolchain documentation, doc comments only"),
    ("rfc-editor.org", "IETF RFC text, doc comments only"),
    ("mozilla.org", "publicsuffix list, test fixture only"),
    ("jaegertracing.io", "upstream project documentation, doc comments only"),
    ("opentelemetry.io", "collector configuration, doc comments only"),
];

/// Reserved for documentation, tests and the local machine. RFC 2606 and
/// RFC 6761, plus the loopback names. Nothing here can receive data.
const RESERVED: &[&str] = &[
    "localhost",
    "127.0.0.1",
    "0.0.0.0",
    "::1",
    "example.com",
    "example.org",
    "example.net",
    "example.test",
    "example.invalid",
    // RFC 6761 reserves the whole `.invalid` TLD. The JWT and OIDC fixtures
    // in crates/ada-identity use `issuer.invalid`, `api.invalid` and
    // `attacker.invalid` precisely because none of them can resolve.
    "invalid",
    "evil.example",
    "bad.example",
    "wiki.example",
    "runbooks.ada.local",
    "control-plane.local",
    "ulysses-star.local",
    "mock-cluster.local",
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("ada-core lives two levels under the workspace root")
        .to_path_buf()
}

fn source_files() -> Vec<PathBuf> {
    let root = repo_root();
    let mut out = Vec::new();
    for dir in CODE_ROOTS.iter().chain(APP_ROOTS.iter()) {
        collect(&root.join(dir), &mut out);
    }
    out.sort();
    out
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // `crates/*/tests/` holds gates, not product code. This file is
            // one of them, and its own module comment names Cloudflare, AWS,
            // Sentry and Postmark while explaining that none is integrated.
            // Without this the gate reads its own explanation as evidence
            // that they are.
            if path.file_name().is_some_and(|n| n == "tests") {
                continue;
            }
            collect(&path, out);
            continue;
        }
        let is_source = matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("rs" | "dart")
        );
        let is_manifest = matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("yml" | "yaml" | "json")
        );
        if is_source || is_manifest {
            out.push(path);
        }
    }
}

/// The part of a line that is code rather than commentary.
///
/// Comments are where plans live. `deploy/k8s/kustomization.yaml` names
/// "external-secrets-operator backed by AWS/GCP/Vault" in a comment, and a
/// Rust doc comment is where a future endpoint gets written down before it
/// is built. Treating either as an integration is how a placeholder row
/// starts looking true.
fn strip_comment(line: &str) -> &str {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") || trimmed.starts_with('#') {
        return "";
    }
    // A trailing `//` is a comment unless the `//` is part of a URL inside a
    // string literal, which is always preceded by `:`. Cutting there would
    // turn `"https://api.stripe.com/v1"` into `"https:` and hide the host.
    let bytes = line.as_bytes();
    let mut i = 0usize;
    while i + 1 < bytes.len() {
        if bytes[i] == b'/' && bytes[i + 1] == b'/' && i > 0 && bytes[i - 1] != b':' {
            return &line[..i];
        }
        i += 1;
    }
    line
}

fn corpus() -> Vec<(PathBuf, String)> {
    source_files()
        .into_iter()
        .filter_map(|p| {
            fs::read_to_string(&p).ok().map(|t| {
                let code = t.lines().map(strip_comment).collect::<Vec<_>>().join("\n");
                (p, code)
            })
        })
        .collect()
}

fn is_reserved(host: &str) -> bool {
    RESERVED
        .iter()
        .any(|r| host == *r || host.ends_with(&format!(".{r}")))
}

/// ALL-CAPS identifiers -- the shape a credential takes
/// (`SLACK_WEBHOOK_URL`, `PAGERDUTY_ROUTING_KEY`, `DEFAULT_STRIPE_BASE_URL`).
///
/// At least three real capitals and four characters, which keeps prose in
/// `SHOUTING_LIKE_THIS` out and keeps `ADA`-length acronyms from matching
/// everything.
fn caps_tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut run = String::new();
    let flush = |run: &mut String, out: &mut Vec<String>| {
        if run.len() >= 4 && run.chars().filter(char::is_ascii_uppercase).count() >= 3 {
            out.push(std::mem::take(run));
        } else {
            run.clear();
        }
    };
    for c in text.chars() {
        if c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_' {
            run.push(c);
        } else {
            flush(&mut run, &mut out);
        }
    }
    flush(&mut run, &mut out);
    out
}

/// Third-party hosts that shipped code can name.
///
/// A host must carry at least one dot and an alphabetic top-level label.
/// Single-label names are in-cluster service DNS -- `http://ada-api-gateway:8080`,
/// `http://otel-collector:4317`, and the `http://host:port` shape in an
/// exporter's doc comment. None of them is reachable from outside the cluster
/// and none is a sub-processor, so counting them made the gate report five
/// impossible findings instead of the three real ones.
fn third_party_hosts(files: &[(PathBuf, String)]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for (_, text) in files {
        let bytes = text.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            // `https://` is eight bytes and `http://` is seven. Matching a
            // fixed seven-byte window against both silently recognises only
            // the plain-scheme URLs, and every one of those in this tree is
            // a loopback or in-cluster service name that the exclusions
            // then discard -- so the scan came back empty and looked like a
            // clean bill of health. Derive the length from the match.
            let rest = &bytes[i..];
            let scheme_len = if rest.starts_with(b"https://") {
                b"https://".len()
            } else if rest.starts_with(b"http://") {
                b"http://".len()
            } else {
                i += 1;
                continue;
            };
            let start = i + scheme_len;
            let mut end = start;
            while end < bytes.len() {
                let c = bytes[end];
                // ASCII only. `(c as char).is_alphanumeric()` is true for
                // the high bytes of a multi-byte character, which walked the
                // cursor into the middle of a character and panicked.
                if c.is_ascii_alphanumeric() || c == b'.' || c == b'-' {
                    end += 1;
                } else {
                    break;
                }
            }
            if end > start {
                // start and end only ever advanced over ASCII bytes, so both
                // are char boundaries.
                let host = text[start..end].to_ascii_lowercase();
                let labels: Vec<&str> = host.split('.').collect();
                let looks_like_a_domain = labels.len() >= 2
                    && labels.last().is_some_and(|t| {
                        t.len() >= 2 && t.chars().all(|c| c.is_ascii_alphabetic())
                    });
                if looks_like_a_domain
                    && !is_reserved(&host)
                    && !NOT_A_PROCESSOR
                        .iter()
                        .any(|(domain, _)| host == *domain || host.ends_with(&format!(".{domain}")))
                {
                    out.insert(host);
                }
            }
            i = end.max(start);
        }
    }
    out
}

fn disclosure() -> String {
    let path = repo_root().join(DISCLOSURE);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

/// First column of every table row inside one delimited section.
fn section_processors(marker: &str) -> Vec<String> {
    let text = disclosure();
    let mut in_section = false;
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == marker {
            in_section = true;
            continue;
        }
        if trimmed == "<!-- end -->" {
            in_section = false;
            continue;
        }
        if !in_section || !trimmed.starts_with('|') {
            continue;
        }
        // Skip the header and the ---|--- separator.
        let first = trimmed
            .trim_matches('|')
            .split('|')
            .next()
            .unwrap_or("")
            .trim();
        if first.is_empty() || first.starts_with('-') || first.eq_ignore_ascii_case("sub-processor")
        {
            continue;
        }
        out.push(first.to_string());
    }
    out
}

/// The words a set of processor names contributes to a host match.
///
/// `api.datadoghq.com` is the processor "Datadog": the DNS label carries an
/// `hq` the trade name does not. Exact equality missed that, and a gate that
/// rejects a correctly declared processor teaches people to ignore it.
fn name_words(names: &[String]) -> Vec<String> {
    names
        .join(" ")
        .to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| w.len() >= 4)
        .map(str::to_string)
        .collect()
}

/// Does this host name the processor, by any substantial label?
fn host_is_processor(host: &str, words: &[String]) -> bool {
    host.split('.')
        .filter(|l| l.len() >= 4)
        .any(|label| words.iter().any(|w| w.contains(label) || label.contains(w)))
}

#[test]
fn every_third_party_host_in_shipped_code_is_declared_as_a_subprocessor() {
    let files = corpus();
    assert!(
        !files.is_empty(),
        "no source or manifest files found under {CODE_ROOTS:?} -- the scan is \
         broken, not the tree"
    );

    let hosts = third_party_hosts(&files);
    assert!(
        !hosts.is_empty(),
        "found no third-party hosts at all. The scanner is pointed at something \
         it cannot read; an empty result here would mean every host is accounted \
         for, which is not the same as every host being checked."
    );

    let words = name_words(&section_processors("<!-- gate:integrated -->"));
    let undeclared: Vec<&String> = hosts
        .iter()
        .filter(|h| !host_is_processor(h, &words))
        .collect();

    assert!(
        undeclared.is_empty(),
        "shipped code can reach {undeclared:?}, and {DISCLOSURE} names no \
         sub-processor for it. Either declare it with the data it receives and \
         a signed DPA, or add it to NOT_A_PROCESSOR in this file with the reason \
         it is not a sub-processor -- an unexplained exclusion would let the \
         next real processor through."
    );
}

#[test]
fn every_declared_subprocessor_matches_the_tree_in_both_directions() {
    let files = corpus();
    let hosts = third_party_hosts(&files);

    // "Integrated" has to mean something narrower than "the word appears
    // somewhere". A substring scan over the whole tree is how a prose
    // sentence in a YAML comment makes an unbuilt integration look real, so
    // an integration is either a host literal under a domain the gate
    // recognises, or an ALL-CAPS identifier -- the shape a credential takes
    // (`SLACK_WEBHOOK_URL`, `PAGERDUTY_ROUTING_KEY`).
    let all_caps: BTreeSet<String> = files.iter().flat_map(|(_, t)| caps_tokens(t)).collect();
    let integrated_name = |name: &str| {
        let words = name_words(&[name.to_string()]);
        let upper = name.to_ascii_uppercase();
        hosts.iter().any(|h| host_is_processor(h, &words))
            || all_caps.iter().any(|c| c.contains(&upper))
    };

    let integrated = section_processors("<!-- gate:integrated -->");
    let not_integrated = section_processors("<!-- gate:not-integrated -->");

    assert!(
        !integrated.is_empty(),
        "no rows between <!-- gate:integrated --> and <!-- end --> in \
         {DISCLOSURE}. Either the markers were lost or the table was emptied; \
         an empty table passes every check and discloses nothing."
    );
    assert!(
        !not_integrated.is_empty(),
        "no rows between <!-- gate:not-integrated --> and <!-- end --> in \
         {DISCLOSURE}; see above."
    );

    let missing: Vec<&String> = integrated.iter().filter(|p| !integrated_name(p)).collect();
    assert!(
        missing.is_empty(),
        "{DISCLOSURE} lists {missing:?} as integrated, but the name appears \
         nowhere in {CODE_ROOTS:?} or {APP_ROOTS:?}. A disclosure that claims a \
         relationship the code does not have is the same defect as one that \
         omits a relationship it has."
    );

    let stale: Vec<&String> = not_integrated
        .iter()
        .filter(|p| integrated_name(p))
        .collect();
    assert!(
        stale.is_empty(),
        "{DISCLOSURE} still lists {stale:?} as not integrated, but the name now \
         appears in {CODE_ROOTS:?} or {APP_ROOTS:?}. Move the row to the \
         integrated table -- leaving it here tells a customer their data does not \
         go somewhere it now goes."
    );
}
