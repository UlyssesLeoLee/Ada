# App Store / Google Play — Compliance Self-Check

A runnable checklist both stores use at submission time. Worker-F owns icon + screenshot
pipeline; this file is the contractual review.

## Apple App Store Review Guideline alignment

| Guideline                                    | Posture                                                                                   |
|----------------------------------------------|-------------------------------------------------------------------------------------------|
| 1.1 Objectionable Content                     | N/A — utility app.                                                                         |
| 2.1 App Completeness                          | All 7 screens land with real placeholder data; no "in-app upgrade to remove demo" lures.   |
| 2.3 Accurate Metadata                         | Screenshots from CI playwright run match current binary build (golden image diff).         |
| 3.1.1 In-App Purchase (no)                    | Confirmed — no IAP, no subscription. Free utility. PR `STORE_LISTING*.md` matches.        |
| 4.0 Design                                   | Material 3 + Cupertino adaptive; passes HIG + Material guidance (USAGE.md a11y section). |
| 4.2 Minimum Functionality                     | Real pipelines list, real detail, real retry — wired to `/api/*` (mocked when offline).   |
| 5.1.1 Privacy Policy                          | `PRIVACY.md` linked from settings; required URL field populated.                          |
| 5.1.2 Privacy Nutrition Labels                 | We collect: **Account email**, **Operational data**, **Optional error telemetry**.        |
| 5.1.3 Children Data                           | None.                                                                                      |
| 6.x Anti-Fraud / Signing                       | Apple Distribution cert; no jailbreak detection tricks.                                    |

## Google Play Data Safety form mapping

| Question                                            | Answer                                               |
|-----------------------------------------------------|------------------------------------------------------|
| Does your app collect or share required data?       | Yes.                                                  |
| Data collected: account info (email)                | Yes (login screen).                                   |
| Data collected: app activity (interactions)         | Yes (audit + telemetry opt-out).                     |
| Data collected: app info & performance (crash logs) | Yes.                                                  |
| Data shared with third parties                      | No (all data stays in deployment you operate).       |
| Data processing purposes                            | App functionality, analytics (opt-out self-host).     |
| Users can request data deletion                     | Yes — `privacy@kanvas.dev` + self-service at v0.4.0. |

The Data Safety JSON file's source lives at
`docs/commercial/google-play-data-safety.json` (worker-F contributes).

## Submission gates (CI)

`mobile-build.yml` must pass:

1. `cargo check -p gm-console --locked` and `cargo test -p gm-console --locked` (Mavis-owned).
2. `flutter pub get` + `flutter analyze --fatal-infos --fatal-warnings` + `flutter test`.
3. `flutter build ios --release --no-codesign` (no actual signing on bare runner).
4. `flutter build appbundle --release` (Play upload-keystore required).
5. `playwright install --with-deps chromium` + run
   `apps/gm-console-web/scripts/render-screenshots.js` against the released binary.
6. Screenshots archived as `release-artifacts/screens-*.png` per device target.
7. `cargo-deny check advisories` (CI-built-in) clean on the merged diff.
8. PR review by 1 independent reviewer (1 人公司 → Mavis 7th-self-driven approval
   stands in lieu, per守门 #14 v3+v4 — DDD Review is the human-replaceable gate).

## What blocks submission

- Missing or broken screenshots (golden diff > 0.5%).
- Disclosure mismatch on Data Safety / Nutrition Labels.
- TestFlight export-compliance question not answered ("Is your app designed to use
  cryptography? Yes — `Yes (exempt)` because cryptography is only used for HTTPS
  link negotiation").
- Any tracked `.p12`/`.mobileprovision`/`.keystore` (CI check via TruffleHog).
- ANY env-derived secret logged (verbatim per memory 2026-08-27 hard ban).

## Review-state monitoring

- apple-itunesconnect: Bot token stored at
  `https://ada.kanvas.dev/secrets/itc-notify` — never in repo.
- google-play-console: API access via service account `ada-publisher@kanvas-project.iam`.
