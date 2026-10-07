//! PR #26 fixed 30 dead `docs/modules/*.md` references. All 30 were in
//! `crates/*/src/*.rs`.
//!
//! ## What it missed
//!
//! Two ways.
//!
//! **It skipped the crate READMEs.** The same crates carried the same broken
//! references in their `README.md`, and PR #26 never opened those files. Six
//! crates, six links a reader can follow and get nothing from.
//!
//! **It changed the filename, not the depth.** Every one of its 30 edits
//! repointed a truncated module-document name at the full one and left the
//! leading parent-directory marker exactly as it was. The referring file is
//! `crates/ada-m01-acquisition/src/connector.rs`, so one level up is
//! `crates/ada-m01-acquisition/` — a directory with no `docs/` in it. The
//! documents are at the repository root, three levels up. All 30 fixes were
//! still dead links when they landed, and nothing noticed, because PR #26
//! added no gate for the class — only `crate_test_counts.rs`, which counts
//! tests.
//!
//! ## What this gate holds
//!
//! Every `docs/**.md` reference under `crates/` resolves to a file that
//! exists — under the repository root **or** under the crate that contains
//! the referring file. The second is not optional:
//!
//! `ada-mock` keeps its design docs inside the crate, so a bare `docs/...`
//! reference written in that crate's README and sources means a path under
//! `crates/ada-mock/`, which exists. Ten of its references are correct that
//! way. A gate resolving against the repository root alone reports them as
//! broken, and the fix for a false positive — editing a working reference
//! until the checker is quiet — is how a gate starts costing more than it
//! catches.
//!
//! Nothing in `.github/workflows/` runs `cargo doc`, so these links are never
//! rendered anywhere. Filesystem resolution is the only definition in play,
//! which is what makes it worth holding to.
//!
//! ## What this gate must not do
//!
//! It scans `crates/`, and this file is in `crates/`. An earlier draft quoted
//! the broken paths it had found in its own module comment, and the gate
//! reported its own explanation as six dead references. The paths are
//! described rather than written out for that reason — the same trap the
//! commercial-disclosure gate walked into when it read its own prose as
//! evidence. The doc comment above is therefore the gate's own test case:
//! if it ever names a document that does not exist, this test fails.

use std::fs;
use std::path::{Path, PathBuf};

/// Scanned surface: crate sources and crate READMEs. This is the union of
/// what PR #26 fixed and what it missed.
const SCAN_ROOTS: &[&str] = &["crates"];
const SUFFIXES: &[&str] = &["rs", "md"];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("ada-core lives two levels under the workspace root")
        .to_path_buf()
}

fn files_under(rel_dir: &str) -> Vec<PathBuf> {
    let dir = repo_root().join(rel_dir);
    let mut out = Vec::new();
    collect(&dir, &mut out);
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
            // Build output. Nothing under it is source.
            if path.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            collect(&path, out);
            continue;
        }
        if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| SUFFIXES.contains(&e.to_ascii_lowercase().as_str()))
        {
            out.push(path);
        }
    }
}

/// `crates/<name>` above `path`, if the path is inside a crate.
///
/// A report file under a crate's own documentation directory belongs to
/// that crate, so its `docs/...` references resolve against the crate root
/// rather than the repository root.
fn crate_root_of(path: &Path) -> Option<PathBuf> {
    let rel = path.strip_prefix(repo_root()).ok()?;
    let mut parts = rel.components();
    if parts.next()?.as_os_str() != "crates" {
        return None;
    }
    let crate_name = parts.next()?;
    // The file IS crates/<name>/Cargo.toml, so it belongs to no crate.
    parts.next()?;
    Some(repo_root().join("crates").join(crate_name))
}

/// Does `reference` name a real file, from the point of view of `from`?
///
/// A leading `../` walks up from the referring file. A bare `docs/...` is
/// tried against the repository root first and then against the containing
/// crate.
fn resolves(reference: &str, from: &Path, crate_root: Option<&Path>) -> bool {
    if reference.starts_with("../") {
        // `PathBuf::pop` returns whether it popped, not the popped value, so
        // the walk-up has to mutate in place.
        let mut dir = from.parent().map(Path::to_path_buf);
        let mut rest = reference;
        while let Some(stripped) = rest.strip_prefix("../") {
            let Some(current) = dir.as_mut() else {
                return false;
            };
            if !current.pop() {
                return false;
            }
            rest = stripped;
        }
        return dir.is_some_and(|d| d.join(rest).exists());
    }
    if repo_root().join(reference).exists() {
        return true;
    }
    crate_root.is_some_and(|c| c.join(reference).exists())
}

/// Every markdown reference into the `docs` tree in a line, with any leading
/// parent-directory marker kept.
/// This function sees its own doc comment, so it must not name a document
/// that does not exist.
///
/// The shape is deliberately narrow: a `docs/` segment, a `.md` ending, and
/// only path characters in between. A looser pattern starts matching
/// prose and the failure message becomes unreadable.
fn references_in(line: &str) -> Vec<String> {
    let chars: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        // The token must not start mid-word.
        if i > 0 && (chars[i - 1].is_alphanumeric() || chars[i - 1] == '_') {
            i += 1;
            continue;
        }
        // Allow a run of `../` before `docs/`.
        let mut j = i;
        while j + 3 <= chars.len() && chars[j] == '.' && chars[j + 1] == '.' && chars[j + 2] == '/'
        {
            j += 3;
        }
        if j + 5 > chars.len() {
            i += 1;
            continue;
        }
        let tail: String = chars[j..].iter().take(5).collect();
        if tail != "docs/" {
            i += 1;
            continue;
        }
        let mut k = j;
        while k < chars.len()
            && (chars[k].is_alphanumeric() || matches!(chars[k], '/' | '_' | '.' | '-'))
        {
            k += 1;
        }
        let token: String = chars[i..k].iter().collect();
        if token
            .rsplit_once('.')
            .is_some_and(|(stem, ext)| !stem.is_empty() && ext.eq_ignore_ascii_case("md"))
        {
            out.push(token);
        }
        i = k.max(i + 1);
    }
    out
}

#[test]
fn every_docs_reference_under_crates_resolves_to_a_file() {
    let files = files_under(SCAN_ROOTS[0]);
    assert!(
        !files.is_empty(),
        "no files found under crates/ -- the scan is broken, not the tree"
    );

    let mut checked = 0usize;
    let mut dead: Vec<String> = Vec::new();

    for path in &files {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        let crate_root = crate_root_of(path);
        let rel = path
            .strip_prefix(repo_root())
            .unwrap_or(path)
            .display()
            .to_string();
        for (lineno, line) in text.lines().enumerate() {
            for reference in references_in(line) {
                checked += 1;
                if !resolves(&reference, path, crate_root.as_deref()) {
                    dead.push(format!(
                        "{rel}:{} names {reference}, which exists neither under the \
                         repository root nor under {}",
                        lineno + 1,
                        crate_root.as_ref().map_or_else(
                            || "(no containing crate)".to_string(),
                            |c| c.display().to_string()
                        )
                    ));
                }
            }
        }
    }

    assert!(
        checked > 100,
        "only {checked} docs references found under crates/; a scan this small \
         cannot be the one that found the original 30"
    );
    assert!(
        dead.is_empty(),
        "{} of {checked} docs references name a file that does not exist:\n  {}",
        dead.len(),
        dead.join("\n  ")
    );
}
