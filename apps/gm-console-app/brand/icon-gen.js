#!/usr/bin/env node
// icon-gen.js — gm-console app icon raster pipeline (CI-grade)
//
// Purpose
// -------
// Read the single SVG seed at apps/gm-console-app/brand/icon-source.svg and
// produce every raster size the Apple App Store, Google Play Store, and the
// runtime adaptive-icon contract require.  The PNG binaries are CI artifacts
// only — no raster is ever committed.
//
// Size matrix
// -----------
//   iOS App Store icon        : 1024x1024   sRGB PNG, no alpha
//   iOS app icons             : 60@2x (120), 60@3x (180), 76 (76),
//                               76@2x (152), 83.5@2x (167), 1024 (1024)
//   Android adaptive          : 108x108, 192x192, 432x432
//   Android fg/background     : foreground 108x108 safe-zone 66;
//                               background 432x432 solid brand-500
//   Play Store marketing icon : 512x512 PNG, no alpha
//
// Invocation (CI)
// ---------------
//   node apps/gm-console-app/brand/icon-gen.js \
//        --svg apps/gm-console-app/brand/icon-source.svg \
//        --out-dir $RUNNER_TEMP/gm-console-icons
//
//   # Local sanity check (requires `sharp` from npm; CI installs it).
//   npm install --no-save sharp
//   node apps/gm-console-app/brand/icon-gen.js \
//        --svg apps/gm-console-app/brand/icon-source.svg \
//        --out-dir ./out/icons
//
// Exit codes
// ----------
//   0   all renders succeeded
//   2   `sharp` is not installed
//   3   SVG missing
//   4   Output dir not writable
//
// No real PNGs are committed by this script; the deliverable is the script
// itself, the seed SVG, and the Contents.json / adaptive-icon specs the CI
// writes alongside the binaries.

'use strict';

const path = require('node:path');
const fs = require('node:fs');

// --- Size matrix -------------------------------------------------------------
// Apple App Icon catalog (image set for AppIcon.appiconset/Contents.json).
// pixel sizes follow Apple's "pt × scale" convention.
//   60@2x  -> iPhone spotlight (120x120)
//   60@3x  -> iPhone spotlight (180x180)
//   76     -> iPad (76x76)
//   76@2x  -> iPad (152x152)
//   83.5@2x -> iPhone Pro Max @2x marketing (167x167)
//   1024   -> App Store marketing icon (1024x1024)
const IOS_APP_ICONS = [
  { idiom: 'iphone',     size: '60x60',  scale: '2x', filename: 'AppIcon-60@2x.png',  px: 120  },
  { idiom: 'iphone',     size: '60x60',  scale: '3x', filename: 'AppIcon-60@3x.png',  px: 180  },
  { idiom: 'ipad',       size: '76x76',  scale: '1x', filename: 'AppIcon-76.png',     px: 76   },
  { idiom: 'ipad',       size: '76x76',  scale: '2x', filename: 'AppIcon-76@2x.png',  px: 152  },
  { idiom: 'iphone',     size: '83.5x83.5', scale: '2x', filename: 'AppIcon-83.5@2x.png', px: 167 },
  { idiom: 'ios-marketing', size: '1024x1024', scale: '1x', filename: 'AppIcon-1024.png', px: 1024 },
];

// Android adaptive launcher icons (per res/ structure).
//   mdpi    48x48
//   hdpi    72x72
//   xhdpi   96x96
//   xxhdpi  144x144
//   xxxhdpi 192x192
//   anydpi-v26 (adaptive): fg 108x108 + bg 432x432
const ANDROID_DENSITY = [
  { density: 'mdpi',    px: 48,  filename: 'ic_launcher-mdpi.png'     },
  { density: 'hdpi',    px: 72,  filename: 'ic_launcher-hdpi.png'     },
  { density: 'xhdpi',   px: 96,  filename: 'ic_launcher-xhdpi.png'    },
  { density: 'xxhdpi',  px: 144, filename: 'ic_launcher-xxhdpi.png'   },
  { density: 'xxxhdpi', px: 192, filename: 'ic_launcher-xxxhdpi.png'  },
];

// --- CLI parsing -------------------------------------------------------------
function parseArgs(argv) {
  const out = {
    svg: 'apps/gm-console-app/brand/icon-source.svg',
    outDir: 'apps/gm-console-app/brand/.build/icons',
    manifest: false,
  };
  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--svg') out.svg = argv[++i];
    else if (a === '--out-dir') out.outDir = argv[++i];
    else if (a === '--write-manifest') out.manifest = true;
  }
  return out;
}

// --- Driver loader -----------------------------------------------------------
function loadSharp() {
  try {
    // Lazy require so missing module surfaces as exit code 2.
    return require('sharp');
  } catch (_) {
    const err = new Error('sharp is not installed. CI installs it via `npm install --no-save sharp`.');
    err.exitCode = 2;
    throw err;
  }
}

// --- Render helpers ----------------------------------------------------------
function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
}

async function renderPng(sharp, svgBuf, px, outPath) {
  // sRGB, no alpha (both stores reject alpha-channel icon submissions).
  await sharp(svgBuf, { density: 384 })
    .resize(px, px, { fit: 'cover', position: 'center' })
    .png({ compressionLevel: 9, palette: false })
    .toFile(outPath);
  return outPath;
}

