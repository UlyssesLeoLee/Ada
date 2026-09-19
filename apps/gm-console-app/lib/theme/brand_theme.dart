// gm-console Mobile — brand theme.
//
// Mirrors docs/commercial/brand/TOKENS.md into Flutter's `ThemeData` shape.
// Two named constructors (`.light`, `.dark`) let `app.dart` switch by user
// preference without leaking `ColorScheme.fromSeed` plumbing into the
// root widget.
//
// Rationale (per TOKENS.md §Use in code):
//   - Start from `ColorScheme.fromSeed(seedColor: Color(0xFF1E88E5))` so
//     Material 3 tonal palettes are derived automatically.
//   - Then override the surfaces/text/brand extensions to match the
//     exact hex tokens (no tonal drift between surfaces).
//
// Caveats:
//   - status-ok/warn/err and motion durations are exposed via
//     `BrandThemeExtension` so screens can read them via
//     `Theme.of(context).extension<BrandThemeExtension>()!`.
//   - Typography uses Roboto fallback chain (no web font bundle) so the
//     seed colors are not visually muted by missing fonts.

import 'package:flutter/material.dart';

/// Brand-specific tokens that don't fit cleanly into `ColorScheme` /
/// `TextTheme` / `ThemeData` — kept in a `ThemeExtension` so screens can
/// pull `statusOk`, motion durations, etc. via `Theme.of(context).extension`.
@immutable
class BrandThemeExtension extends ThemeExtension<BrandThemeExtension> {
  const BrandThemeExtension({
    required this.statusOk,
    required this.statusWarn,
    required this.statusErr,
    required this.brandMuted,
    required this.radiusSm,
    required this.radiusMd,
    required this.radiusLg,
    required this.durFast,
    required this.durBase,
    required this.durSlow,
    required this.easeStandard,
  });

  final Color statusOk;
  final Color statusWarn;
  final Color statusErr;

  /// Subtle brand-tinted background (TOKENS.md `--brand-100`).
  final Color brandMuted;

  final double radiusSm;
  final double radiusMd;
  final double radiusLg;

  final Duration durFast;
  final Duration durBase;
  final Duration durSlow;
  final Curve easeStandard;

  static const _light = BrandThemeExtension(
    statusOk:    Color(0xFF2E7D32),
    statusWarn:  Color(0xFFED6C02),
    statusErr:   Color(0xFFC62828),
    brandMuted:  Color(0xFFE3F2FD),
    radiusSm:    4,
    radiusMd:    8,
    radiusLg:    12,
    durFast:     Duration(milliseconds: 120),
    durBase:     Duration(milliseconds: 180),
    durSlow:     Duration(milliseconds: 240),
    easeStandard: Cubic(0.2, 0, 0, 1),
  );

  static const _dark = BrandThemeExtension(
    statusOk:    Color(0xFF66BB6A), // lifted for contrast on dark surfaces
    statusWarn:  Color(0xFFFFB74D),
    statusErr:   Color(0xFFEF5350),
    brandMuted:  Color(0xFF1A2733),
    radiusSm:    4,
    radiusMd:    8,
    radiusLg:    12,
    durFast:     Duration(milliseconds: 0), // respect prefers-reduced-motion default
    durBase:     Duration(milliseconds: 0),
    durSlow:     Duration(milliseconds: 0),
    easeStandard: Cubic(0.2, 0, 0, 1),
  );

  @override
  BrandThemeExtension copyWith({
    Color? statusOk,
    Color? statusWarn,
    Color? statusErr,
    Color? brandMuted,
    double? radiusSm,
    double? radiusMd,
    double? radiusLg,
    Duration? durFast,
    Duration? durBase,
    Duration? durSlow,
    Curve? easeStandard,
  }) {
    return BrandThemeExtension(
      statusOk: statusOk ?? this.statusOk,
      statusWarn: statusWarn ?? this.statusWarn,
      statusErr: statusErr ?? this.statusErr,
      brandMuted: brandMuted ?? this.brandMuted,
      radiusSm: radiusSm ?? this.radiusSm,
      radiusMd: radiusMd ?? this.radiusMd,
      radiusLg: radiusLg ?? this.radiusLg,
      durFast: durFast ?? this.durFast,
      durBase: durBase ?? this.durBase,
      durSlow: durSlow ?? this.durSlow,
      easeStandard: easeStandard ?? this.easeStandard,
    );
  }

