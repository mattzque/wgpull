use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::str;
use std::sync::Arc;

use crate::node::state::NodeState;
use anyhow::Result;
use async_trait::async_trait;
use log::{error, info};
use shared_lib::command::CommandExecutor;
use shared_lib::file::FileAccessor;

use super::{super::interface::Backend, command::SystemdCommand, SystemdConfig};

const PREAMBLE: &str = "# This file is generated by wgpull, changes will be lost.\n\n";

pub struct SystemdBackend {
    pub config: SystemdConfig,
    pub executor: Arc<dyn CommandExecutor>,
    pub file_accessor: Arc<dyn FileAccessor>,
}

impl SystemdBackend {
    pub fn new(
        config: &SystemdConfig,
        executor: Arc<dyn CommandExecutor>,
        file_accessor: Arc<dyn FileAccessor>,
    ) -> Self {
        Self {
            config: config.clone(),
            executor,
            file_accessor,
        }
    }

    /// Writes the provided `contents` to a file at the specified `path` if the current contents are different.
    ///
    /// If the file does not exist or its contents differ from the provided `contents`, the file will be created or overwritten
    /// with the new contents. The file's permissions will be set to 0640 (`-rw-r-----`), the owner will be set to root, and
    /// the group will be set to "systemd-networkd".
    ///
    /// # Arguments
    ///
    /// * `path` - A reference to a Path for the file to write.
    /// * `contents` - The new contents to write to the file.
    ///
    /// # Returns
    ///
    /// This function returns an `anyhow::Result<bool>`. If the function succeeds, the boolean indicates whether the file
    /// was modified: `true` means the file was modified (or created), and `false` means the file was not modified.
    pub async fn write_if_changed(&self, path: &str, contents: &str) -> Result<bool> {
        let contents_changed = match self.file_accessor.read(path).await {
            Ok(current) if current == contents => false,
            _ => {
                self.file_accessor.write(path, contents).await?;
                self.file_accessor
                    .set_permissions(path, Permissions::from_mode(0o640))
                    .await?;
                self.executor
                    .execute_with_args("chown", &["root:systemd-network", path])
                    .await?;
                true
            }
        };

        Ok(contents_changed)
    }

    pub fn get_interface_netdev_contents(&self, state: &NodeState) -> String {
        let mut content = String::new();

        content.push_str(PREAMBLE);

        content.push_str("[NetDev]\n");
        content.push_str(format!("Name = {}\n", self.config.interface).as_str());
        content.push_str("Kind = wireguard\n");
        content.push_str("Description = Wireguard Interface\n\n");

        content.push_str("[WireGuard]\n");
        content.push_str(format!("PrivateKey = {}\n", state.private_key).as_str());
        content.push_str(format!("ListenPort = {}\n\n", state.listen_port).as_str());

        for peer in &state.peers {
            content.push_str(format!("# Peer: {}\n", peer.hostname).as_str());
            content.push_str("[WireGuardPeer]\n");
            content.push_str(
                format!("Endpoint = {}:{}\n", peer.endpoint_host, peer.endpoint_port).as_str(),
            );
            content.push_str(format!("PublicKey = {}\n", peer.public_key).as_str());
            content.push_str(format!("PresharedKey = {}\n", peer.preshared_key).as_str());
            content.push_str(format!("AllowedIPs = {}\n", peer.allowed_ips.join(", ")).as_str());
            content.push_str(
                format!("PersistentKeepalive = {}\n\n", peer.persistent_keepalive).as_str(),
            );
        }

        content
    }

    pub fn get_interface_network_contents(&self, state: &NodeState) -> String {
        let mut content = String::new();

        content.push_str(PREAMBLE);

        content.push_str("[Match]\n");
        content.push_str(format!("Name = {}\n", self.config.interface).as_str());

        content.push_str("[Network]\n");
        content.push_str(format!("Address = {}\n\n", state.address).as_str());

        if state.route_allowed_ips {
            for peer in &state.peers {
                for allowed_ip in &peer.allowed_ips {
                    content.push_str(
                        format!("\n[Route]\nDestination = {}\nScope=link\n\n", allowed_ip).as_str(),
                    );
                }
            }
        }

        content
    }
}

#[async_trait]
impl Backend for SystemdBackend {
    async fn is_compatible(&self) -> bool {
        // ignore this check, used for limited integration testing
        if std::env::var("WGPULL_IGNORE_AVAILABILITY").unwrap_or("false".to_string()) == "true" {
            return true;
        }

        let command = SystemdCommand::new(self.executor.as_ref());

        // check if the systemd-networkd service is running and enabled
        if !command
            .service_is_running_and_enabled("systemd-networkd")
            .await
        {
            error!("systemd-networkd is not running or enabled");
            return false;
        }

        // make sure the network directory is present and writable (/etc/systemd/network)
        // the writable check is pretty inaccurate but it's better than nothing
        let network_dir = Path::new("/etc/systemd/network");
        if !network_dir.exists() {
            error!("systemd-networkd directory does not exist");
            return false;
        }

        if !network_dir.is_dir() {
            error!("systemd-networkd network directory is not a directory");
            return false;
        }

        true
    }

    async fn update_local_state(&self, state: &NodeState) -> Result<bool> {
        info!(
            "Node systemd backend update of local state, updating with {} peers",
            state.peers.len()
        );

        let netdev_contents = self.get_interface_netdev_contents(state);
        let netdev_path =
            Path::new(&self.config.path).join(format!("{}.netdev", self.config.interface));
        let has_netdev_changed = self
            .write_if_changed(
                netdev_path.into_os_string().into_string().unwrap().as_ref(),
                netdev_contents.as_ref(),
            )
            .await?;

        let network_contents = self.get_interface_network_contents(state);
        let network_path =
            Path::new(&self.config.path).join(format!("{}.network", self.config.interface));
        let has_network_changed = self
            .write_if_changed(
                network_path
                    .into_os_string()
                    .into_string()
                    .unwrap()
                    .as_ref(),
                network_contents.as_ref(),
            )
            .await?;

        if (has_netdev_changed || has_network_changed) && self.config.reload_networkd {
            let command = SystemdCommand::new(self.executor.as_ref());

            if self.config.delete_interface_before_reload {
                let _ = command
                    .delete_wireguard_interface(&self.config.interface)
                    .await;
            }
            command.networkd_reload().await?;
        }

        Ok(false)
    }

    async fn get_hostname(&self) -> Result<String> {
        let (stdout, _) = self.executor.execute_with_args("hostname", &[]).await?;

        Ok(stdout.trim().to_string())
    }
}
