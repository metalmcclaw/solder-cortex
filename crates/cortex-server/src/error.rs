use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Wallet not found: {0}")]
    WalletNotFound(String),

    #[error("Invalid wallet address: {0}")]
    InvalidWallet(String),

    #[error("Database error: {0}")]
    Database(#[from] clickhouse::error::Error),

    #[error("External API error: {0}")]
    ExternalApi(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Invalid parameter: {0}")]
    InvalidParam(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    code: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, log_level) = match &self {
            AppError::WalletNotFound(wallet) => {
                tracing::info!(wallet = %wallet, error_code = "WALLET_NOT_FOUND", "Wallet not found");
                (StatusCode::NOT_FOUND, "WALLET_NOT_FOUND", "info")
            }
            AppError::InvalidWallet(wallet) => {
                tracing::warn!(wallet = %wallet, error_code = "INVALID_WALLET", "Invalid wallet address");
                (StatusCode::BAD_REQUEST, "INVALID_WALLET", "warn")
            }
            AppError::Database(e) => {
                tracing::error!(error = %e, error_code = "DATABASE_ERROR", "Database error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, "DATABASE_ERROR", "error")
            }
            AppError::ExternalApi(msg) => {
                tracing::error!(message = %msg, error_code = "EXTERNAL_API_ERROR", "External API error");
                (StatusCode::BAD_GATEWAY, "EXTERNAL_API_ERROR", "error")
            }
            AppError::Config(msg) => {
                tracing::error!(message = %msg, error_code = "CONFIG_ERROR", "Configuration error");
                (StatusCode::INTERNAL_SERVER_ERROR, "CONFIG_ERROR", "error")
            }
            AppError::InvalidParam(param) => {
                tracing::warn!(param = %param, error_code = "INVALID_PARAM", "Invalid parameter");
                (StatusCode::BAD_REQUEST, "INVALID_PARAM", "warn")
            }
            AppError::Internal(msg) => {
                tracing::error!(message = %msg, error_code = "INTERNAL_ERROR", "Internal error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "error")
            }
        };

        tracing::debug!(
            status_code = %status.as_u16(),
            error_code = %code,
            log_level = %log_level,
            error_message = %self.to_string(),
            "Returning error response"
        );

        let body = Json(ErrorResponse {
            error: self.to_string(),
            code: code.to_string(),
        });

        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
