use anyhow::Result;
use std::str;

use crate::command::CommandExecutor;

#[derive(Debug)]
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

#[derive(Debug)]
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

pub struct WireguardCommand<'a, T: CommandExecutor + ?Sized> {
    executor: &'a T,
}

impl<'a, T: CommandExecutor + ?Sized> WireguardCommand<'a, T> {
    pub fn new(executor: &'a T) -> WireguardCommand<'a, T> {
        Self { executor }
    }

    pub async fn collect(&self) -> Result<Option<WireguardInfo>> {
        let (stdout, _) = self
            .executor
            .execute_with_args("wg", &["show", "all", "dump"])
            .await?;

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

    pub async fn generate_key(&self) -> Result<String> {
        let (stdout, _) = self.executor.execute_with_args("wg", &["genkey"]).await?;
        Ok(stdout.trim().to_string())
    }

    pub async fn generate_psk(&self) -> Result<String> {
        let (stdout, _) = self.executor.execute_with_args("wg", &["genpsk"]).await?;
        Ok(stdout.trim().to_string())
    }

    pub async fn generate_pubkey(&self, private_key: String) -> Result<String> {
        let (stdout, _) = self
            .executor
            .execute_with_args_and_io("wg", &["pubkey"], &private_key)
            .await?;
        Ok(stdout.trim().to_string())
    }

    pub async fn generate_keypair(&self) -> Result<KeyPair> {
        let private_key = self.generate_key().await?;
        let public_key = self.generate_pubkey(private_key.clone()).await?;

        Ok(KeyPair {
            private_key,
            public_key,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::validation::validate_wg_key;

    #[tokio::test]
    async fn test_generate_key() {
        use super::*;
        use crate::command::SystemCommandExecutor;
        let executor = SystemCommandExecutor;

        let command = WireguardCommand::new(&executor);
        let key = command.generate_key().await.unwrap();
        assert_eq!(key.len(), 44);
        validate_wg_key("", &key).unwrap();
    }

    #[tokio::test]
    async fn test_generate_psk() {
        use super::*;
        use crate::command::SystemCommandExecutor;
        let executor = SystemCommandExecutor;

        let command = WireguardCommand::new(&executor);
        let key = command.generate_psk().await.unwrap();
        assert_eq!(key.len(), 44);
        validate_wg_key("", &key).unwrap();
    }

    #[tokio::test]
    async fn test_generate_pubkey() {
        use super::*;
        use crate::command::SystemCommandExecutor;
        let executor = SystemCommandExecutor;

        let command = WireguardCommand::new(&executor);
        let key = command.generate_key().await.unwrap();
        let pubkey = command.generate_pubkey(key).await.unwrap();
        assert_eq!(pubkey.len(), 44);
        validate_wg_key("", &pubkey).unwrap();
    }
}
