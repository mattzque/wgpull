use super::{
    super::interface::Backend,
    command::{UciCommand, UciWireguardConfig, UciWireguardPeer},
    UciConfig,
};
use crate::node::state::NodeState;
use anyhow::Result;
use log::info;
use shared_lib::command::CommandExecutor;

pub struct UciBackend<T: CommandExecutor> {
    pub config: UciConfig,
    pub command: UciCommand<T>,
}

impl<T: CommandExecutor> UciBackend<T> {
    pub fn new(config: &UciConfig, command: UciCommand<T>) -> Self {
        Self {
            config: config.clone(),
            command,
        }
    }
}

impl<T: CommandExecutor> Backend for UciBackend<T> {
    fn is_compatible(&self) -> bool {
        self.command.test_uci()
    }

    fn update_local_state(&self, state: &NodeState) -> Result<bool> {
        let uci_peers = state
            .peers
            .iter()
            .map(|peer| UciWireguardPeer {
                description: peer.hostname.clone(),
                public_key: peer.public_key.clone(),
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

        let changed = if let Ok(current_uci_config) = self.command.get_wireguard_config(&self.config.interface) {
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
            self.command.update_wireguard_config(&self.config.interface, &uci_config)?;
            self.command.commit(&self.config.interface)?;
            Ok(true)
        } else {
            info!(
                "[uci] no changes to local wireguard configuration with {} peers",
                uci_config.peers.len()
            );
            Ok(false)
        }
    }

    fn get_hostname(&self) -> Result<String> {
        let hostname = self.command.get_hostname()?;
        Ok(hostname)
    }
}
