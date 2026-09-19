# gm-console Web Screenshot Catalog

> Owner: worker-E (brand / commercial surface)
> Status: **catalog only — no PNGs committed.**
> All listed PNG paths are placeholders. Real golden images are produced by
> the CI pipeline described in `apps/gm-console-web/scripts/SPEC.md`.

This catalog is the single source of truth for which shots the gm-console Web
SPA must produce for store-listing review. The list is derived directly from
`docs/commercial/SCREENSHOTS_BRIEF.md`.

## Index

| #  | Shot              | Route             | Caption                            | Device                | Dimensions      | Rendered PNG (placeholder)                                                                                | Script                                                    |
|----|-------------------|-------------------|------------------------------------|-----------------------|-----------------|-----------------------------------------------------------------------------------------------------------|-----------------------------------------------------------|
| 1  | login             | `/login`          | Sign in to your tenant             | iPhone 6.7"           | 1290 × 2796     | `iphone-6.7/login__iphone-6.7__1290x2796.png` *(NULL — pending CI render)*                                | `apps/gm-console-web/scripts/render-screenshots.js`        |
| 2  | login             | `/login`          | Sign in to your tenant             | iPhone 6.5"           | 1242 × 2688     | `iphone-6.5/login__iphone-6.5__1242x2688.png` *(NULL — pending CI render)*                                | `apps/gm-console-web/scripts/render-screenshots.js`        |
| 3  | login             | `/login`          | Sign in to your tenant             | Android phone         | 1080 × 1920     | `android-phone/login__android-phone__1080x1920.png` *(NULL — pending CI render)*                          | `apps/gm-console-web/scripts/render-screenshots.js`        |
| 4  | tenants           | `/tenants`        | Switch between workspaces          | iPhone 6.7"           | 1290 × 2796     | `iphone-6.7/tenants__iphone-6.7__1290x2796.png` *(NULL — pending CI render)*                              | `apps/gm-console-web/scripts/render-screenshots.js`        |
| 5  | pipelines         | `/pipelines`      | Pipelines at a glance              | iPhone 6.7"           | 1290 × 2796     | `iphone-6.7/pipelines__iphone-6.7__1290x2796.png` *(NULL — pending CI render)*                            | `apps/gm-console-web/scripts/render-screenshots.js`        |
| 6  | pipelines         | `/pipelines`      | Pipelines at a glance              | Android phone (port.) | 1080 × 1920     | `android-phone-portrait/pipelines__android-phone-portrait__1080x1920.png` *(NULL — pending CI render)*     | `apps/gm-console-web/scripts/render-screenshots.js`        |
| 7  | pipeline-detail   | `/pipelines/1`    | Inspect a run, retry in one tap    | iPhone 6.7"           | 1290 × 2796     | `iphone-6.7/pipeline-detail__iphone-6.7__1290x2796.png` *(NULL — pending CI render)*                      | `apps/gm-console-web/scripts/render-screenshots.js`        |
| 8  | incidents         | `/incidents`      | Ack and resolve live alerts        | iPhone 6.7"           | 1290 × 2796     | `iphone-6.7/incidents__iphone-6.7__1290x2796.png` *(NULL — pending CI render)*                            | `apps/gm-console-web/scripts/render-screenshots.js`        |
| 9  | audit             | `/audit`          | Immutable remediation trail        | iPhone 6.7"           | 1290 × 2796     | `iphone-6.7/audit__iphone-6.7__1290x2796.png` *(NULL — pending CI render)*                                | `apps/gm-console-web/scripts/render-screenshots.js`        |
| 10 | settings          | `/settings`       | Tokens, flavor, build info         | iPhone 6.7"           | 1290 × 2796     | `iphone-6.7/settings__iphone-6.7__1290x2796.png` *(NULL — pending CI render)*                             | `apps/gm-console-web/scripts/render-screenshots.js`        |
| 11 | pipelines         | `/pipelines`      | Pipelines at a glance              | iPad 12.9" (optional) | 2048 × 2732     | `ipad-12.9/pipelines__ipad-12.9__2048x2732.png` *(NULL — pending CI render)*                              | `apps/gm-console-web/scripts/render-screenshots.js`        |
| 12 | pipelines         | `/pipelines`      | Pipelines at a glance              | Play 7" tablet (opt.) | 1200 × 1920     | `play-7-tablet/pipelines__play-7-tablet__1200x1920.png` *(NULL — pending CI render)*                      | `apps/gm-console-web/scripts/render-screenshots.js`        |

**Total shot entries:** 12 (covers 7 unique shots × 5 device classes, with iPhone
6.7" being the hero device).

## Subdirectories

| Directory                              | Purpose                                                              |
|----------------------------------------|----------------------------------------------------------------------|
| `iphone-6.7/`                          | iPhone 15 Pro Max class (1290 × 2796) — hero shots                   |
| `iphone-6.5/`                          | iPhone XS Max / 11 Pro Max class (1242 × 2688)                       |
| `android-phone/`                       | Landscape Android (1080 × 1920)                                      |
| `android-phone-portrait/`              | Portrait Android (1080 × 1920)                                       |
| `ipad-12.9/`                           | iPad Pro 12.9" (2048 × 2732) — optional                              |
| `play-7-tablet/`                       | Google Play 7" tablet (1200 × 1920) — optional                       |
| `goldens/`                             | Golden-image suite used by CI diff gate                              |

## Golden convention

Golden PNGs live in `goldens/` with the same filename pattern. CI renders fresh
PNGs into the device directory and diffs them against the matching golden. A
drift greater than 2% (byte-size heuristic) fails the gate with exit code **4**.

## Why no PNG binaries committed

* Binary diffs are unreadable in code review.
* Store-review images are ephemeral — they get re-rendered before every
  release by the CI workflow, not versioned alongside the code.
* The catalog above is the durable artifact: it's what reviewers, designers,
  and CI all read.

## Regenerating locally (developer workflow)

```
cd apps/gm-console-web
node scripts/render-screenshots.js \
  --base-url http://127.0.0.1:8080 \
  --out-dir  apps/gm-console-web/dist/screenshots \
  --device-filter iphone-6.7
```

PNG files land in `dist/screenshots/iphone-6.7/`. They are not committed.