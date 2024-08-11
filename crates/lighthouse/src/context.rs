use anyhow::Result;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use wgpull_shared::{
    challenge::ChallengeResponse,
    command::CommandExecutor,
    file::FileAccessor,
    request::{NodeMetricsPushRequest, NodePullRequest},
    response::NodePullResponse,
    time::CurrentTime,
};

use super::{config::LighthouseConfig, metrics::LighthouseMetrics, state::LighthouseState};

/// The global context of the lighthouse server.
///
/// This keeps track of connected nodes and peers in the lighthouse state and aggregates
/// metrics from all the connected nodes.
///
/// The time/file/executor are traits injected to access the system time, filesystem and
/// execute system commands. Traits are used to test the context with mocked implementations.
pub struct LighthouseContext {
    pub config: LighthouseConfig,
    pub state: LighthouseState,
    pub metrics: LighthouseMetrics,
    pub time: Arc<dyn CurrentTime + Send + Sync>,
    pub file_accessor: Arc<dyn FileAccessor + Send + Sync>,
    pub executor: Arc<dyn CommandExecutor + Send + Sync>,
}

impl LighthouseContext {
    /// Initialize the lighthouse context from a previous state or initialize a new context.
    pub async fn init(
        config: LighthouseConfig,
        time: Arc<dyn CurrentTime + Send + Sync>,
        file_accessor: Arc<dyn FileAccessor + Send + Sync>,
        executor: Arc<dyn CommandExecutor + Send + Sync>,
    ) -> Result<Self> {
        match LighthouseState::from_file(&config.state_file, file_accessor.as_ref()).await? {
            Some(state) => {
                let context = LighthouseContext {
                    config,
                    state,
                    metrics: LighthouseMetrics::default(),
                    time,
                    file_accessor,
                    executor,
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
                    executor,
                };
                Ok(context)
            }
        }
    }

    /// Verify the shared lighthouse key against the configuration.
    pub fn verify_lighthouse_key(&self, key: &str) -> bool {
        key == self.config.lighthouse_key
    }

    /// Creates a challenge response to send to the node, this is used for the
    /// node to verify the authenticity of the lighthouse.
    pub fn get_node_challenge_response(&self, challenge: &str) -> String {
        ChallengeResponse::with_challenge(self.config.node_key.clone(), challenge).response()
    }

    /// Both pushes the node configuration to the lighthouse and poll the configuration of connected nodes.
    ///
    /// This is the primary function of the lighthouse, it keeps track of all node configurations previously
    /// pushed, this includes their public keys, endpoints, routes, etc. It will then return configuration
    /// for all connected peers of the node, generating preshared keys on the fly for new edges of the mesh
    /// network.
    ///
    /// It also keeps track of the last time a node has pulled, and remove timed out nodes.
    /// This will set a flag for the node to regenerate keys if the key rotation interval has passed.
    ///
    /// Everytime the function is called the lighthouse state will be saved to disk.
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
                .get_peers_response_for_node(&request.hostname, self.executor.clone())
                .await,
        })
    }

    /// The node pushes the latest metrics to the lighthouse, the metrics will be aggregated
    /// in the lighthouse context.
    pub fn update_metrics(&mut self, request: &NodeMetricsPushRequest) -> Result<()> {
        // insert or update the node in the lighthouse state, updating the last_seen time
        self.metrics.upsert_metrics(request);

        Ok(())
    }

    /// Returns aggregated metrics as a prometheus export string.
    pub fn get_metrics_prometheus_export(&self) -> String {
        self.metrics.export_prometheus()
    }
}

/// Wraps the lighthouse context in an Arc<Mutex<>> to allow for concurrent access.
/// Uses the tokio Mutex to allow for async locking.
#[derive(Clone)]
pub struct LighthouseContextProvider {
    pub context: Arc<Mutex<LighthouseContext>>,
}

impl LighthouseContextProvider {
    /// Wraps the context in an Arc<Mutex<>>.
    pub fn new(context: LighthouseContext) -> Self {
        LighthouseContextProvider {
            context: Arc::new(Mutex::new(context)),
        }
    }
}
