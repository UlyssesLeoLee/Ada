// Auth endpoint wrapper — login, logout, refresh.

import '../api_client.dart';

class LoginResult {
  const LoginResult({required this.token, required this.expiresAt});
  final String token;
  final DateTime? expiresAt;
}

class AuthApi {
  AuthApi(this._client);

  final ApiClient _client;

  /// POST /api/v1/auth/login — email + password login.
  ///
  /// Returns the issued JWT and its optional expiry. The token is the caller's
  /// responsibility to persist (typically into [SecureStorage]).
  Future<LoginResult> login({
    required String email,
    required String password,
  }) async {
    final body = await _client.postJson('/api/v1/auth/login', body: {
      'email': email,
      'password': password,
    });
    final token = body['token'] as String?;
    if (token == null || token.isEmpty) {
      throw const ApiUnauthorizedException(
        'login response missing token',
        statusCode: 401,
      );
    }
    DateTime? expiresAt;
    final raw = body['expires_at'];
    if (raw is String) {
      expiresAt = DateTime.tryParse(raw);
    }
    return LoginResult(token: token, expiresAt: expiresAt);
  }

  /// POST /api/v1/auth/logout — revoke the current session.
  Future<void> logout() async {
    await _client.postJson('/api/v1/auth/logout');
  }

  /// POST /api/v1/auth/sso — SSO ticket exchange (stub).
  ///
  /// Real implementation will be filled in once the SSO ticket format is
  /// finalized upstream. Until then this returns an empty body so the call
  /// site can wire its UI without 404-ing on every tap.
  Future<Map<String, dynamic>> ssoExchange(String ticket) {
    return _client.postJson('/api/v1/auth/sso', body: {'ticket': ticket});
  }
}