//! Error types for the remediation engine.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, RemediationError>;

#[derive(Debug, Error)]
pub enum RemediationError {
    #[error("remediation action not found: {0}")]
    ActionNotFound(String),

    #[error("runbook file is invalid: {0}")]
    InvalidRunbook(String),

    #[error("step {index} failed: {message}")]
    StepFailed { index: usize, message: String },

    #[error("command failed (exit={code:?}): {stderr}")]
    CommandFailed { code: Option<i32>, stderr: String },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("invalid state transition: from {from:?} to {to:?}")]
    InvalidStateTransition { from: String, to: String },
}
