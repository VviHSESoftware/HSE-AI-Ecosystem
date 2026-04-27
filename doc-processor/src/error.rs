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
        AppError(StatusCode::BAD_GATEWAY, err)
    }
}