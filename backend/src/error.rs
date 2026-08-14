//! One error type for the whole API, rendered as a stable JSON envelope.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0} not found")]
    NotFound(String),

    #[error("{message}")]
    BadRequest {
        message: String,
        details: Vec<String>,
    },

    #[error("{0}")]
    Conflict(String),

    /// An extractor refused the request before the handler ever ran (bad JSON, a missing
    /// `content-type`, a malformed path parameter). Carried through this type so those replies
    /// use the same `{"error": {...}}` envelope as everything else instead of axum's plain text.
    #[error("{message}")]
    Rejection {
        status: StatusCode,
        code: &'static str,
        message: String,
    },

    #[error("content pack error: {0}")]
    Content(String),

    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl AppError {
    pub fn bad_request(message: impl Into<String>) -> AppError {
        AppError::BadRequest {
            message: message.into(),
            details: Vec::new(),
        }
    }

    pub fn with_details(message: impl Into<String>, details: Vec<String>) -> AppError {
        AppError::BadRequest {
            message: message.into(),
            details,
        }
    }

    /// Wrap an extractor rejection, preserving the status axum chose for it.
    pub fn rejection(status: StatusCode, message: impl Into<String>) -> AppError {
        let code = match status {
            StatusCode::UNSUPPORTED_MEDIA_TYPE => "unsupported_media_type",
            StatusCode::UNPROCESSABLE_ENTITY => "unprocessable_entity",
            StatusCode::PAYLOAD_TOO_LARGE => "payload_too_large",
            _ => "bad_request",
        };
        AppError::Rejection {
            status,
            code,
            message: message.into(),
        }
    }

    pub fn status(&self) -> StatusCode {
        match self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Rejection { status, .. } => *status,
            AppError::Content(_) | AppError::Database(_) | AppError::Json(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            AppError::NotFound(_) => "not_found",
            AppError::BadRequest { .. } => "bad_request",
            AppError::Conflict(_) => "conflict",
            AppError::Rejection { code, .. } => code,
            AppError::Content(_) => "content_error",
            AppError::Database(_) => "database_error",
            AppError::Json(_) => "serialization_error",
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        let code = self.code();
        if status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!(error = %self, "request failed");
        }
        let details = match &self {
            AppError::BadRequest { details, .. } => details.clone(),
            _ => Vec::new(),
        };
        let body = json!({
            "error": {
                "code": code,
                "message": self.to_string(),
                "details": details,
            }
        });
        (status, Json(body)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
