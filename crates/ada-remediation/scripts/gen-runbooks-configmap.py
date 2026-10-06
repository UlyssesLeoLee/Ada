#!/usr/bin/env python3
"""Regenerate the `ada-remediation-runbooks` ConfigMap in
`deploy/k8s/ada-remediation.yaml` from the runbooks in `config/remediation/`.

## Why this exists

The Deployment mounts that ConfigMap at the directory
`REMEDIATION_RUNBOOK_DIR` names, so a runbook that is not a key of it is a
runbook the deployed service never loads. It used to be referenced and never
defined at all, which meant the deployed service matched zero actions while
passing both probes -- see `crates/ada-core/tests/mounted_configmaps.rs`.

Hand-copying five JSON files into a manifest is how that state is produced
again after the next runbook is added. Generating means the committed
ConfigMap is always a rendering of the committed runbooks.

## Correctness is not left to this script

`every_runbook_on_disk_is_mounted_by_the_deployment` in
`crates/ada-core/tests/mounted_configmaps.rs` fails the build if the
ConfigMap and the directory disagree -- on a missing key, on drifted
content, or on a key with no file behind it. So a stale ConfigMap cannot be
merged even if this script is never run, is run against the wrong file, or
is skipped entirely. This script exists to make the right thing easy, not to
be the thing that makes it correct.

## Usage

    python crates/ada-remediation/scripts/gen-runbooks-configmap.py           # rewrite in place
    python crates/ada-remediation/scripts/gen-runbooks-configmap.py --check   # exit 1 if stale

`--check` is the form suitable for CI or a pre-commit hook. It needs no
write access and reports which keys are missing, extra, or drifted.

PyYAML is used only to self-verify the rendered document. Without it the
script still rewrites the file and says it could not verify; the Rust gate
is the authority either way.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

def find_repo_root() -> Path:
    """Walk up from this file until the workspace root.

    Counting parents is brittle: this file sits at
    `<repo>/crates/ada-remediation/scripts/`, so the depth is an accident of
    where the crate lives, and the first version of this script got the count
    wrong by one and looked for runbooks in `crates/config/remediation`.
    Identifying the root by what it contains cannot drift that way.
    """
    for candidate in Path(__file__).resolve().parents:
        if (candidate / "Cargo.toml").is_file() and (
            candidate / "config" / "remediation"
        ).is_dir():
            return candidate
    sys.exit("FAIL: could not find the workspace root (no Cargo.toml + config/remediation above this script)")


REPO = find_repo_root()
RUNBOOK_DIR = REPO / "config" / "remediation"
MANIFEST = REPO / "deploy" / "k8s" / "ada-remediation.yaml"
CONFIGMAP_NAME = "ada-remediation-runbooks"

# The comment line that opens the ConfigMap document in the manifest. It is
# the splice anchor, so it must be unique and must not be edited away.
MARKER = "# ada-remediation-runbooks -- the runbooks the remediation engine executes."

# Kubernetes rejects a ConfigMap over 1 MiB; the etcd object limit is the
# practical ceiling, and a runbook set that large is a design problem.
MAX_BYTES = 1024 * 1024


def read_runbooks() -> dict[str, str]:
    files = sorted(RUNBOOK_DIR.glob("*.json"))
    if not files:
        sys.exit(f"FAIL: no *.json runbooks in {RUNBOOK_DIR}")
    out: dict[str, str] = {}
    for path in files:
        if path.name in out:
            sys.exit(f"FAIL: duplicate runbook name {path.name}")
        body = path.read_text(encoding="utf-8")
        # A runbook must end with exactly one newline, and the reason is
        # mechanical rather than stylistic: the ConfigMap value is a literal
        # block scalar with clip chomping, which always carries a trailing
        # line break. A file without one can therefore never compare equal to
        # its own rendered value, and the failure would surface much later as
        # an opaque "did not round-trip byte for byte" from either this
        # script or `every_runbook_on_disk_is_mounted_by_the_deployment`.
        # Saying so here, while the author is still looking at the file, is
        # the difference between a five-second fix and a mystery.
        if not body.endswith("\n"):
            sys.exit(
                f"FAIL: {path.name} does not end with a newline.\n"
                f"  A ConfigMap literal block scalar always ends with a line "
                f"break, so this file cannot round-trip.\n"
                f"  Add a trailing newline: see "
                f"`{path.relative_to(REPO).as_posix()}`."
            )
        if body.endswith("\n\n"):
            sys.exit(
                f"FAIL: {path.name} ends with a blank line.\n"
                f"  The trailing empty line is a separator, not content, and "
                f"the comparison against the ConfigMap would fail for a "
                f"reason that looks unrelated to a blank line."
            )
        out[path.name] = body
    return out


def render(runbooks: dict[str, str]) -> str:
    total = sum(len(v.encode("utf-8")) for v in runbooks.values())
    if total > MAX_BYTES:
        sys.exit(f"FAIL: {total} bytes exceeds the {MAX_BYTES} byte ConfigMap limit")

    lines = [
        MARKER,
        "#",
        "# GENERATED from config/remediation by",
        "# crates/ada-remediation/scripts/gen-runbooks-configmap.py -- do not edit by hand.",
        "#",
        "# `every_runbook_on_disk_is_mounted_by_the_deployment` in",
        "# crates/ada-core/tests/mounted_configmaps.rs fails the build if this and",
        "# the directory disagree, so this is a rendering of the runbooks rather than",
        "# a second copy of them. Regenerate it after editing anything in there.",
        "apiVersion: v1",
        "kind: ConfigMap",
        "metadata:",
        f"  name: {CONFIGMAP_NAME}",
        "  namespace: observability",
        "  labels:",
        "    app.kubernetes.io/name: ada-remediation",
        "    app.kubernetes.io/version: v0.7.1",
        "data:",
    ]
    for key, body in runbooks.items():
        # `|` (clip) keeps the trailing newline so the value round-trips to
        # the file byte for byte; `|-` would strip it.
        lines.append(f"  {key}: |")
        for line in body.split("\n"):
            lines.append(f"    {line}" if line else "")
    # The trailing blank line separates this document from the next `---`.
    # It is inside the spliced range, so without it every run would delete
    # the separator and produce a diff that looks like a content change.
    lines.append("")
    return "\n".join(lines) + "\n"


def document_bounds(text: str) -> tuple[int, int]:
    """Line range [start, end) of the ConfigMap document, asserted before use.

    Everything this function refuses is a case where a blind replace would
    corrupt an unrelated part of the manifest.
    """
    lines = text.splitlines()
    hits = [i for i, l in enumerate(lines) if l.strip() == MARKER]
    if len(hits) != 1:
        sys.exit(
            f"FAIL: expected exactly one `{MARKER}` anchor in {MANIFEST}, "
            f"found {len(hits)}. Refusing to splice."
        )
    start = hits[0]
    end = next(
        (i for i in range(start + 1, len(lines)) if lines[i].strip() == "---"),
        None,
    )
    if end is None:
        sys.exit(f"FAIL: no `---` after the ConfigMap at line {start + 1}. Refusing to splice.")

    body = "\n".join(lines[start:end]) + "\n"
    try:
        import yaml
    except ImportError:
        print("note: PyYAML absent, skipping the shape check on the existing document")
        return start, end
    parsed = yaml.safe_load(body)
    if not isinstance(parsed, dict) or parsed.get("kind") != "ConfigMap":
        sys.exit(f"FAIL: the document at line {start + 1} is not a ConfigMap")
    if parsed.get("metadata", {}).get("name") != CONFIGMAP_NAME:
        name = parsed.get("metadata", {}).get("name")
        sys.exit(f"FAIL: the document at line {start + 1} is {name!r}, not {CONFIGMAP_NAME!r}")
    return start, end


def self_verify(rendered: str, runbooks: dict[str, str]) -> None:
    """Parse the rendered document and confirm it round-trips byte for byte."""
    try:
        import yaml
    except ImportError:
        print("note: PyYAML absent, skipping round-trip verification")
        return
    parsed = yaml.safe_load(rendered)
    if parsed.get("kind") != "ConfigMap":
        sys.exit(f"FAIL: rendered kind is {parsed.get('kind')!r}")
    data = parsed["data"]
    if set(data) != set(runbooks):
        sys.exit(
            f"FAIL: rendered keys {sorted(data)} != runbook files {sorted(runbooks)}"
        )
    for key, body in runbooks.items():
        if data[key] != body:
            sys.exit(f"FAIL: {key} did not round-trip byte for byte")


def diff_report(runbooks: dict[str, str]) -> int:
    """Describe how far the manifest is from the runbooks. 0 means in sync."""
    text = MANIFEST.read_text(encoding="utf-8")
    start, end = document_bounds(text)
    current = "\n".join(text.splitlines()[start:end]) + "\n"

    try:
        import yaml
    except ImportError:
        if current != render(runbooks):
            print("STALE: the ConfigMap differs from what this script would generate")
            return 1
        return 0

    parsed = yaml.safe_load(current) or {}
    data = parsed.get("data", {})
    missing = sorted(set(runbooks) - set(data))
    extra = sorted(set(data) - set(runbooks))
    drifted = sorted(k for k in set(data) & set(runbooks) if data[k] != runbooks[k])
    unversioned = sorted(k for k in data if k not in runbooks)

    if not (missing or extra or drifted or unversioned):
        print(f"in sync: {len(runbooks)} runbooks match the ConfigMap byte for byte")
        return 0

    print("STALE -- the committed ConfigMap does not match config/remediation/")
    if missing:
        print(f"  in the directory, absent from the ConfigMap: {', '.join(missing)}")
    if extra:
        print(f"  in the ConfigMap, absent from the directory: {', '.join(extra)}")
    if drifted:
        print(f"  present on both sides with different content: {', '.join(drifted)}")
    if unversioned:
        print(
            "  in the ConfigMap with no file behind it, so not version-controlled "
            f"and not reviewable: {', '.join(unversioned)}"
        )
    print("  run without --check to regenerate")
    return 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="report staleness and exit 1, without writing",
    )
    args = parser.parse_args()

    runbooks = read_runbooks()

    if args.check:
        return diff_report(runbooks)

    text = MANIFEST.read_text(encoding="utf-8")
    start, end = document_bounds(text)
    rendered = render(runbooks)
    self_verify(rendered, runbooks)

    lines = text.splitlines()
    # `splitlines`, not `rstrip("\n").splitlines()`: the rendered block ends
    # with a blank separator line and rstrip would take that too, so every
    # run would delete the blank line before the next `---`.
    updated = "\n".join(lines[:start] + rendered.splitlines() + lines[end:])
    # `splitlines` discards the final newline and `join` does not put one
    # back, so a naive splice silently strips it and leaves the manifest
    # without a trailing line break.
    if text.endswith("\n") and not updated.endswith("\n"):
        updated += "\n"
    if updated == text:
        print("already up to date")
        return 0

    # newline="\n" so the committed blob keeps LF on every platform.
    MANIFEST.write_text(updated, encoding="utf-8", newline="\n")
    print(
        f"rewrote {MANIFEST.relative_to(REPO)}: {len(runbooks)} runbooks, "
        f"{sum(len(v.encode('utf-8')) for v in runbooks.values())} bytes"
    )
    for key in sorted(runbooks):
        print(f"  {key}")
    return 0


if __name__ == "__main__":
    sys.exit(main())