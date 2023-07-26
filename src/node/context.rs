use std::sync::Arc;

use super::{agent::NodeAgent, config::NodeConfigFile, state::NodeState};
use crate::node::{backend::get_backend_impl, state::NodeError};
use anyhow::Result;
use log::info;
use shared_lib::{
    client::HttpClient,
    command::CommandExecutor,
    file::FileAccessor,
    request::{NodeMetricsPushRequest, NodeMetricsPushRequestPeer, NodePullRequest},
    validation::Validated,
    wg::{WireguardCommand, WireguardInfo},
};

pub struct NodeContext {
    pub config: NodeConfigFile,
    pub state: NodeState,
    pub executor: Arc<dyn CommandExecutor>,
    pub file_accessor: Arc<dyn FileAccessor>,
    pub http_client: Arc<dyn HttpClient>,
}

impl NodeContext {
    pub async fn init(
        config: &NodeConfigFile,
        executor: Arc<dyn CommandExecutor>,
        file_accessor: Arc<dyn FileAccessor>,
        http_client: Arc<dyn HttpClient>,
    ) -> Result<Self> {
        match NodeState::from_file(&config.node.state_file, file_accessor.as_ref()).await? {
            Some(state) => {
                let context = NodeContext {
                    config: config.clone(),
                    state,
                    executor,
                    file_accessor,
                    http_client,
                };
                Ok(context)
            }
            None => {
                info!("Local node has no state, generating new wireguard keys.");
                let state = NodeState::from_wireguard_config(
                    config,
                    executor.clone(),
                    file_accessor.clone(),
                    http_client.clone(),
                )
                .await?;
                let context = NodeContext {
                    config: config.clone(),
                    state,
                    executor,
                    file_accessor,
                    http_client,
                };
                Ok(context)
            }
        }
    }

    pub async fn pull_wireguard(&mut self) -> Result<()> {
        info!("Pulling Wireguard configuration.");
        let agent = NodeAgent::from_node_config(&self.config.node, self.http_client.as_ref())?;

        let request: NodePullRequest = self.state.clone().into();
        let response = agent.pull_wireguard(request).await?;

        info!(
            "Received configuration of {} peers from lighthouse.",
            response.peers.len()
        );

        // update state from response, replacing all peers and regenerate keys if requested
        self.state
            .update_from_pull_response(&response, self.executor.clone())
            .await?;

        // get backend by configuration
        let backend = get_backend_impl(
            self.config.wireguard.backend.clone(),
            &self.config,
            self.executor.clone(),
            self.file_accessor.clone(),
        );
        if !backend.is_compatible().await {
            return Err(NodeError::BackendNotCompatible.into());
        }

        // configure the local system to match the state
        backend.update_local_state(&self.state).await?;

        // save state to disk
        self.state
            .save(&self.config.node.state_file, self.file_accessor.as_ref())
            .await?;

        Ok(())
    }

    fn metrics_push_request_from_info(
        &self,
        info: WireguardInfo,
    ) -> Result<NodeMetricsPushRequest> {
        let peers = info
            .peers
            .into_iter()
            .map(|peer| NodeMetricsPushRequestPeer {
                hostname: self.state.get_hostname_by_public_key(&peer.public_key),
                endpoint: peer.endpoint,
                latest_handshake: peer.latest_handshake,
                transfer_rx: peer.transfer_rx,
                transfer_tx: peer.transfer_tx,
                persistent_keepalive: peer.persistent_keepalive,
            })
            .collect();

        let request = NodeMetricsPushRequest {
            hostname: self.state.hostname.clone(),
            interface: info.interface,
            listening_port: info.listening_port,
            peers,
        };
        request.validate()?;
        Ok(request)
    }

    pub async fn push_metrics(&self) -> Result<()> {
        let agent = NodeAgent::from_node_config(&self.config.node, self.http_client.as_ref())?;
        let wireguard_command = WireguardCommand::new(self.executor.as_ref());
        // collect local wireguard metrics:
        let info = wireguard_command.collect().await?;
        if let Some(metrics) = info {
            let request = self.metrics_push_request_from_info(metrics)?;
            // push metrics to lighthouse:
            agent.push_metrics(request).await?;
        }
        Ok(())
    }
}
