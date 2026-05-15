//! Unified application error type for Fluvio microservices.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Graph error: {0}")]
    Graph(String),

    #[error("Connector error: {0}")]
    Connector(String),

    #[error("Ingestion error: {0}")]
    Ingestion(String),

    #[error("Internal server error: {0}")]
    Internal(String),
}

impl AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_)    => StatusCode::FORBIDDEN,
            AppError::BadRequest(_)   => StatusCode::BAD_REQUEST,
            AppError::NotFound(_)     => StatusCode::NOT_FOUND,
            AppError::Conflict(_)     => StatusCode::CONFLICT,
            _                         => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn internal(e: impl std::fmt::Display) -> Self {
        Self::Internal(e.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body   = Json(json!({
            "error": {
                "code":    status.as_u16(),
                "message": self.to_string(),
            }
        }));
        (status, body).into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        Self::Internal(e.to_string())
    }
}
