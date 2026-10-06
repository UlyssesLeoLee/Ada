//! Four of the five shipped runbooks name a trigger that no alert rule emits.
//!
//! ## What this records
//!
//! ```text
//! runbook trigger                an alert with that name exists?
//! ServiceDown                    yes
//! DiskSpaceFillingFast           no  -- the rule is LowDiskSpace
//! SLIBurnRateFast                no  -- SLIB is a typo for SLOB, and the rules
//!                                       are SLOBurnRateFast1h
//! SLIBurnRateSlow                no  -- rules are SLOBurnRateSlow24h / 72h
//! DBConnectionPoolExhausted      no  -- no such rule anywhere
//! ```
//!
//! A runbook whose trigger never fires is a page that does not get a response.
//! The step still reports success. PR #21 corrected the `runbook_url` values in
//! these files and documented the dormancy in `config/remediation/README.md`;
//! nothing checked that the list stays accurate, so the names can drift again
//! in either direction -- a rule renamed without the runbook, or a runbook
//! "fixed" to a name that does not exist.
//!
//! ## Why this asserts nothing
//!
//! It reports which triggers are live; it does not fail. Making the mismatch a
//! failure would require either renaming `disk-space-low`'s trigger -- which
//! would arm `find /var/log ... -delete` on a real host, with cooldown still
//! per-process -- or deleting the dormant runbooks, which is a product
//! decision, not a lint fix.
//!
//! The check that is safe is the one that catches *both* directions: every
//! name a runbook claims must be accounted for here, whether it resolves to a
//! live alert or is listed as dormant. A runbook gaining an unknown trigger
//! then fails, and so does this file drifting out of date.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Runbooks whose trigger is known not to match any alert rule, with the
/// reason. A runbook absent from this list must resolve to a live rule.
///
/// This is the file to edit when an alert is renamed: move the entry, or
/// delete it when the runbook starts firing.
const DORMANT: [(&str, &str, &str); 4] = [
    (
        "disk-space-low",
        "DiskSpaceFillingFast",
        "the rule is LowDiskSpace; naming this trigger would arm `find -delete`",
    ),
    (
        "slo-burn-fast-page",
        "SLIBurnRateFast",
        "SLIB is a typo for SLOB; the rules are SLOBurnRateFast1h",
    ),
    (
        "slo-burn-slow-notify",
        "SLIBurnRateSlow",
        "the rules are SLOBurnRateSlow24h and SLOBurnRateSlow72h",
    ),
    (
        "db-pool-exhausted-kill-idle",
        "DBConnectionPoolExhausted",
        "no alert rule has this name anywhere in observability/",
    ),
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("ada-core lives two levels under the workspace root")
        .to_path_buf()
}

/// Every `alert:` name under `observability/`.
fn defined_alerts(root: &Path) -> BTreeSet<String> {
    let dir = root.join("observability");
    let mut out = BTreeSet::new();
    let mut stack = vec![dir];
    while let Some(d) = stack.pop() {
        let Ok(entries) = fs::read_dir(&d) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            let is_yaml = p
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("yml") || e.eq_ignore_ascii_case("yaml"));
            if !is_yaml {
                continue;
            }
            let Ok(text) = fs::read_to_string(&p) else {
                continue;
            };
            for line in text.lines() {
                // The shape in these files is `      - alert: ServiceDown`, so
                // the line starts with a YAML sequence marker, not the key.
                // Stripping only `alert:` and requiring it at the start of the
                // line matches nothing here, which the anti-vacuity assertion
                // below then reports as "no alert rules found" -- a false
                // alarm caused by the scanner's idea of the shape, not by the
                // tree.
                let body = line.trim_start().trim_start_matches('-').trim_start();
                // Skip the commented-out and prose matches a naive scan picks
                // up (`# - alert: derived`, `alert: not set`).
                if body.starts_with('#') {
                    continue;
                }
                let Some(rest) = body.strip_prefix("alert:") else {
                    continue;
                };
                let name = rest.trim();
                if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    out.insert(name.to_string());
                }
            }
        }
    }
    out
}

/// Every runbook's `trigger` value, with the runbook's action id.
fn runbook_triggers(root: &Path) -> Vec<(String, String)> {
    let dir = root.join("config").join("remediation");
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(&dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension()
            .is_none_or(|e| !e.eq_ignore_ascii_case("json"))
        {
            continue;
        }
        let Ok(text) = fs::read_to_string(&p) else {
            continue;
        };
        let id = ["\"id\"", "\"trigger\""]
            .iter()
            .find_map(|k| {
                let i = text.find(k)?;
                let rest = &text[i..];
                let q1 = rest.find('"')? + 1;
                let q2 = rest[q1..].find('"')? + q1 + 1;
                Some(rest[q1..q2].to_string())
            })
            .unwrap_or_default();
        if let Some(i) = text.find("\"trigger\"") {
            let rest = &text[i..];
            if let Some(q1) = rest.find(':') {
                let after = &rest[q1 + 1..];
                if let Some(s) = after.find('"') {
                    let s = s + 1;
                    if let Some(len) = after[s..].find('"') {
                        out.push((id, after[s..s + len].to_string()));
                    }
                }
            }
        }
    }
    out
}

#[test]
fn every_runbook_trigger_is_either_live_or_listed_as_dormant() {
    let root = repo_root();
    let alerts = defined_alerts(&root);
    let runbooks = runbook_triggers(&root);

    assert!(
        !runbooks.is_empty(),
        "no runbook triggers found under config/remediation -- this gate is \
         pointed at a tree it cannot read"
    );
    assert!(
        !alerts.is_empty(),
        "no alert rules found under observability/ -- every trigger would \
         look dormant, which is the wrong conclusion rather than a safe one"
    );

    let known_dormant: BTreeSet<&str> = DORMANT.iter().map(|(_, t, _)| *t).collect();

    let mut unaccounted: Vec<String> = Vec::new();
    for (id, trigger) in &runbooks {
        if alerts.contains(trigger) || known_dormant.contains(trigger.as_str()) {
            continue;
        }
        unaccounted.push(format!(
            "{id}: trigger `{trigger}` is neither a live alert nor listed in \
             DORMANT -- the rule may have been renamed, or this entry is stale"
        ));
    }

    // The reverse direction: a DORMANT entry naming a rule that now exists is
    // just as wrong. Silently leaving it there would keep the document
    // claiming a gate is dead after someone wired it up.
    let stale: Vec<String> = DORMANT
        .iter()
        .filter(|(_, trigger, _)| alerts.contains(*trigger))
        .map(|(id, trigger, _)| {
            format!("{id}: listed as dormant but `{trigger}` is now a live alert")
        })
        .collect();

    assert!(
        unaccounted.is_empty(),
        "runbook triggers that are neither live nor documented as dormant:\n  {}",
        unaccounted.join("\n  ")
    );
    assert!(
        stale.is_empty(),
        "DORMANT entries that no longer describe reality:\n  {}\n\n\
         A runbook listed as unable to fire, whose trigger now resolves to a \
         live rule, is documentation that lies in the safe direction -- which \
         is harder to notice and just as wrong.",
        stale.join("\n  ")
    );
}
