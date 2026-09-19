import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:gm_console_app/api/api_client.dart';
import 'package:gm_console_app/api/api_config.dart';
import 'package:gm_console_app/api/api_exception.dart';
import 'package:http/http.dart' as http;

/// In-memory [http.Client] that returns canned responses without hitting the
/// network. Default 200 + JSON; configurable per path.
class _StubHttpClient extends http.BaseClient {
  _StubHttpClient(this._handler);

  final FutureOr<http.Response> Function(http.BaseRequest request) _handler;

  @override
  Future<http.StreamedResponse> send(http.BaseRequest request) async {
    final response = await _handler(request);
    return http.StreamedResponse(
      Stream.value(response.bodyBytes),
      response.statusCode,
      headers: response.headers,
      reasonPhrase: response.reasonPhrase,
      request: request,
    );
  }
}

void main() {
  const config =
      ApiConfig(baseUrl: 'https://example.test/api', flavor: 'test');

  ApiClient build(FutureOr<http.Response> Function(http.BaseRequest) handler,
      {String? token}) {
    return ApiClient(
      config: config,
      httpClient: _StubHttpClient(handler),
      tokenProvider: () async => token,
    );
  }

  group('ApiClient error mapping', () {
    test('maps 401 to ApiUnauthorizedException', () async {
      final client = build(
        (_) => http.Response(jsonEncode({'error': 'no token'}), 401,
            headers: {'content-type': 'application/json'}),
      );
      expect(
        () => client.getJson('/v1/tenants'),
        throwsA(isA<ApiUnauthorizedException>()
            .having((e) => e.statusCode, 'statusCode', 401)
            .having((e) => e.message, 'message', contains('no token'))),
      );
    });

    test('maps 404 to ApiNotFoundException', () async {
      final client = build(
        (_) => http.Response('not here', 404,
            headers: {'content-type': 'text/plain'}),
      );
      expect(
        () => client.getJson('/v1/pipelines/missing'),
        throwsA(isA<ApiNotFoundException>()
            .having((e) => e.statusCode, 'statusCode', 404)),
      );
    });

    test('maps 500 to ApiServerException', () async {
      final client = build(
        (_) => http.Response(jsonEncode({'error': 'kaboom'}), 500),
      );
      expect(
        () => client.getJson('/v1/audit'),
        throwsA(isA<ApiServerException>()
            .having((e) => e.statusCode, 'statusCode', 500)),
      );
    });

    test('maps network failures to ApiNetworkException', () async {
      final client = build((_) {
        throw const SocketException('host unreachable');
      });
      try {
        await client.getJson('/v1/incidents');
        fail('should have thrown');
      } on ApiException catch (e) {
        expect(e, isA<ApiNetworkException>());
      }
    });

    test('sends bearer token when provided', () async {
      http.BaseRequest? captured;
      final client = build(
        (request) {
          captured = request;
          return http.Response(jsonEncode({'ok': true}), 200);
        },
        token: 'jwt-xyz',
      );
      await client.getJson('/v1/tenants');
      expect(captured!.headers['authorization'], 'Bearer jwt-xyz');
    });
  });
}

/// Stub placeholder retained for any future custom-failure subclasses.
/// Currently unused; real [SocketException] from `dart:io` covers the
/// transport-failure test above.
class _Unused {}