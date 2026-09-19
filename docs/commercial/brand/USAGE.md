# Brand Tokens — Usage Guide

> How to consume [`brand/TOKENS.md`](./TOKENS.md) across the three product
> surfaces (Web / Flutter Mobile / future native iOS). Single source of
> truth; do **not** hardcode hex values in screens.

## 1. Web (gm-console SPA shell)

`apps/gm-console-web/dist/tokens.css` exposes every token as a CSS custom
property. The shell loads it via a synchronous `<link>` so first paint
already has the canonical palette.

```html
<link rel="stylesheet" href="./tokens.css">
<style>
  body  { background: var(--surface-bg); color: var(--text-body); }
  h1    { color: var(--text-strong); font-size: var(--type-h1); }
  .cta  { background: var(--brand-500); color: var(--surface-card);
          padding: var(--space-3) var(--space-5);
          border-radius: var(--radius-md);
          transition: filter var(--dur-fast) var(--ease-standard); }
  .cta:hover { background: var(--brand-600); }
</style>
```

### Theme switching

- **First paint (no JS):** `tokens.css` itself reads
  `@media (prefers-color-scheme: dark)` and flips `--brand-500`,
  `--surface-bg`, `--surface-card`, `--text-strong`, `--text-body`.
- **Runtime override:** JS sets `document.documentElement.dataset.theme =
  "dark" | "light"`. `:root[data-theme="dark"]` rules then win. To make
  the auto-detect block *not* override an explicit user choice, the
  override rule uses `:root:not([data-theme="light"])` — set
  `data-theme="light"` to force light even when OS prefers dark.

## 2. Flutter (gm-console Mobile)

`apps/gm-console-app/lib/theme/brand_theme.dart` exposes
`BrandTheme.light()` and `BrandTheme.dark()` as the only sanctioned
entry points. `app.dart` selects between them via `ThemeMode.system`.

```dart
import 'package:flutter/material.dart';
import 'package:gm_console_app/theme.dart';

MaterialApp(
  theme: BrandTheme.light(),
  darkTheme: BrandTheme.dark(),
  themeMode: ThemeMode.system,
);

// Pull extension tokens from context:
final brand = Theme.of(context).extension<BrandThemeExtension>()!;
final ok    = brand.statusOk;          // Color
final dur   = brand.durBase;           // Duration
final r     = brand.radiusMd;          // 8.0
```

### Why an extension (not just `ColorScheme`)?

`ColorScheme` doesn't have slots for status colors, brand-muted surface,
or motion tokens. We start from `ColorScheme.fromSeed(seedColor:
Color(0xFF1E88E5))` to inherit Material 3 tonal palettes, then override
the surface/text/primary fields so they match `TOKENS.md` exactly. The
remaining tokens live in `BrandThemeExtension`.

## 3. Native iOS (future)

When the iOS app ships, mirror the same hex values into an
`Assets.xcassets` color set + a `Brand.swift` namespace. Reuse the JSON
spec at the bottom of `TOKENS.md` as the contract.

```swift
extension Brand {
    static let brand500 = UIColor(hex: 0x1E88E5)
    static let surfaceBg = UIColor(hex: 0xFAFCFF)
    // ...
}
```

## 4. Decision matrix — mandatory vs local

| Token                              | Where it must be set       |
|------------------------------------|----------------------------|
| `surface-bg`, `surface-card`       | Screen root / Scaffold     |
| `text-strong` (h1, h2, h3)         | Screen root typography     |
| `brand-500`, `brand-600`           | CTA buttons, active links  |
| `status-ok/warn/err`               | Status pills, alerts       |
| `--font-sans`, body line-height    | Screen root body           |
| `radius-md`, `space-3..5`          | Cards, buttons             |
| `dur-base`, `ease-standard`        | Page-level transitions     |
| `type-display`, `type-h2`          | Heading components         |
| Local accent / illustration color  | Component-local (allowed)  |

Rule: **anything that defines the brand's look** (palette, type scale,
primary CTA) is mandatory at the screen root. **Decorative or
illustration-specific** colors are local.

## 5. Accessibility posture

- **Contrast:** all body/background combinations target ≥ 4.5:1 (AA for
  normal text, 3:1 for large text). Light `text-body #52606D` on
  `surface-bg #FAFCFF` measures ≈ 7.0:1; dark `text-body #C9D1D9` on
  `surface-bg #0E1217` measures ≈ 10.5:1.
- **Focus ring:** `--focus-ring` (defaults to `--brand-500`) is always
  applied via `:focus-visible` with a 3px outline + 2px offset.
  Increase outline thickness to 4px when the focus target sits on a
  brand-colored background.
- **Reduced motion:** `tokens.css` zeros all `--dur-*` vars under
  `@media (prefers-reduced-motion: reduce)`. The Flutter
  `BrandThemeExtension._dark` ships motion durations as 0ms; light
  theme durations should be elided by wrapping transitions in
  `MediaQuery.of(context).disableAnimations` checks where the screen
  needs finer control.

## 6. i18n scaling

CJK glyphs (zh / ja) ship a different type rhythm than Latin:

- Headings (`type-h1`, `type-h2`) already use `clamp()` so they grow
  with viewport width; CJK labels render ~10–15% wider than English for
  the same `font-size`, so visual hierarchy is preserved without manual
  override.
- Body type (`--type-body: 16px / 1.6`) reads cleanly across zh-CN /
  ja-JP at the default 16px — do not scale up.
- Spacing tokens are language-agnostic; do not localize them.
- Font fallback chain in `--font-sans` already lists `Hiragino Sans`,
  `Microsoft YaHei`, and `PingFang`-class fallbacks. iOS / Android ship
  these natively; the Web shell relies on system fallback.

## 7. Don'ts

- **Don't** inline a hex value in a screen — pull from the token.
- **Don't** introduce a new `--brand-*` shade without first editing
  `TOKENS.md`.
- **Don't** branch on `prefers-color-scheme` in component CSS; let
  `tokens.css` handle the theme layer.
- **Don't** override `--font-sans` per-screen.