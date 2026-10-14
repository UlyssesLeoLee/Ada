//! The console's document root, the files the image bakes into it, and the
//! files `routes.rs` compiles in must all describe the same frontend.
//!
//! ## The defect this gate exists for
//!
//! `gm-console` served static files from `GM_CONSOLE_STATIC_DIR`, and that
//! variable was set in neither the Dockerfile nor
//! `deploy/k8s/gm-console.yaml`. The image shipped no `dist/`, the
//! variable was empty, so `static_fallback` skipped `try_disk` entirely and
//! answered every path with the `index.html` compiled into the binary.
//!
//! The compiled-in `index.html` is not a *missing* console, which is why
//! this survived: `/` returned 200, `/tokens.css` returned 200 (it is
//! `include_str!`d too), and `/robots.txt` returned 200. The image looked
//! complete to any check that asked the running server whether a path
//! answered. It was serving one hand-authored HTML file and calling it a
//! frontend, with no CSS, no login page, and no image to point the CSS at.
//!
//! ## Why a test and not a build step
//!
//! There is no build step. This repository has no `package.json`, no
//! bundler config, and no frontend source tree; `apps/gm-console-web/dist/`
//! is hand-authored and tracked, and its own meta tag reads
//! `<meta name="generator" content="gm-console shell v0.1.0">`. So the
//! honest gate is agreement, not construction: the files the image copies,
//! the files the server embeds, the path the manifest points at, and the
//! path the image sets must be one consistent story. A build step invented
//! to satisfy the wording would build nothing and would be a fiction
//! wearing a test's clothes.
//!
//! What this cannot catch is a *stale* dist — only a missing or mismatched
//! one. That limit is stated rather than papered over.
//!
//! ## Why the `.dockerignore` half is here
//!
//! `include_str!` inputs are read at compile time from files outside the
//! crate. A local `cargo build` cannot see a missing one, because the file
//! is sitting right there; only a build from a filtered context fails, and
//! that is exactly what the container build is. `.dockerignore` once
//! excluded `docs/` and `*.md` and broke the first CI image this way
//! (`error: couldn't read ...docs/commercial/TERMS.md`). That file is not
//! editable from this crate, so the regression is asserted from here
//! instead: any of the six inputs falling behind an ignore pattern fails
//! `include_str_inputs_are_not_excluded_from_the_build_context`.
//!
//! ## Deliberately crude parsing
//!
//! The workspace has no YAML parser and cannot rely on the network to add
//! one, so the manifests are read as text. Both the Dockerfile `ENV` block
//! and the manifest's `env:` list are simple, and every test below asserts
//! the thing it parsed was non-empty, so a parser that silently found
//! nothing fails rather than passing vacuously.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("gm-console lives two levels under the workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    let p = repo_root().join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("could not read {}: {e}", p.display()))
}

const DOCKERFILE: &str = "deploy/docker/gm-console.Dockerfile";
const MANIFEST: &str = "deploy/k8s/gm-console.yaml";
const ROUTES: &str = "crates/gm-console/src/routes.rs";

/// Where the image puts the frontend, and what it calls the variable.
const STATIC_DIR_VAR: &str = "GM_CONSOLE_STATIC_DIR";

/// The document root, in the image. Asserted literally rather than derived:
/// the point of the gate is that the two files name *one* path, and a
/// constant here is what makes a change to either of them visible.
const EXPECTED_STATIC_DIR: &str = "/srv/static";

/// Files the image must contain for the console to be usable, and which the
/// `for f in ...` completeness loop in the Dockerfile names. Kept in one
/// list so the two cannot drift without this test noticing.
const REQUIRED_BAKED_FILES: &[&str] = &[
    "index.html",
    "login.html",
    "robots.txt",
    "sitemap.xml",
    "tokens.css",
];

/// Dockerfile instructions with backslash continuations joined, and comment
/// lines dropped.
///
/// Both shapes this file reads are written across several physical lines in
/// the Dockerfile: the `ENV` block puts each variable on its own
/// continuation line, and the completeness loop is one continued `RUN`
/// containing `for f in …; do`. Parsing a line at a time would find
/// neither the variable nor the file list, and would report a gate as
/// passing when it had read nothing.
///
/// Comment lines are dropped *after* joining, so a commented-out `ENV` or
/// `for f in` cannot be mistaken for a live instruction.
fn logical_lines(text: &str) -> Vec<String> {
    let joined = text.replace("\\\r\n", " ").replace("\\\n", " ");
    joined
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect()
}

