// Typed exception hierarchy for the API layer.
//
// Error mapping (4xx / 5xx / network) is centralized here so feature code
// can pattern-match on subtypes instead of inspecting raw `http.Response`
// objects.

sealed class ApiException implements Exception {
  const ApiException(this.message, {this.statusCode, this.cause});

  final String message;
  final int? statusCode;
  final Object? cause;

  @override
  String toString() => 'ApiException($message, status=$statusCode)';
}

/// HTTP 400 — caller provided invalid input.
class ApiBadRequestException extends ApiException {
  const ApiBadRequestException(super.message, {super.statusCode, super.cause});
}

/// HTTP 401 — credentials missing or rejected. Triggers re-auth flow.
class ApiUnauthorizedException extends ApiException {
  const ApiUnauthorizedException(super.message,
      {super.statusCode, super.cause});
}

/// HTTP 403 — authenticated but not allowed.
class ApiForbiddenException extends ApiException {
  const ApiForbiddenException(super.message, {super.statusCode, super.cause});
}

/// HTTP 404 — resource not found.
class ApiNotFoundException extends ApiException {
  const ApiNotFoundException(super.message, {super.statusCode, super.cause});
}

/// HTTP 5xx — server-side failure.
class ApiServerException extends ApiException {
  const ApiServerException(super.message, {super.statusCode, super.cause});
}

/// Transport-level failure (DNS, TLS, timeout, socket).
class ApiNetworkException extends ApiException {
  const ApiNetworkException(super.message, {super.cause})
      : super(statusCode: null);
}

/// Anything that doesn't fit the above (e.g. malformed JSON body).
class ApiUnknownException extends ApiException {
  const ApiUnknownException(super.message, {super.statusCode, super.cause});
}

/// Maps an HTTP status code + body to the appropriate [ApiException] subtype.
///
/// [body] is the decoded JSON map when available; raw text otherwise.
ApiException mapHttpStatusToApiException(
  int statusCode,
  String? body, {
  Object? cause,
}) {
  final trimmed = (body ?? '').trim();
  final message = trimmed.isEmpty ? 'HTTP $statusCode' : trimmed;

  switch (statusCode) {
    case 400:
      return ApiBadRequestException(message, statusCode: statusCode, cause: cause);
    case 401:
      return ApiUnauthorizedException(message,
          statusCode: statusCode, cause: cause);
    case 403:
      return ApiForbiddenException(message,
          statusCode: statusCode, cause: cause);
    case 404:
      return ApiNotFoundException(message,
          statusCode: statusCode, cause: cause);
    default:
      if (statusCode >= 400 && statusCode < 500) {
        return ApiUnknownException(message,
            statusCode: statusCode, cause: cause);
      }
      if (statusCode >= 500) {
        return ApiServerException(message,
            statusCode: statusCode, cause: cause);
      }
      return ApiUnknownException(message,
          statusCode: statusCode, cause: cause);
  }
}