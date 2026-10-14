//! Every crate's `lib.rs` states how many unit tests it has, and all eleven of
//! those numbers were wrong.
//!
//! ## What was wrong
//!
//! ```text
//! crate                       claim    actual
//! ada-m01-acquisition             8        30
//! ada-m02-normalizer              9        33
//! ada-m03-data-flow-engine       10        37
//! ada-m04-orchestration          12        42
//! ada-m05-control-flow           10        36
//! ada-m06-plugin-sdk             10        31
//! ada-m09-exporter                9        34
//! ada-m10-tenant-middleware       8        28
//! ada-m11-rbac-collab             9        35
//! ada-m14-module-registry        11        31
//! ada-m15-central-event-bus       9        27
//! ```
//!
//! Not one matched. These were written when the crates were stubs and the tests
//! arrived afterwards; nothing ever compared the two. A comment saying "8 unit
//! tests" in a file that now has 30 is the same defect as a document claiming a
//! Postgres table that does not exist -- a shipped assertion about the system
//! that no check backs and that happens to be false.
//!
//! ## What this gate holds
//!
//! For any `N unit tests` claim in `crates/*/src/lib.rs`, N must equal the
//! number of `#[test]` / `#[tokio::test]` attributes in that crate's `src/` tree.
//!
//! Counting attributes rather than running `cargo test` is deliberate. It keeps
//! the check runnable in the same environment as the other gates, and a name
//! like `http_connector_status_override_is_consumed()` cannot be counted
//! incorrectly by a regex in a way that a hand-written list could.
//!
//! ## Why the crate list is explicit
//!
//! An earlier version made the claim purely optional: "a crate without one is
//! not required to add one". That is right for the fourteen crates that never
//! made a claim, and dangerously wrong for the ten that did. Deleting the
//! number from `ada-m15-central-event-bus` entirely satisfied the optional rule
//! -- and it did not happen in a review. The mutation harness that proves this
//! gate can fail edits every crate's phrase at once; its restore step covered
//! only one file, so the other nine deletions were committed and merged.
//!
//! `MUST_CLAIM` is that lesson encoded. Removing a number from any crate on the
//! list is now a failure, so the state the audit established cannot be quietly
//! undone by the next mutation run.
//!
//! The claim is optional: a crate without one is not required to add one. This
//! gate closes the loop on the ones that exist rather than mandating a new
//! convention across eleven crates.
//!
//! ## What it deliberately does not check
//!
//! The integration count in the same comment ("+ 4 integration tests") is not
//! checked. Two crates already name a file there (`tests/integration.rs`) and
//! the rest do not, so the phrase is inconsistent rather than uniformly wrong,
//! and a rule that fires on some occurrences and not others is a rule nobody
//! trusts. It is left alone rather than half-enforced.

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("ada-core lives two levels under the workspace root")
        .to_path_buf()
}

/// `#[test]` and `#[tokio::test]`, which the test runner treats identically.
fn count_test_attrs(text: &str) -> usize {
    text.matches("#[test]").count() + text.matches("#[tokio::test]").count()
}

fn all_rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            all_rust_files(&p, out);
        } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("rs")) {
            out.push(p);
        }
    }
}

/// Crates that state a unit-test count and therefore must keep doing so.
///
/// Derived from the tree, not from taste: these are the `lib.rs` files that
/// carry an `N unit tests` claim today. The other fourteen crates never made
/// one, and this gate does not force a convention on them.
const MUST_CLAIM: [&str; 11] = [
    "ada-m01-acquisition",
    "ada-m02-normalizer",
    "ada-m03-data-flow-engine",
    "ada-m04-orchestration",
    "ada-m05-control-flow",
    "ada-m06-plugin-sdk",
    "ada-m09-exporter",
    "ada-m10-tenant-middleware",
    "ada-m11-rbac-collab",
    "ada-m14-module-registry",
    "ada-m15-central-event-bus",
];

#[test]
fn claimed_unit_test_counts_match_the_tests_that_exist() {
    let root = repo_root();
    let crates_dir = root.join("crates");
    let Ok(crate_entries) = fs::read_dir(&crates_dir) else {
        panic!("no crates/ directory at {}", crates_dir.display());
    };

    let mut checked = 0usize;
    let mut wrong: Vec<String> = Vec::new();
    let mut claimed_crates: Vec<String> = Vec::new();

    let mut crate_paths: Vec<PathBuf> = crate_entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    crate_paths.sort();

    for crate_path in crate_paths {
        let lib = crate_path.join("src").join("lib.rs");
        let Ok(lib_text) = fs::read_to_string(&lib) else {
            continue;
        };

        // Find `N unit tests`. The claim is a whole phrase; a bare number
        // elsewhere in the file is not this assertion.
        let Some(claim) = lib_text.lines().find_map(|line| {
            let idx = line.find("unit tests")?;
            // `trim_end` before scanning backwards for the digits. Without it
            // the prefix ends in the space that separates the number from the
            // word "unit", so `take_while(is_ascii_digit)` stops immediately,
            // yields an empty string, and the gate silently matches nothing --
            // which the anti-vacuity assertion below then reports as
            // "no crate states a unit test count".
            let digits: String = line[..idx]
                .trim_end()
                .chars()
                .rev()
                .take_while(char::is_ascii_digit)
                .collect();
            if digits.is_empty() {
                return None;
            }
            digits
                .chars()
                .rev()
                .collect::<String>()
                .parse::<usize>()
                .ok()
        }) else {
            continue;
        };

        let src_dir = crate_path.join("src");
        let mut files = Vec::new();
        all_rust_files(&src_dir, &mut files);
        let actual: usize = files
            .iter()
            .filter_map(|f| fs::read_to_string(f).ok())
            .map(|t| count_test_attrs(&t))
            .sum();

        checked += 1;
        let name = crate_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        claimed_crates.push(name.to_string());
        if claim != actual {
            wrong.push(format!("{name}: claims {claim} unit tests, has {actual}"));
        }
    }

    // A crate that used to state a count and no longer does has not been made
    // correct -- it has been made unverifiable. This is the hole that let the
    // mutation harness commit nine deleted numbers into `main`: the optional
    // rule was satisfied by the absence it had caused.
    let missing: Vec<&str> = MUST_CLAIM
        .iter()
        .copied()
        .filter(|c| !claimed_crates.iter().any(|x| x == c))
        .collect();

    assert!(
        checked > 0,
        "no crate states a unit test count -- this gate is pointed at a tree \
         where the comments it checks have all been removed"
    );
    assert!(
        missing.is_empty(),
        "these crates stopped stating a unit test count:\n  {}\n\n\
         Deleting the number is not the same as fixing it. The count was what \
         made the claim checkable; removing it removes the evidence without \
         making the underlying statement true.",
        missing.join("\n  ")
    );
    assert!(
        wrong.is_empty(),
        "lib.rs test counts are stale:\n  {}\n\n\
         These numbers are what a reviewer reads to judge coverage. Derive them \
         from the tests, or delete the count -- a correct number written once \
         and never checked is the defect this gate exists for.",
        wrong.join("\n  ")
    );
}
