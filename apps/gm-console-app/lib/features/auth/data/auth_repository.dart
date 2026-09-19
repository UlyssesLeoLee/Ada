import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../../api/endpoints/auth_api.dart';
import '../../core/secure_storage.dart';
import '../../state/build_info_providers.dart';
import '../../state/locale_controller.dart';

const String kAuthTokenStorageKey = 'auth_token';

class AuthRepository {
  AuthRepository(this._storage, AuthApi api) : _api = api;

  final SecureStorage _storage;
  final AuthApi _api;

  Future<String> login({
    required String email,
    required String password,
  }) async {
    final result = await _api.login(email: email, password: password);
    await _storage.write(kAuthTokenStorageKey, result.token);
    return result.token;
  }

  Future<void> logout() async {
    // Best-effort revoke upstream; even if it fails we still clear local state.
    try {
      await _api.logout();
    } catch (_) {
      // Swallow: caller cares about local revocation.
    }
    await _storage.delete(kAuthTokenStorageKey);
  }

  Future<String?> currentToken() => _storage.read(kAuthTokenStorageKey);

  Future<void> revokeToken() async {
    await _storage.delete(kAuthTokenStorageKey);
  }
}

/// Token provider used by [ApiClient] to attach `Authorization` headers.
TokenProvider tokenProviderFor(Ref ref) {
  final storage = ref.watch(secureStorageProvider);
  return () async => storage.read(kAuthTokenStorageKey);
}

final apiClientProvider = Provider<ApiClient>((ref) {
  final config = ref.watch(apiConfigProvider);
  final client = ApiClient(
    config: config,
    tokenProvider: tokenProviderFor(ref),
  );
  ref.onDispose(client.close);
  return client;
});

final authRepositoryProvider = Provider<AuthRepository>((ref) {
  final storage = ref.watch(secureStorageProvider);
  final api = AuthApi(ref.watch(apiClientProvider));
  return AuthRepository(storage, api);
});