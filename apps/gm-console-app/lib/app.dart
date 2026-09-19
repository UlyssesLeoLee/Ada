import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'features/auth/application/auth_providers.dart';
import 'routing/app_router.dart';
import 'state/locale_controller.dart';

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
    return MaterialApp.router(
      title: 'gm-console',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: const Color(0xFF1E88E5)),
        useMaterial3: true,
      ),
      locale: locale,
      supportedLocales: kSupportedLocales,
      routerConfig: router,
      debugShowCheckedModeBanner: false,
    );
  }
}