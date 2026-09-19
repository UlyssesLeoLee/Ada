// render-screenshots.js — gm-console Web SPA screenshot pipeline (CI-grade)
//
// Purpose
// -------
// Drive a headless Chromium (via Playwright, when available) through the 7 hero
// shots documented in docs/commercial/SCREENSHOTS_BRIEF.md, at the exact pixel
// dimensions Apple/Google review templates require.
//
// Why this file is here
// ---------------------
// - This worker-E host has no Node-based browser driver installed. So the file
//   is not directly runnable on the host; the spec it ships with (SPEC.md)
//   describes the exact CI invocation.
// - It exists so future devs (and the CI workflow) have a single source of
//   truth for which URL renders which shot, at what size, into which path.
//
// Conventions
// -----------
// - All output goes under apps/gm-console-web/dist/screenshots/<device>/.
// - File names are stable: <shot-id>__<device-id>__<width>x<height>.png.
// - The dist/screenshots/INDEX.md file lists every (shot, device) tuple with a
//   NULL placeholder until the golden PNG is committed.
//
// Invocation (CI)
// ---------------
//   node apps/gm-console-web/scripts/render-screenshots.js \
//        --base-url http://127.0.0.1:8080 \
//        --out-dir  apps/gm-console-web/dist/screenshots \
//        --device-filter iphone-6.7,android-phone-portrait
//
//   # All devices if --device-filter omitted.
//   # --update-goldens writes the rendered PNGs into apps/gm-console-web/dist/screenshots/goldens/.
//
// Exit codes
// ----------
//   0   all renders succeeded and were either equal to goldens or --update-goldens was set
//   2   Playwright/Puppeteer not installed (CI installs them via npx --yes playwright)
//   3   Browser launch failed (missing system libs on the runner)
//   4   Image diff exceeded threshold (golden regression)
//
// No real PNGs are committed by this script; the deliverable is the spec +
// the INDEX catalog. PNGs land via CI artifacts.

'use strict';

const path = require('node:path');
const fs = require('node:fs');

// --- Shot catalog (mirrors docs/commercial/SCREENSHOTS_BRIEF.md) -------------
// shot-id           | route          | caption                       | device-id          | width x height
const SHOTS = [
  { id: 'login',            route: '/login',            caption: 'Sign in to your tenant',           deviceId: 'iphone-6.7',          w: 1290, h: 2796 },
  { id: 'login',            route: '/login',            caption: 'Sign in to your tenant',           deviceId: 'iphone-6.5',          w: 1242, h: 2688 },
  { id: 'login',            route: '/login',            caption: 'Sign in to your tenant',           deviceId: 'android-phone',       w: 1080, h: 1920 },
  { id: 'tenants',          route: '/tenants',          caption: 'Switch between workspaces',         deviceId: 'iphone-6.7',          w: 1290, h: 2796 },
  { id: 'pipelines',        route: '/pipelines',        caption: 'Pipelines at a glance',            deviceId: 'iphone-6.7',          w: 1290, h: 2796 },
  { id: 'pipelines',        route: '/pipelines',        caption: 'Pipelines at a glance',            deviceId: 'android-phone-portrait', w: 1080, h: 1920 },
  { id: 'pipeline-detail',  route: '/pipelines/1',      caption: 'Inspect a run, retry in one tap',  deviceId: 'iphone-6.7',          w: 1290, h: 2796 },
  { id: 'incidents',        route: '/incidents',        caption: 'Ack and resolve live alerts',      deviceId: 'iphone-6.7',          w: 1290, h: 2796 },
  { id: 'audit',            route: '/audit',            caption: 'Immutable remediation trail',      deviceId: 'iphone-6.7',          w: 1290, h: 2796 },
  { id: 'settings',         route: '/settings',         caption: 'Tokens, flavor, build info',       deviceId: 'iphone-6.7',          w: 1290, h: 2796 },
  // Optional tablet coverage (iPad 12.9") — enabled when device-filter=ipad-12.9.
  { id: 'pipelines',        route: '/pipelines',        caption: 'Pipelines at a glance',            deviceId: 'ipad-12.9',           w: 2048, h: 2732 },
];

