use anyhow::Result;
use log::{debug, error};
use shared_lib::command::CommandExecutor;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UciError {
    #[error("Error running UCI command: {0}")]
    UciCommandFailed(String),
    #[error("Some UCI value could not be parsed!")]
    ParseError,
}

impl From<std::io::Error> for UciError {
    fn from(err: std::io::Error) -> Self {
        Self::UciCommandFailed(err.to_string())
    }
}

#[derive(Debug, PartialEq)]
pub struct UciWireguardPeer {
    pub description: String,
    pub public_key: String,
    pub endpoint_host: String,
    pub endpoint_port: u32,
    pub persistent_keepalive: u32,
    pub route_allowed_ips: bool,
    pub allowed_ips: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct UciWireguardConfig {
    pub private_key: String,
    pub listen_port: u32,
    pub addresses: String,
    pub peers: Vec<UciWireguardPeer>,
}

pub struct UciCommand<'a, T: CommandExecutor + ?Sized> {
    executor: &'a T,
}

impl<'a, T: CommandExecutor + ?Sized> UciCommand<'a, T> {
    pub fn new(executor: &'a T) -> UciCommand<'a, T> {
        Self { executor }
    }

    pub async fn test_uci(&self) -> bool {
        self.executor
            .execute("uci")
            .await
            .map(|(_, _)| true)
            .unwrap_or_else(|err| {
                error!("uci command failed: {}", err.to_string());
                false
            })
    }

    /// Find peer network sections in UCI,
    ///
    /// e.g. network.cfg1096fc=wireguard_wg0
    /// returns lists of keys like cfg1096fc, ...
    ///
    async fn list_peer_sections(&self, interface: &str) -> Result<Vec<String>, UciError> {
        let (stdout, _) = self
            .executor
            .execute_with_args("uci", &["-X", "show"])
            .await
            .map_err(|e| UciError::UciCommandFailed(e.to_string()))?;

        // find all network sections with wireguard_ prefix
        Ok(stdout
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('=').collect();
                if parts.len() == 2 {
                    let key_parts: Vec<&str> = parts[0].split('.').collect();
                    if key_parts.len() == 2
                        && parts[1] == format!("wireguard_{}", interface).as_str()
                    {
                        Some(key_parts[1].to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect())
    }

    async fn get_value(&self, key: &str) -> Result<String, UciError> {
        let (value, _) = self
            .executor
            .execute_with_args("uci", &["get", key])
            .await?;
        let value = value.trim();
        debug!("uci value getter for key ({}) returned {}", key, value);
        Ok(value.to_string())
    }

    async fn set_value(&self, key: &str, value: &str) -> Result<(), UciError> {
        let setter = format!("{}={}", key, value);
        self.executor
            .execute_with_args("uci", &["set", &setter])
            .await?;
        debug!("uci set key ({}) to value {}", key, value);
        Ok(())
    }

    async fn new_network_section(&self, key: &str) -> Result<String, UciError> {
        let (value, _) = self
            .executor
            .execute_with_args("uci", &["add", "network", key])
            .await?;
        let value = value.trim();
        debug!(
            "uci new network section for key ({}) returned {}",
            key, value
        );
        Ok(value.to_string())
    }

    async fn add_list_value(&self, key: &str, value: &str) -> Result<(), UciError> {
        let setter = format!("{}={}", key, value);
        self.executor
            .execute_with_args("uci", &["add_list", &setter])
            .await?;
        debug!("uci add list, for key ({}) add value {}", key, value);
        Ok(())
    }

    async fn delete_key(&self, key: &str) -> Result<(), UciError> {
        self.executor
            .execute_with_args("uci", &["delete", key])
            .await?;
        debug!("uci delete key ({})", key);
        Ok(())
    }

    async fn get_wireguard_peer_config(&self, key: &str) -> Result<UciWireguardPeer, UciError> {
        Ok(UciWireguardPeer {
            description: self
                .get_value(&format!("network.{}.description", key))
                .await?,
            public_key: self
                .get_value(&format!("network.{}.public_key", key))
                .await?,
            endpoint_host: self
                .get_value(&format!("network.{}.endpoint_host", key))
                .await?,
            endpoint_port: self
                .get_value(&format!("network.{}.endpoint_port", key))
                .await?
                .parse::<u32>()
                .map_err(|_| UciError::ParseError)?,
            persistent_keepalive: self
                .get_value(&format!("network.{}.persistent_keepalive", key))
                .await?
                .parse::<u32>()
                .map_err(|_| UciError::ParseError)?,
            route_allowed_ips: self
                .get_value(&format!("network.{}.route_allowed_ips", key))
                .await?
                == "1",
            allowed_ips: self
                .get_value(&format!("network.{}.allowed_ips", key))
                .await?
                .split(' ')
                .map(|s| s.to_string())
                .collect(),
        })
    }

    pub async fn get_wireguard_config(
        &self,
        interface: &str,
    ) -> Result<UciWireguardConfig, UciError> {
        let mut peers = Vec::new();
        for key in self.list_peer_sections(interface).await? {
            peers.push(self.get_wireguard_peer_config(&key).await?);
        }

        peers.sort_by(|a, b| a.description.cmp(&b.description));

        Ok(UciWireguardConfig {
            private_key: self
                .get_value(&format!("network.{}.private_key", interface))
                .await?,
            listen_port: self
                .get_value(&format!("network.{}.listen_port", interface))
                .await?
                .parse::<u32>()
                .map_err(|_| UciError::ParseError)?,
            addresses: self
                .get_value(&format!("network.{}.addresses", interface))
                .await?,
            peers,
        })
    }

    pub async fn update_wireguard_config(
        &self,
        interface: &str,
        config: &UciWireguardConfig,
    ) -> Result<(), UciError> {
        self.set_value(format!("network.{}", interface).as_str(), "interface")
            .await?;
        self.set_value(format!("network.{}.proto", interface).as_str(), "wireguard")
            .await?;
        self.set_value(
            format!("network.{}.private_key", interface).as_str(),
            &config.private_key,
        )
        .await?;
        self.set_value(
            format!("network.{}.listen_port", interface).as_str(),
            &config.listen_port.to_string(),
        )
        .await?;
        self.set_value(
            format!("network.{}.addresses", interface).as_str(),
            &config.addresses,
        )
        .await?;

        // delete all peer subsections first, its just way easier than merging them
        for peer_key in self.list_peer_sections(interface).await? {
            self.delete_key(format!("network.{}", peer_key).as_str())
                .await?;
        }

        for peer in &config.peers {
            // create a new peer subsection:
            let section_key = self
                .new_network_section(format!("wireguard_{}", interface).as_str())
                .await?;

            self.set_value(
                format!("network.{}.public_key", section_key).as_str(),
                &peer.public_key,
            )
            .await?;
            self.set_value(
                format!("network.{}.description", section_key).as_str(),
                &peer.description,
            )
            .await?;
            self.set_value(
                format!("network.{}.endpoint_host", section_key).as_str(),
                &peer.endpoint_host,
            )
            .await?;
            self.set_value(
                format!("network.{}.endpoint_port", section_key).as_str(),
                &peer.endpoint_port.to_string(),
            )
            .await?;
            self.set_value(
                format!("network.{}.persistent_keepalive", section_key).as_str(),
                &peer.persistent_keepalive.to_string(),
            )
            .await?;
            self.set_value(
                format!("network.{}.route_allowed_ips", section_key).as_str(),
                if peer.route_allowed_ips { "1" } else { "0" },
            )
            .await?;

            for allowed_ip in &peer.allowed_ips {
                self.add_list_value(
                    format!("network.{}.allowed_ips", section_key).as_str(),
                    allowed_ip,
                )
                .await?;
            }
        }

        Ok(())
    }

    pub async fn commit(&self, interface: &str) -> Result<(), UciError> {
        let _ = self.executor.execute_with_args("uci", &["commit"]).await;
        let _ = self
            .executor
            .execute_with_args("ifdown", &[interface])
            .await;
        let _ = self.executor.execute_with_args("ifup", &[interface]).await;
        Ok(())
    }

    pub async fn get_hostname(&self) -> Result<String, UciError> {
        let (value, _) = self
            .executor
            .execute_with_args("uci", &["get", "system.@system[0].hostname"])
            .await?;
        Ok(value.trim().to_string())
    }
}
