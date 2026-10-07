//! Every reference to a test or gate file under `crates/*/tests/` names a
//! file that exists.
//!
//! ## Why this gate exists
//!
//! A deployment manifest can tell an operator that some check will catch a
//! mistake they are about to make. If that check does not exist, the manifest
//! is not documenting a safety net, it is advertising one. Nothing renders
//! these references, no link checker runs over them, and the sentence reads
//! exactly the same whether the named file is real or fictional.
//!
//! The manifest that motivated this gate sat in the shipped remediation
//! Deployment and named a gate file for step credentials that was never
//! written. The property it described really was enforced — by a different
//! test in the same directory, under a name nobody reading the manifest would
//! guess — so the deployment had no coverage gap. What it had was a false
//! claim about where its coverage lived.
//!
//! ## What is and is not checked
//!
//! The subject is `crates/<crate>/tests/<file>.rs` references, read from every
//! tracked text file in the repository: sources, manifests, workflows, docs,
//! scripts. Thirty-one of them exist today, spread across deployment
//! manifests, CI workflows, crate sources and design documents.
//!
//! One subtree is exempt: `docs/decisions/`. Architecture decision records are
//! written before the code they describe. A draft whose whole subject is a
//! module that has not been implemented yet will name the test file that
//! module will grow, and that is correct drafting, not a broken promise. The
//! directory is exempt because it is where forward-looking decisions live,
//! not because a broken reference was found there and needed hiding; the
//! count of exempted references is asserted below so this cannot quietly turn
//! into a blanket suppression.
//!
//! ## Scope
//!
//! This is deliberately narrower than "every path in every file resolves".
//! Such a gate is measured against templates (`..._2026-YY-MM-DD.md`),
//! literal ellipses in test diagnostics, historical changelog entries, and
//! crate-relative paths that need a second resolution root. Each of those is
//! individually defensible, and each is also an exclusion list that a real
//! defect can hide in. Holding one narrow class to a hard rule is worth more
//! than holding every path to a rule with forty exceptions.
//!
//! ## Self-audit
//!
//! The walk covers this file. A reference to a test that does not exist, written
//! anywhere above or below, fails this gate — so this module comment is itself
//! the test case, and the paths it needed to describe are described rather
//! than written out.

use std::fs;
use std::path::{Path, PathBuf};

/// Text formats a shipped reference can hide in.
const SUFFIXES: &[&str] = &[
    "rs", "md", "yaml", "yml", "toml", "json", "txt", "sh", "ps1", "py",
];

/// Directories that hold no source. `target` is build output, `.worktrees`
/// holds sibling checkouts of these same files, and the rest are dependencies.
const SKIP_DIRS: &[&str] = &[
    ".git",
    ".worktrees",
    "target",
    "node_modules",
    ".venv",
    ".dart_tool",
];

/// Forward-looking design records. See the module comment for why.
const EXEMPT_PREFIX: &str = "docs/decisions/";

/// Below this, the walk has stopped finding references rather than finding
/// them all broken. Thirty-one are cited today; twenty leaves room to notice.
const MIN_REFERENCES: usize = 20;

/// The exemption must still be reaching something, or it is dead config that
/// looks like a working safety valve.
const MIN_EXEMPT: usize = 1;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("ada-core lives two levels under the workspace root")
        .to_path_buf()
}

/// Machine-generated fixtures and vendored data can be large and are not
/// written by hand.
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let skip = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| SKIP_DIRS.contains(&n));
            if skip {
                continue;
            }
            collect(&path, out);
            continue;
        }
        let is_text = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| SUFFIXES.contains(&e.to_ascii_lowercase().as_str()));
        let small = path.metadata().is_ok_and(|m| m.len() <= MAX_FILE_BYTES);
        if is_text && small {
            out.push(path);
        }
    }
}

fn tracked_text_files() -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect(&repo_root(), &mut out);
    out.sort();
    out
}

fn rel_of(path: &Path) -> String {
    path.strip_prefix(repo_root())
        .unwrap_or(path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Byte-offset search. Byte slices cannot carry a `str` because the offsets
/// here are not guaranteed to land on character boundaries.
fn find_subslice(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Every `crates/<crate>/tests/<file>.rs` path named anywhere in `line`.
///
/// The match is anchored on the `crates/` segment, so a reference written with
/// a leading parent-directory marker still yields the repository-relative
/// path, which is the only form that can be probed on disk.
///
/// Scans bytes rather than characters. Advancing a `str` index one byte at a
/// time lands mid-character on any line containing non-ASCII text, and this
/// repository is full of Japanese prose — the first run of this gate panicked
/// on a char boundary inside `docs/decisions/`.
fn cited_test_paths(line: &str) -> Vec<String> {
    let b = line.as_bytes();
    let mut found = Vec::new();
    let mut i = 0usize;

    while i < b.len() {
        if !(b[i..].starts_with(b"crates/") || b[i..].starts_with(b"crates\\")) {
            i += 1;
            continue;
        }

        let rest = &b[i + 6..];
        let tests_at = find_subslice(rest, b"tests/").or_else(|| find_subslice(rest, b"tests\\"));

        let Some(offset) = tests_at else {
            i += 1;
            continue;
        };

        let crate_name = &rest[..offset];
        let crate_name = crate_name
            .iter()
            .rposition(|c| *c != b'/' && *c != b'\\')
            .map_or(crate_name, |last| &crate_name[..=last]);

        let after_tests = &rest[offset + 6..];
        let name_end = after_tests
            .iter()
            .position(|c| !(c.is_ascii_alphanumeric() || *c == b'_' || *c == b'-' || *c == b'.'))
            .unwrap_or(after_tests.len());

        if name_end > 3 && after_tests[..name_end].ends_with(b".rs") {
            found.push(format!(
                "crates/{}/tests/{}",
                String::from_utf8_lossy(crate_name),
                String::from_utf8_lossy(&after_tests[..name_end])
            ));
        }
        i += 6;
    }

    found
}

#[test]
fn every_cited_test_gate_exists() {
    let root = repo_root();
    let mut checked = 0usize;
    let mut exempt = 0usize;
    let mut dead: Vec<String> = Vec::new();

    for path in tracked_text_files() {
        let rel = rel_of(&path);
        let is_exempt = rel.starts_with(EXEMPT_PREFIX);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };

        for (lineno, line) in text.lines().enumerate() {
            for reference in cited_test_paths(line) {
                if is_exempt {
                    exempt += 1;
                    continue;
                }
                checked += 1;
                if !root.join(&reference).is_file() {
                    dead.push(format!("{rel}:{} names {reference}", lineno + 1));
                }
            }
        }
    }

    assert!(
        checked >= MIN_REFERENCES,
        "only {checked} crates/*/tests references found outside the exempt tree; \
         the walk has stopped finding references rather than finding them all broken"
    );
    assert!(
        exempt >= MIN_EXEMPT,
        "the exempt tree yielded {exempt} references; the exemption is now \
         dead config that reads like a working safety valve"
    );
    assert!(
        dead.is_empty(),
        "{} of {checked} references name a test or gate file that does not exist. \
         A shipped file that promises a check is not real is advertising a net \
         it does not have:\n  {}",
        dead.len(),
        dead.join("\n  ")
    );
}
