// gm-console Mobile — theme barrel.
//
// Re-exports BrandTheme + the BrandThemeExtension so callers can:
//   import 'package:gm_console_app/theme.dart';
// without needing to know the internal `theme/` folder layout.

export 'theme/brand_theme.dart'
    show
        BrandTheme,
        BrandThemeExtension;