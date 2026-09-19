// Thin wrapper over `flutter_secure_storage`.
//
// All keys are namespaced with `gm_console_` to avoid collision with any
// other module on the same device. Tokens are NEVER logged or printed.

import 'package:flutter_secure_storage/flutter_secure_storage.dart';

/// Secure storage singleton for the gm-console mobile app.
class SecureStorage {
  SecureStorage({FlutterSecureStorage? backend})
      : _backend = backend ??
            const FlutterSecureStorage(
              aOptions: AndroidOptions(encryptedSharedPreferences: true),
              iOptions: IOSOptions(accessibility: KeychainAccessibility.first_unlock),
            );

  static const String _keyPrefix = 'gm_console_';

  final FlutterSecureStorage _backend;

  String _k(String key) => '$_keyPrefix$key';

  Future<String?> read(String key) => _backend.read(key: _k(key));

  Future<void> write(String key, String value) =>
      _backend.write(key: _k(key), value: value);

  Future<void> delete(String key) => _backend.delete(key: _k(key));

  Future<void> deleteAll() => _backend.deleteAll();
}