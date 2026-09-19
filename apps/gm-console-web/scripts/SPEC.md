# Web Screenshot Pipeline — SPEC

> Owner: worker-E (brand / commercial surface)
> Status: spec + scripts in place; CI invocation documented below.
> Runs only when `inputs.snapshots=true` is passed to `workflow_dispatch`, so
> the default CI remains fast.

## Goal

Render the 7 hero shots listed in `docs/commercial/SCREENSHOTS_BRIEF.md` for
**every device size** Apple and Google Play review templates require:

| Device           | Dimensions (W × H) | Notes                                |
|------------------|--------------------|--------------------------------------|
| iPhone 6.7"      | 1290 × 2796        | Hero — required                      |
| iPhone 6.5"      | 1242 × 2688        | Required                             |
| iPad 12.9"       | 2048 × 2732        | Optional, tablet form-factor         |
| Android phone    | 1080 × 1920        | Portrait (landscape optional)        |
| Play 7" tablet   | 1200 × 1920        | Optional                             |

## Shots

1. **login**            → `/login`             "Sign in to your tenant"
2. **tenants**          → `/tenants`           "Switch between workspaces"
3. **pipelines**        → `/pipelines`         "Pipelines at a glance"
4. **pipeline-detail**  → `/pipelines/1`       "Inspect a run, retry in one tap"
5. **incidents**        → `/incidents`         "Ack and resolve live alerts"
6. **audit**            → `/audit`             "Immutable remediation trail"
7. **settings**         → `/settings`          "Tokens, flavor, build info"

Each shot × device produces one PNG at
`apps/gm-console-web/dist/screenshots/<device-id>/<shot-id>__<device-id>__<width>x<height>.png`.

## CI invocation (the one that actually runs)

The runner is `ubuntu-latest` from GitHub Actions. We deliberately **do not**
install Playwright on the local Windows host — the runner pulls it on demand.

```yaml
- name: Install Playwright (Chromium only)
  if: ${{ inputs.snapshots == 'true' }}
  run: |
    npx --yes playwright@1.49.1 install --with-deps chromium

- name: Render Web screenshots
  if: ${{ inputs.snapshots == 'true' }}
  working-directory: apps/gm-console-web
  run: |
    node scripts/render-screenshots.js \
      --base-url http://127.0.0.1:8080 \
      --out-dir  apps/gm-console-web/dist/screenshots \
      --update-goldens

- name: Upload screenshots artifact
  if: ${{ inputs.snapshots == 'true' }}
  uses: actions/upload-artifact@v4
  with:
    name: web-screenshots
    path: apps/gm-console-web/dist/screenshots
    retention-days: 14
```

If the runner image lacks system libs (rare but possible on `ubuntu-latest`),
`--with-deps` falls back to `apt-get install` for Chromium's native deps:

```
libnss3 libnspr4 libatk1.0-0 libatk-bridge2.0-0 libcups2 libxkbcommon0
libatspi2.0-0 libxcomposite1 libxdamage1 libxfixes3 libxrandr2 libgbm1
libpango-1.0-0 libcairo2 libasound2t64 libatspi2.0-0t64
```

(These come from Playwright's official `playwright install --with-deps chromium`
install plan — no manual pinning needed.)

## Golden-image diff hooks

1. `--update-goldens` writes into `dist/screenshots/goldens/<shot>__<device>__WxH.png`.
2. On subsequent runs, the same files are compared against the rendered PNGs via
   a lightweight byte-size drift gate (`±2%` tolerance). Anything beyond the
   threshold exits **4**, failing the CI gate and forcing a human to either
   update the golden or fix the regression.
3. For pixel-perfect review, a reviewer can run:
   ```
   npx --yes pixelmatch \
       apps/gm-console-web/dist/screenshots/goldens/login__iphone-6.7__1290x2796.png \
       apps/gm-console-web/dist/screenshots/iphone-6.7/login__iphone-6.7__1290x2796.png \
       /tmp/diff.png 0.1
   ```

## Why a script + spec, not a real PNG

- No headless browser is installed on this host (Playwright/Puppeteer absent).
- No real PNGs may be committed (binary noise, no review value).
- The deliverable that survives review is the **catalog** (`INDEX.md`) plus
  the **scripts** that the CI workflow will execute.

## Local dry-run (when a dev has Playwright installed)

```
cd apps/gm-console-web
node scripts/render-screenshots.js \
  --base-url http://127.0.0.1:8080 \
  --out-dir  ../dist/screenshots \
  --device-filter iphone-6.7
```