//! `db/README.md` documents V001 and V002. V003 arrived with two tables and
//! two functions and was never written down.
//!
//! ## What was wrong
//!
//! The README's directory tree listed two migrations and one test file. The
//! tree has three of each. Its object sections stopped at 11 tables and 6
//! functions; the tree has 13 and 8. Every "apply V001, apply V002" and
//! "run the V002 test" instruction skipped V003, and the verification table
//! said two things that had stopped being true:
//!
//! * `psql` "host has no psql, never run against a real server" — CI runs
//!   `bash db/run-tests.sh` on `postgres:16` in the `db migrations` job.
//! * "CI integration: TODO, add `.github/workflows/db-test.yml` in another
//!   task" — there is no such file. The job lives in `ci.yml` and is called
//!   `db-migrations`.
//!
//! `db/Makefile` had the same blind spot in a place where it matters more
//! than prose: `clean` TRUNCATEs eleven tables and `drop` DROPs eleven, so
//! `remediation_cooldowns` survived a test reset. A cooldown row that
//! outlives `make clean` suppresses the next remediation, which is exactly
//! the failure a cooldown is supposed to prevent.
//!
//! ## Why the headings, not the prose
//!
//! The README states counts in several shapes -- "11 テーブル (V001)",
//! "合計は **13 テーブル**", "6 本 PL/pgSQL 存过 (V002)". Matching all of them
//! against a single total would report the correct per-migration numbers as
//! errors. The section headings are the structured part: each names its
//! migration. Those are compared per migration, the totals come from a
//! machine-readable inventory block, and the gate fails when a new
//! migration arrives with no section of its own -- which is the defect that
//! happened here, and the one that would happen again at V004.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const README: &str = "db/README.md";
const MAKEFILE: &str = "db/Makefile";
const MIGRATIONS: &str = "db/migrations";
const TESTS: &str = "db/tests";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("ada-core lives two levels under the workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    let path = repo_root().join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

/// Version tag of a migration file: `V003__phase8_remediation.sql` -> `V003`.
fn version_of(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    let (version, _) = name.split_once("__")?;
    version.starts_with('V').then(|| version.to_string())
}

fn sql_files(dir_rel: &str) -> Vec<PathBuf> {
    let dir = repo_root().join(dir_rel);
    let mut out: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "sql"))
        .collect();
    out.sort();
    out
}

/// Table and function names declared by one migration, in source order.
///
/// The DDL uses `CREATE TABLE IF NOT EXISTS <name> (` and
/// `CREATE OR REPLACE FUNCTION <name>(`. It also uses `CREATE SEQUENCE IF
/// NOT EXISTS`, `CREATE INDEX IF NOT EXISTS`, `CREATE UNIQUE INDEX` and
/// `CREATE POLICY <name>`, none of which this counts.
fn declared_objects(path: &Path) -> (Vec<String>, Vec<String>) {
    let text =
        fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    let mut tables = Vec::new();
    let mut functions = Vec::new();
    for raw in text.lines() {
        // Comments first. V001 and V003 both open with a comment reading
        // "CREATE TABLE IF NOT EXISTS / CREATE OR REPLACE FUNCTION", which a
        // naive scan counts as two more declarations each.
        let line = raw.trim_start();
        if line.starts_with("--") {
            continue;
        }
        let mut words = line.split_whitespace();
        if !words
            .next()
            .is_some_and(|w| w.eq_ignore_ascii_case("CREATE"))
        {
            continue;
        }

        // CREATE [OR REPLACE] <kind> ...
        let mut kind = String::new();
        for w in words.by_ref() {
            if w.eq_ignore_ascii_case("OR") || w.eq_ignore_ascii_case("REPLACE") {
                continue;
            }
            kind = w.to_ascii_uppercase();
            break;
        }
        if kind != "TABLE" && kind != "FUNCTION" {
            continue;
        }

        // ... [IF NOT EXISTS] <name>
        let mut name = String::new();
        let mut seen_if = false;
        for w in words.by_ref() {
            let upper = w.to_ascii_uppercase();
            if !seen_if {
                if upper == "IF" {
                    seen_if = true;
                    continue;
                }
                // No `IF NOT EXISTS`: this word is already the name.
                name = w
                    .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                    .to_string();
                break;
            }
            if upper == "NOT" || upper == "EXISTS" {
                continue;
            }
            name = w
                .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .to_string();
            break;
        }
        if name.is_empty() {
            continue;
        }
        if kind == "TABLE" {
            tables.push(name);
        } else {
            functions.push(name);
        }
    }
    (tables, functions)
}

