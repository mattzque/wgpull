use anyhow::Result;
use serde::Serialize;
use std::time::Duration;
use thiserror::Error;
use ureq::Agent;

use shared_lib::challenge::ChallengeResponse;
use shared_lib::headers::{HEADER_LIGHTHOUSE_KEY, HEADER_NODE_CHALLENGE, HEADER_NODE_RESPONSE};
use shared_lib::request::{NodeMetricsPushRequest, NodePullRequest};
use shared_lib::response::NodePullResponse;

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
}

pub struct NodeAgent {
    lighthouse_url: String,
    lighthouse_key: String,
    node_key: String,
    agent: Agent,
}

impl NodeAgent {
    pub fn from_node_config(config: &NodeConfig) -> Result<Self> {
        let agent = ureq::builder().timeout(Duration::from_secs(10)).build();

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
            agent,
        })
    }

    pub fn post<T: Serialize>(
        &self,
        path: &'static str,
        request: &T,
    ) -> Result<String, AgentError> {
        let body = serde_json::to_string(request)
            .map_err(|err| AgentError::ClientSerializationError(err.to_string()))?;

        let challenge = ChallengeResponse::new(self.node_key.clone());
        let resp = self
            .agent
            .post(format!("{}{}", self.lighthouse_url, path).as_str())
            .set(HEADER_LIGHTHOUSE_KEY, &self.lighthouse_key)
            .set(HEADER_NODE_CHALLENGE, challenge.challenge().as_str())
            .send_string(&body);

        match resp {
            Ok(resp) => {
                if resp.status() == 200 {
                    if let Some(challenge_response) = resp.header(HEADER_NODE_RESPONSE) {
                        if challenge.verify(challenge_response) {
                            Ok(resp.into_string().unwrap())
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

    pub fn pull_wireguard(&self, request: NodePullRequest) -> Result<NodePullResponse> {
        request.validate()?;

        let response = self.post("api/v1/pull", &request)?;

        let response: NodePullResponse = serde_json::from_str(&response)
            .map_err(|err| AgentError::ClientSerializationError(err.to_string()))?;

        response.validate()?;

        Ok(response)
    }

    pub fn push_metrics(&self, request: NodeMetricsPushRequest) -> Result<()> {
        request.validate()?;

        self.post("api/v1/push", &request)?;

        Ok(())
    }
}