/// Extract `NAME=value` from a Dockerfile `ENV` line.
fn dockerfile_env(text: &str, name: &str) -> Option<String> {
    for line in logical_lines(text) {
        let Some(rest) = line.strip_prefix("ENV ") else {
            continue;
        };
        for assignment in rest.split_whitespace() {
            let Some((key, value)) = assignment.split_once('=') else {
                continue;
            };
            if key == name {
                return Some(unquote(value));
            }
        }
    }
    None
}

/// Extract the value of a container `env:` entry from a manifest:
/// ```yaml
///             - name: GM_CONSOLE_STATIC_DIR
///               value: "/srv/static"
/// ```
/// `value:` must be the following line, which is how every entry in every
/// manifest here is written; a manifest that puts the value on the same
/// line is reported rather than half-parsed.
fn manifest_env(text: &str, name: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        let Some(rest) = t
            .strip_prefix("- name:")
            .or_else(|| t.strip_prefix("name:"))
        else {
            continue;
        };
        if unquote(rest) != name {
            continue;
        }
        let next = lines.get(i + 1)?.trim();
        let value = next.strip_prefix("value:")?;
        return Some(unquote(value));
    }
    None
}

fn unquote(s: &str) -> String {
    s.trim().trim_matches('"').trim_matches('\'').to_owned()
}

/// The source directory the image copies the frontend from, taken from the
/// `COPY --from=build <src> /srv/static` line. Returns the part before the
/// destination so the test can check the directory exists in the
/// repository, not just that a line mentioning it is present.
/// The build stage's `WORKDIR`. The `COPY --from=build` source is a path
/// *inside that stage*, so it is only meaningful relative to it.
fn build_stage_workdir(text: &str) -> Option<String> {
    let mut in_build = false;
    for line in logical_lines(text) {
        if line.starts_with("FROM ") {
            in_build = line.ends_with(" AS build");
            continue;
        }
        if !in_build {
            continue;
        }
        if let Some(rest) = line.strip_prefix("WORKDIR ") {
            return Some(unquote(rest));
        }
    }
    None
}

fn baked_dist_source(text: &str) -> Option<String> {
    for line in logical_lines(text) {
        let Some(rest) = line.strip_prefix("COPY --from=build ") else {
            continue;
        };
        let parts: Vec<&str> = rest.split_whitespace().collect();
        // COPY src... dest  -- the last field is the destination, and
        // Docker writes both paths with a trailing slash here, so it is
        // compared without one.
        let (srcs, dest) = parts.split_at(parts.len().checked_sub(1)?);
        if unquote(dest[0]).trim_end_matches('/') != EXPECTED_STATIC_DIR {
            continue;
        }
        return srcs.iter().map(|s| unquote(s)).find_map(|s| {
            let trimmed = s.trim_end_matches('/');
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        });
    }
    None
}

/// Every `include_str!` target in a Rust source file, as repository-relative
/// paths, normalised to `/` separators.
///
/// `include_str!` paths in `routes.rs` are relative to the *file*, and they
/// reach outside the crate (`../../../apps/gm-console-web/dist/robots.txt`),
/// which is the whole reason this list is interesting.
///
/// The argument is allowed to sit on the line *after* the macro name, and
/// one of the six is written that way:
///
/// ```text
///     xml_response(include_str!(
///         "../../../apps/gm-console-web/dist/sitemap.xml"
///     ))
/// ```
///
/// so the whitespace is skipped after the name rather than requiring the
/// quote to be adjacent. Matching only the single-line form silently drops
/// `sitemap.xml` from the set, which is the kind of gap that makes a gate
/// report agreement it never checked.
///
/// Only string literals are recognised; a path built through `concat!` or a
/// variable would be invisible here, and the tests below assert the list is
/// non-empty and long enough that such a gap fails loudly.
fn include_str_targets(rust: &str, source_rel: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut cursor = rust;
    while let Some(idx) = cursor.find("include_str!(") {
        let rest = &cursor[idx + "include_str!(".len()..];
        let rest = rest.trim_start();
        let Some(after_quote) = rest.strip_prefix('"') else {
            break;
        };
        let Some(end) = after_quote.find('"') else {
            break;
        };
        let rel = after_quote[..end].to_owned();
        cursor = &after_quote[end..];

        let source_dir = Path::new(source_rel).parent().unwrap_or(Path::new(""));
        let joined = source_dir.join(&rel);
        let mut parts: Vec<String> = Vec::new();
        for c in joined.components() {
            match c {
                std::path::Component::ParentDir => {
                    parts.pop();
                }
                std::path::Component::CurDir => {}
                other => parts.push(other.as_os_str().to_string_lossy().into_owned()),
            }
        }
        out.insert(parts.join("/"));
    }
    out
}

