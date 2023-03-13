use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub(crate) struct HandlerError(anyhow::Error);

impl IntoResponse for HandlerError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Internal processing issue: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for HandlerError
where
    E: Into<anyhow::Error>,
{
    fn from(value: E) -> Self {
        Self(value.into())
    }
}
