use anyhow::Result;
use shared_lib::{
    challenge::ChallengeResponse,
    command::SystemCommandExecutor,
    file::FileAccessor,
    request::{NodeMetricsPushRequest, NodePullRequest},
    response::NodePullResponse,
    time::CurrentTime,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use super::{config::LighthouseConfig, metrics::LighthouseMetrics, state::LighthouseState};

pub struct LighthouseContext {
    pub config: LighthouseConfig,
    pub state: LighthouseState,
    pub metrics: LighthouseMetrics,
    pub time: Box<dyn CurrentTime + Send + Sync>,
    pub file_accessor: Box<dyn FileAccessor + Send + Sync>,
}

impl LighthouseContext {
    pub async fn init(
        config: LighthouseConfig,
        time: Box<dyn CurrentTime + Send + Sync>,
        file_accessor: Box<dyn FileAccessor + Send + Sync>,
    ) -> Result<Self> {
        match LighthouseState::from_file(&config.state_file, file_accessor.as_ref()).await? {
            Some(state) => {
                let context = LighthouseContext {
                    config,
                    state,
                    metrics: LighthouseMetrics::default(),
                    time,
                    file_accessor,
                };
                Ok(context)
            }
            None => {
                let state = LighthouseState {
                    nodes: HashMap::new(),
                    preshared_keys: HashMap::new(),
                    last_modified: time.now(),
                };
                let context = LighthouseContext {
                    config,
                    state,
                    metrics: LighthouseMetrics::default(),
                    time,
                    file_accessor,
                };
                Ok(context)
            }
        }
    }

    pub fn verify_lighthouse_key(&self, key: &str) -> bool {
        key == self.config.lighthouse_key
    }

    pub fn get_node_challenge_response(&self, challenge: &str) -> String {
        ChallengeResponse::with_challenge(self.config.node_key.clone(), challenge).response()
    }

    pub async fn node_pull(&mut self, request: &NodePullRequest) -> Result<NodePullResponse> {
        // insert or update the node in the lighthouse state, updating the last_seen time
        self.state
            .upsert_node_lease_from_pull_request(request, self.time.as_ref());

        // retreive the regenerate key flag, this also sets the last rotation
        //   time on the node lease
        let regenerate_keys = self.state.should_regenerate_keys(
            &request.hostname,
            self.config.key_rotation_interval_seconds,
            self.config.key_rotation_tod,
            self.time.as_ref(),
        )?;

        self.state
            .remove_expired_nodes(self.config.node_timeout_seconds, self.time.as_ref());

        self.state
            .save(&self.config.state_file, self.file_accessor.as_ref())
            .await?;

        Ok(NodePullResponse {
            regenerate_keys,
            peers: self
                .state
                .get_peers_response_for_node(&request.hostname, SystemCommandExecutor)
                .await,
        })
    }

    pub fn update_metrics(&mut self, request: &NodeMetricsPushRequest) -> Result<()> {
        // insert or update the node in the lighthouse state, updating the last_seen time
        self.metrics.upsert_metrics(request);

        Ok(())
    }

    pub fn get_metrics_prometheus_export(&self) -> Result<String> {
        Ok(self.metrics.export_prometheus())
    }
}

#[derive(Clone)]
pub struct LighthouseContextProvider {
    pub context: Arc<Mutex<LighthouseContext>>,
}

impl LighthouseContextProvider {
    pub fn new(context: LighthouseContext) -> Self {
        LighthouseContextProvider {
            context: Arc::new(Mutex::new(context)),
        }
    }
}