/// The files the image's completeness loop requires, as parsed back out of
/// the Dockerfile. Parsed rather than trusted from the constant above, so
/// that deleting an entry from the `for f in` loop is a test failure and
/// not a silent loss of coverage.
fn dockerfile_required_files(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in logical_lines(text) {
        let Some(rest) = line.split_once("for f in").map(|(_, r)| r) else {
            continue;
        };
        // The list ends at `; do`. Everything after it is the loop body.
        let Some(list) = rest.split_once("; do").map(|(l, _)| l) else {
            continue;
        };
        for name in list.split_whitespace() {
            out.insert(unquote(name));
        }
    }
    out
}

/// Does `rel` (repository-relative, `/`-separated) fall under the directory
/// `sub`?
fn under(rel: &str, sub: &str) -> bool {
    rel == sub || rel.starts_with(&format!("{sub}/"))
}

/// ## 1. The Dockerfile and the manifest must name one document root.
///
/// A mismatch does not fail loudly. `try_disk` returns `None` for a path it
/// cannot read and the request falls through to the compiled-in
/// `index.html`, so a `GM_CONSOLE_STATIC_DIR` pointing into empty space in
/// the image produces a healthy-looking 200 that serves the wrong bytes.
/// This is the shape of the original defect, and the only way to catch it
/// is to compare the two files against each other.
#[test]
fn baked_static_dir_agrees_between_the_image_and_the_manifest() {
    let dockerfile = read(DOCKERFILE);
    let manifest = read(MANIFEST);

    let from_image = dockerfile_env(&dockerfile, STATIC_DIR_VAR).unwrap_or_else(|| {
        panic!(
            "{DOCKERFILE} does not set {STATIC_DIR_VAR}. The image then ships \
             no document root, every request falls through to the \
             index.html compiled into the binary, and the console serves one \
             hand-authored HTML file as if it were the frontend."
        )
    });
    let from_manifest = manifest_env(&manifest, STATIC_DIR_VAR).unwrap_or_else(|| {
        panic!(
            "{MANIFEST} does not set {STATIC_DIR_VAR} on the gm-console \
             container, so the pod starts with no document root regardless of \
             what the image contains."
        )
    });

    assert_eq!(
        from_image, EXPECTED_STATIC_DIR,
        "{DOCKERFILE} bakes the frontend somewhere other than {EXPECTED_STATIC_DIR}; \
         update EXPECTED_STATIC_DIR here deliberately, and update the \
         completeness loop and the `COPY --from=build` line with it."
    );
    assert_eq!(
        from_manifest, from_image,
        "{MANIFEST} sets {STATIC_DIR_VAR}={from_manifest} but {DOCKERFILE} sets \
         {from_image}. try_disk returns None for a path the image does not \
         have, and the handler then serves the compiled-in index.html instead \
         of erroring -- so a wrong value here degrades to a 200 that serves \
         the wrong bytes, and a health probe on /healthz never notices."
    );
}

/// ## 2. The image must bake the committed dist, not an invented path.
///
/// If the `COPY --from=build` source ever names a directory that is not in
/// the repository, the copy either fails (good) or copies something else
/// (bad). The Dockerfile asserts its file list at build time; this asserts
/// the source directory is the real committed frontend.
#[test]
fn the_image_bakes_the_committed_dist() {
    let dockerfile = read(DOCKERFILE);
    let source = baked_dist_source(&dockerfile).unwrap_or_else(|| {
        panic!(
            "{DOCKERFILE} has no `COPY --from=build <src> {EXPECTED_STATIC_DIR}` \
             line, so nothing is baked into the document root."
        )
    });
    let expected_source = "apps/gm-console-web/dist";
    let dist = repo_root().join(expected_source);
    assert!(
        dist.is_dir(),
        "{expected_source} is not a directory in the repository, so there is \
         no frontend for the image to bake."
    );

    // The COPY source is a path inside the build stage, so it is checked
    // against that stage's WORKDIR rather than against the repository root:
    // `<workdir>/apps/gm-console-web/dist` is the one statement that ties the
    // bytes in the image to the directory in this repository.
    let workdir = build_stage_workdir(&dockerfile).unwrap_or_else(|| {
        panic!(
            "{DOCKERFILE} has no WORKDIR in the build stage, so the \
                `COPY --from=build` source cannot be resolved to a path"
        )
    });
    assert_eq!(
        source,
        format!("{workdir}/{expected_source}"),
        "the image copies `{source}` into {EXPECTED_STATIC_DIR}, but the \
         build stage works in `{workdir}` and the committed frontend is \
         `{expected_source}`. Either the dist moved, in which case this \
         constant should change on purpose, or the COPY line points at \
         something that is not the console."
    );
}

