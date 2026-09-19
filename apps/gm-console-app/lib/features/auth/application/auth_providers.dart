import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../data/auth_repository.dart';

class AuthState {
  const AuthState({
    required this.isAuthenticated,
    this.token,
    this.isBusy = false,
    this.error,
  });

  final bool isAuthenticated;
  final String? token;
  final bool isBusy;
  final String? error;

  AuthState copyWith({
    bool? isAuthenticated,
    String? token,
    bool? isBusy,
    Object? error = _sentinel,
  }) {
    return AuthState(
      isAuthenticated: isAuthenticated ?? this.isAuthenticated,
      token: token ?? this.token,
      isBusy: isBusy ?? this.isBusy,
      error: identical(error, _sentinel) ? this.error : error as String?,
    );
  }

  static const Object _sentinel = Object();
}

class AuthController extends StateNotifier<AuthState> {
  AuthController(this._repo) : super(const AuthState(isAuthenticated: false));

  final AuthRepository _repo;

  Future<void> bootstrap() async {
    final token = await _repo.currentToken();
    if (token == null || token.isEmpty) return;
    state = state.copyWith(isAuthenticated: true, token: token);
  }

  Future<void> login({required String email, required String password}) async {
    state = state.copyWith(isBusy: true, error: null);
    try {
      final token = await _repo.login(email: email, password: password);
      state = state.copyWith(
        isAuthenticated: true,
        token: token,
        isBusy: false,
        error: null,
      );
    } catch (e) {
      state = state.copyWith(isBusy: false, error: _readableError(e));
    }
  }

  Future<void> logout() async {
    state = state.copyWith(isBusy: true, error: null);
    await _repo.logout();
    state = const AuthState(isAuthenticated: false);
  }

  Future<void> revoke() async {
    await _repo.revokeToken();
    state = const AuthState(isAuthenticated: false);
  }

  String _readableError(Object e) {
    final msg = e.toString();
    if (msg.length > 240) return '${msg.substring(0, 240)}…';
    return msg;
  }
}

final authControllerProvider =
    StateNotifierProvider<AuthController, AuthState>((ref) {
  return AuthController(ref.watch(authRepositoryProvider));
});