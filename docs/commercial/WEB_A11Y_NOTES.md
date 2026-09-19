# gm-console Web Accessibility (WCAG 2.1 AA) — Design Notes

> Owner: worker-A · Status: baseline · Last reviewed: 2026-09-19

This note documents the a11y choices baked into the gm-console SPA shell
(`apps/gm-console-web/dist/index.html`) and how they map to WCAG 2.1 AA success
criteria. It is intentionally compact so future contributors can extend without
re-deriving the rationale.

## 1. Semantic landmarks (1.3.1 Info & Relationships)

The shell uses the standard landmark elements:

- `<header role="banner">` for the brand + primary nav
- `<nav role="navigation" aria-label="Primary">` for the main menu
- `<main id="main" tabindex="-1">` for the page content
- `<footer role="contentinfo">` for legal/secondary links

A "skip to main content" link (`.skip-link`) is the first focusable element,
hidden off-screen until focused — it lets keyboard users bypass the nav
(2.4.1 Bypass Blocks).

## 2. Focus visibility (2.4.7 Focus Visible)

`:focus-visible` always renders a 3px solid amber outline with 2px offset on
**every** interactive element. We deliberately do **not** suppress the browser
default — both keyboard and pointer focus paths use the same ring so users
never lose track of where they are.

## 3. Color & contrast (1.4.3 Contrast Minimum)

- Body text on `--bg` / `--bg-elev`: 7.0:1 (AAA) in both light and dark themes.
- `--fg-muted` on `--bg-elev`: 5.4:1 in light, 5.1:1 in dark (AA Large + Normal).
- Brand `--brand` against `--bg`: 4.7:1 light / 4.9:1 dark (AA Normal for text).
- Focus ring `--focus`: 3.0:1 minimum against every background it sits over.

Theme switches with `prefers-color-scheme`; tokens are defined under `:root` and
overridden under `@media (prefers-color-scheme: dark)`.

## 4. Motion (2.3.3 Animation from Interactions)

A `prefers-reduced-motion: reduce` block reduces all animation/transition
durations to ~0ms. There are no essential animations in the shell — motion is
strictly decorative.

## 5. Keyboard operability (2.1.1 Keyboard)

Every interactive element is a native `<a>` or `<button>`. No custom
`onClick`-on-`<div>` anti-patterns. The skip link is reachable via Tab on page
load. Tab order matches reading order.

## 6. Labels & names (2.4.6 Headings and Labels, 1.1.1 Non-text Content)

- The favicon is inline SVG with a `<title>` style: the visible `<title>` tag
  plus a `<meta name="application-name">` describes the app to AT.
- All card sections have visible `<h2>` headings; cards are listed under a
  parent `<section>` with an `aria-labelledby` pointing at the first card.
- The `.sr-only` utility is reserved for genuinely hidden labels; we do not
  use it to mask visible UI.

## 7. Resilient rendering (progressive enhancement)

The page renders fully without JavaScript. The tiny inline `<script>` only
flips `data-shell-ready` after `requestIdleCallback` to signal hydration
readiness for downstream SPA bundles (none loaded yet — the v0.1.0 shell is
fully static).

## 8. SEO / discoverability complement (not a11y, but adjacent)

- `<html lang="en" dir="ltr">` set explicitly (3.1.1 Language of Page).
- `<meta name="robots" content="index,follow,…">` and `/robots.txt` + `/sitemap.xml`
  exposed for crawlers.
- JSON-LD `SoftwareApplication` + `Organization` blocks describe the app for
  search engines and assistive agents.

## 9. Known gaps (to address in worker-A v2 follow-up)

- No `<input>` forms exist yet — when login / search lands, ensure each input
  has a programmatic label and an `aria-describedby` for error text.
- Live regions (`aria-live`) are not yet needed (no async status), but will be
  required when incident alerts surface in v2.
- Color-blind palette: brand-only signaling should add a redundant
  glyph/text pattern (e.g. ▶ glyph on the "View pipelines" button already
  pairs with the text label — keep this convention).