/// ## 3. Every file the image claims to require must exist, and the
///    Dockerfile's own list must not shrink behind this test's back.
///
/// Two halves. The first is the obvious one. The second exists because a
/// loop in a Dockerfile is invisible to review: dropping `login.html` from
/// `for f in` would leave the image building happily while serving a 404
/// on /login, and nothing else in the build would notice.
#[test]
fn every_required_baked_file_exists_in_the_committed_dist() {
    let dockerfile = read(DOCKERFILE);
    let declared = dockerfile_required_files(&dockerfile);
    assert!(
        !declared.is_empty(),
        "no `for f in ...` completeness loop found in {DOCKERFILE}. Without \
         it a dist missing an asset ships silently, so if the check was \
         removed that should be a deliberate, visible change."
    );

    let expected: BTreeSet<String> = REQUIRED_BAKED_FILES.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        declared, expected,
        "the completeness loop in {DOCKERFILE} no longer names the same file \
         list as REQUIRED_BAKED_FILES. The image's build-time check and this \
         gate have diverged, so one of them is now checking less than it was."
    );

    let dist = repo_root().join("apps/gm-console-web/dist");
    for name in &declared {
        let p = dist.join(name);
        assert!(
            p.is_file(),
            "{} is required by the image but missing from the committed dist. \
             The Dockerfile's completeness loop fails the image build on this, \
             and this test fails before that.",
            p.display()
        );
    }
}

/// ## 4. The `include_str!` set and the baked set must agree.
///
/// `routes.rs` embeds three of the six inputs from `dist/`, and serves
/// `index.html` and `login.html` as compiled-in fallbacks when no document
/// root is configured. Two consequences, both asserted here:
///
/// - every `include_str!` out of `dist/` must be a file the image bakes,
///   or the compiled-in copy and the served copy can differ with nothing to
///   say so;
/// - every `include_str!` must resolve to a file that exists *in the
///   repository*, which is the property a filtered build context can
///   break and a local `cargo build` cannot see.
#[test]
fn the_include_str_set_and_the_baked_set_agree() {
    let routes = read(ROUTES);
    let targets = include_str_targets(&routes, ROUTES);
    assert!(
        targets.len() >= 7,
        "expected the seven `include_str!` inputs in {ROUTES} (LICENSE, \
         TERMS.md, PRIVACY.md, robots.txt, sitemap.xml, index.html, \
         login.html) but found {}: {targets:?}. If the count changed on \
         purpose, update this test; if the extractor broke, every assertion \
         below is currently vacuous.",
        targets.len()
    );

    const DIST: &str = "apps/gm-console-web/dist";
    let from_dist: Vec<&String> = targets.iter().filter(|t| under(t, DIST)).collect();
    assert!(
        !from_dist.is_empty(),
        "no `include_str!` in {ROUTES} points into {DIST}. The extractor is \
         broken, and the two halves of this gate would no longer be talking \
         about the same files."
    );

    // The COPY source is a build-stage path; strip the stage's WORKDIR to
    // get the repository-relative directory the `include_str!` targets are
    // also expressed in, so the two are comparable.
    let dockerfile = read(DOCKERFILE);
    let workdir = build_stage_workdir(&dockerfile)
        .expect("the build stage has a WORKDIR, asserted in the test above");
    let baked = baked_dist_source(&dockerfile).expect("asserted in the test above");
    let baked_rel = baked
        .strip_prefix(&format!("{workdir}/"))
        .unwrap_or(baked.as_str());
    for target in &from_dist {
        assert!(
            under(target, baked_rel),
            "{ROUTES} embeds `{target}`, but the image copies `{baked}` into \
             {EXPECTED_STATIC_DIR} -- so the compiled-in copy of that file and \
             the served copy are different files with no check between them."
        );
    }

    // The reverse half, and the one a filtered build context breaks.
    for target in &targets {
        let p = repo_root().join(target);
        assert!(
            p.is_file(),
            "{ROUTES} embeds `{target}`, which does not exist at {}.",
            p.display()
        );
    }
}

