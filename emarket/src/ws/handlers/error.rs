use std::borrow::Cow;

use axum::response::IntoResponse;
use reqwest::StatusCode;

use crate::data::ApiError;

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, message): (StatusCode, Cow<'static, str>) = match self {
            ApiError::BadRequest(msg, details) => {
                tracing::warn!("{}: {}", msg, details);
                (StatusCode::BAD_REQUEST, Cow::Owned(msg))
            }
            ApiError::Server(msg) => {
                tracing::error!("{}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Cow::Borrowed("Internal Server Error"),
                )
            }
            ApiError::Other(err) => {
                tracing::error!("{}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Cow::Borrowed("Internal Server Error"),
                )
            }
        };

        (status, message).into_response()
    }
}
