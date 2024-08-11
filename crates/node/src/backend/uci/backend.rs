use std::sync::Arc;

use super::{
    super::interface::Backend,
    command::{UciCommand, UciWireguardConfig, UciWireguardPeer},
    UciConfig,
};
use crate::state::NodeState;
use anyhow::Result;
use async_trait::async_trait;
use log::info;
use wgpull_shared::command::CommandExecutor;

pub struct UciBackend {
    pub config: UciConfig,
    pub executor: Arc<dyn CommandExecutor>,
}

impl UciBackend {
    pub fn new(config: &UciConfig, executor: Arc<dyn CommandExecutor>) -> Self {
        Self {
            config: config.clone(),
            executor,
        }
    }
}

#[async_trait]
impl Backend for UciBackend {
    async fn is_compatible(&self) -> bool {
        let command = UciCommand::new(self.executor.as_ref());
        command.test_uci().await
    }

    async fn update_local_state(&self, state: &NodeState) -> Result<bool> {
        let uci_peers = state
            .peers
            .iter()
            .map(|peer| UciWireguardPeer {
                description: peer.hostname.clone(),
                public_key: peer.public_key.clone(),
                preshared_key: peer.preshared_key.clone(),
                endpoint_host: peer.endpoint_host.clone(),
                endpoint_port: peer.endpoint_port,
                persistent_keepalive: peer.persistent_keepalive,
                route_allowed_ips: peer.route_allowed_ips,
                allowed_ips: peer.allowed_ips.clone(),
            })
            .collect();

        let uci_config = UciWireguardConfig {
            private_key: state.private_key.clone(),
            listen_port: state.listen_port,
            addresses: state.address.clone(),
            peers: uci_peers,
        };

        let command = UciCommand::new(self.executor.as_ref());

        let changed = if let Ok(current_uci_config) =
            command.get_wireguard_config(&self.config.interface).await
        {
            info!(
                "current_uci_config.peers: {}",
                current_uci_config.peers.len()
            );
            info!("uci_config.peers: {}", uci_config.peers.len());
            current_uci_config != uci_config
        } else {
            true
        };

        if changed {
            info!(
                "[uci] update local wireguard configuration, using {} peers",
                uci_config.peers.len()
            );
            command
                .update_wireguard_config(&self.config.interface, &uci_config)
                .await?;
            command.commit(&self.config.interface).await?;
            Ok(true)
        } else {
            info!(
                "[uci] no changes to local wireguard configuration with {} peers",
                uci_config.peers.len()
            );
            Ok(false)
        }
    }

    async fn get_hostname(&self) -> Result<String> {
        let command = UciCommand::new(self.executor.as_ref());
        let hostname = command.get_hostname().await?;
        Ok(hostname)
    }
}
