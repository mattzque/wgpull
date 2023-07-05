use serde::Deserialize;


/// Systemd backend configuration of a node.
#[derive(Debug, Clone, Deserialize)]
pub struct SystemdConfig {
    /// The name of the WireGuard interface (wg0).
    pub interface: String,
    /// The path to systemd-networkd interface files (/etc/systemd/network).
    pub path: String,
    /// Reload networkctl after changes to interface files.
    ///   does run: networkctl reload
    pub reload_networkd: bool,
    /// Deletes the wireguard interface before reloading the networkd.
    ///   this is necessary due to a systemd bug:
    /// https://github.com/systemd/systemd/issues/25547
    /// its a 5 year old bug so I don't expect this to ever be fixed.
    /// TODO implement workaround (manually configure interface)
    ///      for this a raw backend might be a good idea, which uses
    ///      the wg command line tool directly to configure the interface.
    pub delete_interface_before_reload: bool,
}
