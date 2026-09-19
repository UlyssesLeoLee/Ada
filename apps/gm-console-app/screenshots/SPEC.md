# Flutter Mobile Screenshot Pipeline — SPEC

> Owner: worker-E (brand / commercial surface)
> Status: spec + POSIX script shipped; **Flutter SDK is not installed on this
> worker host**, so the script will not run here. The CI job below is where it
> will actually execute.

## Goal

Render the same 7 hero shots as the Web pipeline (see
`docs/commercial/SCREENSCREENSHOTS_BRIEF.md`) on a real Android device
emulator for each store flavor (`dev`, `staging`, `release`).

## Per-flavor invocation

```
for flavor in dev staging release; do
  flutter build apk --flavor $flavor --release
  flutter test  --update-goldens --tags screenshot
done
```

Implemented by `render.sh` in this directory.

## Golden storage

* `apps/gm-console-app/test/goldens/dev/...`
* `apps/gm-console-app/test/goldens/staging/...`
* `apps/gm-console-app/test/goldens/release/...`

Each `flutter test --update-goldens --tags screenshot` run is responsible for
keeping the goldens in sync. Any drift past 1% pixel diff fails CI.

## Shot ↔ test mapping (planned; not yet implemented in code)

| Shot            | Screen widget                                  | Test file                                |
|-----------------|------------------------------------------------|------------------------------------------|
| login           | `LoginScreen`                                  | `test/login_screen_golden_test.dart`     |
| tenants         | `TenantsScreen`                                | `test/tenants_screen_golden_test.dart`   |
| pipelines       | `PipelinesScreen`                              | `test/pipelines_screen_golden_test.dart` |
| pipeline-detail | `PipelineDetailScreen`                         | `test/pipeline_detail_golden_test.dart`  |
| incidents       | `IncidentsScreen`                              | `test/incidents_screen_golden_test.dart` |
| audit           | `AuditScreen`                                  | `test/audit_screen_golden_test.dart`     |
| settings        | `SettingsScreen`                               | `test/settings_screen_golden_test.dart`  |

> Note: this is the planned mapping. The current test files in
> `apps/gm-console-app/test/` (`login_screen_test.dart`,
> `pipelines_screen_test.dart`, `api_client_test.dart`) cover widget behavior,
> not goldens. Adding golden tests is out of scope for worker-E (no Flutter
> SDK on host).

## CI invocation (planned, not enabled)

```yaml
- name: Update mobile goldens
  if: ${{ inputs.snapshots == 'true' }}
  working-directory: apps/gm-console-app
  run: bash screenshots/render.sh
```

A future worker (when the Flutter SDK is on the runner) can wire this up.

## Caveats

* The `apps/gm-console-app/.flutter-version` file referenced by
  `.github/workflows/ci.yml` does not exist on this host. Fixing that is out
  of scope for worker-E.
* No emulator boot is required for golden generation — `flutter test` runs
  headless via the test harness.