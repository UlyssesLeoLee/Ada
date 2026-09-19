# Brand Tokens (gm-console / Ada Platform)

> Single source of truth for color, typography, spacing, motion, radius. Implemented as
> CSS custom properties + an example JSON design-token spec.

## Color

### Light theme

| Token             | Value     | Use                          |
|-------------------|-----------|------------------------------|
| `--brand-500`     | `#1E88E5` | Primary brand, CTA           |
| `--brand-600`     | `#1976D2` | Hover, pressed               |
| `--brand-100`     | `#E3F2FD` | Subtle backgrounds           |
| `--surface-bg`    | `#FAFCFF` | Page background              |
| `--surface-card`  | `#FFFFFF` | Cards                        |
| `--surface-muted` | `#EEF4FB` | Subtle wells                 |
| `--text-strong`   | `#1F2933` | Headings                     |
| `--text-body`     | `#52606D` | Paragraphs                   |
| `--text-faint`    | `#7B8794` | Meta, footer                 |
| `--status-ok`     | `#2E7D32` | Success / healthy            |
| `--status-warn`   | `#ED6C02` | Degraded                     |
| `--status-err`    | `#C62828` | Error / down                 |

### Dark theme

| Token             | Value     |
|-------------------|-----------|
| `--brand-500`     | `#42A5F5` |
| `--surface-bg`    | `#0E1217` |
| `--surface-card`  | `#161B22` |
| `--text-strong`   | `#F0F6FC` |
| `--text-body`     | `#C9D1D9` |

## Typography

| Token             | Value                                              |
|-------------------|----------------------------------------------------|
| `--font-sans`     | `ui-sans-serif, system-ui, "Segoe UI", Roboto, "Hiragino Sans", "Microsoft YaHei", sans-serif` |
| `--font-mono`     | `ui-monospace, SFMono-Regular, Menlo, monospace`   |
| `--type-display`  | `clamp(32px, 5vw, 56px)`                           |
| `--type-h1`       | `clamp(24px, 3vw, 32px)`                           |
| `--type-h2`       | `clamp(20px, 2.5vw, 24px)`                         |
| `--type-body`     | `16px / 1.6`                                       |
| `--type-small`    | `13px / 1.5`                                       |

## Spacing scale (4px grid)

| Token             | Value |
|-------------------|-------|
| `--space-1`       | 4px   |
| `--space-2`       | 8px   |
| `--space-3`       | 12px  |
| `--space-4`       | 16px  |
| `--space-5`       | 24px  |
| `--space-6`       | 32px  |
| `--space-8`       | 48px  |

## Radius

| Token         | Value |
|---------------|-------|
| `--radius-sm` | 4px   |
| `--radius-md` | 8px   |
| `--radius-lg` | 12px  |

## Motion

| Token             | Value                       |
|-------------------|-----------------------------|
| `--ease-standard` | `cubic-bezier(.2, 0, 0, 1)` |
| `--dur-fast`      | `120ms`                     |
| `--dur-base`      | `180ms`                     |
| `--dur-slow`      | `240ms`                     |

Respect `@media (prefers-reduced-motion: reduce)` by zeroing motion tokens under that media
query.

## Example JSON design-token spec

```json
{
  "color": {
    "brand": { "500": { "value": "#1E88E5" } },
    "surface": {
      "bg":   { "value": "#FAFCFF" },
      "card": { "value": "#FFFFFF" }
    }
  },
  "space": { "4": { "value": "4px" } },
  "radius": { "md": { "value": "8px" } },
  "motion": { "ease": { "standard": { "value": "cubic-bezier(.2,0,0,1)" } } }
}
```

## Use in code

- **Web**: `var(--brand-500)`
- **Mobile (Flutter)**: mirror via `ColorScheme.fromSeed(seedColor: Color(0xFF1E88E5))`
  plus an extension class on `ThemeData`
