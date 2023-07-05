use futures_util::future::{self, FutureExt};
use gotham::handler::HandlerFuture;
use gotham::helpers::http::response::{create_empty_response, create_response};
use gotham::hyper::{body, Body, HeaderMap, StatusCode};
use gotham::prelude::*;
use gotham::mime;
use gotham::state::State;
use log::info;
use shared_lib::time::CurrentSystemTime;
use std::pin::Pin;

use crate::lighthouse::context::LighthouseContext;
use crate::lighthouse::handler::helpers::{verify_lighthouse_key, get_challenge_response};
use shared_lib::headers::HEADER_NODE_RESPONSE;
use shared_lib::request::NodePullRequest;

pub fn post_pull_handler(mut state: State) -> Pin<Box<HandlerFuture>> {
    let f = body::to_bytes(Body::take_from(&mut state)).then(|full_body| match full_body {
        Ok(valid_body) => {
            let context = LighthouseContext::borrow_from(&state);
            let headers = HeaderMap::borrow_from(&state);

            info!("received pull request from node");

            // verify the lighthouse key
            if !verify_lighthouse_key(context, headers) {
                let res = create_empty_response(&state, StatusCode::UNAUTHORIZED);
                return future::ok((state, res));
            }

            // prepare the challenge response
            let challenge_response = match get_challenge_response(context, headers) {
                Some(response) => response,
                None => {
                    let res = create_empty_response(&state, StatusCode::BAD_REQUEST);
                    return future::ok((state, res));
                }
            };

            let time = CurrentSystemTime::default();

            let body_content = String::from_utf8(valid_body.to_vec()).unwrap();
            let request: NodePullRequest = serde_json::from_str(&body_content).unwrap();
            let response = context.node_pull(&request, &time);

            let response = response.unwrap();
            let response_json = serde_json::to_string(&response).unwrap();

            // create response with challenge response header:
            let mut res = create_response(
                &state,
                StatusCode::OK,
                mime::APPLICATION_JSON,
                response_json,
            );
            res.headers_mut().insert(
                HEADER_NODE_RESPONSE,
                challenge_response.as_str().parse().unwrap(),
            );

            future::ok((state, res))
        }
        Err(e) => future::err((state, e.into())),
    });

    f.boxed()
}
