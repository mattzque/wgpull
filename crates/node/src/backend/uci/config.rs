use serde::Deserialize;

/// UciConfig backend configuration of a node.
#[derive(Debug, Clone, Deserialize)]
pub struct UciConfig {
    /// The name of the WireGuard interface (wg0).
    pub interface: String,
}
