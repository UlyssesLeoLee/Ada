// Typed HTTP client built on `package:http`.
//
// Responsibilities:
//   * Build URLs from [ApiConfig]
//   * Inject bearer token if present (no logging, no printing)
//   * Decode JSON responses
//   * Map non-2xx responses to the typed [ApiException] hierarchy
//   * Surface network-level failures as [ApiNetworkException]

import 'dart:async';
import 'dart:convert';

import 'package:http/http.dart' as http;

import 'api_config.dart';
import 'api_exception.dart';

typedef TokenProvider = Future<String?> Function();

class ApiClient {
  ApiClient({
    required this.config,
    required this.tokenProvider,
    http.Client? httpClient,
    Duration timeout = const Duration(seconds: 30),
  })  : _http = httpClient ?? http.Client(),
        _timeout = timeout;

  final ApiConfig config;
  final TokenProvider tokenProvider;
  final http.Client _http;
  final Duration _timeout;

  Future<Map<String, dynamic>> getJson(
    String path, {
    Map<String, String>? query,
  }) {
    return _send('GET', path, query: query);
  }

  Future<Map<String, dynamic>> postJson(
    String path, {
    Object? body,
  }) {
    return _send('POST', path, body: body);
  }

  Future<Map<String, dynamic>> putJson(
    String path, {
    Object? body,
  }) {
    return _send('PUT', path, body: body);
  }

  Future<Map<String, dynamic>> patchJson(
    String path, {
    Object? body,
  }) {
    return _send('PATCH', path, body: body);
  }

  Future<Map<String, dynamic>> deleteJson(String path) {
    return _send('DELETE', path);
  }

  Future<Map<String, dynamic>> _send(
    String method,
    String path, {
    Map<String, String>? query,
    Object? body,
  }) async {
    final url = _buildUri(path, query);
    final headers = await _buildHeaders(body);

    http.Response response;
    try {
      final request = http.Request(method, url)..headers.addAll(headers);
      if (body != null) {
        request.body = jsonEncode(body);
      }
      final streamed = await _http.send(request).timeout(_timeout);
      response = await http.Response.fromStream(streamed);
    } on TimeoutException catch (e) {
      throw ApiNetworkException('Request timed out', cause: e);
    } catch (e) {
      throw ApiNetworkException('Network failure: ${e.runtimeType}',
          cause: e);
    }

    final decoded = _safeDecode(response.body);

    if (response.statusCode >= 200 && response.statusCode < 300) {
      if (decoded == null || decoded is! Map<String, dynamic>) {
        throw const ApiUnknownException(
            'Expected JSON object body',
            statusCode: null);
      }
      return decoded;
    }

    final errorBody = decoded is String
        ? decoded
        : (decoded is Map<String, dynamic>
            ? jsonEncode(decoded)
            : response.body);
    throw mapHttpStatusToApiException(
      response.statusCode,
      errorBody,
    );
  }

  Uri _buildUri(String path, Map<String, String>? query) {
    final resolved = config.resolve(path);
    final base = Uri.parse(resolved);
    if (query == null || query.isEmpty) return base;
    return base.replace(queryParameters: {
      ...base.queryParameters,
      ...query,
    });
  }

  Future<Map<String, String>> _buildHeaders(Object? body) async {
    final headers = <String, String>{
      'accept': 'application/json',
    };
    if (body != null) {
      headers['content-type'] = 'application/json';
    }
    final token = await tokenProvider();
    if (token != null && token.isNotEmpty) {
      headers['authorization'] = 'Bearer $token';
    }
    return headers;
  }

  Object? _safeDecode(String body) {
    final trimmed = body.trim();
    if (trimmed.isEmpty) return null;
    try {
      return jsonDecode(trimmed);
    } catch (_) {
      return body;
    }
  }

  void close() => _http.close();
}