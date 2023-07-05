use anyhow::Result;
use gotham::state::StateData;
use shared_lib::{
    challenge::ChallengeResponse,
    command::SystemCommandExecutor,
    request::{NodeMetricsPushRequest, NodePullRequest},
    response::NodePullResponse,
    time::CurrentTime,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use super::{
    config::LighthouseConfig,
    metrics::LighthouseMetrics,
    state::{LighthouseError, LighthouseState},
};

#[derive(Clone, StateData)]
pub struct LighthouseContext {
    pub config: LighthouseConfig,
    pub state: Arc<Mutex<LighthouseState>>,
    pub metrics: Arc<Mutex<LighthouseMetrics>>,
}

impl LighthouseContext {
    pub fn init<T>(config: LighthouseConfig, time: &T) -> Result<Self>
    where
        T: CurrentTime,
    {
        match LighthouseState::from_file(&config.state_file)? {
            Some(state) => {
                let context = LighthouseContext {
                    config,
                    state: Arc::new(Mutex::new(state)),
                    metrics: Arc::new(Mutex::new(LighthouseMetrics::default())),
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
                    state: Arc::new(Mutex::new(state)),
                    metrics: Arc::new(Mutex::new(LighthouseMetrics::default())),
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

    pub fn node_pull<T>(&self, request: &NodePullRequest, time: &T) -> Result<NodePullResponse>
    where
        T: CurrentTime,
     {
        let mut state = self
            .state
            .lock()
            .map_err(|_| LighthouseError::StateLockPoisoned)?;

        // insert or update the node in the lighthouse state, updating the last_seen time
        state.upsert_node_lease_from_pull_request(request, time);

        // retreive the regenerate key flag, this also sets the last rotation
        //   time on the node lease
        let regenerate_keys = state.should_regenerate_keys(
            &request.hostname,
            self.config.key_rotation_interval_seconds,
            self.config.key_rotation_tod,
            time,
        )?;

        state.remove_expired_nodes(self.config.node_timeout_seconds, time);

        state.save(&self.config.state_file)?;

        Ok(NodePullResponse {
            regenerate_keys,
            peers: state
                .get_peers_response_for_node(&request.hostname, SystemCommandExecutor::default()),
        })
    }

    pub fn update_metrics(&self, request: &NodeMetricsPushRequest) -> Result<()> {
        let mut metrics = self
            .metrics
            .lock()
            .map_err(|_| LighthouseError::StateLockPoisoned)?;

        // insert or update the node in the lighthouse state, updating the last_seen time
        metrics.upsert_metrics(request);

        Ok(())
    }
    
    pub fn get_metrics_prometheus_export(&self) -> Result<String> {
        let metrics = self
            .metrics
            .lock()
            .map_err(|_| LighthouseError::StateLockPoisoned)?;

        Ok(metrics.export_prometheus())
    }
}
