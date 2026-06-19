use thiserror::Error;
use tonic::Status;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Authorization denied: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Rate limit exceeded")]
    RateLimit,

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<AppError> for Status {
    fn from(err: AppError) -> Self {
        match err {
            AppError::Database(e) => {
                tracing::error!("Database error: {}", e);
                Status::internal("Internal server error")
            }
            AppError::Auth(msg) => Status::unauthenticated(msg),
            AppError::Forbidden(msg) => Status::permission_denied(msg),
            AppError::NotFound(msg) => Status::not_found(msg),
            AppError::Validation(msg) => Status::invalid_argument(msg),
            AppError::Conflict(msg) => Status::already_exists(msg),
            AppError::RateLimit => Status::resource_exhausted("Rate limit exceeded"),
            AppError::Encryption(msg) => {
                tracing::error!("Encryption error: {}", msg);
                Status::internal("Internal server error")
            }
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                Status::internal("Internal server error")
            }
        }
    }
}
