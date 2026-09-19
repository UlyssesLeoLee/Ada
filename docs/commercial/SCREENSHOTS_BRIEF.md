# gm-console Screenshot Brief

worker-C owns actual screenshot capture & design. This file lists the **must-have** shots
to satisfy Apple/Google review guidance + commercial-product baseline.

## Required dimensions

- **Apple iPhone 6.7"** (iPhone 15 Pro Max class): 1290 × 2796 px (or matching template)
- **Apple iPhone 6.5"** (Plus / XS Max): 1242 × 2688 px
- **Apple iPad 12.9"**: 2048 × 2732 px (recommended but optional for v0.1)
- **Android phone**: 1080 × 1920 px landscape + portrait each
- **Google Play 7-inch tablet**: 1200 × 1920 (optional)

PNG, no transparency, sRGB.

## Shot list (priority order)

1. **Login / SSO entry** — clean Material 3 / Cupertino adaptive surface
2. **Tenant switcher** — switches between workspaces
3. **Pipeline dashboard** — list of pipelines + status pills (green / amber / red)
4. **Pipeline detail** — node graph, last run, error log + retry
5. **Incidents feed** — chronological list, ack / resolve actions
6. **Audit timeline** — recent remediation events with operator pills
7. **Profile / settings** — token revoke, flavor toggle, build info

## Localization rounds

M0: English (default).
M1: Japanese + Simplified Chinese.
M2+: per store config.

## Acceptance

- App is hero-shot-ready on iPhone 6.7" and Android phone portrait.
- Each shot has a short caption (≤ 30 chars) Apple / Google can show below the image.
- All text respects localizable strings — no lorem ipsum, no hardcoded brand color outside
  the seed scheme.
