use serde::Deserialize;

/// Lighthouse configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct LighthouseConfig {
    /// Key used by the lighthouse to authenticate the nodes.
    pub lighthouse_key: String,
    /// Key used by nodes to authenticate with the lighthouse server.
    pub node_key: String,
    /// Port to listen on for incoming connections.
    pub port: u16,
    /// Host to bind to for incoming connections.
    pub bindhost: String,
    /// Interval in seconds to rotate wireguard private, public and preshared keys.
    pub key_rotation_interval_seconds: u64,
    /// Time of day (in min/max hours) to rotate wireguard private, public and preshared keys.
    pub key_rotation_tod: (u8, u8),
    /// The time in seconds to wait before a node is considered offline.
    pub node_timeout_seconds: u64,
    /// State file to store the lighthouse's state.
    pub state_file: String,
}

impl LighthouseConfig {
    pub fn get_listen_addr(&self) -> String {
        format!("{}:{}", self.bindhost, self.port)
    }
}


/// Configuration for a lighthouse.
#[derive(Debug, Deserialize)]
pub struct LighthouseConfigFile {
    /// Lighthouse configuration.
    pub lighthouse: LighthouseConfig,
}
