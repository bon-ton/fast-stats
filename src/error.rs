use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),

    #[error("Too many values in batch (max is 10,000)")]
    TooManyValues,

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = match self {
            Error::InvalidRequest(_) | Error::TooManyValues => StatusCode::BAD_REQUEST,
            Error::SymbolNotFound(_) => StatusCode::NOT_FOUND,
            Error::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = Json(json!({
            "error": self.to_string()
        }));

        (status, body).into_response()
    }
}
