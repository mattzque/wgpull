use serde::{Deserialize, Serialize};

use crate::validation::{
    validate_cidr, validate_hostname, validate_hostname_or_ip, validate_wg_key, ValidationError,
};

/// The request sent by a node to the lighthouse.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePullResponsePeer {
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

/// The response sent by the lighthouse to a node pull request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePullResponse {
    /// Indicates to the node that it should regenerate its public and private keys.
    pub regenerate_keys: bool,

    /// Peer configuration for the node provided by the lighthouse.
    pub peers: Vec<NodePullResponsePeer>,
}

impl NodePullResponse {
    /// Validates the pull response, returns true if valid, false otherwise.
    pub fn validate(&self) -> Result<(), ValidationError> {
        for peer in &self.peers {
            validate_hostname("hostname", &peer.hostname)?;
            validate_wg_key("public_key", &peer.public_key)?;
            validate_wg_key("preshared_key", &peer.preshared_key)?;
            validate_hostname_or_ip("endpoint_host", &peer.endpoint_host)?;
            for allowed_ip in &peer.allowed_ips {
                validate_cidr("allowed_ip[]", allowed_ip)?;
            }
        }

        Ok(())
    }
}
