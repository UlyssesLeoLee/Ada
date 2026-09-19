// API base configuration.
//
// `GM_API_BASE` defaults to the public gm-console reverse-proxy host
// (`https://gm-console.kanvas.dev/api`). For local development, build with
// `--dart-define=GM_API_BASE=http://localhost:8080/api`. The build flavor is
// separately configurable via `GM_BUILD_FLAVOR` (default `dev`).

import 'package:flutter/foundation.dart';

@immutable
class ApiConfig {
  const ApiConfig({
    required this.baseUrl,
    required this.flavor,
  });

  /// Default API base URL. Override with `--dart-define=GM_API_BASE=...`.
  static const String defaultBaseUrl =
      String.fromEnvironment('GM_API_BASE',
          defaultValue: 'https://gm-console.kanvas.dev/api');

  /// Build flavor (`dev`, `staging`, `release`). Override with
  /// `--dart-define=GM_BUILD_FLAVOR=...`.
  static const String defaultFlavor = String.fromEnvironment(
    'GM_BUILD_FLAVOR',
    defaultValue: 'dev',
  );

  final String baseUrl;
  final String flavor;

  factory ApiConfig.fromEnvironment() {
    return const ApiConfig(
      baseUrl: defaultBaseUrl,
      flavor: defaultFlavor,
    );
  }

  /// Resolves a path against [baseUrl]. Ensures exactly one `/` separator.
  String resolve(String path) {
    final trimmed = path.startsWith('/') ? path : '/$path';
    if (baseUrl.endsWith('/')) {
      return '$baseUrl${trimmed.substring(1)}';
    }
    return '$baseUrl$trimmed';
  }

  @override
  String toString() => 'ApiConfig(baseUrl: $baseUrl, flavor: $flavor)';
}