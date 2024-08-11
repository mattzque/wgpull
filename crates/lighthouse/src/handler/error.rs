use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LighthouseResponseError {
    #[error("Invalid lighthouse key in request!")]
    InvalidLighthouseKey,
    #[error("Invalid node key in request!")]
    InvalidNodeKey,
    #[error("Request body is invalid!")]
    BadRequestBody,
    #[error("Response body is invalid!")]
    BadResponseBody,
    #[error("Internal error in lighthouse context!")]
    InternalError,
}

// Tell axum how to convert `LighthouseResponseError` into a response.
impl IntoResponse for LighthouseResponseError {
    fn into_response(self) -> Response {
        (
            match self {
                LighthouseResponseError::InvalidLighthouseKey => StatusCode::UNAUTHORIZED,
                LighthouseResponseError::InvalidNodeKey => StatusCode::UNAUTHORIZED,
                LighthouseResponseError::BadRequestBody => StatusCode::BAD_REQUEST,
                LighthouseResponseError::BadResponseBody => StatusCode::INTERNAL_SERVER_ERROR,
                LighthouseResponseError::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
            },
            format!("{}", self),
        )
            .into_response()
    }
}
