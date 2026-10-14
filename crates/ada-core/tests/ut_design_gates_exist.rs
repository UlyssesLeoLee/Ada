//! UT-design.md §14/§15 described CI gates that block a merge. None existed.
//!
//! ## What was wrong
//!
//! ```text
//! ## 14. 覆盖率与质量门禁
//! - **行覆盖**：≥ 80%（CI 卡点）
//! - **P0 用例通过率**：100%（CI 卡点，任意 P0 失败阻塞合并）
//!
//! ## 15. 持续集成
//! - 每日定时任务执行完整 UT 套件 + 覆盖率报告
//! - 报告产物：`coverage/html/index.html` + `coverage/summary.txt`
//! ```
//!
//! Checked against the tree rather than assumed: `.github/workflows/ci.yml`
//! contains no `cargo-llvm-cov`, no `cargo-tarpaulin` and no `schedule:`, and
//! neither `Cargo.toml` nor `deny.toml` declares a coverage dependency at all.
//! `CI 卡点` means a gate that blocks the merge. Written beside a target number
//! that nothing produces, it makes a reader believe coverage is enforced. It is
//! not: `cargo test` runs, coverage is never measured, nothing fails on it.
//!
//! ## Why this gate is shaped the way it is
//!
//! The first three versions tried to detect the false claim lexically -- find
//! lines saying "CI 卡点", then check whether the tools they named exist in a
//! workflow. Each version was wrong in a different direction, and both
//! directions are fatal:
//!
//! 1. Checking only *named tools* missed the most common shape, `≥ 80%（CI
//!    卡点）`, which names nothing. Restoring that line passed the gate.
//! 2. Checking *any asserting line* flagged the correction note that explains
//!    the tools are absent -- the document discussing the gate rather than
//!    claiming it. Restricting to "has a threshold" then flagged the revision
//!    log, which contains both the phrase and a number.
//!
//! Prose cannot be classified reliably by keyword. So the document now carries
//! an explicit, machine-readable inventory, and this gate validates the
//! inventory against the repository.
//!
//! ## What this gate holds
//!
//! §14.5 lists each CI gate with its enforcing tool as a table row. For any row
//! marked **implemented**, the tool must appear in `.github/workflows/`. Rows
//! marked **planned** are exempt -- that is the whole point of the distinction,
//! and it is what lets the document say what it intends without claiming it.
//!
//! The anti-vacuity assertion fails if the inventory is missing or empty, so the
//! gate cannot pass by the document being rewritten out from under it.

use std::fs;
use std::path::{Path, PathBuf};

const DOC: &str = "docs/tests/UT-design.md";
const WORKFLOW_DIR: &str = ".github/workflows";
const INVENTORY_HEADING: &str = "### 14.5";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("ada-core lives two levels under the workspace root")
        .to_path_buf()
}

fn workflow_text(root: &Path) -> String {
    let dir = root.join(WORKFLOW_DIR);
    let Ok(entries) = fs::read_dir(&dir) else {
        panic!("no {WORKFLOW_DIR} at {}", dir.display());
    };
    let mut all = String::new();
    for entry in entries.flatten() {
        let p = entry.path();
        let is_yaml = p
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("yml") || e.eq_ignore_ascii_case("yaml"));
        if is_yaml {
            if let Ok(t) = fs::read_to_string(&p) {
                all.push_str(&t);
                all.push('\n');
            }
        }
    }
    all
}

/// Everything from `### 14.5` up to the next `###` heading.
///
/// Returns `None` when the inventory is absent. An earlier version fell back
/// to `doc.len()`, which slices from the end of the document to the end of the
/// document -- and that span still contained the words the assertions look
/// for, so deleting the entire inventory made this gate pass. A missing
/// inventory has to be distinguishable from an empty one, or the anti-vacuity
/// check below is checking the wrong bytes.
fn inventory(doc: &str) -> Option<&str> {
    let start = doc.find(INVENTORY_HEADING)?;
    let tail = &doc[start..];
    let end = tail[1..].find("\n###").map_or(tail.len(), |i| i + 1);
    Some(&tail[..end])
}

#[test]
fn ut_design_gate_inventory_matches_the_workflows() {
    let root = repo_root();
    let doc_path = root.join(DOC);
    let doc = fs::read_to_string(&doc_path).unwrap_or_else(|e| panic!("read {DOC}: {e}"));
    let workflows = workflow_text(&root);
    let inv = inventory(&doc).unwrap_or_else(|| {
        panic!(
            "{DOC} has no `{INVENTORY_HEADING}` inventory of CI gates. This gate \
             exists to keep that inventory honest; without it there is nothing to \
             check, and the document can claim whatever it likes again."
        )
    });

    let mut rows = 0usize;
    let mut implemented = 0usize;
    let mut violations: Vec<String> = Vec::new();

    for (i, line) in inv.lines().enumerate() {
        if !line.starts_with('|') {
            continue;
        }
        // Skip the header and separator rows.
        if line.contains("---") || line.contains("门禁") || line.contains("强制工具") {
            continue;
        }
        let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
        if cells.len() < 3 {
            continue;
        }
        let (name, tool, status) = (cells[0], cells[1], cells[2]);
        if name.is_empty() || tool.is_empty() {
            continue;
        }
        rows += 1;

        if !status.starts_with("已实现") {
            continue;
        }
        implemented += 1;
        // The enforcing tool must literally appear in a workflow. The backticks
        // in the cell are markdown, not part of the name.
        let bare = tool.trim_matches('`');
        if !workflows.contains(bare) {
            violations.push(format!(
                "{DOC} §14.5 lists `{name}` as implemented via `{bare}`, \
                 but that string appears in no workflow"
            ));
        }
        let _ = i;
    }

    assert!(
        rows > 0,
        "the §14.5 inventory parsed to zero rows -- the table shape changed and \
         this gate is silently checking nothing"
    );
    assert!(
        implemented > 0,
        "the §14.5 inventory lists no gate as implemented. At least one exists: \
         `cargo test --workspace` runs on every PR in ci.yml. An inventory with \
         nothing marked implemented means the column is being filled in wrong."
    );
    assert!(
        violations.is_empty(),
        "UT-design.md marks these gates as implemented but no workflow does:\n  {}\n\n\
         The status column is what a reader trusts when deciding whether \
         coverage is enforced.",
        violations.join("\n  ")
    );
}
