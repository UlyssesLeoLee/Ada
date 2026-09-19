# Mobile Signing Runbook — gm-console Mobile

> **Operational handbook.** Last reviewed by Mavis on 2026-09-19 as part of
> worker-G landing (commit `ci(mobile): signing + push credentials + submit
> lanes + TruffleHog`). Mavis signs this as the human-replacement reviewer per
> 守门 #14 v3 + v4 (2026-09-11 23:11 JST). When a real platform team owner
> joins, they re-sign in the revision history.

This runbook covers the **signing & submission secret boundary** for the
gm-console Mobile app. It does NOT cover app behavior, store listing copy, or
review responses — those live in `COMPLIANCE_SELFCHECK.md` and
`STORE_LISTING*.md`.

## 1. Threat model in one paragraph

A leaked Apple Distribution certificate lets an attacker sign malicious apps
that App Store reviewers trust as coming from us. A leaked Play upload
keystore lets an attacker push trojanized bundles to Google Play that we then
have to roll back, losing trust + review state. A leaked `*.jks` /
`*.mobileprovision` / `*.p12` in this git repository is irreversible — once
exposed, the only safe path is rotation. So we never commit them, never print
them, and never let them sit in a CI log.

## 2. Where the secrets live

Every signing credential below is stored in **exactly one** of:

| Location                                        | Type                        | Notes                                         |
|-------------------------------------------------|-----------------------------|-----------------------------------------------|
| GitHub Actions → repo secrets                   | Encrypted at rest           | Primary CI source of truth.                   |
| 1Password vault at `https://vault.kanvas.dev`   | Password manager            | Operator-facing source. Placeholder URL.      |
| Apple App Store Connect (key list)              | Apple's own KMS             | Public-key registry; no private material.     |
| Google Play Console → App Integrity             | Google-managed              | Play App Signing keys; we never see them.    |

We do NOT use:
- `.env` files at the repo root (`.gitignore` excludes them already).
- Vault tokens committed to a `secrets/` directory.
- Slack / email / Discord shares of any secret value.
- `Get-ChildItem env:` / `echo $VAR` / `printenv` style exposure in any
  terminal or log (per memory 2026-08-27 11:06 JST hard ban).

## 3. Secret inventory

Each row maps a CI secret to its human-readable purpose, its 1Password item,
and the rotation cadence.

| Secret name (GH Actions)                | Purpose                                                   | 1Password item reference                  | Rotation cadence     |
|-----------------------------------------|-----------------------------------------------------------|------------------------------------------|----------------------|
| `APPLE_P12_BASE64`                      | Apple Distribution certificate (`.p12`, base64)           | `Ada Project / gm-console / Apple Cert`  | Annual + on-incident |
| `APPLE_P12_PASSWORD`                    | Password for the `.p12` above                              | same                                      | Annual + on-incident |
| `APPLE_KEYCHAIN_PASSWORD`               | Random keychain password used by import-codesign-certs     | `Ada Project / gm-console / CI Keychain`  | Per-runner rebuild   |
| `APPLE_PROVISION_BASE64`                | Ad-hoc or App Store provisioning profile (`.mobileprovision`) | `Ada Project / gm-console / Provisioning Profile` | Annual + on-incident |
| `APPLE_TEAM_ID`                         | Apple Developer Team ID (10-char alphanumeric, public-ish) | `Ada Project / gm-console / Apple Team ID` | Static                |
| `APPLE_BUNDLE_ID`                       | `dev.kanvas.gmconsole`                                    | `Ada Project / gm-console / Bundle IDs`    | Static                |
| `GM_CONSOLE_PLAY_UPLOAD_KEY`            | Play upload keystore (`.jks`, base64) — Play App Signing handles the actual signing key | `Ada Project / gm-console / Play Upload Key` | Annual + on-incident |
| `PLAY_UPLOAD_KEY_PASSWORD`              | Password for the upload keystore                          | same                                      | Annual + on-incident |
| `PLAY_KEY_ALIAS`                        | Key alias inside the `.jks`                               | same                                      | On-keystore creation |
| `PLAY_PUBLISHER_SERVICE_ACCOUNT`        | Google Play Console API service account JSON (base64)     | `Ada Project / gm-console / Play API SA`  | Annual               |
| `PLAY_PACKAGE_NAME`                     | `dev.kanvas.gmconsole`                                    | `Ada Project / gm-console / Bundle IDs`    | Static                |

None of these values appear in this document. Pasted values belong only in
1Password and GitHub Actions UI. If you need the value, click through to the
1Password item via the link above; do not paste it anywhere.

## 4. How CI consumes each secret

`mobile-build.yml` runs four jobs:

1. **`verify-no-secrets-leaked`** — TruffleHog diff scan. Runs first so a
   leaked secret in the diff aborts the run before any signing material is
   touched. No secrets are read by this job.

2. **`sign-and-pack-ios`** — gated on `APPLE_P12_BASE64 != ''`. The runner
   imports the cert with `apple-actions/import-codesign-certs@v3.2.0`,
   installs the provisioning profile with
   `apple-actions/install-provisioning-profile@v4.0.0`, then calls
   `flutter build ipa --export-options-plist=ios/ExportOptions.plist`.
   The IPA is uploaded as a build artifact.

