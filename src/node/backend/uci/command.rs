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

pub struct UciCommand<T: CommandExecutor> {
    executor: T,
}

impl<T: CommandExecutor> UciCommand<T> {
    pub fn new(executor: T) -> UciCommand<T> {
        Self { executor }
    }

    pub fn test_uci(&self) -> bool {
        self.executor
            .execute("uci")
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
    fn list_peer_sections(&self, interface: &str) -> Result<Vec<String>, UciError> {
        let (stdout, _) = self
            .executor
            .execute_with_args("uci", &["-X", "show"])
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

    fn get_value(&self, key: &str) -> Result<String, UciError> {
        let (value, _) = self.executor.execute_with_args("uci", &["get", key])?;
        let value = value.trim();
        debug!("uci value getter for key ({}) returned {}", key, value);
        Ok(value.to_string())
    }

    fn set_value(&self, key: &str, value: &str) -> Result<(), UciError> {
        let setter = format!("{}={}", key, value);
        self.executor.execute_with_args("uci", &["set", &setter])?;
        debug!("uci set key ({}) to value {}", key, value);
        Ok(())
    }

    fn new_network_section(&self, key: &str) -> Result<String, UciError> {
        let (value, _) = self
            .executor
            .execute_with_args("uci", &["add", "network", key])?;
        let value = value.trim();
        debug!(
            "uci new network section for key ({}) returned {}",
            key, value
        );
        Ok(value.to_string())
    }

    fn add_list_value(&self, key: &str, value: &str) -> Result<(), UciError> {
        let setter = format!("{}={}", key, value);
        self.executor
            .execute_with_args("uci", &["add_list", &setter])?;
        debug!("uci add list, for key ({}) add value {}", key, value);
        Ok(())
    }

    fn delete_key(&self, key: &str) -> Result<(), UciError> {
        self.executor.execute_with_args("uci", &["delete", key])?;
        debug!("uci delete key ({})", key);
        Ok(())
    }

    fn get_wireguard_peer_config(&self, key: &str) -> Result<UciWireguardPeer, UciError> {
        Ok(UciWireguardPeer {
            description: self.get_value(&format!("network.{}.description", key))?,
            public_key: self.get_value(&format!("network.{}.public_key", key))?,
            endpoint_host: self.get_value(&format!("network.{}.endpoint_host", key))?,
            endpoint_port: self
                .get_value(&format!("network.{}.endpoint_port", key))?
                .parse::<u32>()
                .map_err(|_| UciError::ParseError)?,
            persistent_keepalive: self
                .get_value(&format!("network.{}.persistent_keepalive", key))?
                .parse::<u32>()
                .map_err(|_| UciError::ParseError)?,
            route_allowed_ips: self.get_value(&format!("network.{}.route_allowed_ips", key))?
                == "1",
            allowed_ips: self
                .get_value(&format!("network.{}.allowed_ips", key))?
                .split(' ')
                .map(|s| s.to_string())
                .collect(),
        })
    }

    pub fn get_wireguard_config(&self, interface: &str) -> Result<UciWireguardConfig, UciError> {
        let peers: Result<Vec<UciWireguardPeer>, UciError> = self
            .list_peer_sections(interface)?
            .iter()
            .map(|key| self.get_wireguard_peer_config(key))
            .collect();
        let mut peers = peers?;
        peers.sort_by(|a, b| a.description.cmp(&b.description));
        Ok(UciWireguardConfig {
            private_key: self.get_value(&format!("network.{}.private_key", interface))?,
            listen_port: self
                .get_value(&format!("network.{}.listen_port", interface))?
                .parse::<u32>()
                .map_err(|_| UciError::ParseError)?,
            addresses: self.get_value(&format!("network.{}.addresses", interface))?,
            peers,
        })
    }

    pub fn update_wireguard_config(
        &self,
        interface: &str,
        config: &UciWireguardConfig,
    ) -> Result<(), UciError> {
        self.set_value(format!("network.{}", interface).as_str(), "interface")?;
        self.set_value(format!("network.{}.proto", interface).as_str(), "wireguard")?;
        self.set_value(
            format!("network.{}.private_key", interface).as_str(),
            &config.private_key,
        )?;
        self.set_value(
            format!("network.{}.listen_port", interface).as_str(),
            &config.listen_port.to_string(),
        )?;
        self.set_value(
            format!("network.{}.addresses", interface).as_str(),
            &config.addresses,
        )?;

        // delete all peer subsections first, its just way easier than merging them
        for peer_key in self.list_peer_sections(interface)? {
            self.delete_key(format!("network.{}", peer_key).as_str())?;
        }

        for peer in &config.peers {
            // create a new peer subsection:
            let section_key =
                self.new_network_section(format!("wireguard_{}", interface).as_str())?;

            self.set_value(
                format!("network.{}.public_key", section_key).as_str(),
                &peer.public_key,
            )?;
            self.set_value(
                format!("network.{}.description", section_key).as_str(),
                &peer.description,
            )?;
            self.set_value(
                format!("network.{}.endpoint_host", section_key).as_str(),
                &peer.endpoint_host,
            )?;
            self.set_value(
                format!("network.{}.endpoint_port", section_key).as_str(),
                &peer.endpoint_port.to_string(),
            )?;
            self.set_value(
                format!("network.{}.persistent_keepalive", section_key).as_str(),
                &peer.persistent_keepalive.to_string(),
            )?;
            self.set_value(
                format!("network.{}.route_allowed_ips", section_key).as_str(),
                if peer.route_allowed_ips { "1" } else { "0" },
            )?;

            for allowed_ip in &peer.allowed_ips {
                self.add_list_value(
                    format!("network.{}.allowed_ips", section_key).as_str(),
                    allowed_ip,
                )?;
            }
        }

        Ok(())
    }

    pub fn commit(&self, interface: &str) -> Result<(), UciError> {
        let _ = self.executor.execute_with_args("uci", &["commit"]);
        let _ = self.executor.execute_with_args("ifdown", &[interface]);
        let _ = self.executor.execute_with_args("ifup", &[interface]);
        Ok(())
    }

    pub fn get_hostname(&self) -> Result<String, UciError> {
        let (value, _) = self
            .executor
            .execute_with_args("uci", &["get", "system.@system[0].hostname"])?;
        Ok(value.trim().to_string())
    }
}
