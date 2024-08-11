use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

/// A pair of two peers by their hostname, where the order of the peers doesn't matter.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PeerPair {
    peers: (String, String),
}

impl PeerPair {
    /// Creates a new peer pair, the order of the two hostnames doesn't matter.
    pub fn new(a: String, b: String) -> Self {
        let peers = if a < b { (a, b) } else { (b, a) };
        Self { peers }
    }
}

impl Hash for PeerPair {
    /// Hash function for PeerPair, combining the hash of both hostnames.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.peers.0.hash(state);
        self.peers.1.hash(state);
    }
}
