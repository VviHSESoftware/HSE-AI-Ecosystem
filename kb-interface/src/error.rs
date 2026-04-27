use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde_json::json;

pub struct AppError(pub StatusCode, pub String);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = Json(json!({ "detail": self.1 }));
        (self.0, body).into_response()
    }
}

impl From<String> for AppError {
    fn from(err: String) -> Self {
        AppError(StatusCode::INTERNAL_SERVER_ERROR, err)
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError(StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", err))
    }
}

impl From<bincode::Error> for AppError {
    fn from(err: bincode::Error) -> Self {
        AppError(StatusCode::INTERNAL_SERVER_ERROR, format!("Serialization error: {}", err))
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        AppError(StatusCode::BAD_GATEWAY, format!("Network error: {}", err))
    }
}