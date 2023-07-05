use super::{agent::NodeAgent, config::NodeConfigFile, state::NodeState};
use crate::node::{backend::get_backend_impl, state::NodeError};
use anyhow::Result;
use gotham::state::StateData;
use log::info;
use shared_lib::{
    command::SystemCommandExecutor,
    request::{NodeMetricsPushRequest, NodeMetricsPushRequestPeer, NodePullRequest},
    wg::{WireguardCommand, WireguardInfo},
};
use std::sync::{Arc, Mutex};

#[derive(Clone, StateData)]
pub struct NodeContext {
    pub config: NodeConfigFile,
    pub state: Arc<Mutex<NodeState>>,
    pub agent: Arc<Mutex<NodeAgent>>,
}

impl NodeContext {
    pub fn init(config: &NodeConfigFile) -> Result<Self> {
        match NodeState::from_file(&config.node.state_file)? {
            Some(state) => {
                let context = NodeContext {
                    config: config.clone(),
                    state: Arc::new(Mutex::new(state)),
                    agent: Arc::new(Mutex::new(NodeAgent::from_node_config(&config.node)?)),
                };
                Ok(context)
            }
            None => {
                info!("Local node has no state, generating new wireguard keys.");
                let state = NodeState::from_wireguard_config(config)?;
                let context = NodeContext {
                    config: config.clone(),
                    state: Arc::new(Mutex::new(state)),
                    agent: Arc::new(Mutex::new(NodeAgent::from_node_config(&config.node)?)),
                };
                Ok(context)
            }
        }
    }

    pub fn pull_wireguard(&self) -> Result<()> {
        info!("Pulling Wireguard configuration.");
        // Lock the state to ensure exclusive access.
        let mut state = self
            .state
            .lock()
            .map_err(|_| NodeError::StateLockPoisoned)?;
        let agent = self
            .agent
            .lock()
            .map_err(|_| NodeError::StateLockPoisoned)?;

        let request: NodePullRequest = state.clone().into();
        let response = agent.pull_wireguard(request)?;

        info!(
            "Received configuration of {} peers from lighthouse.",
            response.peers.len()
        );

        // update state from response, replacing all peers and regenerate keys if requested
        state.update_from_pull_response(&response)?;

        // get backend by configuration
        let backend = get_backend_impl(self.config.wireguard.backend.clone(), &self.config);
        if !backend.is_compatible() {
            return Err(NodeError::BackendNotCompatible.into());
        }

        // configure the local system to match the state
        backend.update_local_state(&state)?;

        // save state to disk
        state.save(&self.config.node.state_file)?;

        Ok(())
    }

    fn metrics_push_request_from_info(
        &self,
        info: WireguardInfo,
    ) -> Result<NodeMetricsPushRequest> {
        let state = self
            .state
            .lock()
            .map_err(|_| NodeError::StateLockPoisoned)?;
        let peers = info
            .peers
            .into_iter()
            .map(|peer| NodeMetricsPushRequestPeer {
                hostname: state.get_hostname_by_public_key(&peer.public_key),
                endpoint: peer.endpoint,
                latest_handshake: peer.latest_handshake,
                transfer_rx: peer.transfer_rx,
                transfer_tx: peer.transfer_tx,
                persistent_keepalive: peer.persistent_keepalive,
            })
            .collect();

        let request = NodeMetricsPushRequest {
            hostname: state.hostname.clone(),
            interface: info.interface,
            listening_port: info.listening_port,
            peers,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn push_metrics(&self) -> Result<()> {
        let wireguard_command = WireguardCommand::new(SystemCommandExecutor::default());
        // get agent
        let agent = self
            .agent
            .lock()
            .map_err(|_| NodeError::StateLockPoisoned)?;
        // collect local wireguard metrics:
        let info = wireguard_command.collect()?;
        if let Some(metrics) = info {
            let request = self.metrics_push_request_from_info(metrics)?;
            // push metrics to lighthouse:
            agent.push_metrics(request)?;
        }
        Ok(())
    }
}
