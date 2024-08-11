use anyhow::Result;
use log::error;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Serialize;
use thiserror::Error;
use wgpull_shared::client::HttpClient;
use wgpull_shared::validation::Validated;

use wgpull_shared::challenge::ChallengeResponse;
use wgpull_shared::headers::{HEADER_LIGHTHOUSE_KEY, HEADER_NODE_CHALLENGE, HEADER_NODE_RESPONSE};
use wgpull_shared::request::{NodeMetricsPushRequest, NodePullRequest};
use wgpull_shared::response::NodePullResponse;

use super::config::NodeConfig;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Client Error: {0}")]
    ClientError(String),
    #[error("Error serializing/unserializing a request or response")]
    ClientSerializationError(String),
    #[error("Challenge Response Error")]
    ChallengeResponseIncorrect,
    #[error("Challenge Response Missing in response")]
    NoChallengeResponse,
    #[error("Error validating request to send")]
    RequestValidationError,
}

pub struct NodeAgent<'a, T: HttpClient + ?Sized> {
    lighthouse_url: String,
    lighthouse_key: String,
    node_key: String,
    client: &'a T,
}

impl<'a, T: HttpClient + ?Sized> NodeAgent<'a, T> {
    pub fn from_node_config(config: &NodeConfig, client: &'a T) -> Result<Self> {
        Ok(Self {
            lighthouse_url: format!(
                "{}://{}:{}/{}",
                config.get_lighthouse_scheme(),
                config.lighthouse_host,
                config.lighthouse_port,
                config.lighthouse_path_prefix
            ),
            lighthouse_key: config.lighthouse_key.clone(),
            node_key: config.node_key.clone(),
            client,
        })
    }

    pub async fn post<Body: Serialize + Validated>(
        &self,
        path: &'static str,
        request: &Body,
    ) -> Result<String, AgentError> {
        request.validate().map_err(|err| {
            error!("Error validating the request to send: {}", err.to_string());
            AgentError::RequestValidationError
        })?;
        let body = serde_json::to_string(request)
            .map_err(|err| AgentError::ClientSerializationError(err.to_string()))?;

        let challenge = ChallengeResponse::new(self.node_key.clone());

        let mut headers = HeaderMap::new();
        headers.insert(
            HEADER_LIGHTHOUSE_KEY,
            HeaderValue::from_str(&self.lighthouse_key).unwrap(),
        );
        headers.insert(
            HEADER_NODE_CHALLENGE,
            HeaderValue::from_str(challenge.challenge().as_str()).unwrap(),
        );
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));

        let url = format!("{}{}", self.lighthouse_url, path);
        let resp = self.client.post(&url, headers, body).await;

        match resp {
            Ok(resp) => {
                if resp.status().is_success() {
                    if let Some(challenge_response) = resp.headers().get(HEADER_NODE_RESPONSE) {
                        if challenge.verify(challenge_response.to_str().unwrap()) {
                            Ok(resp.text().await.unwrap())
                        } else {
                            Err(AgentError::ChallengeResponseIncorrect)
                        }
                    } else {
                        Err(AgentError::NoChallengeResponse)
                    }
                } else {
                    Err(AgentError::ClientError(format!(
                        "Response Status: {}",
                        resp.status()
                    )))
                }
            }
            Err(e) => Err(AgentError::ClientError(e.to_string())),
        }
    }

    pub async fn pull_wireguard(&self, request: NodePullRequest) -> Result<NodePullResponse> {
        let response = self.post("api/v1/pull", &request).await?;

        let response: NodePullResponse = serde_json::from_str(&response)
            .map_err(|err| AgentError::ClientSerializationError(err.to_string()))?;

        response.validate()?;

        Ok(response)
    }

    pub async fn push_metrics(&self, request: NodeMetricsPushRequest) -> Result<()> {
        self.post("api/v1/metrics", &request).await?;

        Ok(())
    }
}
