use serde::{Deserialize, Serialize};


/// The request sent by a node to the lighthouse.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePullRequest {
    /// The hostname of the local node.
    pub hostname: String,
    /// The endpoint of the node (host/ip).
    pub endpoint: String,
    /// The public key of the node.
    pub public_key: String,
    /// The listening port of the node.
    pub listen_port: u32,
    /// The persistent keepalive interval for the node.
    pub persistent_keepalive: u32,
    /// The allowed IPs of the node.
    pub allowed_ips: Vec<String>,
    /// Whether or not the allowed ips should route through the wireguard interface.
    pub route_allowed_ips: bool,
}

/// Wireguard metrics for a peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetricsPushRequestPeer {
    /// The hostname of the connected peer.
    pub hostname: String,

    /// Endpoint of the peer.
    pub endpoint: String,

    /// Allowed IPs of the peer.
    pub allowed_ips: String,

    /// Latest handshake of the peer.
    pub latest_handshake: u64,

    /// Received bytes of the peer.
    pub transfer_rx: i64,

    /// Sent bytes of the peer.
    pub transfer_tx: i64,

    /// Persistent keepalive interval of the peer.
    pub persistent_keepalive: i64,
}

/// Pushes wireguard metrics to the lighthouse.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetricsPushRequest {
    /// The hostname of the local node.
    pub hostname: String,

    /// The interface name of the node.
    pub interface: String,

    /// The upd port number that wireguard listens on.
    pub listening_port: u16,

    /// Information about connected peers.
    pub peers: Vec<NodeMetricsPushRequestPeer>,
}