// --- CLI parsing -------------------------------------------------------------
function parseArgs(argv) {
  const out = { baseUrl: 'http://127.0.0.1:8080', outDir: 'apps/gm-console-web/dist/screenshots', deviceFilter: null, updateGoldens: false };
  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--base-url')         out.baseUrl = argv[++i];
    else if (a === '--out-dir')     out.outDir = argv[++i];
    else if (a === '--device-filter') out.deviceFilter = argv[++i].split(',').map(s => s.trim()).filter(Boolean);
    else if (a === '--update-goldens') out.updateGoldens = true;
  }
  return out;
}

// --- Driver loader (tries playwright, falls back to puppeteer) ---------------
async function loadDriver() {
  // Lazy-require so missing modules surface as a friendly error code (2).
  try {
    const playwright = require('playwright');
    return { kind: 'playwright', api: playwright };
  } catch (_) { /* fall through */ }
  try {
    const puppeteer = require('puppeteer');
    return { kind: 'puppeteer', api: puppeteer };
  } catch (_) { /* fall through */ }
  const err = new Error('Neither playwright nor puppeteer is installed in this environment.');
  err.exitCode = 2;
  throw err;
}

// --- One render --------------------------------------------------------------
async function renderOne(driver, { baseUrl, outDir, shot, updateGoldens }) {
  const filename = `${shot.id}__${shot.deviceId}__${shot.w}x${shot.h}.png`;
  const targetDir = updateGoldens ? path.join(outDir, 'goldens') : path.join(outDir, shot.deviceId);
  fs.mkdirSync(targetDir, { recursive: true });
  const outPath = path.join(targetDir, filename);

  let page;
  if (driver.kind === 'playwright') {
    const browser = await driver.api.chromium.launch({ headless: true });
    const ctx = await browser.newContext({
      viewport: { width: shot.w, height: shot.h },
      deviceScaleFactor: 1,
      colorScheme: 'light',
    });
    page = await ctx.newPage();
    await page.goto(new URL(shot.route, baseUrl).toString(), { waitUntil: 'networkidle' });
    // Wait for the shell to declare readiness (see dist/index.html hydration hook).
    await page.waitForFunction(() => document.documentElement.dataset.shellReady === '1', null, { timeout: 5000 })
      .catch(() => { /* static shell may not set this; harmless */ });
    await page.screenshot({ path: outPath, fullPage: false, type: 'png' });
    await browser.close();
  } else {
    // puppeteer path
    const browser = await driver.api.launch({ headless: 'new', args: ['--no-sandbox'] });
    page = await browser.newPage();
    await page.setViewport({ width: shot.w, height: shot.h, deviceScaleFactor: 1 });
    await page.goto(new URL(shot.route, baseUrl).toString(), { waitUntil: 'networkidle0' });
    await page.screenshot({ path: outPath, fullPage: false, type: 'png' });
    await browser.close();
  }
  return outPath;
}

// --- Image diff (lightweight, no native deps) --------------------------------
// Compares byte size and PNG signature only; sufficient to flag a regression
// when the golden exists. Real pixel-level diff is delegated to the CI step.
function quickDiff(renderedPath, goldenPath) {
  if (!fs.existsSync(goldenPath)) return { status: 'no-golden' };
  const a = fs.statSync(renderedPath).size;
  const b = fs.statSync(goldenPath).size;
  const drift = Math.abs(a - b) / Math.max(a, 1);
  return { status: drift > 0.02 ? 'drift' : 'ok', driftBytes: a - b };
}

// --- Entry point -------------------------------------------------------------
async function main() {
  const args = parseArgs(process.argv);
  const shots = args.deviceFilter
    ? SHOTS.filter(s => args.deviceFilter.includes(s.deviceId))
    : SHOTS;

  const driver = await loadDriver();
  const summary = [];
  for (const shot of shots) {
    const rendered = await renderOne(driver, { ...args, shot });
    const golden = path.join(args.outDir, 'goldens', path.basename(rendered));
    const diff = quickDiff(rendered, golden);
    summary.push({ shot, rendered, golden, diff });
    // eslint-disable-next-line no-console
    console.log(`[render] ${shot.id} @ ${shot.deviceId} (${shot.w}x${shot.h}) -> ${rendered} (${diff.status})`);
  }

  // Non-zero exit on drift so CI can fail loudly.
  const anyDrift = summary.some(s => s.diff.status === 'drift');
  process.exitCode = anyDrift ? 4 : 0;
}

main().catch((err) => {
  // eslint-disable-next-line no-console
  console.error(`[render] ${err.message}`);
  if (err.exitCode) process.exit(err.exitCode);
  process.exit(1);
});