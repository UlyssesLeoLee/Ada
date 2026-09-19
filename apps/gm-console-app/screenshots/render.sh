#!/usr/bin/env bash
# render.sh — gm-console (Flutter) mobile screenshot pipeline.
#
# Purpose
# -------
# Build the gm-console app for each store flavor (dev / staging / release) and
# update the golden-image suite for the 7 hero shots in
# docs/commercial/SCREENSHOTS_BRIEF.md.
#
# Where this runs
# ---------------
# * A Flutter-equipped host (macOS or Linux runner with Flutter stable).
# * NOT this Windows worker host — Flutter SDK is absent here, so the script
#   is a spec for now (see SPEC.md in this directory).
#
# Conventions
# -----------
# * Goldens live in apps/gm-console-app/test/goldens/<flavor>/.
# * The script is idempotent: re-running with the same inputs is safe.
# * Exit codes: 0 success, 1 build failure, 2 test failure, 3 SDK missing.

set -euo pipefail

# --- Config -------------------------------------------------------------------
FLAVORS=(dev staging release)
APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SHOT_TAG="screenshot"   # matches the @Tags('screenshot') annotation on shot tests

log()  { printf '\033[1;34m[render]\033[0m %s\n' "$*"; }
fail() { printf '\033[1;31m[render]\033[0m %s\n' "$*" >&2; exit "${2:-1}"; }

# --- Pre-flight --------------------------------------------------------------
command -v flutter >/dev/null 2>&1 || fail "Flutter SDK not on PATH" 3

cd "$APP_DIR"

log "Flutter version"
flutter --version | head -n1

# --- Build + golden per flavor -----------------------------------------------
for flavor in "${FLAVORS[@]}"; do
  log "Build: --flavor=$flavor --release"
  flutter build apk \
    --flavor "$flavor" \
    --release \
    --no-tree-shake-icons \
    || fail "build failed for flavor=$flavor" 1

  log "Update goldens: flavor=$flavor tag=$SHOT_TAG"
  flutter test \
    --update-goldens \
    --tags "$SHOT_TAG" \
    || fail "golden update failed for flavor=$flavor" 2
done

log "All flavors built and goldens refreshed under $APP_DIR/test/goldens/"
log "Done."