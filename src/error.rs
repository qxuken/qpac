use axum::{
    http::{Response, StatusCode},
    response::IntoResponse,
};
use std::error::Error;
use thiserror::Error;

pub type Result<T, E = Report> = color_eyre::Result<T, E>;
pub struct Report(color_eyre::Report);

impl std::fmt::Debug for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<E> From<E> for Report
where
    E: Into<color_eyre::Report>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

impl IntoResponse for Report {
    fn into_response(self) -> Response<axum::body::Body> {
        let err = self.0;
        let err_string = format!("{err:?}");

        tracing::error!("{err_string}");

        if let Some(err) = err.downcast_ref::<AppError>() {
            return err.into_response();
        }

        // Fallback
        (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
    }
}

#[derive(Error, Debug, Clone, PartialEq)]
pub enum AppError {
    #[error("PreconditionFailed: {0}")]
    PreconditionFailed(String),

    #[error("NotFound")]
    NotFound,

    #[error("Internal error: {0}")]
    Other(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        match self {
            AppError::PreconditionFailed(_) => {
                (StatusCode::BAD_REQUEST, self.to_string()).into_response()
            }
            AppError::NotFound => (StatusCode::NOT_FOUND, self.to_string()).into_response(),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response(),
        }
    }
}

impl IntoResponse for &AppError {
    fn into_response(self) -> axum::response::Response {
        self.clone().into_response()
    }
}

impl From<Box<dyn Error + Send + Sync>> for AppError {
    fn from(value: Box<dyn Error + Send + Sync>) -> Self {
        Self::Other(value.to_string())
    }
}
