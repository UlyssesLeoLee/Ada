# gm-console Android shell

> Status (2026-09-19): scaffold only. The actual Gradle project files are produced by
> `flutter create --platforms=android --org dev.kanvas .` from this directory on a host with
> the Flutter SDK installed.

## Gradle identity

| Field              | Value                          |
|--------------------|--------------------------------|
| ApplicationId      | `dev.kanvas.gmconsole`         |
| ApplicationIdSuffix (debug) | `.debug`             |
| MinSdkVersion      | `24` (Android 7.0)             |
| TargetSdkVersion   | `34` (Play Store baseline)     |
| CompileSdkVersion  | `34`                           |
| VersionName        | `0.1.0`                        |
| VersionCode        | `1`                            |
| TestInstrumentationRunner | `androidx.test.runner.AndroidJUnitRunner` |

## Resource/icon dir reference (post-`flutter create`)

`android/app/src/main/res/`:

| Path                          | Purpose                                         |
|-------------------------------|-------------------------------------------------|
| `mipmap-*/ic_launcher.png`    | Adaptive launcher icons                          |
| `mipmap-*/ic_launcher_round.png` | Round variant                                 |
| `values/strings.xml`          | `<string name="app_name">gm-console</string>`   |
| `values/styles.xml`           | Material 3 / DayNight theme                     |
| `values-night/styles.xml`     | Dark variant                                    |
| `xml/network_security_config.xml` | TLS-only by default; `cleartextTrafficPermitted=false` |

## Permissions (AndroidManifest.xml)

gm-console Mobile v0.3.0 ships with:

```xml
<uses-permission android:name="android.permission.INTERNET"/>
<uses-permission android:name="android.permission.ACCESS_NETWORK_STATE"/>
```

We do NOT request:
- `READ_EXTERNAL_STORAGE`, `WRITE_EXTERNAL_STORAGE` — UNUSED at v0.3.0.
- `CAMERA`, `RECORD_AUDIO` — UNUSED at v0.3.0.
- `POST_NOTIFICATIONS` — Phase 2 (incidents feed push). When added, runtime permission
  prompt must be gated behind explicit user action (RN-friendly Flutter `permission_handler`
  flow).
- `FOREGROUND_SERVICE` — Phase 2 only, paired with the appropriate `foregroundServiceType`.

If/when incident attachments land, `READ_MEDIA_IMAGES` (Tiramisu+) must be added.

## Signing posture

- Play App Signing: **enabled** (Google manages the upload-keystore).
- Upload-keystore: stored as **base64** in GitHub Actions secret
  `GM_CONSOLE_PLAY_UPLOAD_KEY` (per user policy 2026-08-27 — env values never printed);
  decoded into a transient `signing.gradle` inside the runner only.
- Local dev: debug keystore at `~/.android/debug.keystore`.
- Release SHA256 fingerprint reported to the GM-console client team for cert pinning.

## Build flavors

Two flavors are wired by `android/app/build.gradle`:

| Flavor   | applicationId                          | Points at                           |
|----------|----------------------------------------|-------------------------------------|
| `dev`    | `dev.kanvas.gmconsole.dev`             | `http://localhost:8080/api`         |
| `release`| `dev.kanvas.gmconsole`                 | `https://gm-console.kanvas.dev/api` |

(Phase 2: add `staging`.)

## Bring-up commands

```bash
# On a host with Flutter 3.24.5 + Android SDK platform-tools:
cd apps/gm-console-app
flutter create --platforms=android --org dev.kanvas --project-name gm_console_app .

# Then edit android/app/build.gradle:
#   defaultConfig.applicationId = 'dev.kanvas.gmconsole'
#   minSdkVersion 24
#   targetSdkVersion 34

# Generate debug-signed APK:
flutter build apk --debug

# Generate Play-Store-signed bundle (CI only — needs upload keystore):
flutter build appbundle --release
```

## What is NOT in git

| What                                          | Why                                                |
|-----------------------------------------------|----------------------------------------------------|
| `android/.gradle/`, `android/local.properties` | local SDK paths                                    |
| `android/app/google-services.json`            | Firebase config (v0.3.0: N/A; phase 2)            |
| `*.jks`, `*.keystore`                         | uploaded to Google Play App Signing, sealed-secret elsewhere |

CI integration lives in `.github/workflows/mobile-build.yml` (worker-G target).
