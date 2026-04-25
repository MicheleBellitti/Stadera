//! HTTP error type for the API.
//!
//! `AppError` implements [`IntoResponse`] so handlers can return
//! `Result<T, AppError>` and the failure path produces a structured JSON
//! body. Internal/storage errors are **logged** but **never leaked** to
//! the client — only a generic "internal error" message is returned.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use thiserror::Error;
use tracing::error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("not found")]
    NotFound,

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("unauthorized")]
    Unauthorized,

    #[error("forbidden")]
    Forbidden,

    /// Wraps any storage-layer failure. Details are logged, not returned.
    #[error("storage error: {0}")]
    Storage(#[from] stadera_storage::StorageError),

    /// Catch-all for unexpected failures. Details are logged, not returned.
    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

#[derive(Serialize)]
struct ErrorBody {
    error: &'static str,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "not_found", "not found".to_string()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "bad_request", msg.clone()),
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "unauthorized".to_string(),
            ),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "forbidden", "forbidden".to_string()),
            AppError::Storage(_) | AppError::Internal(_) => {
                error!(error = %self, "internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "internal server error".to_string(),
                )
            }
        };
        let body = Json(ErrorBody {
            error: code,
            message,
        });
        (status, body).into_response()
    }
}