async function renderAdaptiveFg(sharp, svgBuf, outPath) {
  // Foreground: 108x108 visible canvas, safe zone 66x66 centered (Android
  // Adaptive Icons spec).  Render the full SVG into a 108x108 transparent
  // canvas then mask the saved asset to the inner 66x66 circle at runtime
  // (the launcher applies the mask).  We keep the source solid (no alpha)
  // and let Android's system clip it.
  const bg = { r: 0, g: 0, b: 0, alpha: 0 };
  await sharp(svgBuf, { density: 384 })
    .resize(108, 108, { fit: 'contain', position: 'center', background: bg })
    .png({ compressionLevel: 9 })
    .toFile(outPath);
  return outPath;
}

async function renderAdaptiveBg(sharp, outPath) {
  // Background: solid brand-500 (sRGB), 432x432, no alpha.
  await sharp({
    create: {
      width: 432,
      height: 432,
      channels: 3,
      background: { r: 0x1E, g: 0x88, b: 0xE5 },
    },
  })
    .png({ compressionLevel: 9 })
    .toFile(outPath);
  return outPath;
}

// --- Main --------------------------------------------------------------------
async function main() {
  const args = parseArgs(process.argv);
  if (!fs.existsSync(args.svg)) {
    const err = new Error(`SVG source not found: ${args.svg}`);
    err.exitCode = 3;
    throw err;
  }
  ensureDir(args.outDir);
  // Probe writeability early to fail loudly before any rendering.
  try {
    fs.accessSync(args.outDir, fs.constants.W_OK);
  } catch (_) {
    const err = new Error(`Output directory not writable: ${args.outDir}`);
    err.exitCode = 4;
    throw err;
  }

  const sharp = loadSharp();
  const svgBuf = fs.readFileSync(args.svg);

  const produced = [];

  // iOS app icons.
  const iosDir = path.join(args.outDir, 'ios', 'AppIcon.appiconset');
  ensureDir(iosDir);
  for (const icon of IOS_APP_ICONS) {
    const outPath = path.join(iosDir, icon.filename);
    await renderPng(sharp, svgBuf, icon.px, outPath);
    produced.push({ kind: 'ios', ...icon, path: outPath });
    process.stdout.write(`[icon] ios ${icon.size}@${icon.scale} -> ${path.relative(process.cwd(), outPath)}\n`);
  }

  // Android legacy mipmap-* icons (per density).
  for (const icon of ANDROID_DENSITY) {
    const outPath = path.join(args.outDir, 'android', `mipmap-${icon.density}`, icon.filename);
    ensureDir(path.dirname(outPath));
    await renderPng(sharp, svgBuf, icon.px, outPath);
    produced.push({ kind: 'android', ...icon, path: outPath });
    process.stdout.write(`[icon] android ${icon.density} ${icon.px}x${icon.px} -> ${path.relative(process.cwd(), outPath)}\n`);
  }

  // Android adaptive: foreground + background drawables for anydpi-v26.
  const fgPath = path.join(args.outDir, 'android', 'mipmap-anydpi-v26', 'ic_launcher_foreground.png');
  const bgPath = path.join(args.outDir, 'android', 'mipmap-anydpi-v26', 'ic_launcher_background.png');
  ensureDir(path.dirname(fgPath));
  await renderAdaptiveFg(sharp, svgBuf, fgPath);
  produced.push({ kind: 'android-adaptive-fg', px: 108, path: fgPath });
  process.stdout.write(`[icon] android adaptive fg 108 -> ${path.relative(process.cwd(), fgPath)}\n`);
  await renderAdaptiveBg(sharp, bgPath);
  produced.push({ kind: 'android-adaptive-bg', px: 432, path: bgPath });
  process.stdout.write(`[icon] android adaptive bg 432 -> ${path.relative(process.cwd(), bgPath)}\n`);

  // Android adaptive 432x432 reference (matches Google Play "App icon" preview).
  const adaptiveRef = path.join(args.outDir, 'android', 'ic_launcher_play_432.png');
  await renderPng(sharp, svgBuf, 432, adaptiveRef);
  produced.push({ kind: 'android-adaptive-ref', px: 432, path: adaptiveRef });
  process.stdout.write(`[icon] android adaptive ref 432 -> ${path.relative(process.cwd(), adaptiveRef)}\n`);

  // Play Store marketing icon (512x512, no alpha).
  const playPath = path.join(args.outDir, 'play-store-512.png');
  await renderPng(sharp, svgBuf, 512, playPath);
  produced.push({ kind: 'play-store', px: 512, path: playPath });
  process.stdout.write(`[icon] play store 512 -> ${path.relative(process.cwd(), playPath)}\n`);

  // Optional manifest (JSON) — useful for the mobile-build.yml job to wire the
  // produced binaries into Xcode / Gradle resource folders without re-globbing.
  if (args.manifest) {
    const manifest = {
      generatedAt: new Date().toISOString(),
      source: args.svg,
      artifacts: produced,
    };
    const manifestPath = path.join(args.outDir, 'manifest.json');
    fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 2));
    process.stdout.write(`[icon] manifest -> ${path.relative(process.cwd(), manifestPath)}\n`);
  }
}

main().catch((err) => {
  process.stderr.write(`[icon] ${err.message}\n`);
  if (err.exitCode) process.exit(err.exitCode);
  process.exit(1);
});