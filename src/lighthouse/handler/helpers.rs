use gotham::hyper::HeaderMap;
use crate::lighthouse::context::LighthouseContext;
use shared_lib::headers::{HEADER_LIGHTHOUSE_KEY, HEADER_NODE_CHALLENGE};

pub fn verify_lighthouse_key(context: &LighthouseContext, headers: &HeaderMap) -> bool {
    let received_lighthouse_key = headers
        .get(HEADER_LIGHTHOUSE_KEY)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    context.verify_lighthouse_key(received_lighthouse_key)
}

pub fn get_challenge_response(context: &LighthouseContext, headers: &HeaderMap) -> Option<String> {
    headers
        .get(HEADER_NODE_CHALLENGE)
        .and_then(|value| value.to_str().ok())
        .map(|challenge| context.get_node_challenge_response(challenge))
}