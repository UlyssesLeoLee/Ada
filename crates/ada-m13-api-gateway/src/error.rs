//! API gateway error surface.
//!
//! This module defines [`ApiError`], the error type returned by all
//! gateway handlers, plus a [`Result`] alias. [`ApiError`] implements
//! `axum::response::IntoResponse` so it can be returned directly from
//! handler functions.
//!
//! ## Status code mapping
//!
//! | Variant             | HTTP status |
//! |---------------------|-------------|
//! | `NotFound`          | 404         |
//! | `BadRequest`        | 400         |
//! | `Unauthorized`      | 401         |
//! | `ServiceUnavailable`| 503         |
//! | `Internal`          | 500         |
//!
//! See [`DOC-MOD-013`](../docs/modules/M-13-api-gateway.md) §3.1 中間
//! ウェアチェーン and `docs/api/error-codes.md` §2 for the canonical
//! error-code table that this mapping is a subset of.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// Errors that can be returned from a gateway handler.
#[derive(Debug, Error)]
pub enum ApiError {
    /// Resource not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// Caller supplied a malformed or invalid request.
    #[error("bad request: {0}")]
    BadRequest(String),

    /// Authentication or authorization failure.
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    /// A backing service is unavailable (DB, upstream, peer).
    #[error("service unavailable: {0}")]
    ServiceUnavailable(String),

    /// Catch-all for unexpected / invariant-violation failures.
    #[error("internal error: {0}")]
    Internal(String),
}

impl ApiError {
    /// Map the variant to its canonical HTTP status code.
    #[must_use]
    pub const fn status(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        let body = Json(json!({
            "error": {
                "code": status.as_u16(),
                "message": self.to_string(),
            }
        }));
        let mut resp = (status, body).into_response();
        // A 401 without a challenge is a malformed 401: RFC 7235 §3.1
        // requires the server to advertise the scheme it wants, and a
        // client that follows the spec retries with a credential rather
        // than showing the user a bare error. The gateway only ever
        // accepts `Bearer`, so that is the only challenge emitted.
        if status == StatusCode::UNAUTHORIZED {
            resp.headers_mut().insert(
                axum::http::header::WWW_AUTHENTICATE,
                axum::http::HeaderValue::from_static("Bearer"),
            );
        }
        resp
    }
}

/// `Result` alias for fallible gateway operations.
pub type Result<T> = core::result::Result<T, ApiError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_code_mapping() {
        assert_eq!(
            ApiError::NotFound("x".into()).status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            ApiError::BadRequest("x".into()).status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ApiError::Unauthorized("x".into()).status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            ApiError::ServiceUnavailable("x".into()).status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            ApiError::Internal("x".into()).status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn display_strings_have_prefix() {
        assert_eq!(
            ApiError::NotFound("thing".into()).to_string(),
            "not found: thing"
        );
        assert_eq!(
            ApiError::BadRequest("bad".into()).to_string(),
            "bad request: bad"
        );
    }

    #[test]
    fn into_response_returns_status() {
        let resp = ApiError::Unauthorized("nope".into()).into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    /// A 401 has to carry the challenge, or a spec-following client
    /// never learns which credential to send. Asserted here rather than
    /// left implicit: the header is one line and silently dropping it
    /// regresses every unauthenticated response at once.
    #[test]
    fn a_401_carries_the_bearer_challenge() {
        let resp = ApiError::Unauthorized("nope".into()).into_response();
        let challenge = resp
            .headers()
            .get("www-authenticate")
            .and_then(|v| v.to_str().ok());
        assert_eq!(challenge, Some("Bearer"));
    }

    /// The challenge belongs to 401 only. Leaking it onto a 404 or a
    /// 500 tells the client to retry a request that will never be
    /// authenticated.
    #[test]
    fn a_non_401_carries_no_challenge() {
        for e in [
            ApiError::NotFound("x".into()),
            ApiError::BadRequest("x".into()),
            ApiError::ServiceUnavailable("x".into()),
            ApiError::Internal("x".into()),
        ] {
            let resp = e.into_response();
            assert!(
                resp.headers().get("www-authenticate").is_none(),
                "{} must not advertise a challenge",
                resp.status()
            );
        }
    }
}