/// ## 5. `.dockerignore` must not hide any `include_str!` input from the
///    image build.
///
/// `.dockerignore` is not editable from this crate, so this asserts the
/// property instead of fixing it in place. It exists because the exclusion
/// already happened once: `docs/` and `*.md` were excluded, and the first
/// CI image of this service failed with
///
/// ```text
/// error: couldn't read `crates/gm-console/src/../../../docs/commercial/TERMS.md`
/// ```
///
/// which no local build can reproduce, because locally the file is there.
///
/// Deliberately narrow: only the pattern forms that actually appear in the
/// file are interpreted — a directory, `**/<dir>`, a `<dir>/*` subtree, a
/// literal path, and a `*.<ext>` / `*` suffix glob. An unrecognised form is
/// treated as *not* excluding, and reported by
/// `every_dockerignore_pattern_is_understood` so it cannot hide here.
#[test]
fn include_str_inputs_are_not_excluded_from_the_build_context() {
    let ignore = read(".dockerignore");
    let targets = include_str_targets(&read(ROUTES), ROUTES);
    assert!(
        !targets.is_empty(),
        "no `include_str!` inputs found, so this gate is measuring nothing"
    );

    for target in &targets {
        if let Some(pattern) = excluding_pattern(&ignore, target) {
            panic!(
                ".dockerignore excludes `{pattern}`, which hides `{target}` from \
                 the image build. `include_str!` reads it at compile time, so \
                 the container build fails with\n  \
                 error: couldn't read `crates/gm-console/src/../../../{target}`\n\
                 while every local `cargo build` passes, because locally the \
                 file is sitting right there. A local build cannot see this; \
                 only the CI `images` job can."
            );
        }
    }
}

/// The first pattern in `dockerignore` that excludes `rel`, if any.
fn excluding_pattern(dockerignore: &str, rel: &str) -> Option<String> {
    for line in dockerignore.lines() {
        let raw = line.trim();
        if raw.is_empty() || raw.starts_with('#') {
            continue;
        }
        if pattern_excludes(raw, rel) {
            return Some(raw.to_string());
        }
    }
    None
}

fn pattern_excludes(pattern: &str, rel: &str) -> bool {
    // Trailing `/` means "this directory and everything under it".
    let p = pattern.trim_end_matches('/');
    // A leading `/` anchors at the context root; the repository-relative
    // paths this gate compares against are already anchored.
    let p = p.strip_prefix('/').unwrap_or(p);
    if p.is_empty() {
        return false;
    }

    // `**/name` and `name/**`: match at any depth, on the basename.
    if let Some(base) = p.strip_prefix("**/") {
        let base = base.trim_end_matches("/*").trim_end_matches('/');
        if base.is_empty() || base.contains('/') {
            return false;
        }
        return rel.split('/').any(|seg| seg == base);
    }

    // `dir/*` — the subtree, not the directory entry itself.
    if let Some(base) = p.strip_suffix("/*") {
        return under(rel, base);
    }

    // A glob: only the suffix forms used in this file are interpreted.
    if p.contains('*') || p.contains('?') {
        return glob_excludes(p, rel);
    }

    // A bare name or path: the directory and everything under it, or the
    // file itself.
    rel == p || under(rel, p)
}

/// `*.<ext>` and `*` only. Anything else containing a wildcard is reported
/// by `every_dockerignore_pattern_is_understood` rather than guessed at.
fn glob_excludes(pattern: &str, rel: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(ext) = pattern.strip_prefix('*') {
        if !ext.contains('*') && !ext.contains('?') && !ext.is_empty() {
            return rel.ends_with(ext);
        }
    }
    false
}

/// ## 6. No pattern in `.dockerignore` is being silently ignored by the
///    matcher above.
///
/// Without this, someone adds `**/*.png` and gate 5 goes quiet about it
/// without anyone noticing that it went quiet. Cheaper to fail here than to
/// have the pattern's real effect discovered by a broken image build.
#[test]
fn every_dockerignore_pattern_is_understood() {
    let ignore = read(".dockerignore");

    let mut seen = 0usize;
    for line in ignore.lines() {
        let raw = line.trim();
        if raw.is_empty() || raw.starts_with('#') {
            continue;
        }
        seen += 1;
        let p = raw.trim_end_matches('/').trim_start_matches('/');
        let understood = p == "*"
            || p.is_empty()
            || p.strip_prefix("**/").is_some_and(|b| !b.contains('*'))
            || p.ends_with("/*")
            || (!p.contains('*') && !p.contains('?'))
            || p.strip_prefix('*')
                .is_some_and(|ext| !ext.contains(['*', '?']) && !ext.is_empty());
        assert!(
            understood,
            ".dockerignore pattern `{raw}` uses a form the matcher in \
             include_str_inputs_are_not_excluded_from_the_build_context does \
             not interpret, so that gate cannot see what it would exclude. \
             Either extend the matcher or remove the pattern -- do not leave \
             it in a state where the gate is blind to it."
        );
    }
    assert!(
        seen > 0,
        ".dockerignore has no patterns at all -- if that is deliberate, this \
         gate is now measuring nothing and should be revisited."
    );
}
