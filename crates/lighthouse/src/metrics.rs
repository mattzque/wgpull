use std::collections::HashMap;

use wgpull_shared::request::NodeMetricsPushRequest;

/// A collected peer metrics of a node.
pub struct LighthouseMetricsPeer {
    /// Hostname of the peer.
    pub hostname: String,

    /// Latest handshake of the peer.
    pub latest_handshake: u64,

    /// Received bytes of the peer.
    pub transfer_rx: i64,

    /// Sent bytes of the peer.
    pub transfer_tx: i64,
}

/// The collected metrics of a node.
pub struct LighthouseCollectedMetric {
    /// Hostname of the node of this collected metric.
    pub hostname: String,

    /// The interface name of the node of this collected metric.
    pub interface: String,

    /// The upd port number that wireguard listens on.
    pub listening_port: u16,

    /// Information about connected peers.
    pub peers: Vec<LighthouseMetricsPeer>,
}

/// The collected metrics of all nodes.
#[derive(Default)]
pub struct LighthouseMetrics {
    metrics: HashMap<String, LighthouseCollectedMetric>,
}

impl LighthouseMetrics {
    /// Upserts the metrics of a single node, aggregating the metrics of all nodes by hostname.
    pub fn upsert_metrics(&mut self, request: &NodeMetricsPushRequest) {
        let peers = request
            .peers
            .iter()
            .map(|peer| LighthouseMetricsPeer {
                hostname: peer.hostname.clone(),
                latest_handshake: peer.latest_handshake,
                transfer_rx: peer.transfer_rx,
                transfer_tx: peer.transfer_tx,
            })
            .collect();

        let metric = LighthouseCollectedMetric {
            hostname: request.hostname.clone(),
            interface: request.interface.clone(),
            listening_port: request.listening_port,
            peers,
        };

        self.metrics.insert(request.hostname.clone(), metric);
    }

    /// Export metrics for prometheus.
    pub fn export_prometheus(&self) -> String {
        let mut export = String::new();

        for (hostname, metric) in &self.metrics {
            export.push_str(&format!(
                "lighthouse_node_up{{hostname=\"{}\"}} 1\n",
                hostname
            ));
            for peer in &metric.peers {
                export.push_str(&format!(
                    "lighthouse_peer_latest_handshake{{hostname=\"{}\",peer_hostname=\"{}\"}} {}\n",
                    hostname, peer.hostname, peer.latest_handshake
                ));
                export.push_str(&format!(
                    "lighthouse_peer_transfer_rx{{hostname=\"{}\",peer_hostname=\"{}\"}} {}\n",
                    hostname, peer.hostname, peer.transfer_rx
                ));
                export.push_str(&format!(
                    "lighthouse_peer_transfer_tx{{hostname=\"{}\",peer_hostname=\"{}\"}} {}\n",
                    hostname, peer.hostname, peer.transfer_tx
                ));
            }
        }

        export
    }
}
