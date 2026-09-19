//! gm-console error types. Public API errors are mapped to user-safe JSON responses;
//! internal errors retain their chain in tracing logs only.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("config error: {0}")]
    Config(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("upstream error: {0}")]
    Upstream(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            Error::Config(_) | Error::Io(_) => (StatusCode::INTERNAL_SERVER_ERROR, "CONFIG_ERROR"),
            Error::Upstream(_) => (StatusCode::BAD_GATEWAY, "UPSTREAM_ERROR"),
            Error::NotFound(_) => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            Error::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
        };

        if status.is_server_error() {
            tracing::error!(error = ?self, "gm-console server error");
        }

        let body = Json(json!({
            "error": {
                "code": code,
                "message": self.to_string(),
            }
        }));

        (status, body).into_response()
    }
}
