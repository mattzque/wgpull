use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use shared_lib::headers::{HEADER_LIGHTHOUSE_KEY, HEADER_NODE_CHALLENGE, HEADER_NODE_RESPONSE};

use crate::lighthouse::context::LighthouseContextProvider;

use super::LighthouseResponseError;

/// Middleware to verify the lighthouse key and to inject the challenge response of the requested node key.
///
/// The client sends <HEADER_LIGHTHOUSE_KEY> and <HEADER_NODE_CHALLENGE> in the request headers.
/// The server verifies the lighthouse key, terminating the request early if it is invalid.
/// Using the node challenge it generates a challenge response and injects it in the final response header.
///
/// This allows the lighthouse to authenticate the clients and vice-versa the nodes the lighthouse.
pub async fn lighthouse_keys_layer(
    State(context): State<LighthouseContextProvider>,
    request: Request,
    next: Next,
) -> Response {
    let received_lighthouse_key = request
        .headers()
        .get(HEADER_LIGHTHOUSE_KEY)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    let challenge_response;
    {
        let context = context.context.lock().await;

        if !context.verify_lighthouse_key(received_lighthouse_key) {
            return LighthouseResponseError::InvalidLighthouseKey.into_response();
        }

        challenge_response = match request
            .headers()
            .get(HEADER_NODE_CHALLENGE)
            .and_then(|value| value.to_str().ok())
            .map(|challenge| context.get_node_challenge_response(challenge))
        {
            Some(response) => response,
            None => return LighthouseResponseError::InvalidNodeKey.into_response(),
        };
    }

    let mut response = next.run(request).await;

    // inject the challenge response
    response
        .headers_mut()
        .insert(HEADER_NODE_RESPONSE, challenge_response.parse().unwrap());

    response
}
