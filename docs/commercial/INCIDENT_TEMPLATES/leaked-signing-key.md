# Incident — Leaked Signing Credential

Use this template the moment a leak is suspected. Severity: **P0**.

## Facts

- Reporter:
- Date (incident): YYYY-MM-DD HH:MM JST
- Date (detected):
- Detection signal (e.g. TruffleHog CI failure, GitHub secret scan alert, internal report):
- Affected credentials (check all that apply):
  - [ ] Apple Distribution p12 (`secrets.APPLE_P12_BASE64`)
  - [ ] Apple Provisioning profile (`secrets.APPLE_PROVISION_BASE64`)
  - [ ] Apple p12 password (`secrets.APPLE_P12_PASSWORD`)
  - [ ] Keychain password (`secrets.APPLE_KEYCHAIN_PASSWORD`)
  - [ ] Apple Team ID (`secrets.APPLE_TEAM_ID`)
  - [ ] Apple Bundle ID (`secrets.APPLE_BUNDLE_ID`)
  - [ ] Play Publisher service-account JSON (`secrets.PLAY_PUBLISHER_SERVICE_ACCOUNT`)
  - [ ] Play upload keystore (`secrets.GM_CONSOLE_PLAY_UPLOAD_KEY`)
  - [ ] Play upload keystore password (`secrets.GM_CONSOLE_PLAY_UPLOAD_PASSWORD`)
  - [ ] 1Password source-of-truth item: `Ada Project / gm-console / <name>`

## Immediate containment (≤ 30 minutes)

1. **Revoke** the affected secret in 1Password. Do NOT delete the audit log entry.
2. In Apple Developer Account → Certificates → **revoke** the cert if it is the leaked
   Apple Distribution p12.
3. In Google Play Console → Setup → App integrity → **request** a new upload key for
   the affected app (`dev.kanvas.gmconsole`). Send the new `.jks` to 1Password.
4. Disable workflow runs that depend on the leaked secret until rotation completes.
   - [ ] `.github/workflows/mobile-build.yml` is gated by `if: env.SECRET != ''` —
     a zero-byte fallback. Verify the gate correctly skips.
   - [ ] `.github/workflows/mobile-submit.yml` likewise.

## Rotation

Re-create the secret, paste the new base64 into 1Password, promote to the
`Ada Project / gm-console / ...` item. Update GitHub Actions secret value with the
new value. Pin the new cert / keystore UUID in `docs/commercial/SIGNING_RUNBOOK.md`.

## Post-mortem

- Root cause (technical + procedural):
- Detection latency (target: ≤ 30 min from leak):
- Containment latency (target: ≤ 30 min from detection):
- Recovery latency (target: ≤ 24 hours from leak):
- Action items (assign owners):
