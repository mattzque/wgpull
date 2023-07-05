use crate::lighthouse::context::LighthouseContext;
use futures_util::future::{self, FutureExt};
use gotham::handler::HandlerFuture;
use gotham::helpers::http::response::{create_empty_response, create_response};
use gotham::hyper::{body, Body, HeaderMap, Response, StatusCode};
use gotham::prelude::*;
use gotham::state::State;
use gotham::mime;
use log::error;
use shared_lib::headers::HEADER_NODE_RESPONSE;
use shared_lib::request::NodeMetricsPushRequest;
use std::pin::Pin;

use super::helpers::{verify_lighthouse_key, get_challenge_response};

pub fn post_metrics_handler(mut state: State) -> Pin<Box<HandlerFuture>> {
    let f = body::to_bytes(Body::take_from(&mut state)).then(|full_body| match full_body {
        Ok(valid_body) => {
            let context = LighthouseContext::borrow_from(&state);
            let headers = HeaderMap::borrow_from(&state);

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

            let body_content = String::from_utf8(valid_body.to_vec()).unwrap();
            let request: NodeMetricsPushRequest = serde_json::from_str(&body_content).unwrap();

            if let Err(err) = context.update_metrics(&request) {
                error!("Error updating metrics: {}", err);
                let res = create_empty_response(&state, StatusCode::INTERNAL_SERVER_ERROR);
                return future::ok((state, res));
            }

            let mut res = create_empty_response(&state, StatusCode::OK);

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

pub fn get_metrics_handler(state: State) -> (State, Response<Body>) {
    let context = LighthouseContext::borrow_from(&state);
    if let Ok(prometheus_metrics) = context.get_metrics_prometheus_export() {
        // create response with challenge response header:
        let res = create_response(
            &state,
            StatusCode::OK,
            mime::TEXT_PLAIN,
            prometheus_metrics,
        );

        (state, res)
    }
    else {
        let res = create_empty_response(&state, StatusCode::INTERNAL_SERVER_ERROR);
        (state, res)
    }
}