  @override
  BrandThemeExtension lerp(ThemeExtension<BrandThemeExtension>? other, double t) {
    if (other is! BrandThemeExtension) return this;
    return BrandThemeExtension(
      statusOk:    Color.lerp(statusOk,    other.statusOk,    t)!,
      statusWarn:  Color.lerp(statusWarn,  other.statusWarn,  t)!,
      statusErr:   Color.lerp(statusErr,   other.statusErr,   t)!,
      brandMuted:  Color.lerp(brandMuted,  other.brandMuted,  t)!,
      radiusSm:    radiusSm,
      radiusMd:    radiusMd,
      radiusLg:    radiusLg,
      durFast:     durFast,
      durBase:     durBase,
      durSlow:     durSlow,
      easeStandard: easeStandard,
    );
  }
}

/// Brand-aware ThemeData factory.
///
/// `BrandTheme.light()` and `BrandTheme.dark()` are the only sanctioned
/// entry points. `app.dart` selects between them based on the platform
/// `MediaQuery.platformBrightness`.
class BrandTheme {
  BrandTheme._();

  static const Color _seed = Color(0xFF1E88E5);

  /// Light theme — primary brand `#1E88E5` on `#FAFCFF` surface.
  static ThemeData light() {
    final scheme = ColorScheme.fromSeed(
      seedColor: _seed,
      brightness: Brightness.light,
    );
    // Override surfaces/text so they match TOKENS.md exactly (no Material 3
    // tonal drift). Status colors / brand-muted / motion live in the
    // extension so screens can pull them by name.
    final overridden = scheme.copyWith(
      surface:  const Color(0xFFFAFCFF),
      surfaceContainerHighest: const Color(0xFFEEF4FB),
      onSurface: const Color(0xFF1F2933),
      onSurfaceVariant: const Color(0xFF52606D),
      primary: const Color(0xFF1E88E5),
      secondary: const Color(0xFF1976D2),
    );

    return _baseTheme(overridden).copyWith(
      extensions: const [BrandThemeExtension._light],
    );
  }

  /// Dark theme — primary brand `#42A5F5` on `#0E1217` surface.
  static ThemeData dark() {
    final scheme = ColorScheme.fromSeed(
      seedColor: _seed,
      brightness: Brightness.dark,
    );
    final overridden = scheme.copyWith(
      surface:  const Color(0xFF0E1217),
      surfaceContainerHighest: const Color(0xFF161B22),
      onSurface: const Color(0xFFF0F6FC),
      onSurfaceVariant: const Color(0xFFC9D1D9),
      primary: const Color(0xFF42A5F5),
      secondary: const Color(0xFF64B5F6),
    );

    return _baseTheme(overridden).copyWith(
      extensions: const [BrandThemeExtension._dark],
    );
  }

  static ThemeData _baseTheme(ColorScheme scheme) {
    return ThemeData(
      colorScheme: scheme,
      useMaterial3: true,
      visualDensity: VisualDensity.adaptivePlatformDensity,
      // Body type — per TOKENS.md §Typography.
      textTheme: const TextTheme(
        displayLarge: TextStyle(fontSize: 56, fontWeight: FontWeight.w700),
        displayMedium: TextStyle(fontSize: 44, fontWeight: FontWeight.w700),
        displaySmall: TextStyle(fontSize: 32, fontWeight: FontWeight.w700),
        headlineLarge: TextStyle(fontSize: 32, fontWeight: FontWeight.w700),
        headlineMedium: TextStyle(fontSize: 28, fontWeight: FontWeight.w700),
        headlineSmall: TextStyle(fontSize: 24, fontWeight: FontWeight.w700),
        titleLarge: TextStyle(fontSize: 20, fontWeight: FontWeight.w600),
        titleMedium: TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
        bodyLarge: TextStyle(fontSize: 16, height: 1.6),
        bodyMedium: TextStyle(fontSize: 14, height: 1.5),
        bodySmall: TextStyle(fontSize: 13, height: 1.5),
        labelLarge: TextStyle(fontSize: 14, fontWeight: FontWeight.w600),
        labelMedium: TextStyle(fontSize: 12, fontWeight: FontWeight.w600),
        labelSmall: TextStyle(fontSize: 11, fontWeight: FontWeight.w500),
      ),
    );
  }
}