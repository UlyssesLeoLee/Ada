# gm-console Mobile (Flutter)

Commercial-grade companion app for the gm-console Web dashboard. Single Dart codebase targets
both iOS and Android (worker-B owns the deep implementation).

## Layer

- Talks to the upstream Ada api-gateway through the gm-console reverse-proxy surface (`/api/*`)
- Token-based auth (login screen → JWT stored in `flutter_secure_storage`)
- Riverpod state management; GoRouter for nav; Material 3 + Cupertino adaptive

## Run

```bash
flutter pub get
flutter run -d <device>           # dev
flutter build apk --release       # Android APK
flutter build ios --release       # iOS IPA (requires signing)
flutter build web                 # SPA fallback (single deploy)
```

## Configuration

| Env/flag         | Default                                       | Purpose                        |
|------------------|-----------------------------------------------|--------------------------------|
| `GM_API_BASE`    | `https://gm-console.kanvas.dev/api`            | upstream API base              |
| `GM_BUILD_FLAVOR`| `dev`                                         | `dev` \| `staging` \| `release`|

## Status (Mavis scaffold)

- pubspec with deps ✓
- lib/main.dart scaffold pending worker-B
- Android/iOS shells pending worker-B
- Screenshots, store listing, signing material → worker-C
