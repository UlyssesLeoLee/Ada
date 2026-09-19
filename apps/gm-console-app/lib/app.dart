import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'features/auth/application/auth_providers.dart';
import 'routing/app_router.dart';
import 'state/locale_controller.dart';
import 'theme.dart';

class GmConsoleApp extends ConsumerStatefulWidget {
  const GmConsoleApp({super.key});

  @override
  ConsumerState<GmConsoleApp> createState() => _GmConsoleAppState();
}

class _GmConsoleAppState extends ConsumerState<GmConsoleApp> {
  @override
  void initState() {
    super.initState();
    // Bootstrap auth from secure storage; load persisted locale.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      ref.read(authControllerProvider.notifier).bootstrap();
      ref.read(localeControllerProvider.notifier).load();
    });
  }

  @override
  Widget build(BuildContext context) {
    final router = ref.watch(routerProvider);
    final locale = ref.watch(localeControllerProvider);
    // Brand-themed MaterialApp. light/dark chosen by platform brightness so
    // the runtime honors `MediaQueryData.platformBrightness` (and indirectly
    // iOS/Android dark-mode toggles). No inline `ColorScheme.fromSeed`
    // remains — brand tokens live in `lib/theme/brand_theme.dart`.
    return MaterialApp.router(
      title: 'gm-console',
      theme: BrandTheme.light(),
      darkTheme: BrandTheme.dark(),
      themeMode: ThemeMode.system,
      locale: locale,
      supportedLocales: kSupportedLocales,
      routerConfig: router,
      debugShowCheckedModeBanner: false,
    );
  }
}