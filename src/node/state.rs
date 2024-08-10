use std::sync::Arc;

use super::{backend::get_backend_impl, config::NodeConfigFile, discover::discover_public_ip};
use anyhow::Result;
use log::info;
use serde::{Deserialize, Serialize};
use shared_lib::{
    client::HttpClient, command::CommandExecutor, file::FileAccessor, request::NodePullRequest,
    response::NodePullResponse, wg::WireguardCommand,
};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum NodeError {
    #[error("The configured backend is not compatible with this system")]
    BackendNotCompatible,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePeer {
    /// The hostname of the peer.
    pub hostname: String,

    /// The public key of the peer.
    pub public_key: String,

    /// The preshared key of the peer.
    pub preshared_key: String,

    /// The endpoint host/ip of the peer.
    pub endpoint_host: String,

    /// The endpoint port of the peer.
    pub endpoint_port: u32,

    /// The allowed IPs of the peer.
    pub allowed_ips: Vec<String>,

    /// The persistent keepalive interval for the peer.
    pub persistent_keepalive: u32,

    /// Whether or not the allowed ips should route through the wireguard interface.
    /// Indicates if routes should be added for each allowed_ip entry.
    pub route_allowed_ips: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeState {
    /// The local hostname of the node.
    pub hostname: String,

    /// The private key of the node.
    pub private_key: String,

    /// The public key of the node.
    pub public_key: String,

    /// The public IP address of the node.
    pub address: String,

    /// The endpoint host of the node (just ip/hostname).
    pub endpoint: String,

    /// The port that the node listens on.
    pub listen_port: u32,

    /// The persistent keepalive interval for the node.
    pub persistent_keepalive: u32,

    /// The allowed IPs of the node.
    pub allowed_ips: Vec<String>,

    /// List of peers that the node is connected to.
    pub peers: Vec<NodePeer>,

    /// Whether or not the allowed ips should route through the wireguard interface.
    /// Indicates if routes should be added for each allowed_ip entry.
    pub route_allowed_ips: bool,
}

impl From<NodeState> for NodePullRequest {
    fn from(state: NodeState) -> Self {
        NodePullRequest {
            hostname: state.hostname,
            endpoint: state.endpoint,
            public_key: state.public_key,
            listen_port: state.listen_port,
            persistent_keepalive: state.persistent_keepalive,
            allowed_ips: state.allowed_ips,
            route_allowed_ips: state.route_allowed_ips,
        }
    }
}

impl NodeState {
    pub fn get_hostname_by_public_key(&self, public_key: &str) -> String {
        self.peers
            .iter()
            .find(|p| p.public_key == public_key)
            .map(|p| p.hostname.clone())
            .unwrap_or("unknown".to_string())
    }

    pub async fn from_wireguard_config(
        config: &NodeConfigFile,
        executor: Arc<dyn CommandExecutor>,
        file_accessor: Arc<dyn FileAccessor>,
        http_client: Arc<dyn HttpClient>,
    ) -> Result<Self> {
        let wireguard_command = WireguardCommand::new(executor.as_ref());

        let keypair = wireguard_command.generate_keypair().await?;
        let endpoint = if config.wireguard.endpoint == "discover" {
            let public_ip = discover_public_ip(http_client.as_ref()).await?;
            info!("Using public ip discovery for node endpoint: {}", public_ip);
            public_ip
        } else {
            config.wireguard.endpoint.clone()
        };

        let hostname = {
            // get backend by configuration
            let backend = get_backend_impl(
                config.wireguard.backend.clone(),
                config,
                executor,
                file_accessor,
            );
            if !backend.is_compatible().await {
                return Err(NodeError::BackendNotCompatible.into());
            }
            backend.get_hostname().await?
        };

        Ok(NodeState {
            endpoint,
            address: config.wireguard.address.clone(),
            hostname,
            private_key: keypair.private_key,
            public_key: keypair.public_key,
            listen_port: config.wireguard.listen_port,
            persistent_keepalive: config.wireguard.persistent_keepalive,
            allowed_ips: config.wireguard.allowed_ips.clone(),
            route_allowed_ips: config.wireguard.route_allowed_ips,
            peers: Vec::new(),
        })
    }

    pub async fn update_from_pull_response(
        &mut self,
        response: &NodePullResponse,
        executor: Arc<dyn CommandExecutor>,
    ) -> Result<()> {
        let wireguard_command = WireguardCommand::new(executor.as_ref());

        if response.regenerate_keys {
            info!("Regenerating keys as requested by lighthouse.");
            // generate new keys
            let keypair = wireguard_command.generate_keypair().await?;
            self.private_key = keypair.private_key;
            self.public_key = keypair.public_key;
            // TODO the generated keys will not be known by the lighthouse, so we need to do
            //    another pull request after this one to update the peers. This is still not
            //    optimal, because the peers will be disconnected for a short time (a few seconds).
        }

        // update peer list from response
        self.peers = response
            .peers
            .iter()
            .map(|peer| NodePeer {
                hostname: peer.hostname.clone(),
                public_key: peer.public_key.clone(),
                preshared_key: peer.preshared_key.clone(),
                endpoint_host: peer.endpoint_host.clone(),
                endpoint_port: peer.endpoint_port,
                allowed_ips: peer.allowed_ips.clone(),
                persistent_keepalive: peer.persistent_keepalive,
                route_allowed_ips: peer.route_allowed_ips,
            })
            .collect();

        Ok(())
    }

    pub async fn from_file(path: &str, accessor: &dyn FileAccessor) -> Result<Option<NodeState>> {
        info!("Restoring node state from {}", path);
        let contents = match accessor.read(path).await {
            Ok(file) => file,
            Err(_) => return Ok(None),
        };

        let state = match toml::from_str(&contents) {
            Ok(state) => state,
            Err(_) => return Err(anyhow::anyhow!("Error parsing state file.")),
        };

        Ok(Some(state))
    }

    pub async fn save(&self, path: &str, accessor: &dyn FileAccessor) -> Result<()> {
        info!("Saving node state to {}", path);
        // Convert the state to a TOML string.
        let contents = toml::to_string(self)?;
        // Write the TOML string to the state file.
        accessor.write(path, &contents).await?;

        Ok(())
    }
}
