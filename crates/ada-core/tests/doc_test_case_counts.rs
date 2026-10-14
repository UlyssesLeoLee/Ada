//! A document that states how many cases a test file contains is a claim
//! about the source, and it decays silently.
//!
//! ## What was wrong
//!
//! `docs/observability/14-auto-remediation.md` §8.3 said
//!
//! ```text
//! 7 ケース (SAVEPOINT 単位):
//!
//! - t_check_cooldown_active / inactive
//! ```
//!
//! and `db/tests/V003__phase8_remediation_test.sql` labels eight savepoints:
//! `t_tables_exist`, `t_record_success`, `t_record_failure`,
//! `t_record_cooldown_idempotent`, `t_record_invalid`, `t_check_cooldown_active`,
//! `t_check_cooldown_inactive`, `t_outcome_chk`. The bullet list had the right
//! names in it, but `active / inactive` was written as one line, so the count
//! and the enumeration disagreed by one. The count is the part a reader
//! trusts, and it was the part that was wrong.
//!
//! This is the smallest possible member of a class this repository keeps
//! hitting: a shipped file asserts a fact about the system, nothing checks it,
//! and the assertion is false. The fix is not "be careful when editing" -- the
//! count has to come from the source, not from a human counting a list.
//!
//! ## What this gate holds
//!
//! Every `N ケース` claim in that document must equal the number of distinct
//! `SAVEPOINT` labels in the SQL test file the section names. The test file is
//! located from the §8.3 heading itself, so renaming the file moves the gate
//! with it instead of silently disabling it.
//!
//! Deliberately scoped to that one section. The document also contains `5 個`
//! and `5 門`, which are counts of Rust unit tests and of CI commands; those
//! live in the crate sources and in the workflow, and guessing at them from
//! here would be the "checker reimplements the thing it checks" mistake. A
//! gate that guesses wrong is worse than no gate.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The section whose count is checked, and the phrase that introduces it.
const DOC: &str = "docs/observability/14-auto-remediation.md";
const SECTION_MARKER: &str = "### 8.3";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("ada-core lives two levels under the workspace root")
        .to_path_buf()
}

/// Distinct `SAVEPOINT <name>` labels in `text`.
///
/// Counted from the labels the file actually sets, not from the notices it
/// raises. Those two agree today, but the savepoint is the unit the document
/// says it is counting, and a rollback to an undeclared name would then show
/// up as a missing case rather than as a silent pass.
fn savepoint_labels(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let after = trimmed
            .strip_prefix("SAVEPOINT ")
            .or_else(|| trimmed.strip_prefix("ROLLBACK TO SAVEPOINT "));
        let Some(name) = after else { continue };
        // `SAVEPOINT t_x;` on its own line, or a bare name in a `ROLLBACK TO`.
        // No `trim()` before `split_whitespace`: it does nothing here and
        // clippy::trim_split_whitespace is right that it is noise.
        let name = name
            .trim_end_matches(';')
            .split_whitespace()
            .next()
            .unwrap_or_default();
        if name.starts_with("t_") {
            out.insert(name.to_string());
        }
    }
    out
}

/// The case count stated on the `N ... (SAVEPOINT ...)` line of the §8.3 block.
///
/// Matched on the shape of the line, not on its exact wording. The first
/// version compared against a literal `" (SAVEPOINT \u{7a4a}\u{4f4d}):"`
/// suffix and never matched: the document writes 単 as U+5352, the new form,
/// not U+7A4A. A gate that hardcodes a glyph fails on correct input for a
/// reason no reader would ever guess. The number is the claim under test; the
/// surrounding words are decoration.
fn claimed_case_count(doc: &str) -> Option<usize> {
    let (_, block) = doc.split_once(SECTION_MARKER)?;
    for line in block.lines() {
        if !line.contains("SAVEPOINT") {
            continue;
        }
        let digits: String = line
            .trim()
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        if !digits.is_empty() {
            return digits.parse().ok();
        }
    }
    None
}

/// The `db/tests/...sql` path named in the §8.3 heading.
///
/// The path is written inside a Markdown code span, so the raw slice between
/// the parentheses arrives wrapped in backticks and ends with `` ` `` rather
/// than `.sql`. Trimming the tick first is the difference between a gate that
/// reads the file and one that throws away the working configuration — the
/// first version of this function filtered on `ends_with(".sql")` against the
/// untrimmed slice, never matched, and failed on the correct document.
fn named_test_file(doc: &str) -> Option<String> {
    let (_, block) = doc.split_once(SECTION_MARKER)?;
    let heading = block.lines().next()?;
    // Accept both bracket widths; the document uses ASCII, but a full-width
    // bracket in a Japanese sentence is a normal thing to type and should not
    // silently disarm the gate.
    let inner = heading
        .split_once('(')
        .or_else(|| heading.split_once('\u{ff08}'))?
        .1;
    let path = inner
        .split_once(')')
        .or_else(|| inner.split_once('\u{ff09}'))?
        .0;
    let path = path.trim().trim_matches('`').trim();
    // `Path::extension` rather than `ends_with(".sql")`: clippy's
    // case_sensitive_file_extension_comparisons is right that `V003.SQL` is
    // the same file, and the same rule already governs the manifest scan in
    // remediation_links.rs.
    let is_sql = Path::new(path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("sql"));
    (!path.is_empty() && is_sql).then(|| path.to_string())
}

#[test]
fn documented_savepoint_case_count_matches_the_sql() {
    let root = repo_root();
    let doc_path = root.join(DOC);
    let doc = fs::read_to_string(&doc_path).unwrap_or_else(|e| panic!("read {DOC}: {e}"));

    let test_file = named_test_file(&doc).unwrap_or_else(|| {
        panic!(
            "{DOC} §8.3 no longer names a `db/tests/*.sql` file -- this gate is \
             pointed at a section that no longer describes the test suite"
        )
    });
    let sql_path = root.join(&test_file);
    let sql = fs::read_to_string(&sql_path).unwrap_or_else(|e| panic!("read {test_file}: {e}"));

    let labels = savepoint_labels(&sql);
    let claimed = claimed_case_count(&doc).unwrap_or_else(|| {
        panic!(
            "{DOC} §8.3 no longer states an `N \u{30b1}\u{30fc}\u{30b9} (SAVEPOINT \u{7a4a}\u{4f4d}):` \
             count -- this gate has nothing to compare the source against"
        )
    });

    assert!(
        !labels.is_empty(),
        "no SAVEPOINT labels found in {test_file} -- the counter matched nothing"
    );
    assert_eq!(
        claimed,
        labels.len(),
        "{DOC} §8.3 says {claimed} cases but {test_file} labels {} savepoints:\n  {}\n\n\
         The count is what a reader trusts. Derive it from the labels, and keep \
         the bullet list one entry per label so the two cannot drift apart again.",
        labels.len(),
        labels.into_iter().collect::<Vec<_>>().join("\n  ")
    );
}