3. **`sign-and-pack-android`** — gated on `GM_CONSOLE_PLAY_UPLOAD_KEY != ''`.
   The runner base64-decodes the keystore into an ephemeral
   `.signing/upload-key.jks`, writes a transient `key.properties` referencing
   it, then runs `flutter build appbundle --release`. The workspace is wiped
   when the runner exits.

4. **`submit-testflight`** — gated on `startsWith(ref, 'refs/tags/mobile-v')`
   AND the iOS cert being present. This job uploads the IPA built in step 2
   to TestFlight. It is intentionally NOT triggered on PRs.

`mobile-submit.yml` is a separate workflow for promoting from TestFlight /
internal Play track to the public stores. It re-verifies the diff with
TruffleHog before publishing anything.

## 5. Adding a new secret

1. Generate the value offline. Never paste a private value into chat,
   terminal output, or commit message.
2. Add the value to 1Password under the matching item in
   `https://vault.kanvas.dev`.
3. Add the secret to GitHub Actions at the repo / org level. Use the
   `Add Secret` button — never commit the value to a file.
4. Document the secret in §3 of this runbook. Do not paste the value.
5. Reference the secret name (e.g. `secrets.NEW_SECRET`) in the workflow YAML.
6. Add a hard `if: env.NEW_SECRET != ''` gate at the job level so a missing
   secret produces a hard skip, not a silent zero-byte value.

## 6. Rotating a secret

The canonical rotation procedure. Use this whenever an `INCIDENT_2026-XX.md`
record or a TruffleHog finding flags a secret as compromised.

### 6.1 Apple Distribution certificate

1. On a Mac with Xcode installed, open **Xcode → Settings → Accounts →
   Manage Certificates → + → Apple Distribution**.
2. Export the new `.p12` (Xcode → Certificates → right-click → Export).
3. In 1Password, replace the value at
   `Ada Project / gm-console / Apple Cert` and add the new password.
4. Replace `APPLE_P12_BASE64`, `APPLE_P12_PASSWORD`,
   `APPLE_KEYCHAIN_PASSWORD` in GitHub Actions.
5. Re-run `mobile-build.yml → sign-and-pack-ios` on a `mobile-v*` tag; the
   new IPA must arrive in TestFlight within 15 min.
6. After the new cert is live, revoke the old one in App Store Connect
   (`Users and Access → Keys → Certificates → Revoke`).
7. File an `INCIDENT_2026-XX.md` (use `INCIDENT_TEMPLATES/INCIDENT_TEMPLATE.md`)
   if the rotation was incident-driven.

### 6.2 Apple Provisioning Profile

1. In Apple Developer Portal, regenerate the profile bound to the new cert.
2. Export `.mobileprovision`, base64 it, replace
   `APPLE_PROVISION_BASE64` in GitHub Actions.
3. Re-run `sign-and-pack-ios` to confirm `flutter build ipa` no longer
   complains about "Provisioning profile doesn't match certificate".

### 6.3 Play upload keystore

1. Generate a fresh `.jks`:
   ```bash
   keytool -genkey -v \
     -keystore upload-key.jks \
     -keyalg RSA -keysize 2048 -validity 10000 \
     -alias upload
   ```
2. Register the new upload key in Google Play Console → Setup → App
   Integrity → Request upload key reset (Play allows one reset per app
   lifetime).
3. Replace `GM_CONSOLE_PLAY_UPLOAD_KEY`, `PLAY_UPLOAD_KEY_PASSWORD`,
   `PLAY_KEY_ALIAS` in GitHub Actions.
4. Re-run `sign-and-pack-android` on a `mobile-v*` tag.

### 6.4 Play publisher service account

1. In Google Cloud Console, create a new service account JSON key with the
   `Google Play Android Developer` role.
2. Base64 the JSON, replace `PLAY_PUBLISHER_SERVICE_ACCOUNT`.
3. Re-run `mobile-submit.yml` on the `internal` track; verify the upload
   finishes before promoting to `production`.

## 7. Hard bans (do not propose a PR that violates these)

- **Never** commit `*.p12`, `*.cer`, `*.mobileprovision`, `*.jks`,
  `*.keystore`, `key.properties`, `google-services.json`,
  `GoogleService-Info.plist`, `key.json`, `*.asc` to any branch.
- **Never** `echo $SECRET` / `printenv` / `Get-ChildItem env:` / log the
  value of any secret. Use the `'present'` / `''` empty-string check pattern.
- **Never** share a secret value over chat, email, or a Notion page.
- **Never** pin the cert-import action or TestFlight action to `latest` — pin
  to a specific tag or SHA at PR time.
- **Never** let `mobile-submit.yml` auto-trigger. It must be
  `workflow_dispatch` only.

## 8. Incident template

For all security incidents related to signing material (leaked cert, leaked
keystore, suspicious TestFlight upload, etc.), copy
`docs/commercial/INCIDENT_TEMPLATES/INCIDENT_TEMPLATE.md` to
`docs/commercial/INCIDENT_2026-YY-MM-DD.md`, fill it in, and follow the
runbook at the top of the template.