/// Per-version object counts, keyed by version tag.
fn objects_by_version() -> BTreeMap<String, (usize, usize)> {
    let mut out = BTreeMap::new();
    for path in sql_files(MIGRATIONS) {
        let Some(version) = version_of(&path) else {
            continue;
        };
        let (tables, functions) = declared_objects(&path);
        out.insert(version, (tables.len(), functions.len()));
    }
    out
}

/// The README's `<!-- gate:inventory -->` block, as `key -> number`.
fn inventory() -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    let mut inside = false;
    for line in read(README).lines() {
        let trimmed = line.trim();
        if trimmed == "<!-- gate:inventory -->" {
            inside = true;
            continue;
        }
        if trimmed == "<!-- end -->" {
            inside = false;
            continue;
        }
        if !inside {
            continue;
        }
        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        if let Ok(n) = value.trim().parse::<usize>() {
            out.insert(key.trim().to_string(), n);
        }
    }
    out
}

/// `### 11 テーブル (V001)` and `### 6 本 PL/pgSQL 存过 (V002)` headings,
/// as `(version, is_table_section, claimed_count)`.
fn section_headings() -> Vec<(String, bool, usize)> {
    let mut out = Vec::new();
    for line in read(README).lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("### ") else {
            continue;
        };
        // `### 11 テーブル (V001)` -- the version tag follows the *first*
        // bracket and ends at the first `)`, so a trailing annotation like
        // `### 1 本 PL/pgSQL 存过 (V004) (harness probe)` still parses. Using
        // the last bracket silently dropped the version and made the whole
        // section invisible to the gate.
        let Some((_, inside)) = rest.split_once('(') else {
            continue;
        };
        let version = inside.split(')').next().unwrap_or("").trim().to_string();
        if !version.starts_with('V') {
            continue;
        }
        let is_table_section = rest.contains("テーブル");
        if !is_table_section && !rest.contains("PL/pgSQL") {
            continue;
        }
        let digits: String = rest
            .split_whitespace()
            .find_map(|w| {
                let d: String = w.chars().take_while(char::is_ascii_digit).collect();
                (!d.is_empty()).then_some(d)
            })
            .unwrap_or_default();
        if let Ok(n) = digits.parse::<usize>() {
            out.push((version, is_table_section, n));
        }
    }
    out
}

/// Table names listed inside one Makefile recipe's `TRUNCATE` / `DROP TABLE`.
fn makefile_table_list(command: &str) -> Vec<String> {
    let text = read(MAKEFILE);
    let mut out = Vec::new();
    let mut lines: Vec<&str> = Vec::new();
    let mut collecting = false;
    for line in text.lines() {
        let trimmed = line.trim();
        // The SQL argument, not the `echo` line above it. The recipe reads
        // `@$(PSQL) -c "TRUNCATE \`, so anchoring on the command alone never
        // matched and the scan found no list at all.
        if trimmed.contains("-c \"") && trimmed.contains(command) {
            collecting = true;
            lines.clear();
            lines.push(trimmed);
            continue;
        }
        if collecting {
            // The recipe continues while the line ends with a backslash.
            let continues = trimmed.ends_with('\\');
            lines.push(trimmed.trim_end_matches('\\'));
            if !continues {
                collecting = false;
                let joined = lines.join(" ");
                for token in joined.split(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
                    if !token.is_empty() {
                        out.push(token.to_string());
                    }
                }
            }
        }
    }
    out
}

