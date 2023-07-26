use super::LighthouseResponseError;
use crate::lighthouse::context::LighthouseContextProvider;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use log::error;
use shared_lib::request::NodeMetricsPushRequest;
use shared_lib::validation::Validated;

pub async fn post_metrics_handler(
    State(context): State<LighthouseContextProvider>,
    Json(request): Json<NodeMetricsPushRequest>,
) -> Result<Response, LighthouseResponseError> {
    if request.validate().is_err() {
        return Err(LighthouseResponseError::BadRequestBody);
    }

    let mut context = context.context.lock().await;

    let response = context.update_metrics(&request);
    if let Err(err) = response {
        error!("Error creating pull response: {}", err);
        return Err(LighthouseResponseError::InternalError);
    }

    Ok((StatusCode::OK, "").into_response())
}

pub async fn get_metrics_handler(
    State(context): State<LighthouseContextProvider>,
) -> Result<Response, LighthouseResponseError> {
    let context = context.context.lock().await;

    let prometheus_metrics = context.get_metrics_prometheus_export();
    Ok((StatusCode::OK, prometheus_metrics).into_response())
}
