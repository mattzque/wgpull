use anyhow::Result;
use std::str;

use crate::command::CommandExecutor;

pub struct PeerInfo {
    pub interface: String,
    pub public_key: String,
    pub private_key: String,
    pub endpoint: String,
    pub allowed_ips: String,
    pub latest_handshake: u64,
    pub transfer_rx: i64,
    pub transfer_tx: i64,
    pub persistent_keepalive: i64,
}

pub struct WireguardInfo {
    pub interface: String,
    pub public_key: String,
    pub private_key: String,
    pub listening_port: u16,
    pub peers: Vec<PeerInfo>,
}

pub struct KeyPair {
    pub public_key: String,
    pub private_key: String,
}

pub struct WireguardCommand<T: CommandExecutor> {
    executor: T,
}

impl<T: CommandExecutor> WireguardCommand<T> {
    pub fn new(executor: T) -> WireguardCommand<T> {
        Self { executor }
    }

    pub fn collect(&self) -> Result<Option<WireguardInfo>> {
        let (stdout, _) = self
            .executor
            .execute_with_args("wg", &["show", "all", "dump"])?;

        if stdout.lines().count() == 0 {
            return Ok(None);
        }

        let mut lines = stdout.lines();
        let interface_info = lines.next().unwrap();
        let interface_parts: Vec<&str> = interface_info.split_whitespace().collect();

        let mut peers = Vec::new();
        for line in lines {
            let parts: Vec<&str> = line.split_whitespace().collect();
            peers.push(PeerInfo {
                interface: parts[0].to_string(),
                public_key: parts[1].to_string(),
                private_key: parts[2].to_string(),
                endpoint: parts[3].to_string(),
                allowed_ips: parts[4].to_string(),
                latest_handshake: parts[5].parse()?,
                transfer_rx: parts[6].parse()?,
                transfer_tx: parts[7].parse()?,
                persistent_keepalive: parts[8].parse()?,
            });
        }

        Ok(Some(WireguardInfo {
            interface: interface_parts[0].to_string(),
            public_key: interface_parts[1].to_string(),
            private_key: interface_parts[2].to_string(),
            listening_port: interface_parts[3].parse()?,
            peers,
        }))
    }

    pub fn generate_key(&self) -> Result<String> {
        let (stdout, _) = self.executor.execute_with_args("wg", &["genkey"])?;
        Ok(stdout.trim().to_string())
    }

    pub fn generate_psk(&self) -> Result<String> {
        let (stdout, _) = self.executor.execute_with_args("wg", &["genpsk"])?;
        Ok(stdout.trim().to_string())
    }

    pub fn generate_pubkey(&self, private_key: String) -> Result<String> {
        let (stdout, _) =
            self.executor
                .execute_with_args_and_io("wg", &["pubkey"], &private_key)?;
        Ok(stdout.trim().to_string())
    }

    pub fn generate_keypair(&self) -> Result<KeyPair> {
        let private_key = self.generate_key()?;
        let public_key = self.generate_pubkey(private_key.clone())?;

        Ok(KeyPair {
            private_key,
            public_key,
        })
    }
}