#[test]
fn readme_documents_every_migration_and_its_objects() {
    let by_version = objects_by_version();
    assert!(
        !by_version.is_empty(),
        "no migrations found under {MIGRATIONS}; the scan is broken, not the db"
    );

    let total_tables: usize = by_version.values().map(|(t, _)| t).sum();
    let total_functions: usize = by_version.values().map(|(_, f)| f).sum();

    assert!(
        total_tables >= 13,
        "expected at least 13 tables across the migrations, derived {total_tables}. \
         The floor is what the `db migrations` CI job asserts against a live \
         Postgres; if this drops, that assertion is checking nothing."
    );
    assert!(
        total_functions >= 8,
        "expected at least 8 functions across the migrations, derived \
         {total_functions}; the CI job asserts the same floor"
    );

    // Every migration needs a section for the objects it actually declares,
    // or the README is describing the past. A migration that declares none of
    // a kind -- V001 has tables but no functions, V002 the reverse -- needs
    // no section for it.
    let headings = section_headings();
    for (version, (tables, functions)) in &by_version {
        if *tables > 0 {
            let table_section = headings
                .iter()
                .find(|(v, is_table, _)| v == version && *is_table);
            let Some((_, _, claimed_tables)) = table_section else {
                panic!(
                    "{README} has no `### <n> テーブル ({version})` section, but \
                     {version} declares {tables} table(s). A migration that \
                     arrives without a section is how this README ended up \
                     describing V001 and V002 only."
                );
            };
            assert_eq!(
                *claimed_tables, *tables,
                "{README} says {version} declares {claimed_tables} tables; the \
                 SQL declares {tables}"
            );
        }

        if *functions > 0 {
            let function_section = headings
                .iter()
                .find(|(v, is_table, _)| v == version && !*is_table);
            let Some((_, _, claimed_functions)) = function_section else {
                panic!(
                    "{README} has no `### <n> 本 PL/pgSQL 存过 ({version})` \
                     section, but {version} declares {functions} function(s)"
                );
            };
            assert_eq!(
                *claimed_functions, *functions,
                "{README} says {version} declares {claimed_functions} \
                 functions; the SQL declares {functions}"
            );
        }
    }

    let inv = inventory();
    assert!(
        !inv.is_empty(),
        "{README} carries no machine-readable inventory block. The prose states \
         counts in several shapes and only the `<!-- gate:inventory -->` block \
         can be compared against what the SQL declares."
    );
    for (key, actual) in [
        ("migrations", by_version.len()),
        ("tables", total_tables),
        ("functions", total_functions),
        ("tests", sql_files(TESTS).len()),
    ] {
        let claimed = inv
            .get(key)
            .unwrap_or_else(|| panic!("{README} inventory has no `{key}` entry"));
        assert_eq!(
            *claimed, actual,
            "{README} inventory says {key} = {claimed}; the tree has {actual}"
        );
    }
}

#[test]
fn every_migration_has_a_test_file() {
    let migrations: Vec<String> = sql_files(MIGRATIONS)
        .iter()
        .filter_map(|p| version_of(p))
        .collect();
    assert!(
        !migrations.is_empty(),
        "no migrations found under {MIGRATIONS}; the scan is broken, not the db"
    );
    let tested: Vec<String> = sql_files(TESTS)
        .iter()
        .filter_map(|p| version_of(p))
        .collect();
    assert!(
        !tested.is_empty(),
        "no test files found under {TESTS}; a run with zero tests passes every \
         count assertion in run-tests.sh"
    );

    let untested: Vec<&String> = migrations.iter().filter(|m| !tested.contains(m)).collect();
    assert!(
        untested.is_empty(),
        "migrations {untested:?} have no matching file under {TESTS}. \
         run-tests.sh globs whatever is there, so a migration with no test \
         makes the suite quieter rather than red."
    );
}

#[test]
fn makefile_clean_and_drop_cover_every_declared_table() {
    let migrations = sql_files(MIGRATIONS);
    assert!(
        !migrations.is_empty(),
        "no migrations found under {MIGRATIONS}; the scan is broken, not the db"
    );

    // A table is declared exactly once across the migrations, so collecting
    // in file order gives a stable, deduplicated list.
    let mut declared: Vec<String> = Vec::new();
    for path in &migrations {
        let (tables, _functions) = declared_objects(path);
        for name in tables {
            if !declared.contains(&name) {
                declared.push(name);
            }
        }
    }
    assert!(
        declared.len() >= 13,
        "derived only {} tables from {MIGRATIONS}; the parse is broken, not the db",
        declared.len()
    );

    for command in ["TRUNCATE", "DROP TABLE IF EXISTS"] {
        let listed = makefile_table_list(command);
        assert!(
            !listed.is_empty(),
            "found no table list after `{command}` in {MAKEFILE}; the scan is \
             broken, not the Makefile"
        );
        let missing: Vec<&String> = declared.iter().filter(|t| !listed.contains(t)).collect();
        assert!(
            missing.is_empty(),
            "`make -C db {command}` does not mention {missing:?}. A cooldown or \
             history row that survives `make clean` suppresses the next \
             remediation -- the exact failure the cooldown exists to prevent."
        );
    }
}
