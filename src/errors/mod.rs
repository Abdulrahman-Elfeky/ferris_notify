use axum::{response::IntoResponse, Json};
use reqwest::StatusCode;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Http client error")]
    HttpClient(#[from] reqwest::Error),
    #[error("Validation error {0}")]
    Validation(String),
    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),
    #[error("Database error occurred")]
    Database(#[from] sqlx::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message, error_code) = match &self {
            ApiError::Validation(_) => (
                StatusCode::BAD_REQUEST,
                self.to_string(),
                "VALIDATION_ERROR",
            ),
            ApiError::HttpClient(_) => {
                tracing::error!("HTTP client error: {:?}", self);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "External service error".to_string(),
                    "EXTERNAL_SERVICE_ERROR",
                )
            }
            ApiError::Internal(_) => {
                tracing::error!("Internal error: {:?}", self);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    self.to_string(),
                    "INTERNAL_ERROR",
                )
            }
            ApiError::Database(_) => {
                tracing::error!("Database error: {:?}", self);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    self.to_string(),
                    "DATABASE_ERROR",
                )
            }
        };
        let body = json!({
            "message":error_message,
            "code":error_code,
        });
        (status, Json(body)).into_response()
    }
}
