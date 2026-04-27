use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde_json::json;
use tracing::error;

pub struct AppError(pub StatusCode, pub String);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = Json(json!({ "detail": self.1 }));
        (self.0, body).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        error!("Database error: {}", err);
        AppError(StatusCode::INTERNAL_SERVER_ERROR, "Database error".into())
    }
}

impl From<fang::FangError> for AppError {
    fn from(err: fang::FangError) -> Self {
        error!("Job Queue error: {}", err);
        AppError(StatusCode::INTERNAL_SERVER_ERROR, "Queue error".into())
    }
}

impl AppError {
    pub fn not_found(msg: &str) -> Self {
        AppError(StatusCode::NOT_FOUND, msg.to_string())
    }
}