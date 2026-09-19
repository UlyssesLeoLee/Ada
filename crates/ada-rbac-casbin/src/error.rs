use thiserror::Error;

pub type Result<T> = std::result::Result<T, RbacCasbinError>;

#[derive(Debug, Error)]
pub enum RbacCasbinError {
    #[error("casbin: {0}")]
    Internal(String),

    #[error("invalid subject (user id required): {0}")]
    InvalidSubject(String),

    #[error("invalid object: {0}")]
    InvalidObject(String),

    #[error("invalid action token: {0}")]
    InvalidAction(String),

    #[error("admin api: caller lacks Role::Owner")]
    AdminDenied,

    #[error("policy reload failed: {0}")]
    ReloadFailed(String),
}