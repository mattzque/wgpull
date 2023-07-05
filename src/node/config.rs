use serde::Deserialize;
use super::backend::{BackendType, SystemdConfig, UciConfig};

/// Node configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct NodeConfig {
    /// Hostname or IP address of the lighthouse server.
    pub lighthouse_host: String,
    /// Port of the lighthouse server.
    pub lighthouse_port: u16,
    /// Path prefix of the lighthouse server.
    pub lighthouse_path_prefix: String,
    /// Whether or not to use SSL when connecting to the lighthouse.
    pub lighthouse_ssl: bool,
    /// Key used by the lighthouse to authenticate the nodes.
    pub lighthouse_key: String,
    /// Key used by node to authenticate with the lighthouse server.
    pub node_key: String,
    /// Time inbetween each pull of the lighthouse's node configuration.
    pub pull_interval: u32,
    /// Time inbetween each push of the node's metrics to the lighthouse.
    pub metrics_interval: u32,
    /// State file to store the node's state.
    pub state_file: String,
}

impl NodeConfig {
    pub fn get_lighthouse_scheme(&self) -> &'static str {
        if self.lighthouse_ssl {
            "https"
        } else {
            "http"
        }
    }
}

/// Wireguard configuration of a node.
#[derive(Debug, Clone, Deserialize)]
pub struct WireguardConfig {
    /// Type of backend to use to setup the local wireguard.
    pub backend: BackendType,

    /// IP Address of the wireguard node.
    pub address: String,

    /// Public IP Address or Hostname of the wireguard node.
    /// If set to discover the public IP address will be discovered using the
    /// https://api.ipify.org API.
    pub endpoint: String,

    /// Wireguard port to use. (UDP)
    pub listen_port: u32,

    /// Wireguard PersistentKeepalive configuration.
    pub persistent_keepalive: u32,

    /// List of IP addresses to allow incoming connections from (AllowedIPs).
    pub allowed_ips: Vec<String>,

    /// Whether or not the allowed ips should route through the wireguard interface.
    pub route_allowed_ips: bool,
}


/// Configuration for a node.
#[derive(Clone, Debug, Deserialize)]
pub struct NodeConfigFile {
    /// Node configuration.
    pub node: NodeConfig,
    /// Wireguard configuration of a node.
    pub wireguard: WireguardConfig,
    /// Systemd configuration.
    pub systemd: SystemdConfig,
    /// UCI configuration.
    pub uci: UciConfig,
}
