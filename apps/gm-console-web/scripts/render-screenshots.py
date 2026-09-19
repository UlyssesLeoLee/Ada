#!/usr/bin/env python3
"""render-screenshots.py — gm-console Web SPA screenshot pipeline (Python mirror).

This is the Python twin of render-screenshots.js. Either driver can be wired
into CI; the JavaScript file is the canonical implementation because Playwright
ships an official Node binding. The Python mirror exists so:

  * CI runners without Node can still execute the pipeline.
  * Local contributors who prefer `python` over `node` have a familiar entry.

The shot catalog, device dimensions, and golden-image conventions are kept in
sync with render-screenshots.js and docs/commercial/SCREENSHOTS_BRIEF.md.
"""

from __future__ import annotations

import argparse
import os
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


@dataclass(frozen=True)
class Shot:
    shot_id: str
    route: str
    caption: str
    device_id: str
    width: int
    height: int

    @property
    def filename(self) -> str:
        return f"{self.shot_id}__{self.device_id}__{self.width}x{self.height}.png"


# Mirror of the JavaScript catalog. Keep these two in sync.
SHOTS: tuple[Shot, ...] = (
    Shot("login",           "/login",       "Sign in to your tenant",          "iphone-6.7",            1290, 2796),
    Shot("login",           "/login",       "Sign in to your tenant",          "iphone-6.5",            1242, 2688),
    Shot("login",           "/login",       "Sign in to your tenant",          "android-phone",         1080, 1920),
    Shot("tenants",         "/tenants",     "Switch between workspaces",        "iphone-6.7",            1290, 2796),
    Shot("pipelines",       "/pipelines",   "Pipelines at a glance",           "iphone-6.7",            1290, 2796),
    Shot("pipelines",       "/pipelines",   "Pipelines at a glance",           "android-phone-portrait", 1080, 1920),
    Shot("pipeline-detail", "/pipelines/1", "Inspect a run, retry in one tap", "iphone-6.7",            1290, 2796),
    Shot("incidents",       "/incidents",   "Ack and resolve live alerts",     "iphone-6.7",            1290, 2796),
    Shot("audit",           "/audit",       "Immutable remediation trail",     "iphone-6.7",            1290, 2796),
    Shot("settings",        "/settings",    "Tokens, flavor, build info",      "iphone-6.7",            1290, 2796),
    Shot("pipelines",       "/pipelines",   "Pipelines at a glance",           "ipad-12.9",             2048, 2732),
)


def _load_driver():
    """Try Playwright (sync) first, then a manual Chromium subprocess fallback."""
    try:
        from playwright.sync_api import sync_playwright  # type: ignore
        return ("playwright", sync_playwright)
    except Exception:
        pass
    try:
        from playwright.async_api import async_playwright  # type: ignore
        return ("playwright-async", async_playwright)
    except Exception:
        pass
    raise SystemExit(2)  # No driver installed; CI installs playwright via pip.


def _render_one(driver, base_url: str, out_dir: Path, shot: Shot, update_goldens: bool) -> Path:
    target_dir = out_dir / ("goldens" if update_goldens else shot.device_id)
    target_dir.mkdir(parents=True, exist_ok=True)
    out_path = target_dir / shot.filename

    kind, api = driver
    if kind.startswith("playwright"):
        with api() as p:
            browser = p.chromium.launch(headless=True)
            ctx = browser.new_context(viewport={"width": shot.width, "height": shot.height},
                                      device_scale_factor=1, color_scheme="light")
            page = ctx.new_page()
            page.goto(f"{base_url.rstrip('/')}{shot.route}", wait_until="networkidle")
            page.screenshot(path=str(out_path), full_page=False, type="png")
            browser.close()
    else:
        raise RuntimeError(f"unsupported driver kind: {kind}")
    return out_path


def _quick_diff(rendered: Path, golden: Path) -> str:
    if not golden.exists():
        return "no-golden"
    a = rendered.stat().st_size
    b = golden.stat().st_size
    drift = abs(a - b) / max(a, 1)
    return "drift" if drift > 0.02 else "ok"


def _parse_args(argv: list[str]) -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Render gm-console Web SPA screenshots.")
    p.add_argument("--base-url", default="http://127.0.0.1:8080")
    p.add_argument("--out-dir", default="apps/gm-console-web/dist/screenshots")
    p.add_argument("--device-filter", default=None, help="comma-separated device ids")
    p.add_argument("--update-goldens", action="store_true")
    return p.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = _parse_args(list(argv) if argv is not None else sys.argv[1:])
    out_dir = Path(args.out_dir).resolve()
    devices: Iterable[str] | None = (
        [d.strip() for d in args.device_filter.split(",") if d.strip()] if args.device_filter else None
    )
    shots = [s for s in SHOTS if devices is None or s.device_id in devices]
    driver = _load_driver()

    any_drift = False
    for shot in shots:
        rendered = _render_one(driver, args.base_url, out_dir, shot, args.update_goldens)
        golden = out_dir / "goldens" / shot.filename
        status = _quick_diff(rendered, golden)
        print(f"[render] {shot.shot_id} @ {shot.device_id} ({shot.width}x{shot.height}) -> {rendered} ({status})")
        if status == "drift":
            any_drift = True
    return 4 if any_drift else 0


if __name__ == "__main__":
    raise SystemExit(main())