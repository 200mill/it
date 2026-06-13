use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

/// Application-wide error type. Wraps database, request, and upstream (AI) failures and
/// renders them as the appropriate HTTP status instead of panicking.
pub enum AppError {
    Db(sqlx::Error),
    NotFound(String),
    BadRequest(String),
    Upstream(String),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Db(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Db(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg).into_response(),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
            AppError::Upstream(msg) => (StatusCode::BAD_GATEWAY, msg).into_response(),
        }
    }
}
