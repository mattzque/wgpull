use super::LighthouseResponseError;
use crate::lighthouse::context::LighthouseContextProvider;
use axum::extract::State;
use axum::Json;
use axum_macros::debug_handler;
use log::error;
use shared_lib::request::NodePullRequest;
use shared_lib::response::NodePullResponse;
use shared_lib::validation::Validated;

#[debug_handler]
pub async fn post_pull_handler(
    State(context): State<LighthouseContextProvider>,
    Json(request): Json<NodePullRequest>,
) -> Result<Json<NodePullResponse>, LighthouseResponseError> {
    if request.validate().is_err() {
        return Err(LighthouseResponseError::BadRequestBody);
    }

    let mut context = context.context.lock().await;

    let response = context.node_pull(&request).await;
    if let Err(err) = response {
        error!("Error creating pull response: {}", err);
        return Err(LighthouseResponseError::InternalError);
    }
    let response = response.unwrap();
    if let Err(err) = response.validate() {
        error!("Error validating node pull response: {}", err);
        return Err(LighthouseResponseError::BadResponseBody);
    }

    Ok(Json(response))
}
