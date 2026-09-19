import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/secure_storage.dart';

const Locale kLocaleEn = Locale('en');
const Locale kLocaleJa = Locale('ja');
const Locale kLocaleZhCn = Locale('zh', 'CN');

const List<Locale> kSupportedLocales = <Locale>[
  kLocaleEn,
  kLocaleJa,
  kLocaleZhCn,
];

final localeControllerProvider =
    StateNotifierProvider<LocaleController, Locale>((ref) {
  return LocaleController(ref.watch(secureStorageProvider));
});

class LocaleController extends StateNotifier<Locale> {
  LocaleController(this._storage)
      : super(_resolveInitial(_storage));

  final SecureStorage _storage;

  static const String _key = 'locale';

  static Locale _resolveInitial(SecureStorage storage) {
    // Synchronous fallback to English until async load finishes.
    // [Future.microtask] in `load` will update if storage held a different
    // value.
    return kLocaleEn;
  }

  Future<void> load() async {
    final raw = await _storage.read(_key);
    if (raw == null) return;
    final parsed = _parseLocale(raw);
    if (parsed != null) state = parsed;
  }

  Future<void> setLocale(Locale locale) async {
    state = locale;
    await _storage.write(_key, _localeToKey(locale));
  }

  static Locale? _parseLocale(String raw) {
    final parts = raw.replaceAll('_', '-').split('-');
    if (parts.isEmpty) return null;
    final language = parts[0];
    final country = parts.length > 1 ? parts[1] : null;
    if (country != null && country.isNotEmpty) {
      return Locale(language, country);
    }
    return Locale(language);
  }

  static String _localeToKey(Locale locale) {
    if (locale.countryCode != null && locale.countryCode!.isNotEmpty) {
      return '${locale.languageCode}-${locale.countryCode}';
    }
    return locale.languageCode;
  }
}

final secureStorageProvider = Provider<SecureStorage>((ref) {
  return SecureStorage();
});